use super::repository::RefundRepository;
use super::schema::CreateRefundRequest;
use rust_decimal::Decimal;
use rustbill_core::error::BillingError;
use std::str::FromStr;

fn value_string<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(serde_json::Value::as_str)
}

fn decimal_from_f64(value: f64) -> Decimal {
    Decimal::from_str(&value.to_string()).unwrap_or(Decimal::ZERO)
}

fn decimal_from_value(value: &serde_json::Value) -> Decimal {
    match value {
        serde_json::Value::String(value) => Decimal::from_str(value).unwrap_or(Decimal::ZERO),
        serde_json::Value::Number(value) => {
            Decimal::from_str(&value.to_string()).unwrap_or(Decimal::ZERO)
        }
        _ => Decimal::ZERO,
    }
}

pub async fn list_admin<R: RefundRepository>(
    repo: &R,
) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list_admin().await
}

pub async fn get_admin<R: RefundRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get_admin(id).await
}

pub async fn create_admin<R: RefundRepository>(
    repo: &R,
    body: &CreateRefundRequest,
) -> Result<serde_json::Value, BillingError> {
    let amount = body.amount.unwrap_or(0.0);
    let row = repo.create_admin(body, amount).await?;

    let should_emit = row
        .get("invoice_id")
        .and_then(serde_json::Value::as_str)
        .is_some()
        && row
            .get("payment_id")
            .and_then(serde_json::Value::as_str)
            .is_some()
        && row
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .is_some();
    if should_emit {
        let amount_dec = decimal_from_f64(amount);
        if let Err(err) = repo.emit_created_event(&row, amount_dec).await {
            let refund_id = value_string(&row, "id").unwrap_or_default();
            tracing::warn!(error = %err, refund_id, "failed to emit refund.created");
        }
    }

    Ok(row)
}

pub async fn update_admin<R: RefundRepository>(
    repo: &R,
    id: &str,
    body: &super::schema::UpdateRefundRequest,
) -> Result<serde_json::Value, BillingError> {
    let before = repo.get_admin(id).await?;
    let row = repo.update_admin(id, body.status.as_deref()).await?;

    let before_status = value_string(&before, "status").unwrap_or("pending");
    let after_status = value_string(&row, "status").unwrap_or("pending");
    if before_status != "completed" && after_status == "completed" {
        let amount_dec = decimal_from_value(&row["amount"]);
        if let Err(err) = repo.emit_completed_event(id, &row, amount_dec).await {
            tracing::warn!(error = %err, refund_id = %id, "failed to emit refund.completed");
        }
    }

    Ok(row)
}

pub async fn delete_admin<R: RefundRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let before = repo.get_admin(id).await?;
    let affected = repo.delete_admin(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("refund", id));
    }

    if matches!(value_string(&before, "status"), Some("completed")) {
        let amount_dec = decimal_from_value(&before["amount"]);
        let completed_event = repo.find_completed_event(id).await?;
        if let Err(err) = repo
            .emit_reversal_event(id, &before, amount_dec, completed_event)
            .await
        {
            tracing::warn!(error = %err, refund_id = %id, "failed to emit refund.reversal");
        }
    }

    Ok(serde_json::json!({ "success": true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::billing::refunds::repository::RefundRepository;
    use async_trait::async_trait;
    use rust_decimal::Decimal;
    use rustbill_core::error::BillingError;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;

    struct MockRefundRepository {
        current: serde_json::Value,
        next_row: serde_json::Value,
        completed_event: Option<(String, String)>,
        create_called: AtomicBool,
        emit_created_called: AtomicBool,
        emit_completed_called: AtomicBool,
        emit_reversal_called: AtomicBool,
        update_called: AtomicBool,
        delete_called: AtomicBool,
        find_completed_calls: AtomicUsize,
        last_create_amount: Mutex<Option<f64>>,
        last_update_status: Mutex<Option<Option<String>>>,
    }

    impl MockRefundRepository {
        fn sample_row(status: &str) -> serde_json::Value {
            serde_json::json!({
                "id": "refund-1",
                "payment_id": "pay-1",
                "invoice_id": "inv-1",
                "amount": 10.25,
                "reason": "duplicate charge",
                "status": status,
                "stripe_refund_id": null,
                "processed_at": null,
                "deleted_at": null,
                "created_at": "2026-01-01T00:00:00",
            })
        }

        fn new(before_status: &str, after_status: &str) -> Self {
            Self {
                current: Self::sample_row(before_status),
                next_row: Self::sample_row(after_status),
                completed_event: Some(("evt-1".to_string(), "refund.completed".to_string())),
                create_called: AtomicBool::new(false),
                emit_created_called: AtomicBool::new(false),
                emit_completed_called: AtomicBool::new(false),
                emit_reversal_called: AtomicBool::new(false),
                update_called: AtomicBool::new(false),
                delete_called: AtomicBool::new(false),
                find_completed_calls: AtomicUsize::new(0),
                last_create_amount: Mutex::new(None),
                last_update_status: Mutex::new(None),
            }
        }

        fn with_created_row(mut self, row: serde_json::Value) -> Self {
            self.next_row = row;
            self
        }
    }

    #[async_trait]
    impl RefundRepository for MockRefundRepository {
        async fn list_admin(&self) -> Result<Vec<serde_json::Value>, BillingError> {
            Ok(vec![self.current.clone()])
        }

        async fn get_admin(&self, _id: &str) -> Result<serde_json::Value, BillingError> {
            Ok(self.current.clone())
        }

        async fn create_admin(
            &self,
            _body: &CreateRefundRequest,
            amount: f64,
        ) -> Result<serde_json::Value, BillingError> {
            self.create_called.store(true, Ordering::SeqCst);
            if let Ok(mut guard) = self.last_create_amount.lock() {
                *guard = Some(amount);
            }
            Ok(self.next_row.clone())
        }

        async fn update_admin(
            &self,
            _id: &str,
            status: Option<&str>,
        ) -> Result<serde_json::Value, BillingError> {
            self.update_called.store(true, Ordering::SeqCst);
            if let Ok(mut guard) = self.last_update_status.lock() {
                *guard = Some(status.map(std::borrow::ToOwned::to_owned));
            }
            Ok(self.next_row.clone())
        }

        async fn delete_admin(&self, _id: &str) -> Result<u64, BillingError> {
            self.delete_called.store(true, Ordering::SeqCst);
            Ok(1)
        }

        async fn find_completed_event(
            &self,
            _id: &str,
        ) -> Result<Option<(String, String)>, BillingError> {
            self.find_completed_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.completed_event.clone())
        }

        async fn emit_created_event(
            &self,
            _row: &serde_json::Value,
            _amount: Decimal,
        ) -> Result<(), BillingError> {
            self.emit_created_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn emit_completed_event(
            &self,
            _id: &str,
            _row: &serde_json::Value,
            _amount: Decimal,
        ) -> Result<(), BillingError> {
            self.emit_completed_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn emit_reversal_event(
            &self,
            _id: &str,
            _before: &serde_json::Value,
            _amount: Decimal,
            _completed_event: Option<(String, String)>,
        ) -> Result<(), BillingError> {
            self.emit_reversal_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn create_admin_emits_sales_event_when_row_is_complete() {
        let repo = MockRefundRepository::new("pending", "pending");
        let body = CreateRefundRequest {
            payment_id: Some("pay-1".to_string()),
            invoice_id: Some("inv-1".to_string()),
            amount: Some(10.25),
            reason: Some("duplicate charge".to_string()),
            stripe_refund_id: None,
        };

        let row = create_admin(&repo, &body).await;
        assert!(row.is_ok());
        assert!(repo.create_called.load(Ordering::SeqCst));
        assert!(repo.emit_created_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn create_admin_skips_event_when_fields_are_missing() {
        let repo =
            MockRefundRepository::new("pending", "pending").with_created_row(serde_json::json!({
                "id": "refund-1",
                "payment_id": null,
                "invoice_id": "inv-1",
                "amount": 10.25,
                "reason": "duplicate charge",
                "status": "pending"
            }));
        let body = CreateRefundRequest {
            payment_id: None,
            invoice_id: Some("inv-1".to_string()),
            amount: Some(10.25),
            reason: Some("duplicate charge".to_string()),
            stripe_refund_id: None,
        };

        let row = create_admin(&repo, &body).await;
        assert!(row.is_ok());
        assert!(!repo.emit_created_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn update_admin_emits_completion_only_on_transition() {
        let repo = MockRefundRepository::new("pending", "completed");
        let body = super::super::schema::UpdateRefundRequest {
            status: Some("completed".to_string()),
        };

        let row = update_admin(&repo, "refund-1", &body).await;
        assert!(row.is_ok());
        assert!(repo.update_called.load(Ordering::SeqCst));
        assert!(repo.emit_completed_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn delete_admin_emits_reversal_for_completed_refund() {
        let repo = MockRefundRepository::new("completed", "completed");
        let row = delete_admin(&repo, "refund-1").await;
        assert!(row.is_ok());
        assert!(repo.delete_called.load(Ordering::SeqCst));
        assert_eq!(repo.find_completed_calls.load(Ordering::SeqCst), 1);
        assert!(repo.emit_reversal_called.load(Ordering::SeqCst));
    }
}
