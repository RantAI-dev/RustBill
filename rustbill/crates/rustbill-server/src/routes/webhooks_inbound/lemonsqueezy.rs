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
    // TODO: restore handle_webhook once axum Handler trait issue is resolved
    // (the full handler's async future doesn't satisfy Send bound)
    Router::new().route("/", post(handle_webhook_stub))
}

async fn handle_webhook_stub(State(_state): State<SharedState>) -> StatusCode {
    tracing::warn!("LemonSqueezy webhook handler is currently stubbed out");
    StatusCode::OK
}

/// Verify a LemonSqueezy HMAC-SHA256 signature.
fn verify_lemonsqueezy_signature(body: &str, signature_hex: &str, secret: &str) -> bool {
    let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(body.as_bytes());
    let computed = hex::encode(mac.finalize().into_bytes());

    // Constant-time comparison of hex strings
    let a = computed.as_bytes();
    let b = signature_hex.as_bytes();
    a.len() == b.len() && a.ct_eq(b).into()
}

async fn handle_webhook(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body_bytes: axum::body::Bytes,
) -> ApiResult<StatusCode> {
    let body = String::from_utf8(body_bytes.to_vec()).map_err(|_| {
        rustbill_core::error::BillingError::BadRequest("Invalid UTF-8 in request body".into())
    })?;
    let signature = headers
        .get("x-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    tracing::info!(
        signature_len = signature.len(),
        "Received LemonSqueezy webhook"
    );

    // Verify HMAC-SHA256 signature
    let secret = state
        .provider_cache
        .get("lemonsqueezy_webhook_secret")
        .await;
    if secret.is_empty() {
        tracing::warn!("LemonSqueezy webhook secret not configured — skipping signature verification (dev mode)");
    } else if !verify_lemonsqueezy_signature(&body, signature, &secret) {
        tracing::warn!("LemonSqueezy signature verification failed");
        return Err(rustbill_core::error::BillingError::Unauthorized.into());
    }

    let event: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        rustbill_core::error::BillingError::BadRequest(format!("Invalid JSON: {e}"))
    })?;

    let event_type = headers
        .get("x-event-name")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    tracing::info!(event_type, "Processing LemonSqueezy event");

    // Record the event (best-effort: event_type is an enum, so unknown types will be skipped)
    let mapped_event_type = match event_type {
        "order_created" => Some("payment.received"),
        "order_refunded" => Some("payment.refunded"),
        _ => None,
    };
    if let Some(mapped) = mapped_event_type {
        let _ = sqlx::query(
            r#"INSERT INTO billing_events (id, event_type, resource_type, resource_id, data, created_at)
               VALUES (gen_random_uuid()::text, $1::billing_event_type, 'lemonsqueezy', COALESCE($2, ''), $3, now())"#,
        )
        .bind(mapped)
        .bind(event["data"]["id"].as_str())
        .bind(&event)
        .execute(&state.db)
        .await;
    }

    // Dispatch based on event type
    match event_type {
        "order_created" => {
            // Find invoice by custom_data.invoiceId embedded in the checkout
            let invoice_id = event["meta"]["custom_data"]["invoiceId"]
                .as_str()
                .or(
                    event["data"]["attributes"]["first_order_item"]["custom_data"]["invoiceId"]
                        .as_str(),
                );

            if let Some(invoice_id) = invoice_id {
                let invoice: Option<rustbill_core::db::models::Invoice> =
                    sqlx::query_as("SELECT * FROM invoices WHERE id = $1 AND deleted_at IS NULL")
                        .bind(invoice_id)
                        .fetch_optional(&state.db)
                        .await
                        .map_err(rustbill_core::error::BillingError::from)?;

                if let Some(invoice) = invoice {
                    if invoice.status != rustbill_core::db::models::InvoiceStatus::Paid {
                        let now = chrono::Utc::now().naive_utc();

                        // Update invoice status to paid
                        sqlx::query(
                            "UPDATE invoices SET status = 'paid', paid_at = $2, version = version + 1, updated_at = NOW() WHERE id = $1",
                        )
                        .bind(&invoice.id)
                        .bind(now)
                        .execute(&state.db)
                        .await
                        .map_err(rustbill_core::error::BillingError::from)?;

                        // Extract amount from order data
                        let amount = event["data"]["attributes"]["total"]
                            .as_i64()
                            .map(|a| Decimal::from(a) / Decimal::from(100)) // LS amounts are in cents
                            .unwrap_or(invoice.total);

                        let ls_order_id = event["data"]["id"].as_str().map(|s| s.to_string());

                        // Store LS order ID on invoice
                        if let Some(ref order_id) = ls_order_id {
                            sqlx::query(
                                "UPDATE invoices SET lemonsqueezy_order_id = $2, updated_at = NOW() WHERE id = $1",
                            )
                            .bind(&invoice.id)
                            .bind(order_id)
                            .execute(&state.db)
                            .await
                            .map_err(rustbill_core::error::BillingError::from)?;
                        }

                        let req = rustbill_core::billing::payments::CreatePaymentRequest {
                            invoice_id: invoice.id.clone(),
                            amount,
                            method: rustbill_core::db::models::PaymentMethod::Lemonsqueezy,
                            reference: ls_order_id
                                .as_deref()
                                .map(|id| format!("lemonsqueezy:{id}"))
                                .or(Some("lemonsqueezy:unknown".to_string())),
                            paid_at: Some(now),
                            notes: None,
                            stripe_payment_intent_id: None,
                            xendit_payment_id: None,
                            lemonsqueezy_order_id: ls_order_id,
                        };

                        if let Err(e) =
                            rustbill_core::billing::payments::create_payment_with_notification(
                                &state.db,
                                req,
                                state.email_sender.as_ref(),
                            )
                            .await
                        {
                            tracing::warn!("Failed to create payment record for LemonSqueezy order_created: {e}");
                        }
                    }
                } else {
                    tracing::warn!(
                        invoice_id,
                        "No matching invoice found for LemonSqueezy order_created"
                    );
                }
            } else {
                tracing::warn!("LemonSqueezy order_created event missing custom_data.invoiceId");
            }
        }

        "order_refunded" => {
            // Create refund record
            let invoice_id = event["meta"]["custom_data"]["invoiceId"]
                .as_str()
                .or(
                    event["data"]["attributes"]["first_order_item"]["custom_data"]["invoiceId"]
                        .as_str(),
                );

            if let Some(invoice_id) = invoice_id {
                // Find the most recent payment for this invoice via LemonSqueezy
                let payment: Option<rustbill_core::db::models::Payment> = sqlx::query_as(
                    "SELECT * FROM payments WHERE invoice_id = $1 AND method = 'lemonsqueezy' ORDER BY created_at DESC LIMIT 1",
                )
                .bind(invoice_id)
                .fetch_optional(&state.db)
                .await
                .map_err(rustbill_core::error::BillingError::from)?;

                if let Some(payment) = payment {
                    let refund_amount = event["data"]["attributes"]["refunded_amount"]
                        .as_i64()
                        .map(|a| Decimal::from(a) / Decimal::from(100))
                        .unwrap_or(payment.amount);

                    let req = rustbill_core::billing::refunds::CreateRefundRequest {
                        payment_id: payment.id.clone(),
                        invoice_id: invoice_id.to_string(),
                        amount: refund_amount,
                        reason: "LemonSqueezy order refunded".to_string(),
                        status: Some(rustbill_core::db::models::RefundStatus::Completed),
                        stripe_refund_id: None,
                    };

                    if let Err(e) =
                        rustbill_core::billing::refunds::create_refund(&state.db, req).await
                    {
                        tracing::warn!(
                            "Failed to create refund record for LemonSqueezy order_refunded: {e}"
                        );
                    }
                } else {
                    tracing::warn!(
                        invoice_id,
                        "No matching payment found for LemonSqueezy order_refunded"
                    );
                }
            }
        }

        _ => {
            tracing::debug!(event_type, "Unhandled LemonSqueezy event type");
        }
    }

    Ok(StatusCode::OK)
}
