//! Billing event emission + outbound webhook dispatch with retry.

use crate::db::models::BillingEventType;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use std::time::Duration;

type HmacSha256 = Hmac<Sha256>;

/// Emit a billing event: log to DB + dispatch to subscribed webhook endpoints.
pub async fn emit_billing_event(
    pool: &PgPool,
    http: &reqwest::Client,
    event_type: BillingEventType,
    resource_type: &str,
    resource_id: &str,
    customer_id: Option<&str>,
    data: Option<serde_json::Value>,
) -> crate::error::Result<String> {
    // Insert event
    let event_id: String = sqlx::query_scalar(
        r#"
        INSERT INTO billing_events (id, event_type, resource_type, resource_id, customer_id, data)
        VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(&event_type)
    .bind(resource_type)
    .bind(resource_id)
    .bind(customer_id)
    .bind(&data)
    .fetch_one(pool)
    .await?;

    // Find matching webhook endpoints
    let endpoints: Vec<WebhookRow> = sqlx::query_as(
        "SELECT id, url, secret, events FROM webhook_endpoints WHERE status = 'active'",
    )
    .fetch_all(pool)
    .await?;

    let event_type_str = serde_json::to_value(&event_type)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    for ep in endpoints {
        // Check if endpoint subscribes to this event
        let subscribed = ep
            .events
            .as_array()
            .map(|arr| {
                arr.iter()
                    .any(|e| e.as_str() == Some(&event_type_str) || e.as_str() == Some("*"))
            })
            .unwrap_or(false);

        if !subscribed {
            continue;
        }

        // Dispatch webhook (fire-and-forget with retry)
        let pool = pool.clone();
        let http = http.clone();
        let event_id = event_id.clone();
        let payload = serde_json::json!({
            "event": event_type_str,
            "data": data,
            "resourceType": resource_type,
            "resourceId": resource_id,
            "customerId": customer_id,
        });

        tokio::spawn(async move {
            dispatch_webhook(&pool, &http, &ep, &event_id, &payload).await;
        });
    }

    Ok(event_id)
}

async fn dispatch_webhook(
    pool: &PgPool,
    http: &reqwest::Client,
    endpoint: &WebhookRow,
    event_id: &str,
    payload: &serde_json::Value,
) {
    let payload_str = serde_json::to_string(payload).unwrap_or_default();

    // Create HMAC signature
    let signature = {
        let mut mac = HmacSha256::new_from_slice(endpoint.secret.as_bytes()).unwrap();
        mac.update(payload_str.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    };

    // Insert delivery record
    let delivery_id: Option<String> = sqlx::query_scalar(
        r#"
        INSERT INTO webhook_deliveries (id, endpoint_id, event_id, payload, attempts)
        VALUES (gen_random_uuid()::text, $1, $2, $3, 0)
        RETURNING id
        "#,
    )
    .bind(&endpoint.id)
    .bind(event_id)
    .bind(payload)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    // Retry up to 3 times with exponential backoff (1s, 4s, 16s)
    let backoff = [1, 4, 16];
    for (attempt, delay_secs) in backoff.iter().enumerate() {
        let result = http
            .post(&endpoint.url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", &signature)
            .header("X-Webhook-Event", payload["event"].as_str().unwrap_or(""))
            .header("X-Webhook-Id", event_id)
            .timeout(Duration::from_secs(10))
            .body(payload_str.clone())
            .send()
            .await;

        match result {
            Ok(resp) => {
                let status = resp.status().as_u16() as i32;
                let body = resp.text().await.unwrap_or_default();

                // Update delivery
                if let Some(ref did) = delivery_id {
                    let _ = sqlx::query(
                        "UPDATE webhook_deliveries SET response_code = $2, response_body = $3, attempts = $4, delivered_at = NOW() WHERE id = $1"
                    )
                    .bind(did)
                    .bind(status)
                    .bind(&body[..body.len().min(1000)])
                    .bind((attempt + 1) as i32)
                    .execute(pool)
                    .await;
                }

                // Success or non-retryable client error (4xx except 429)
                if (200..300).contains(&(status as u16))
                    || ((400..500).contains(&(status as u16)) && status != 429)
                {
                    return;
                }
            }
            Err(e) => {
                tracing::warn!(
                    endpoint = %endpoint.url,
                    attempt = attempt + 1,
                    error = %e,
                    "Webhook delivery failed"
                );
            }
        }

        if attempt < backoff.len() - 1 {
            tokio::time::sleep(Duration::from_secs(*delay_secs)).await;
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct WebhookRow {
    id: String,
    url: String,
    secret: String,
    events: serde_json::Value,
}
