use super::repository::XenditWebhookRepository;
use super::schema::XenditWebhookEvent;
use rust_decimal::Decimal;
use rustbill_core::billing::payments::CreatePaymentRequest;
use rustbill_core::db::models::{InvoiceStatus, PaymentMethod};
use rustbill_core::error::BillingError;
use rustbill_core::notifications::email::EmailSender;
use subtle::ConstantTimeEq;

fn verify_callback_token(callback_token: &str, expected_token: &str) -> Result<(), BillingError> {
    if expected_token.is_empty() {
        tracing::warn!("Xendit webhook token not configured — skipping verification (dev mode)");
        return Ok(());
    }

    let provided = callback_token.as_bytes();
    let expected = expected_token.as_bytes();
    if provided.len() != expected.len() || !bool::from(provided.ct_eq(expected)) {
        tracing::warn!("Xendit callback token verification failed");
        return Err(BillingError::Unauthorized);
    }

    Ok(())
}

fn decimal_from_cents(value: &serde_json::Value, fallback: Decimal) -> Decimal {
    value
        .as_f64()
        .map(|amount| Decimal::try_from(amount).unwrap_or(fallback))
        .unwrap_or(fallback)
}

pub async fn handle_webhook<R: XenditWebhookRepository>(
    repo: &R,
    body: &str,
    callback_token: &str,
    expected_token: &str,
    email_sender: Option<&EmailSender>,
) -> Result<(), BillingError> {
    verify_callback_token(callback_token, expected_token)?;

    let event_json: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| BillingError::BadRequest(format!("Invalid JSON: {e}")))?;
    let event: XenditWebhookEvent = serde_json::from_value(event_json.clone())
        .map_err(|e| BillingError::BadRequest(format!("Invalid JSON: {e}")))?;
    let event_type = event.event.as_deref().unwrap_or("unknown");
    tracing::info!(event_type, "Processing Xendit event");

    let status = event.status.as_deref().unwrap_or_default();
    let external_id = event.external_id.as_deref().or_else(|| {
        event
            .data
            .as_ref()
            .and_then(|data| data["external_id"].as_str())
    });

    if let Some(mapped) = match status {
        "PAID" | "SETTLED" => Some("payment.received"),
        "EXPIRED" => Some("invoice.overdue"),
        _ => None,
    } {
        let resource_id = event
            .data
            .as_ref()
            .and_then(|data| data["id"].as_str())
            .or(event.id.as_deref());
        let _ = repo.record_event(mapped, resource_id, &event_json).await;
    }

    match status {
        "PAID" | "SETTLED" => {
            if let Some(ext_id) = external_id {
                if let Some(invoice) = repo.find_invoice_by_external_id_or_id(ext_id).await? {
                    if invoice.status != InvoiceStatus::Paid {
                        let now = chrono::Utc::now().naive_utc();
                        repo.mark_invoice_paid(&invoice.id, now).await?;

                        let amount = event
                            .paid_amount
                            .as_ref()
                            .or(event.amount.as_ref())
                            .map(|value| decimal_from_cents(value, invoice.total))
                            .unwrap_or(invoice.total);
                        let xendit_payment_id = event.id.clone().or_else(|| {
                            event
                                .data
                                .as_ref()
                                .and_then(|data| data["id"].as_str().map(|id| id.to_string()))
                        });

                        let req = CreatePaymentRequest {
                            invoice_id: invoice.id.clone(),
                            amount,
                            method: PaymentMethod::Xendit,
                            reference: Some(format!("xendit:{ext_id}")),
                            paid_at: Some(now),
                            notes: None,
                            stripe_payment_intent_id: None,
                            xendit_payment_id,
                            lemonsqueezy_order_id: None,
                        };

                        if let Err(err) = repo
                            .create_payment_with_notification(req, email_sender)
                            .await
                        {
                            tracing::warn!(
                                "Failed to create payment record for Xendit event: {err}"
                            );
                        }
                    }
                } else {
                    tracing::warn!(
                        external_id = ext_id,
                        "No matching invoice found for Xendit PAID event"
                    );
                }
            }
        }
        "EXPIRED" => {
            if let Some(ext_id) = external_id {
                repo.mark_invoice_overdue_by_external_id_or_id(ext_id)
                    .await?;
            }
        }
        _ => {
            tracing::debug!(status, event_type, "Unhandled Xendit event status");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rustbill_core::billing::payments::CreatePaymentRequest;
    use rustbill_core::db::models::{Invoice, InvoiceStatus};
    use rustbill_core::error::BillingError;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn sample_invoice() -> Invoice {
        Invoice {
            id: "inv-1".to_string(),
            invoice_number: "INV-1".to_string(),
            customer_id: "cust-1".to_string(),
            subscription_id: None,
            status: InvoiceStatus::Draft,
            issued_at: None,
            due_at: None,
            paid_at: None,
            subtotal: Decimal::from(100),
            tax: Decimal::from(0),
            total: Decimal::from(100),
            currency: "USD".to_string(),
            notes: None,
            stripe_invoice_id: None,
            xendit_invoice_id: Some("xendit-1".to_string()),
            lemonsqueezy_order_id: None,
            version: 1,
            deleted_at: None,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            tax_name: None,
            tax_rate: None,
            tax_inclusive: false,
            credits_applied: Decimal::ZERO,
            amount_due: Decimal::from(100),
            auto_charge_attempts: 0,
            idempotency_key: None,
        }
    }

    struct MockRepo {
        invoice: Option<Invoice>,
        create_payment_calls: AtomicUsize,
        record_event_calls: AtomicUsize,
    }

    #[async_trait]
    impl XenditWebhookRepository for MockRepo {
        async fn record_event(
            &self,
            _event_type: &str,
            _resource_id: Option<&str>,
            _data: &serde_json::Value,
        ) -> Result<(), BillingError> {
            self.record_event_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn find_invoice_by_external_id_or_id(
            &self,
            _external_id: &str,
        ) -> Result<Option<Invoice>, BillingError> {
            Ok(self.invoice.clone())
        }

        async fn mark_invoice_paid(
            &self,
            _invoice_id: &str,
            _paid_at: chrono::NaiveDateTime,
        ) -> Result<(), BillingError> {
            Ok(())
        }

        async fn mark_invoice_overdue_by_external_id_or_id(
            &self,
            _external_id: &str,
        ) -> Result<(), BillingError> {
            Ok(())
        }

        async fn create_payment_with_notification(
            &self,
            _req: CreatePaymentRequest,
            _email_sender: Option<&EmailSender>,
        ) -> Result<(), BillingError> {
            self.create_payment_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn invalid_token_is_rejected() {
        let repo = MockRepo {
            invoice: None,
            create_payment_calls: AtomicUsize::new(0),
            record_event_calls: AtomicUsize::new(0),
        };
        let body = r#"{"status":"PAID"}"#;

        let result = handle_webhook(&repo, body, "wrong", "expected", None).await;
        assert!(matches!(result, Err(BillingError::Unauthorized)));
    }

    #[tokio::test]
    async fn paid_event_triggers_payment_creation() {
        let repo = MockRepo {
            invoice: Some(sample_invoice()),
            create_payment_calls: AtomicUsize::new(0),
            record_event_calls: AtomicUsize::new(0),
        };
        let body = serde_json::json!({
            "event": "invoice.paid",
            "status": "PAID",
            "external_id": "xendit-1",
            "id": "payment-1",
            "paid_amount": 50000,
            "data": { "id": "payment-data-1" }
        })
        .to_string();

        let result = handle_webhook(&repo, &body, "expected", "expected", None).await;
        assert!(result.is_ok());
        assert_eq!(repo.create_payment_calls.load(Ordering::SeqCst), 1);
        assert_eq!(repo.record_event_calls.load(Ordering::SeqCst), 1);
    }
}
