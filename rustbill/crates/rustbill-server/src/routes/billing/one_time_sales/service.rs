use super::repository::{status_was_voided, OneTimeSalesRepository};
use super::schema::{CreateOneTimeSaleRequest, UpdateOneTimeSaleRequest};
use rustbill_core::error::BillingError;

pub async fn list_admin<R: OneTimeSalesRepository>(
    repo: &R,
) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list_admin().await
}

pub async fn get_admin<R: OneTimeSalesRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get_admin(id).await
}

pub async fn create_admin<R: OneTimeSalesRepository>(
    repo: &R,
    body: &CreateOneTimeSaleRequest,
) -> Result<serde_json::Value, BillingError> {
    let customer_id = body.customer_id.trim();
    if customer_id.is_empty() {
        return Err(BillingError::bad_request("customerId is required"));
    }

    let subtotal = body.normalized_subtotal();
    let total = body.normalized_total(subtotal);

    let row = repo.create_admin(body, subtotal, total).await?;
    let invoice_id = row["id"].as_str().unwrap_or_default().to_string();
    if !invoice_id.is_empty() {
        repo.insert_invoice_items(&invoice_id, body.normalized_items())
            .await?;
        repo.emit_created_event(&invoice_id, body, subtotal, total)
            .await?;
    }

    Ok(row)
}

pub async fn update_admin<R: OneTimeSalesRepository>(
    repo: &R,
    id: &str,
    body: &UpdateOneTimeSaleRequest,
) -> Result<serde_json::Value, BillingError> {
    let before = repo.find_admin_invoice(id).await?;
    let row = repo.update_admin(id, body).await?;
    let after = repo.find_admin_invoice(id).await?;

    if status_was_voided(&before, &after) {
        repo.emit_void_reversal(&after, "one_time_sale_update")
            .await?;
    }

    Ok(row)
}

pub async fn delete_admin<R: OneTimeSalesRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let invoice = repo.find_admin_invoice(id).await?;
    let affected = repo.delete_admin(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("one_time_sale", id));
    }

    repo.emit_void_reversal(&invoice, "one_time_sale_delete")
        .await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn list_scoped<R: OneTimeSalesRepository>(
    repo: &R,
    status: Option<&str>,
    customer_id: &str,
) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list_scoped(status, customer_id).await
}

pub async fn get_scoped<R: OneTimeSalesRepository>(
    repo: &R,
    id: &str,
    customer_id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get_scoped(id, customer_id).await
}

pub async fn create_scoped<R: OneTimeSalesRepository>(
    repo: &R,
    customer_id: &str,
    body: &CreateOneTimeSaleRequest,
) -> Result<serde_json::Value, BillingError> {
    let subtotal = body.normalized_subtotal();
    let total = body.normalized_total(subtotal);
    repo.create_scoped(customer_id, body, subtotal, total).await
}

pub async fn update_scoped<R: OneTimeSalesRepository>(
    repo: &R,
    id: &str,
    customer_id: &str,
    body: &UpdateOneTimeSaleRequest,
) -> Result<serde_json::Value, BillingError> {
    repo.update_scoped(id, customer_id, body).await
}

pub async fn delete_scoped<R: OneTimeSalesRepository>(
    repo: &R,
    id: &str,
    customer_id: &str,
) -> Result<serde_json::Value, BillingError> {
    let affected = repo.delete_scoped(id, customer_id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("one_time_sale", id));
    }
    Ok(serde_json::json!({ "success": true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::billing::one_time_sales::repository::OneTimeSalesRepository;
    use crate::routes::billing::one_time_sales::schema::{
        CreateOneTimeSaleRequest, OneTimeSaleItemInput, UpdateOneTimeSaleRequest,
    };
    use async_trait::async_trait;
    use rustbill_core::db::models::{Invoice, InvoiceStatus};
    use rustbill_core::error::BillingError;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockRepo {
        create_called: AtomicBool,
    }

    impl MockRepo {
        fn new() -> Self {
            Self {
                create_called: AtomicBool::new(false),
            }
        }
    }

    #[async_trait]
    impl OneTimeSalesRepository for MockRepo {
        async fn list_admin(&self) -> Result<Vec<serde_json::Value>, BillingError> {
            Ok(vec![])
        }
        async fn get_admin(&self, _id: &str) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({}))
        }
        async fn create_admin(
            &self,
            _body: &CreateOneTimeSaleRequest,
            _subtotal: f64,
            _total: f64,
        ) -> Result<serde_json::Value, BillingError> {
            self.create_called.store(true, Ordering::SeqCst);
            Ok(serde_json::json!({ "id": "inv-1" }))
        }
        async fn update_admin(
            &self,
            _id: &str,
            _body: &UpdateOneTimeSaleRequest,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({}))
        }
        async fn find_admin_invoice(&self, _id: &str) -> Result<Invoice, BillingError> {
            Ok(Invoice {
                id: "inv-1".into(),
                invoice_number: "INV-1".into(),
                customer_id: "cust-1".into(),
                subscription_id: None,
                status: InvoiceStatus::Issued,
                issued_at: None,
                due_at: None,
                paid_at: None,
                subtotal: rust_decimal::Decimal::ZERO,
                tax: rust_decimal::Decimal::ZERO,
                total: rust_decimal::Decimal::ZERO,
                currency: "USD".into(),
                notes: None,
                stripe_invoice_id: None,
                xendit_invoice_id: None,
                lemonsqueezy_order_id: None,
                version: 1,
                deleted_at: None,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
                tax_name: None,
                tax_rate: Some(rust_decimal::Decimal::ZERO),
                tax_inclusive: false,
                credits_applied: rust_decimal::Decimal::ZERO,
                amount_due: rust_decimal::Decimal::ZERO,
                auto_charge_attempts: 0,
                idempotency_key: None,
            })
        }
        async fn delete_admin(&self, _id: &str) -> Result<u64, BillingError> {
            Ok(1)
        }
        async fn insert_invoice_items(
            &self,
            _invoice_id: &str,
            _items: &[OneTimeSaleItemInput],
        ) -> Result<(), BillingError> {
            Ok(())
        }
        async fn emit_created_event(
            &self,
            _invoice_id: &str,
            _body: &CreateOneTimeSaleRequest,
            _subtotal: f64,
            _total: f64,
        ) -> Result<(), BillingError> {
            Ok(())
        }
        async fn emit_void_reversal(
            &self,
            _invoice: &Invoice,
            _trigger: &str,
        ) -> Result<(), BillingError> {
            Ok(())
        }
        async fn list_scoped(
            &self,
            _status: Option<&str>,
            _customer_id: &str,
        ) -> Result<Vec<serde_json::Value>, BillingError> {
            Ok(vec![])
        }
        async fn get_scoped(
            &self,
            _id: &str,
            _customer_id: &str,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({}))
        }
        async fn create_scoped(
            &self,
            _customer_id: &str,
            _body: &CreateOneTimeSaleRequest,
            _subtotal: f64,
            _total: f64,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({}))
        }
        async fn update_scoped(
            &self,
            _id: &str,
            _customer_id: &str,
            _body: &UpdateOneTimeSaleRequest,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({}))
        }
        async fn delete_scoped(&self, _id: &str, _customer_id: &str) -> Result<u64, BillingError> {
            Ok(1)
        }
    }

    #[tokio::test]
    async fn create_admin_rejects_blank_customer() {
        let repo = MockRepo::new();
        let body = CreateOneTimeSaleRequest {
            customer_id: "   ".to_string(),
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
    async fn create_admin_accepts_items_for_subtotal_fallback() {
        let repo = MockRepo::new();
        let body = CreateOneTimeSaleRequest {
            customer_id: "cust-1".to_string(),
            status: None,
            currency: Some("USD".to_string()),
            subtotal: Some(0.0),
            tax: Some(5.0),
            total: None,
            due_at: None,
            issued_at: None,
            notes: None,
            items: Some(vec![OneTimeSaleItemInput {
                description: Some("Line item".to_string()),
                quantity: Some(2.0),
                unit_price: Some(10.0),
                amount: None,
            }]),
        };

        let result = create_admin(&repo, &body).await;
        assert!(result.is_ok());
        assert!(repo.create_called.load(Ordering::SeqCst));
    }
}
