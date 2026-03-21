use super::repository::StripeWebhookRepository;
use super::schema::StripeWebhookEvent;
use hmac::Mac;
use rust_decimal::Decimal;
use rustbill_core::billing::payments::CreatePaymentRequest;
use rustbill_core::billing::refunds::CreateRefundRequest;
use rustbill_core::db::models::{InvoiceStatus, PaymentMethod, RefundStatus};
use rustbill_core::error::BillingError;
use rustbill_core::notifications::email::EmailSender;
use subtle::ConstantTimeEq;

fn parse_stripe_signature(header: &str) -> Option<(String, String)> {
    let mut timestamp = None;
    let mut sig_v1 = None;
    for part in header.split(',') {
        let part = part.trim();
        if let Some(t) = part.strip_prefix("t=") {
            timestamp = Some(t.to_string());
        } else if let Some(s) = part.strip_prefix("v1=") {
            sig_v1 = Some(s.to_string());
        }
    }
    match (timestamp, sig_v1) {
        (Some(t), Some(s)) => Some((t, s)),
        _ => None,
    }
}

fn verify_stripe_signature(body: &str, sig_header: &str, secret: &str) -> Result<(), BillingError> {
    let (timestamp, sig_hex) = parse_stripe_signature(sig_header)
        .ok_or_else(|| BillingError::bad_request("Invalid stripe-signature header format"))?;

    let ts: i64 = timestamp
        .parse()
        .map_err(|_| BillingError::bad_request("Invalid timestamp"))?;
    let now = chrono::Utc::now().timestamp();
    if (now - ts).unsigned_abs() > 300 {
        return Err(BillingError::bad_request("Webhook timestamp too old"));
    }

    let signed_payload = format!("{timestamp}.{body}");
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| BillingError::bad_request("Invalid webhook secret"))?;
    mac.update(signed_payload.as_bytes());
    let expected = mac.finalize().into_bytes();
    let provided =
        hex::decode(sig_hex).map_err(|_| BillingError::bad_request("Invalid signature hex"))?;

    if expected.as_slice().ct_eq(&provided).into() {
        Ok(())
    } else {
        Err(BillingError::Unauthorized)
    }
}

fn mapped_event_type(event_type: &str) -> Option<&'static str> {
    match event_type {
        "invoice.paid" => Some("invoice.paid"),
        "invoice.payment_failed" => Some("invoice.overdue"),
        "checkout.session.completed" => Some("payment.received"),
        "charge.refunded" => Some("payment.refunded"),
        "customer.subscription.deleted" => Some("subscription.canceled"),
        _ => None,
    }
}

pub async fn handle_webhook<R: StripeWebhookRepository>(
    repo: &R,
    body: &str,
    signature: &str,
    secret: &str,
    email_sender: Option<&EmailSender>,
) -> Result<(), BillingError> {
    if !secret.is_empty() {
        verify_stripe_signature(body, signature, secret).map_err(|err| {
            tracing::warn!("Stripe signature verification failed: {err}");
            BillingError::Unauthorized
        })?;
    } else {
        tracing::warn!(
            "Stripe webhook secret not configured — skipping signature verification (dev mode)"
        );
    }

    let event_json: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| BillingError::BadRequest(format!("Invalid JSON: {e}")))?;
    let event: StripeWebhookEvent = serde_json::from_value(event_json.clone())
        .map_err(|e| BillingError::BadRequest(format!("Invalid JSON: {e}")))?;
    let event_type = event.event_type.as_str();
    tracing::info!(event_type, "Processing Stripe event");

    if let Some(mapped) = mapped_event_type(event_type) {
        let resource_id = event.data.object["id"].as_str();
        let _ = repo.record_event(mapped, resource_id, &event_json).await;
    }

    let obj = &event.data.object;
    match event_type {
        "checkout.session.completed" | "invoice.paid" => {
            let stripe_invoice_id = if event_type == "checkout.session.completed" {
                obj["invoice"].as_str().or(obj["id"].as_str())
            } else {
                obj["id"].as_str()
            };

            if let Some(stripe_id) = stripe_invoice_id {
                if let Some(invoice) = repo.find_invoice_by_stripe_invoice_id(stripe_id).await? {
                    if invoice.status != InvoiceStatus::Paid {
                        let now = chrono::Utc::now().naive_utc();
                        repo.mark_invoice_paid(&invoice.id, now).await?;

                        let amount_paid = obj["amount_paid"]
                            .as_i64()
                            .or_else(|| obj["amount_total"].as_i64())
                            .map(|value| Decimal::from(value) / Decimal::from(100))
                            .unwrap_or(invoice.total);
                        let stripe_payment_intent_id = obj["payment_intent"]
                            .as_str()
                            .map(|value| value.to_string());

                        let req = CreatePaymentRequest {
                            invoice_id: invoice.id.clone(),
                            amount: amount_paid,
                            method: PaymentMethod::Stripe,
                            reference: Some(format!("stripe:{stripe_id}")),
                            paid_at: Some(now),
                            notes: None,
                            stripe_payment_intent_id,
                            xendit_payment_id: None,
                            lemonsqueezy_order_id: None,
                        };

                        if let Err(err) = repo
                            .create_payment_with_notification(req, email_sender)
                            .await
                        {
                            tracing::warn!(
                                "Failed to create payment record for stripe event: {err}"
                            );
                        }
                    }
                } else {
                    tracing::warn!(
                        stripe_invoice_id = stripe_id,
                        "No matching invoice found for Stripe event"
                    );
                }
            }
        }
        "invoice.payment_failed" => {
            if let Some(stripe_id) = obj["id"].as_str() {
                repo.mark_invoice_overdue_by_stripe_invoice_id(stripe_id)
                    .await?;
            }
        }
        "customer.subscription.deleted" => {
            if let Some(stripe_sub_id) = obj["id"].as_str() {
                let now = chrono::Utc::now().naive_utc();
                repo.mark_subscription_canceled_by_stripe_subscription_id(stripe_sub_id, now)
                    .await?;
            }
        }
        "charge.refunded" => {
            let stripe_charge_id = obj["id"].as_str().unwrap_or_default();
            let refund_amount = obj["amount_refunded"]
                .as_i64()
                .map(|value| Decimal::from(value) / Decimal::from(100))
                .unwrap_or_default();

            if let Some(pi_id) = obj["payment_intent"].as_str() {
                if let Some(payment) = repo.find_payment_by_stripe_payment_intent_id(pi_id).await? {
                    let req = CreateRefundRequest {
                        payment_id: payment.id.clone(),
                        invoice_id: payment.invoice_id.clone(),
                        amount: refund_amount,
                        reason: format!("Stripe charge refunded: {stripe_charge_id}"),
                        status: Some(RefundStatus::Completed),
                        stripe_refund_id: Some(stripe_charge_id.to_string()),
                    };

                    if let Err(err) = repo.create_refund(req).await {
                        tracing::warn!(
                            "Failed to create refund record for stripe charge.refunded: {err}"
                        );
                    }
                } else {
                    tracing::warn!(
                        payment_intent_id = pi_id,
                        "No matching payment found for Stripe charge.refunded"
                    );
                }
            }
        }
        _ => {
            tracing::debug!(event_type, "Unhandled Stripe event type");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rustbill_core::billing::payments::CreatePaymentRequest;
    use rustbill_core::billing::refunds::CreateRefundRequest;
    use rustbill_core::db::models::{Invoice, InvoiceStatus, Payment, PaymentMethod};
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
            stripe_invoice_id: Some("stripe-invoice-1".to_string()),
            xendit_invoice_id: None,
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

    fn sample_payment() -> Payment {
        Payment {
            id: "pay-1".to_string(),
            invoice_id: "inv-1".to_string(),
            amount: Decimal::from(100),
            method: PaymentMethod::Stripe,
            reference: Some("stripe:pi-1".to_string()),
            paid_at: chrono::Utc::now().naive_utc(),
            notes: None,
            stripe_payment_intent_id: Some("pi-1".to_string()),
            xendit_payment_id: None,
            lemonsqueezy_order_id: None,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }

    struct MockRepo {
        invoice: Option<Invoice>,
        payment: Option<Payment>,
        created_payments: AtomicUsize,
        refunds: AtomicUsize,
        events: AtomicUsize,
        overwritten: AtomicUsize,
    }

    impl MockRepo {
        fn new(invoice: Option<Invoice>) -> Self {
            Self {
                invoice,
                payment: Some(sample_payment()),
                created_payments: AtomicUsize::new(0),
                refunds: AtomicUsize::new(0),
                events: AtomicUsize::new(0),
                overwritten: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl StripeWebhookRepository for MockRepo {
        async fn record_event(
            &self,
            _event_type: &str,
            _resource_id: Option<&str>,
            _data: &serde_json::Value,
        ) -> Result<(), BillingError> {
            self.events.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn find_invoice_by_stripe_invoice_id(
            &self,
            _stripe_invoice_id: &str,
        ) -> Result<Option<Invoice>, BillingError> {
            Ok(self.invoice.clone())
        }

        async fn mark_invoice_paid(
            &self,
            _invoice_id: &str,
            _paid_at: chrono::NaiveDateTime,
        ) -> Result<(), BillingError> {
            self.overwritten.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn mark_invoice_overdue_by_stripe_invoice_id(
            &self,
            _stripe_invoice_id: &str,
        ) -> Result<(), BillingError> {
            Ok(())
        }

        async fn mark_subscription_canceled_by_stripe_subscription_id(
            &self,
            _stripe_subscription_id: &str,
            _canceled_at: chrono::NaiveDateTime,
        ) -> Result<(), BillingError> {
            Ok(())
        }

        async fn find_payment_by_stripe_payment_intent_id(
            &self,
            _stripe_payment_intent_id: &str,
        ) -> Result<Option<Payment>, BillingError> {
            Ok(self.payment.clone())
        }

        async fn create_payment_with_notification(
            &self,
            _req: CreatePaymentRequest,
            _email_sender: Option<&EmailSender>,
        ) -> Result<(), BillingError> {
            self.created_payments.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn create_refund(&self, _req: CreateRefundRequest) -> Result<(), BillingError> {
            self.refunds.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn invalid_signature_is_rejected() {
        let repo = MockRepo::new(None);
        let body = r#"{"type":"invoice.paid","data":{"object":{"id":"in_1"}}}"#;
        let ts = chrono::Utc::now().timestamp();
        let result =
            handle_webhook(&repo, body, &format!("t={ts},v1=deadbeef"), "secret", None).await;

        assert!(matches!(result, Err(BillingError::Unauthorized)));
    }

    #[tokio::test]
    async fn invoice_paid_triggers_payment_creation() {
        let repo = MockRepo::new(Some(sample_invoice()));
        let body = serde_json::json!({
            "type": "invoice.paid",
            "data": { "object": { "id": "stripe-invoice-1", "amount_paid": 10000, "payment_intent": "pi-1" } }
        });
        let body_str = body.to_string();
        let ts = chrono::Utc::now().timestamp();
        let sig = {
            let payload = format!("{ts}.{body_str}");
            let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(b"secret")
                .unwrap_or_else(|_| panic!("hmac"));
            mac.update(payload.as_bytes());
            format!("t={ts},v1={}", hex::encode(mac.finalize().into_bytes()))
        };

        let result = handle_webhook(&repo, &body_str, &sig, "secret", None).await;
        assert!(result.is_ok());
        assert_eq!(repo.created_payments.load(Ordering::SeqCst), 1);
        assert_eq!(repo.overwritten.load(Ordering::SeqCst), 1);
    }
}
