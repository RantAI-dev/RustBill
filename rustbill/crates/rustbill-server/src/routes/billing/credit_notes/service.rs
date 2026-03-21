use super::repository::CreditNoteRepository;
use super::schema::CreateCreditNoteRequest;
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

pub async fn list<R: CreditNoteRepository>(
    repo: &R,
    role_customer_id: Option<&str>,
) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list_admin(role_customer_id).await
}

pub async fn get_admin<R: CreditNoteRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get_admin(id).await
}

pub async fn create_admin<R: CreditNoteRepository>(
    repo: &R,
    body: &CreateCreditNoteRequest,
) -> Result<serde_json::Value, BillingError> {
    let amount = body.amount.unwrap_or(0.0);
    let row = repo.create_admin(body, amount).await?;

    let should_emit = row
        .get("invoice_id")
        .and_then(serde_json::Value::as_str)
        .is_some()
        && row
            .get("customer_id")
            .and_then(serde_json::Value::as_str)
            .is_some()
        && row
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .is_some();
    if should_emit {
        let amount_dec = decimal_from_f64(amount);
        if let Err(err) = repo.emit_created_event(&row, amount_dec).await {
            let credit_note_id = value_string(&row, "id").unwrap_or_default();
            tracing::warn!(error = %err, credit_note_id, "failed to emit credit_note.created");
        }
    }

    Ok(row)
}

pub async fn update_admin<R: CreditNoteRepository>(
    repo: &R,
    id: &str,
    body: &super::schema::UpdateCreditNoteRequest,
) -> Result<serde_json::Value, BillingError> {
    let before = repo.get_admin(id).await?;
    let row = repo.update_admin(id, body.status.as_deref()).await?;

    let before_status = value_string(&before, "status").unwrap_or("draft");
    let after_status = value_string(&row, "status").unwrap_or("draft");
    if before_status != "issued" && after_status == "issued" {
        let amount_dec = decimal_from_value(&row["amount"]);
        if let Err(err) = repo.emit_issued_event(id, &row, amount_dec).await {
            tracing::warn!(error = %err, credit_note_id = %id, "failed to emit credit_note.issued");
        }
    }

    Ok(row)
}

pub async fn delete_admin<R: CreditNoteRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let before = repo.get_admin(id).await?;
    let affected = repo.delete_admin(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("credit_note", id));
    }

    let prior_event = repo.find_prior_event(id).await?;
    let amount_dec = decimal_from_value(&before["amount"]);
    if let Err(err) = repo
        .emit_reversal_event(id, &before, amount_dec, prior_event)
        .await
    {
        tracing::warn!(error = %err, credit_note_id = %id, "failed to emit credit_note.reversal");
    }

    Ok(serde_json::json!({ "success": true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::billing::credit_notes::repository::CreditNoteRepository;
    use async_trait::async_trait;
    use rust_decimal::Decimal;
    use rustbill_core::error::BillingError;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;

    struct MockCreditNoteRepository {
        current: serde_json::Value,
        next_row: serde_json::Value,
        prior_event: Option<(String, String)>,
        create_called: AtomicBool,
        emit_created_called: AtomicBool,
        emit_issued_called: AtomicBool,
        emit_reversal_called: AtomicBool,
        update_called: AtomicBool,
        delete_called: AtomicBool,
        find_prior_calls: AtomicUsize,
        last_create_amount: Mutex<Option<f64>>,
        last_update_status: Mutex<Option<Option<String>>>,
        list_customer_id: Mutex<Option<Option<String>>>,
    }

    impl MockCreditNoteRepository {
        fn sample_row(status: &str) -> serde_json::Value {
            serde_json::json!({
                "id": "cn-1",
                "credit_note_number": "CN-00000000000001",
                "invoice_id": "inv-1",
                "customer_id": "cust-1",
                "amount": 25.50,
                "reason": "billing correction",
                "status": status,
                "issued_at": null,
                "deleted_at": null,
                "created_at": "2026-01-01T00:00:00",
                "updated_at": "2026-01-01T00:00:00",
            })
        }

        fn new(before_status: &str, after_status: &str) -> Self {
            Self {
                current: Self::sample_row(before_status),
                next_row: Self::sample_row(after_status),
                prior_event: Some(("evt-2".to_string(), "credit_note.issued".to_string())),
                create_called: AtomicBool::new(false),
                emit_created_called: AtomicBool::new(false),
                emit_issued_called: AtomicBool::new(false),
                emit_reversal_called: AtomicBool::new(false),
                update_called: AtomicBool::new(false),
                delete_called: AtomicBool::new(false),
                find_prior_calls: AtomicUsize::new(0),
                last_create_amount: Mutex::new(None),
                last_update_status: Mutex::new(None),
                list_customer_id: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl CreditNoteRepository for MockCreditNoteRepository {
        async fn list_admin(
            &self,
            role_customer_id: Option<&str>,
        ) -> Result<Vec<serde_json::Value>, BillingError> {
            if let Ok(mut guard) = self.list_customer_id.lock() {
                *guard = Some(role_customer_id.map(std::borrow::ToOwned::to_owned));
            }
            Ok(vec![self.current.clone()])
        }

        async fn get_admin(&self, _id: &str) -> Result<serde_json::Value, BillingError> {
            Ok(self.current.clone())
        }

        async fn create_admin(
            &self,
            _body: &CreateCreditNoteRequest,
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

        async fn find_prior_event(
            &self,
            _id: &str,
        ) -> Result<Option<(String, String)>, BillingError> {
            self.find_prior_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.prior_event.clone())
        }

        async fn emit_created_event(
            &self,
            _row: &serde_json::Value,
            _amount: Decimal,
        ) -> Result<(), BillingError> {
            self.emit_created_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn emit_issued_event(
            &self,
            _id: &str,
            _row: &serde_json::Value,
            _amount: Decimal,
        ) -> Result<(), BillingError> {
            self.emit_issued_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn emit_reversal_event(
            &self,
            _id: &str,
            _before: &serde_json::Value,
            _amount: Decimal,
            _prior_event: Option<(String, String)>,
        ) -> Result<(), BillingError> {
            self.emit_reversal_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn create_admin_emits_sales_event_when_row_is_complete() {
        let repo = MockCreditNoteRepository::new("draft", "draft");
        let body = CreateCreditNoteRequest {
            invoice_id: Some("inv-1".to_string()),
            customer_id: Some("cust-1".to_string()),
            amount: Some(25.50),
            reason: Some("billing correction".to_string()),
        };

        let row = create_admin(&repo, &body).await;
        assert!(row.is_ok());
        assert!(repo.create_called.load(Ordering::SeqCst));
        assert!(repo.emit_created_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn update_admin_emits_issue_only_on_transition() {
        let repo = MockCreditNoteRepository::new("draft", "issued");
        let body = super::super::schema::UpdateCreditNoteRequest {
            status: Some("issued".to_string()),
        };

        let row = update_admin(&repo, "cn-1", &body).await;
        assert!(row.is_ok());
        assert!(repo.update_called.load(Ordering::SeqCst));
        assert!(repo.emit_issued_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn delete_admin_emits_reversal_and_uses_prior_event_lookup() {
        let repo = MockCreditNoteRepository::new("issued", "issued");
        let row = delete_admin(&repo, "cn-1").await;
        assert!(row.is_ok());
        assert!(repo.delete_called.load(Ordering::SeqCst));
        assert_eq!(repo.find_prior_calls.load(Ordering::SeqCst), 1);
        assert!(repo.emit_reversal_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn list_passes_customer_filter_through() {
        let repo = MockCreditNoteRepository::new("draft", "draft");
        let rows = list(&repo, Some("cust-1")).await;
        assert!(rows.is_ok());
        let filter = repo
            .list_customer_id
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        assert_eq!(filter, Some(Some("cust-1".to_string())));
    }
}
