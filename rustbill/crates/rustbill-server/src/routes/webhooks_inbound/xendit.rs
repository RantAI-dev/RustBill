use crate::app::SharedState;
use crate::routes::ApiResult;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use rust_decimal::Decimal;
use subtle::ConstantTimeEq;

pub fn router() -> Router<SharedState> {
    Router::new().route("/", post(handle_webhook))
}

async fn handle_webhook(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: String,
) -> ApiResult<StatusCode> {
    let callback_token = headers
        .get("x-callback-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    tracing::info!(
        has_token = !callback_token.is_empty(),
        "Received Xendit webhook"
    );

    // Verify callback token against stored Xendit webhook verification token
    let expected_token = state.provider_cache.get("xendit_webhook_token").await;
    if expected_token.is_empty() {
        tracing::warn!("Xendit webhook token not configured — skipping verification (dev mode)");
    } else {
        let provided = callback_token.as_bytes();
        let expected = expected_token.as_bytes();
        if provided.len() != expected.len() || !bool::from(provided.ct_eq(expected)) {
            tracing::warn!("Xendit callback token verification failed");
            return Err(rustbill_core::error::BillingError::Unauthorized.into());
        }
    }

    let event: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        rustbill_core::error::BillingError::BadRequest(format!("Invalid JSON: {e}"))
    })?;

    let event_type = event["event"].as_str().unwrap_or("unknown");
    tracing::info!(event_type, "Processing Xendit event");

    // Record the event
    sqlx::query(
        r#"INSERT INTO billing_events (id, event_type, provider, entity_id, payload, created_at)
           VALUES (gen_random_uuid()::text, $1, 'xendit', $2, $3, now())"#,
    )
    .bind(event_type)
    .bind(event["data"]["id"].as_str().or(event["id"].as_str()))
    .bind(&event)
    .execute(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    // Dispatch based on Xendit invoice status
    // Xendit sends invoice callback with a "status" field at the top level
    let status = event["status"].as_str().unwrap_or_default();
    let external_id = event["external_id"]
        .as_str()
        .or(event["data"]["external_id"].as_str());

    match status {
        "PAID" | "SETTLED" => {
            // Find invoice by xendit_invoice_id (we stored it during checkout)
            // or by external_id (which we set to invoice.id during checkout)
            if let Some(ext_id) = external_id {
                let invoice: Option<rustbill_core::db::models::Invoice> = sqlx::query_as(
                    "SELECT * FROM invoices WHERE (xendit_invoice_id = $1 OR id = $1) AND deleted_at IS NULL",
                )
                .bind(ext_id)
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

                        // Create payment record
                        let amount = event["paid_amount"]
                            .as_f64()
                            .or(event["amount"].as_f64())
                            .map(|a| Decimal::try_from(a).unwrap_or(invoice.total))
                            .unwrap_or(invoice.total);

                        let xendit_payment_id = event["id"]
                            .as_str()
                            .or(event["data"]["id"].as_str())
                            .map(|s| s.to_string());

                        let req = rustbill_core::billing::payments::CreatePaymentRequest {
                            invoice_id: invoice.id.clone(),
                            amount,
                            method: rustbill_core::db::models::PaymentMethod::Xendit,
                            reference: Some(format!("xendit:{}", ext_id)),
                            paid_at: Some(now),
                            notes: None,
                            stripe_payment_intent_id: None,
                            xendit_payment_id,
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
                            tracing::warn!("Failed to create payment record for Xendit event: {e}");
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
            // Update invoice to Overdue
            if let Some(ext_id) = external_id {
                sqlx::query(
                    "UPDATE invoices SET status = 'overdue', version = version + 1, updated_at = NOW() WHERE (xendit_invoice_id = $1 OR id = $1) AND deleted_at IS NULL AND status != 'paid'",
                )
                .bind(ext_id)
                .execute(&state.db)
                .await
                .map_err(rustbill_core::error::BillingError::from)?;
            }
        }

        _ => {
            tracing::debug!(status, event_type, "Unhandled Xendit event status");
        }
    }

    Ok(StatusCode::OK)
}
