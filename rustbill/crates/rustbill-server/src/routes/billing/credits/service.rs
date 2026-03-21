use super::repository::CreditsRepository;
use super::schema::{AdjustRequest, AdjustUpdateRequest};
use rust_decimal::Decimal;
use rustbill_core::db::models::{CreditReason, CustomerCredit};
use rustbill_core::error::BillingError;

pub async fn adjust<R: CreditsRepository>(
    repo: &R,
    body: &AdjustRequest,
) -> Result<CustomerCredit, BillingError> {
    repo.adjust(
        &body.customer_id,
        &body.currency,
        body.amount,
        CreditReason::Manual,
        &body.description,
        None,
    )
    .await
}

pub async fn update_adjustment<R: CreditsRepository>(
    repo: &R,
    id: &str,
    body: &AdjustUpdateRequest,
) -> Result<CustomerCredit, BillingError> {
    if body.amount <= Decimal::ZERO {
        return Err(BillingError::bad_request("amount must be positive"));
    }

    let existing = repo.find_adjustment(id).await?;
    if existing.reason != CreditReason::Manual || existing.invoice_id.is_some() {
        return Err(BillingError::bad_request(
            "only manual adjustments can be edited",
        ));
    }

    let delta = body.amount - existing.amount;
    if delta == Decimal::ZERO {
        return Ok(existing);
    }

    let description = match body.description.clone() {
        Some(description) => description,
        None => format!("Adjusted entry {}", existing.id),
    };

    repo.adjust(
        &existing.customer_id,
        &existing.currency,
        delta,
        CreditReason::Manual,
        &description,
        existing.invoice_id.as_deref(),
    )
    .await
}

pub async fn delete_adjustment<R: CreditsRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let existing = repo.find_adjustment(id).await?;
    if existing.reason != CreditReason::Manual || existing.invoice_id.is_some() {
        return Err(BillingError::bad_request(
            "only manual adjustments can be deleted",
        ));
    }

    repo.adjust(
        &existing.customer_id,
        &existing.currency,
        -existing.amount,
        CreditReason::Manual,
        &format!("Reversal of entry {}", existing.id),
        existing.invoice_id.as_deref(),
    )
    .await?;

    Ok(serde_json::json!({ "success": true }))
}

pub async fn get_customer_credits<R: CreditsRepository>(
    repo: &R,
    customer_id: &str,
    currency: Option<&str>,
) -> Result<serde_json::Value, BillingError> {
    let currency_value = currency.unwrap_or("USD");

    let balance = repo.get_balance(customer_id, currency_value).await?;
    let history = repo.list_credits(customer_id, currency).await?;

    Ok(serde_json::json!({
        "balance": balance,
        "currency": currency_value,
        "history": history
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::billing::credits::repository::CreditsRepository;
    use async_trait::async_trait;
    use rust_decimal::Decimal;
    use rustbill_core::db::models::{CreditReason, CustomerCredit};
    use rustbill_core::error::BillingError;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    type LastAdjust = (String, String, Decimal, CreditReason, Option<String>);

    struct MockCreditsRepository {
        existing: CustomerCredit,
        adjust_calls: AtomicUsize,
        last_adjust: Mutex<Option<LastAdjust>>,
        balance_currency: Mutex<Option<String>>,
        history_currency: Mutex<Option<Option<String>>>,
    }

    impl MockCreditsRepository {
        fn new(existing: CustomerCredit) -> Self {
            Self {
                existing,
                adjust_calls: AtomicUsize::new(0),
                last_adjust: Mutex::new(None),
                balance_currency: Mutex::new(None),
                history_currency: Mutex::new(None),
            }
        }

        fn adjustment_count(&self) -> usize {
            self.adjust_calls.load(Ordering::SeqCst)
        }
    }

    fn sample_credit(
        id: &str,
        amount: Decimal,
        reason: CreditReason,
        invoice_id: Option<&str>,
    ) -> CustomerCredit {
        CustomerCredit {
            id: id.to_string(),
            customer_id: "cust-1".to_string(),
            currency: "USD".to_string(),
            amount,
            balance_after: Decimal::from(25),
            reason,
            description: "Existing adjustment".to_string(),
            invoice_id: invoice_id.map(|value| value.to_string()),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }

    #[async_trait]
    impl CreditsRepository for MockCreditsRepository {
        async fn adjust(
            &self,
            customer_id: &str,
            currency: &str,
            amount: Decimal,
            reason: CreditReason,
            description: &str,
            invoice_id: Option<&str>,
        ) -> Result<CustomerCredit, BillingError> {
            self.adjust_calls.fetch_add(1, Ordering::SeqCst);
            let mut last_adjust = self.last_adjust.lock().expect("mutex poisoned");
            *last_adjust = Some((
                customer_id.to_string(),
                currency.to_string(),
                amount,
                reason.clone(),
                invoice_id.map(|value| value.to_string()),
            ));

            let mut row = self.existing.clone();
            row.customer_id = customer_id.to_string();
            row.currency = currency.to_string();
            row.amount = amount;
            row.reason = reason;
            row.description = description.to_string();
            row.invoice_id = invoice_id.map(|value| value.to_string());
            Ok(row)
        }

        async fn find_adjustment(&self, _id: &str) -> Result<CustomerCredit, BillingError> {
            Ok(self.existing.clone())
        }

        async fn get_balance(
            &self,
            _customer_id: &str,
            currency: &str,
        ) -> Result<Decimal, BillingError> {
            let mut guard = self.balance_currency.lock().expect("mutex poisoned");
            *guard = Some(currency.to_string());
            Ok(Decimal::from(42))
        }

        async fn list_credits(
            &self,
            _customer_id: &str,
            currency: Option<&str>,
        ) -> Result<Vec<CustomerCredit>, BillingError> {
            let mut guard = self.history_currency.lock().expect("mutex poisoned");
            *guard = Some(currency.map(|value| value.to_string()));
            Ok(vec![self.existing.clone()])
        }
    }

    #[tokio::test]
    async fn update_adjustment_keeps_existing_when_amount_is_unchanged() {
        let repo = MockCreditsRepository::new(sample_credit(
            "adj-1",
            Decimal::from(10),
            CreditReason::Manual,
            None,
        ));
        let body = AdjustUpdateRequest {
            amount: Decimal::from(10),
            description: Some("ignored".to_string()),
        };

        let result = update_adjustment(&repo, "adj-1", &body).await.unwrap();
        assert_eq!(result.id, "adj-1");
        assert_eq!(repo.adjustment_count(), 0);
    }

    #[tokio::test]
    async fn delete_adjustment_reverses_manual_credit() {
        let repo = MockCreditsRepository::new(sample_credit(
            "adj-2",
            Decimal::from(7),
            CreditReason::Manual,
            None,
        ));

        let result = delete_adjustment(&repo, "adj-2").await.unwrap();
        assert_eq!(result["success"], serde_json::json!(true));
        assert_eq!(repo.adjustment_count(), 1);

        let last_adjust = repo.last_adjust.lock().expect("mutex poisoned");
        let (customer_id, currency, amount, reason, invoice_id) = last_adjust
            .as_ref()
            .expect("adjust call should be captured");
        assert_eq!(customer_id, "cust-1");
        assert_eq!(currency, "USD");
        assert_eq!(amount, &Decimal::from(-7));
        assert_eq!(reason, &CreditReason::Manual);
        assert_eq!(invoice_id, &None);
    }

    #[tokio::test]
    async fn get_customer_credits_defaults_to_usd_for_balance() {
        let repo = MockCreditsRepository::new(sample_credit(
            "adj-3",
            Decimal::from(3),
            CreditReason::Manual,
            None,
        ));

        let result = get_customer_credits(&repo, "cust-1", None).await.unwrap();
        assert_eq!(result["currency"], serde_json::json!("USD"));

        let balance_currency = repo.balance_currency.lock().expect("mutex poisoned");
        assert_eq!(balance_currency.as_deref(), Some("USD"));

        let history_currency = repo.history_currency.lock().expect("mutex poisoned");
        assert_eq!(
            history_currency.as_ref().and_then(|value| value.as_deref()),
            None
        );
    }
}
