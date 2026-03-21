use super::repository::CreditsRepository;
use super::schema::{
    ApplyCreditRequest, CreditAdjustmentRequest, CreditBalanceRequest, ListCreditsRequest,
};
use crate::db::models::CustomerCredit;
use crate::error::{BillingError, Result};
use rust_decimal::Decimal;

pub async fn get_balance<R: CreditsRepository + ?Sized>(
    repo: &mut R,
    req: CreditBalanceRequest,
) -> Result<Decimal> {
    let balance = repo.get_balance(&req).await?.unwrap_or(Decimal::ZERO);
    Ok(balance)
}

pub async fn list_credits<R: CreditsRepository + ?Sized>(
    repo: &mut R,
    req: ListCreditsRequest,
) -> Result<Vec<CustomerCredit>> {
    repo.list_credits(&req).await
}

pub async fn adjust<R: CreditsRepository + ?Sized>(
    repo: &mut R,
    req: CreditAdjustmentRequest,
) -> Result<CustomerCredit> {
    if req.amount == Decimal::ZERO {
        return Err(BillingError::bad_request("adjust amount must be non-zero"));
    }

    repo.adjust_credit(&req).await
}

pub async fn deposit<R: CreditsRepository + ?Sized>(
    repo: &mut R,
    req: CreditAdjustmentRequest,
) -> Result<CustomerCredit> {
    adjust(repo, req).await
}

pub async fn adjust_in_tx<R: CreditsRepository + ?Sized>(
    repo: &mut R,
    req: CreditAdjustmentRequest,
) -> Result<CustomerCredit> {
    adjust(repo, req).await
}

pub async fn deposit_in_tx<R: CreditsRepository + ?Sized>(
    repo: &mut R,
    req: CreditAdjustmentRequest,
) -> Result<CustomerCredit> {
    if req.amount <= Decimal::ZERO {
        return Err(BillingError::bad_request("deposit amount must be positive"));
    }

    repo.deposit_credit(&req).await
}

pub async fn apply_to_invoice<R: CreditsRepository + ?Sized>(
    repo: &mut R,
    req: ApplyCreditRequest,
) -> Result<Decimal> {
    if req.max_amount <= Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    repo.apply_credit_to_invoice(&req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{CreditReason, CustomerCredit};
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        balance: Option<Decimal>,
        credits: Vec<CustomerCredit>,
        adjusted: Option<CreditAdjustmentRequest>,
        deposited: Option<CreditAdjustmentRequest>,
        applied: Option<ApplyCreditRequest>,
        adjustment_result: Option<CustomerCredit>,
        applied_amount: Decimal,
    }

    #[derive(Clone, Default)]
    struct StubRepo {
        state: Arc<Mutex<StubState>>,
    }

    impl StubRepo {
        fn with_state(state: StubState) -> Self {
            Self {
                state: Arc::new(Mutex::new(state)),
            }
        }
    }

    #[async_trait]
    impl CreditsRepository for StubRepo {
        async fn get_balance(&mut self, _req: &CreditBalanceRequest) -> Result<Option<Decimal>> {
            Ok(self.state.lock().expect("mutex").balance)
        }

        async fn list_credits(&mut self, _req: &ListCreditsRequest) -> Result<Vec<CustomerCredit>> {
            Ok(self.state.lock().expect("mutex").credits.clone())
        }

        async fn adjust_credit(&mut self, req: &CreditAdjustmentRequest) -> Result<CustomerCredit> {
            let mut state = self.state.lock().expect("mutex");
            state.adjusted = Some(req.clone());
            Ok(state.adjustment_result.clone().expect("adjustment_result"))
        }

        async fn deposit_credit(
            &mut self,
            req: &CreditAdjustmentRequest,
        ) -> Result<CustomerCredit> {
            let mut state = self.state.lock().expect("mutex");
            state.deposited = Some(req.clone());
            Ok(state.adjustment_result.clone().expect("adjustment_result"))
        }

        async fn apply_credit_to_invoice(&mut self, req: &ApplyCreditRequest) -> Result<Decimal> {
            let mut state = self.state.lock().expect("mutex");
            state.applied = Some(req.clone());
            Ok(state.applied_amount)
        }
    }

    fn sample_credit(amount: Decimal) -> CustomerCredit {
        CustomerCredit {
            id: "cred_1".to_string(),
            customer_id: "cust_1".to_string(),
            currency: "USD".to_string(),
            amount,
            balance_after: Decimal::from(100),
            reason: CreditReason::Manual,
            description: "test".to_string(),
            invoice_id: None,
            created_at: Utc::now().naive_utc(),
        }
    }

    #[tokio::test]
    async fn get_balance_defaults_to_zero() {
        let repo = StubRepo::with_state(StubState::default());
        let mut repo = repo;
        let balance = get_balance(
            &mut repo,
            CreditBalanceRequest {
                customer_id: "cust_1".to_string(),
                currency: "USD".to_string(),
            },
        )
        .await
        .expect("get_balance");
        assert_eq!(balance, Decimal::ZERO);
    }

    #[tokio::test]
    async fn adjust_rejects_zero_amount() {
        let repo = StubRepo::with_state(StubState::default());
        let mut repo = repo;
        let err = adjust(
            &mut repo,
            CreditAdjustmentRequest {
                customer_id: "cust_1".to_string(),
                currency: "USD".to_string(),
                amount: Decimal::ZERO,
                reason: CreditReason::Manual,
                description: "test".to_string(),
                invoice_id: None,
            },
        )
        .await
        .expect_err("should fail");
        assert!(err.to_string().contains("non-zero"));
    }

    #[tokio::test]
    async fn deposit_aliases_adjust_and_list_forwards_to_repository() {
        let repo = StubRepo::with_state(StubState {
            balance: Some(Decimal::from(25)),
            credits: vec![sample_credit(Decimal::from(10))],
            adjustment_result: Some(sample_credit(Decimal::from(10))),
            ..StubState::default()
        });
        let mut repo = repo;

        let credit = deposit(
            &mut repo,
            CreditAdjustmentRequest {
                customer_id: "cust_1".to_string(),
                currency: "USD".to_string(),
                amount: Decimal::from(10),
                reason: CreditReason::Manual,
                description: "deposit".to_string(),
                invoice_id: Some("inv_1".to_string()),
            },
        )
        .await
        .expect("deposit");
        assert_eq!(credit.id, "cred_1");

        let rows = list_credits(
            &mut repo,
            ListCreditsRequest {
                customer_id: "cust_1".to_string(),
                currency: Some("USD".to_string()),
            },
        )
        .await
        .expect("list_credits");
        assert_eq!(rows.len(), 1);

        let state = repo.state.lock().expect("mutex");
        assert!(state.adjusted.is_some());
        assert!(state.deposited.is_none());
    }

    #[tokio::test]
    async fn deposit_in_tx_forwards_to_deposit_repository_method() {
        let repo = StubRepo::with_state(StubState {
            adjustment_result: Some(sample_credit(Decimal::from(10))),
            ..StubState::default()
        });
        let mut repo = repo;

        let credit = deposit_in_tx(
            &mut repo,
            CreditAdjustmentRequest {
                customer_id: "cust_1".to_string(),
                currency: "USD".to_string(),
                amount: Decimal::from(10),
                reason: CreditReason::Manual,
                description: "deposit".to_string(),
                invoice_id: Some("inv_1".to_string()),
            },
        )
        .await
        .expect("deposit_in_tx");
        assert_eq!(credit.id, "cred_1");

        let state = repo.state.lock().expect("mutex");
        assert!(state.deposited.is_some());
    }

    #[tokio::test]
    async fn deposit_in_tx_rejects_non_positive_amount() {
        let repo = StubRepo::with_state(StubState::default());
        let mut repo = repo;
        let err = deposit_in_tx(
            &mut repo,
            CreditAdjustmentRequest {
                customer_id: "cust_1".to_string(),
                currency: "USD".to_string(),
                amount: Decimal::ZERO,
                reason: CreditReason::Manual,
                description: "deposit".to_string(),
                invoice_id: None,
            },
        )
        .await
        .expect_err("should fail");
        assert!(err.to_string().contains("positive"));
    }

    #[tokio::test]
    async fn apply_to_invoice_returns_zero_when_max_is_non_positive() {
        let repo = StubRepo::with_state(StubState::default());
        let mut repo = repo;
        let applied = apply_to_invoice(
            &mut repo,
            ApplyCreditRequest {
                customer_id: "cust_1".to_string(),
                invoice_id: "inv_1".to_string(),
                currency: "USD".to_string(),
                max_amount: Decimal::ZERO,
            },
        )
        .await
        .expect("apply_to_invoice");
        assert_eq!(applied, Decimal::ZERO);
    }
}
