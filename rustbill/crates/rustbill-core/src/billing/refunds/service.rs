use super::repository::RefundRepository;
use super::schema::{CreateRefundRequest, ListRefundsFilter};
use crate::db::models::{Refund, RefundStatus};
use crate::error::{BillingError, Result};
use chrono::Utc;
use validator::Validate;

pub async fn list_refunds<R: RefundRepository + ?Sized>(
    repo: &R,
    filter: &ListRefundsFilter,
) -> Result<Vec<Refund>> {
    repo.list_refunds(filter).await
}

pub async fn create_refund<R: RefundRepository + ?Sized>(
    repo: &R,
    req: CreateRefundRequest,
) -> Result<Refund> {
    req.validate().map_err(BillingError::from_validation)?;

    let payment = repo
        .find_payment(&req.payment_id)
        .await?
        .ok_or_else(|| BillingError::not_found("payment", &req.payment_id))?;

    let existing_refunds = repo
        .non_failed_refund_total_for_payment(&req.payment_id)
        .await?;

    let total_after = existing_refunds + req.amount;
    if total_after > payment.amount {
        return Err(BillingError::bad_request(format!(
            "refund total ({total_after}) would exceed payment amount ({})",
            payment.amount
        )));
    }

    let status = req.status.clone().unwrap_or(RefundStatus::Pending);
    let processed_at = if status == RefundStatus::Completed {
        Some(Utc::now().naive_utc())
    } else {
        None
    };

    let refund = repo
        .create_refund_with_side_effects(&req, status.clone(), processed_at)
        .await?;

    if status == RefundStatus::Completed {
        if let Err(err) = repo.emit_completed_refund_event(&req, &refund).await {
            tracing::warn!(
                error = %err,
                refund_id = %refund.id,
                "failed to emit sales event refund.completed"
            );
        }
    }

    Ok(refund)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{Payment, PaymentMethod};
    use async_trait::async_trait;
    use chrono::NaiveDateTime;
    use rust_decimal::Decimal;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        payment: Option<Payment>,
        existing_refunds: Decimal,
        created_refund: Option<Refund>,
        emit_event_err: Option<String>,
        created_status: Option<RefundStatus>,
        created_processed_at: Option<NaiveDateTime>,
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
    impl RefundRepository for StubRepo {
        async fn list_refunds(&self, _filter: &ListRefundsFilter) -> Result<Vec<Refund>> {
            Ok(Vec::new())
        }

        async fn find_payment(&self, _payment_id: &str) -> Result<Option<Payment>> {
            Ok(self.state.lock().expect("mutex").payment.clone())
        }

        async fn non_failed_refund_total_for_payment(&self, _payment_id: &str) -> Result<Decimal> {
            Ok(self.state.lock().expect("mutex").existing_refunds)
        }

        async fn create_refund_with_side_effects(
            &self,
            _req: &CreateRefundRequest,
            status: RefundStatus,
            processed_at: Option<NaiveDateTime>,
        ) -> Result<Refund> {
            let mut state = self.state.lock().expect("mutex");
            state.created_status = Some(status);
            state.created_processed_at = processed_at;
            Ok(state.created_refund.clone().expect("created_refund"))
        }

        async fn emit_completed_refund_event(
            &self,
            _req: &CreateRefundRequest,
            _refund: &Refund,
        ) -> Result<()> {
            let state = self.state.lock().expect("mutex");
            if let Some(msg) = &state.emit_event_err {
                return Err(BillingError::bad_request(msg.clone()));
            }
            Ok(())
        }
    }

    fn sample_payment(amount: Decimal) -> Payment {
        Payment {
            id: "pay_1".to_string(),
            invoice_id: "inv_1".to_string(),
            amount,
            method: PaymentMethod::Stripe,
            reference: None,
            paid_at: Utc::now().naive_utc(),
            notes: None,
            stripe_payment_intent_id: None,
            xendit_payment_id: None,
            lemonsqueezy_order_id: None,
            created_at: Utc::now().naive_utc(),
        }
    }

    fn sample_refund() -> Refund {
        Refund {
            id: "ref_1".to_string(),
            payment_id: "pay_1".to_string(),
            invoice_id: "inv_1".to_string(),
            amount: Decimal::from(10),
            reason: "requested".to_string(),
            status: RefundStatus::Pending,
            stripe_refund_id: None,
            processed_at: None,
            created_at: Utc::now().naive_utc(),
            deleted_at: None,
        }
    }

    fn sample_request(amount: Decimal) -> CreateRefundRequest {
        CreateRefundRequest {
            payment_id: "pay_1".to_string(),
            invoice_id: "inv_1".to_string(),
            amount,
            reason: "requested".to_string(),
            status: None,
            stripe_refund_id: None,
        }
    }

    #[tokio::test]
    async fn create_refund_happy_path_defaults_to_pending() {
        let repo = StubRepo::with_state(StubState {
            payment: Some(sample_payment(Decimal::from(100))),
            existing_refunds: Decimal::from(20),
            created_refund: Some(sample_refund()),
            ..StubState::default()
        });

        let result = create_refund(&repo, sample_request(Decimal::from(10)))
            .await
            .expect("create_refund");

        let state = repo.state.lock().expect("mutex");
        assert_eq!(result.id, "ref_1");
        assert_eq!(state.created_status, Some(RefundStatus::Pending));
        assert!(state.created_processed_at.is_none());
    }

    #[tokio::test]
    async fn create_refund_rejects_when_total_exceeds_payment() {
        let repo = StubRepo::with_state(StubState {
            payment: Some(sample_payment(Decimal::from(25))),
            existing_refunds: Decimal::from(20),
            created_refund: Some(sample_refund()),
            ..StubState::default()
        });

        let err = create_refund(&repo, sample_request(Decimal::from(10)))
            .await
            .expect_err("should fail");
        assert!(err.to_string().contains("would exceed payment amount"));
    }

    #[tokio::test]
    async fn create_refund_returns_not_found_when_payment_missing() {
        let repo = StubRepo::with_state(StubState {
            payment: None,
            existing_refunds: Decimal::ZERO,
            created_refund: Some(sample_refund()),
            ..StubState::default()
        });

        let err = create_refund(&repo, sample_request(Decimal::from(10)))
            .await
            .expect_err("should fail");
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn create_refund_completed_event_failure_is_non_fatal() {
        let repo = StubRepo::with_state(StubState {
            payment: Some(sample_payment(Decimal::from(100))),
            existing_refunds: Decimal::ZERO,
            created_refund: Some(sample_refund()),
            emit_event_err: Some("emit failed".to_string()),
            ..StubState::default()
        });

        let mut req = sample_request(Decimal::from(10));
        req.status = Some(RefundStatus::Completed);

        let result = create_refund(&repo, req).await.expect("create_refund");
        let state = repo.state.lock().expect("mutex");

        assert_eq!(result.id, "ref_1");
        assert_eq!(state.created_status, Some(RefundStatus::Completed));
        assert!(state.created_processed_at.is_some());
    }
}
