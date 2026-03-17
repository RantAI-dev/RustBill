use crate::app::SharedState;
use crate::routes::ApiResult;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use hmac::{Hmac, Mac};
use rust_decimal::Decimal;
use sha2::Sha256;
use subtle::ConstantTimeEq;

pub fn router() -> Router<SharedState> {
    Router::new().route("/", post(handle_webhook))
}

/// Parse the Stripe-Signature header into (timestamp, signature_hex).
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

/// Verify a Stripe webhook signature.
fn verify_stripe_signature(body: &str, sig_header: &str, secret: &str) -> Result<(), &'static str> {
    let (timestamp, sig_hex) =
        parse_stripe_signature(sig_header).ok_or("Invalid stripe-signature header format")?;

    // Check timestamp freshness (reject if > 300s old)
    let ts: i64 = timestamp.parse().map_err(|_| "Invalid timestamp")?;
    let now = chrono::Utc::now().timestamp();
    if (now - ts).unsigned_abs() > 300 {
        return Err("Webhook timestamp too old");
    }

    // Compute expected signature: HMAC-SHA256(timestamp + "." + body, secret)
    let signed_payload = format!("{timestamp}.{body}");
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| "Invalid webhook secret")?;
    mac.update(signed_payload.as_bytes());
    let expected = mac.finalize().into_bytes();

    // Decode the provided hex signature
    let provided = hex::decode(&sig_hex).map_err(|_| "Invalid signature hex")?;

    if expected.as_slice().ct_eq(&provided).into() {
        Ok(())
    } else {
        Err("Signature mismatch")
    }
}

async fn handle_webhook(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: String,
) -> ApiResult<StatusCode> {
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    tracing::info!(signature_len = signature.len(), "Received Stripe webhook");

    // Verify signature using the Stripe webhook secret from provider_cache
    let secret = state.provider_cache.get("stripe_webhook_secret").await;
    if secret.is_empty() {
        tracing::warn!(
            "Stripe webhook secret not configured — skipping signature verification (dev mode)"
        );
    } else {
        verify_stripe_signature(&body, signature, &secret).map_err(|e| {
            tracing::warn!("Stripe signature verification failed: {e}");
            rustbill_core::error::BillingError::Unauthorized
        })?;
    }

    let event: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        rustbill_core::error::BillingError::BadRequest(format!("Invalid JSON: {e}"))
    })?;

    let event_type = event["type"].as_str().unwrap_or("unknown");
    tracing::info!(event_type, "Processing Stripe event");

    // Record the event (best-effort: event_type is an enum, so unknown types will be skipped)
    let mapped_event_type = match event_type {
        "invoice.paid" => Some("invoice.paid"),
        "invoice.payment_failed" => Some("invoice.overdue"),
        "checkout.session.completed" => Some("payment.received"),
        "charge.refunded" => Some("payment.refunded"),
        "customer.subscription.deleted" => Some("subscription.canceled"),
        _ => None,
    };
    if let Some(mapped) = mapped_event_type {
        let _ = sqlx::query(
            r#"INSERT INTO billing_events (id, event_type, resource_type, resource_id, data, created_at)
               VALUES (gen_random_uuid()::text, $1::billing_event_type, 'stripe', COALESCE($2, ''), $3, now())"#,
        )
        .bind(mapped)
        .bind(event["data"]["object"]["id"].as_str())
        .bind(&event)
        .execute(&state.db)
        .await;
    }

    // Dispatch to appropriate handler based on event_type
    let obj = &event["data"]["object"];

    match event_type {
        "checkout.session.completed" | "invoice.paid" => {
            // Find invoice by stripe_invoice_id, update status to Paid, create payment record
            let stripe_invoice_id = if event_type == "checkout.session.completed" {
                obj["invoice"].as_str().or(obj["id"].as_str())
            } else {
                obj["id"].as_str()
            };

            if let Some(stripe_id) = stripe_invoice_id {
                let invoice: Option<rustbill_core::db::models::Invoice> = sqlx::query_as(
                    "SELECT * FROM invoices WHERE stripe_invoice_id = $1 AND deleted_at IS NULL",
                )
                .bind(stripe_id)
                .fetch_optional(&state.db)
                .await
                .map_err(rustbill_core::error::BillingError::from)?;

                if let Some(invoice) = invoice {
                    if invoice.status != rustbill_core::db::models::InvoiceStatus::Paid {
                        // Update invoice status to paid
                        let now = chrono::Utc::now().naive_utc();
                        sqlx::query(
                            "UPDATE invoices SET status = 'paid', paid_at = $2, version = version + 1, updated_at = NOW() WHERE id = $1",
                        )
                        .bind(&invoice.id)
                        .bind(now)
                        .execute(&state.db)
                        .await
                        .map_err(rustbill_core::error::BillingError::from)?;

                        // Create payment record
                        let amount_paid = obj["amount_paid"]
                            .as_i64()
                            .or_else(|| obj["amount_total"].as_i64())
                            .map(|a| Decimal::from(a) / Decimal::from(100)) // Stripe amounts are in cents
                            .unwrap_or(invoice.total);

                        let stripe_payment_intent_id =
                            obj["payment_intent"].as_str().map(|s| s.to_string());

                        let req = rustbill_core::billing::payments::CreatePaymentRequest {
                            invoice_id: invoice.id.clone(),
                            amount: amount_paid,
                            method: rustbill_core::db::models::PaymentMethod::Stripe,
                            reference: Some(format!("stripe:{}", stripe_id)),
                            paid_at: Some(now),
                            notes: None,
                            stripe_payment_intent_id,
                            xendit_payment_id: None,
                            lemonsqueezy_order_id: None,
                        };

                        if let Err(e) =
                            rustbill_core::billing::payments::create_payment_with_notification(
                                &state.db,
                                req,
                                state.email_sender.as_ref(),
                            )
                            .await
                        {
                            tracing::warn!("Failed to create payment record for stripe event: {e}");
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
            // Update invoice status to Overdue
            let stripe_id = obj["id"].as_str();
            if let Some(stripe_id) = stripe_id {
                sqlx::query(
                    "UPDATE invoices SET status = 'overdue', version = version + 1, updated_at = NOW() WHERE stripe_invoice_id = $1 AND deleted_at IS NULL AND status != 'paid'",
                )
                .bind(stripe_id)
                .execute(&state.db)
                .await
                .map_err(rustbill_core::error::BillingError::from)?;
            }
        }

        "customer.subscription.deleted" => {
            // Find subscription by stripe_subscription_id, update status to Canceled
            let stripe_sub_id = obj["id"].as_str();
            if let Some(stripe_sub_id) = stripe_sub_id {
                let now = chrono::Utc::now().naive_utc();
                sqlx::query(
                    "UPDATE subscriptions SET status = 'canceled', canceled_at = $2, version = version + 1, updated_at = NOW() WHERE stripe_subscription_id = $1 AND deleted_at IS NULL",
                )
                .bind(stripe_sub_id)
                .bind(now)
                .execute(&state.db)
                .await
                .map_err(rustbill_core::error::BillingError::from)?;
            }
        }

        "charge.refunded" => {
            // Create refund record
            let stripe_charge_id = obj["id"].as_str().unwrap_or_default();
            let refund_amount = obj["amount_refunded"]
                .as_i64()
                .map(|a| Decimal::from(a) / Decimal::from(100))
                .unwrap_or_default();

            // Find payment by stripe_payment_intent_id (charge's payment_intent)
            let payment_intent_id = obj["payment_intent"].as_str();
            if let Some(pi_id) = payment_intent_id {
                let payment: Option<rustbill_core::db::models::Payment> =
                    sqlx::query_as("SELECT * FROM payments WHERE stripe_payment_intent_id = $1")
                        .bind(pi_id)
                        .fetch_optional(&state.db)
                        .await
                        .map_err(rustbill_core::error::BillingError::from)?;

                if let Some(payment) = payment {
                    let req = rustbill_core::billing::refunds::CreateRefundRequest {
                        payment_id: payment.id.clone(),
                        invoice_id: payment.invoice_id.clone(),
                        amount: refund_amount,
                        reason: format!("Stripe charge refunded: {}", stripe_charge_id),
                        status: Some(rustbill_core::db::models::RefundStatus::Completed),
                        stripe_refund_id: Some(stripe_charge_id.to_string()),
                    };

                    if let Err(e) =
                        rustbill_core::billing::refunds::create_refund(&state.db, req).await
                    {
                        tracing::warn!(
                            "Failed to create refund record for stripe charge.refunded: {e}"
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

    Ok(StatusCode::OK)
}
