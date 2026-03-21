use super::repository::{status_was_voided, InvoiceRepository};
use super::schema::{CreateInvoiceRequest, InvoiceItemInput, UpdateInvoiceRequest};
use rustbill_core::error::BillingError;

pub async fn list_admin<R: InvoiceRepository>(
    repo: &R,
) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list_admin().await
}

pub async fn get_admin<R: InvoiceRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get_admin(id).await
}

pub async fn create_admin<R: InvoiceRepository>(
    repo: &R,
    body: &CreateInvoiceRequest,
) -> Result<serde_json::Value, BillingError> {
    if body.customer_id.trim().is_empty() {
        return Err(BillingError::bad_request("customerId is required"));
    }

    let subtotal = body.normalized_subtotal();
    let total = body.normalized_total(subtotal);

    let row = repo.create_admin(body, subtotal, total).await?;
    if let Some(invoice_id) = row.get("id").and_then(serde_json::Value::as_str) {
        if let Err(err) = repo
            .emit_created_event(invoice_id, body, subtotal, total)
            .await
        {
            tracing::warn!(error = %err, invoice_id = %invoice_id, "failed to emit sales event invoice.created");
        }
    }

    Ok(row)
}

pub async fn update_admin<R: InvoiceRepository>(
    repo: &R,
    id: &str,
    body: &UpdateInvoiceRequest,
) -> Result<serde_json::Value, BillingError> {
    let before = repo.find_admin_invoice(id).await?;
    let row = repo.update_admin(id, body).await?;
    let after = repo.find_admin_invoice(id).await?;

    if status_was_voided(&before, &after) {
        if let Err(err) = repo.emit_void_reversal(&after, "invoice_update").await {
            tracing::warn!(error = %err, invoice_id = %after.id, "failed to emit invoice.reversal");
        }
    }

    Ok(row)
}

pub async fn delete_admin<R: InvoiceRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let invoice = repo.find_admin_invoice(id).await?;
    let affected = repo.delete_admin(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("invoice", id));
    }

    if let Err(err) = repo.emit_void_reversal(&invoice, "invoice_delete").await {
        tracing::warn!(error = %err, invoice_id = %invoice.id, "failed to emit invoice.reversal");
    }

    Ok(serde_json::json!({ "success": true }))
}

pub async fn list_v1<R: InvoiceRepository>(
    repo: &R,
    status: Option<&str>,
    customer_id: Option<&str>,
) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list_v1(status, customer_id).await
}

pub async fn get_v1<R: InvoiceRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get_v1(id).await
}

pub async fn list_items<R: InvoiceRepository>(
    repo: &R,
    invoice_id: &str,
) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list_items(invoice_id).await
}

pub async fn add_item<R: InvoiceRepository>(
    repo: &R,
    invoice_id: &str,
    body: &InvoiceItemInput,
) -> Result<serde_json::Value, BillingError> {
    repo.add_item(invoice_id, body).await
}

pub async fn get_pdf<R: InvoiceRepository>(
    repo: &R,
    invoice_id: &str,
) -> Result<(Vec<u8>, Option<String>), BillingError> {
    let pdf_bytes = repo.generate_pdf(invoice_id).await?;
    let invoice_number = repo.get_invoice_number(invoice_id).await?;
    Ok((pdf_bytes, invoice_number))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::billing::invoices::repository::InvoiceRepository;
    use async_trait::async_trait;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use rustbill_core::db::models::{Invoice, InvoiceStatus};
    use rustbill_core::error::BillingError;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex,
    };

    struct MockRepo {
        create_called: AtomicBool,
        emit_created_called: AtomicBool,
        emit_void_called: AtomicBool,
        update_called: AtomicBool,
        delete_called: AtomicBool,
        find_calls: AtomicUsize,
        created_subtotal: Mutex<Option<f64>>,
        created_total: Mutex<Option<f64>>,
        delete_rows: u64,
    }

    impl MockRepo {
        fn new() -> Self {
            Self {
                create_called: AtomicBool::new(false),
                emit_created_called: AtomicBool::new(false),
                emit_void_called: AtomicBool::new(false),
                update_called: AtomicBool::new(false),
                delete_called: AtomicBool::new(false),
                find_calls: AtomicUsize::new(0),
                created_subtotal: Mutex::new(None),
                created_total: Mutex::new(None),
                delete_rows: 1,
            }
        }

        fn invoice(status: InvoiceStatus) -> Invoice {
            Invoice {
                id: "inv-1".to_string(),
                invoice_number: "INV-00000001".to_string(),
                customer_id: "cust-1".to_string(),
                subscription_id: None,
                status,
                issued_at: None,
                due_at: None,
                paid_at: None,
                subtotal: Decimal::ZERO,
                tax: Decimal::ZERO,
                total: Decimal::ZERO,
                currency: "USD".to_string(),
                notes: None,
                stripe_invoice_id: None,
                xendit_invoice_id: None,
                lemonsqueezy_order_id: None,
                version: 1,
                deleted_at: None,
                created_at: Utc::now().naive_utc(),
                updated_at: Utc::now().naive_utc(),
                tax_name: None,
                tax_rate: None,
                tax_inclusive: false,
                credits_applied: Decimal::ZERO,
                amount_due: Decimal::ZERO,
                auto_charge_attempts: 0,
                idempotency_key: None,
            }
        }
    }

    #[async_trait]
    impl InvoiceRepository for MockRepo {
        async fn list_admin(&self) -> Result<Vec<serde_json::Value>, BillingError> {
            Ok(vec![])
        }

        async fn get_admin(&self, _id: &str) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "inv-1" }))
        }

        async fn create_admin(
            &self,
            _body: &CreateInvoiceRequest,
            subtotal: f64,
            total: f64,
        ) -> Result<serde_json::Value, BillingError> {
            self.create_called.store(true, Ordering::SeqCst);
            *self.created_subtotal.lock().unwrap() = Some(subtotal);
            *self.created_total.lock().unwrap() = Some(total);
            Ok(serde_json::json!({ "id": "inv-1" }))
        }

        async fn update_admin(
            &self,
            _id: &str,
            _body: &UpdateInvoiceRequest,
        ) -> Result<serde_json::Value, BillingError> {
            self.update_called.store(true, Ordering::SeqCst);
            Ok(serde_json::json!({ "id": "inv-1" }))
        }

        async fn find_admin_invoice(&self, _id: &str) -> Result<Invoice, BillingError> {
            let call = self.find_calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                Ok(Self::invoice(InvoiceStatus::Issued))
            } else {
                Ok(Self::invoice(InvoiceStatus::Void))
            }
        }

        async fn delete_admin(&self, _id: &str) -> Result<u64, BillingError> {
            self.delete_called.store(true, Ordering::SeqCst);
            Ok(self.delete_rows)
        }

        async fn add_item(
            &self,
            _invoice_id: &str,
            _body: &InvoiceItemInput,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "item-1" }))
        }

        async fn emit_created_event(
            &self,
            _invoice_id: &str,
            _body: &CreateInvoiceRequest,
            _subtotal: f64,
            _total: f64,
        ) -> Result<(), BillingError> {
            self.emit_created_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn emit_void_reversal(
            &self,
            _invoice: &Invoice,
            _trigger: &str,
        ) -> Result<(), BillingError> {
            self.emit_void_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn list_v1(
            &self,
            _status: Option<&str>,
            _customer_id: Option<&str>,
        ) -> Result<Vec<serde_json::Value>, BillingError> {
            Ok(vec![serde_json::json!({ "id": "inv-1" })])
        }

        async fn get_v1(&self, _id: &str) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "inv-1" }))
        }

        async fn list_items(
            &self,
            _invoice_id: &str,
        ) -> Result<Vec<serde_json::Value>, BillingError> {
            Ok(vec![])
        }

        async fn generate_pdf(&self, _invoice_id: &str) -> Result<Vec<u8>, BillingError> {
            Ok(vec![1, 2, 3])
        }

        async fn get_invoice_number(
            &self,
            _invoice_id: &str,
        ) -> Result<Option<String>, BillingError> {
            Ok(Some("INV-00000001".to_string()))
        }
    }

    #[tokio::test]
    async fn create_admin_rejects_blank_customer() {
        let repo = MockRepo::new();
        let body = CreateInvoiceRequest {
            customer_id: "   ".to_string(),
            subscription_id: None,
            status: None,
            currency: None,
            subtotal: Some(0.0),
            tax: Some(0.0),
            total: None,
            due_at: None,
            issued_at: None,
            notes: None,
            items: None,
        };

        let result = create_admin(&repo, &body).await;
        assert!(matches!(result, Err(BillingError::BadRequest(_))));
        assert!(!repo.create_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn create_admin_uses_item_subtotal_fallback() {
        let repo = MockRepo::new();
        let body = CreateInvoiceRequest {
            customer_id: "cust-1".to_string(),
            subscription_id: None,
            status: None,
            currency: Some("USD".to_string()),
            subtotal: Some(0.0),
            tax: Some(5.0),
            total: None,
            due_at: None,
            issued_at: None,
            notes: None,
            items: Some(vec![InvoiceItemInput {
                description: Some("Line item".to_string()),
                quantity: Some(2.0),
                unit_price: Some(10.0),
                amount: None,
                period_start: None,
                period_end: None,
            }]),
        };

        let result = create_admin(&repo, &body).await;
        assert!(result.is_ok());
        assert!(repo.create_called.load(Ordering::SeqCst));
        assert!(repo.emit_created_called.load(Ordering::SeqCst));
        assert_eq!(*repo.created_subtotal.lock().unwrap(), Some(20.0));
        assert_eq!(*repo.created_total.lock().unwrap(), Some(25.0));
    }

    #[tokio::test]
    async fn update_admin_emits_void_reversal() {
        let repo = MockRepo::new();
        let body = UpdateInvoiceRequest {
            status: Some("void".to_string()),
            notes: None,
            due_at: None,
        };

        let result = update_admin(&repo, "inv-1", &body).await;
        assert!(result.is_ok());
        assert!(repo.update_called.load(Ordering::SeqCst));
        assert!(repo.emit_void_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn delete_admin_returns_success_and_emits_void_reversal() {
        let repo = MockRepo::new();

        let result = delete_admin(&repo, "inv-1").await.unwrap();
        assert_eq!(result["success"], serde_json::json!(true));
        assert!(repo.delete_called.load(Ordering::SeqCst));
        assert!(repo.emit_void_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn list_v1_proxies_to_repository() {
        let repo = MockRepo::new();
        let rows = list_v1(&repo, Some("issued"), Some("cust-1"))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn get_pdf_returns_bytes_and_filename() {
        let repo = MockRepo::new();
        let (bytes, invoice_number) = get_pdf(&repo, "inv-1").await.unwrap();
        assert_eq!(bytes, vec![1, 2, 3]);
        assert_eq!(invoice_number.as_deref(), Some("INV-00000001"));
    }
}
