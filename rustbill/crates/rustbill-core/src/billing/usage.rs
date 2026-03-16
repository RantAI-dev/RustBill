use crate::db::models::*;
use crate::error::{BillingError, Result};
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::Deserialize;
use sqlx::PgPool;
use validator::Validate;

// ---- Request types ----

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUsageEventRequest {
    #[validate(length(min = 1, message = "subscription_id is required"))]
    pub subscription_id: String,

    #[validate(length(min = 1, message = "metric_name is required"))]
    pub metric_name: String,

    pub value: Decimal,
    pub timestamp: Option<NaiveDateTime>,
    pub idempotency_key: Option<String>,
    pub properties: Option<serde_json::Value>,
}

// ---- Filter types ----

#[derive(Debug, Deserialize, Default)]
pub struct ListUsageEventsFilter {
    pub subscription_id: Option<String>,
    /// Customer role isolation.
    pub role_customer_id: Option<String>,
}

// ---- Service functions ----

pub async fn list_usage_events(pool: &PgPool, subscription_id: &str) -> Result<Vec<UsageEvent>> {
    let rows = sqlx::query_as::<_, UsageEvent>(
        r#"
        SELECT * FROM usage_events
        WHERE subscription_id = $1
        ORDER BY timestamp DESC
        "#,
    )
    .bind(subscription_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn create_usage_event(pool: &PgPool, req: CreateUsageEventRequest) -> Result<UsageEvent> {
    req.validate().map_err(BillingError::from_validation)?;

    // Idempotency check
    if let Some(ref key) = req.idempotency_key {
        let existing = sqlx::query_as::<_, UsageEvent>(
            "SELECT * FROM usage_events WHERE idempotency_key = $1",
        )
        .bind(key)
        .fetch_optional(pool)
        .await?;

        if let Some(event) = existing {
            return Ok(event);
        }
    }

    let timestamp = req
        .timestamp
        .unwrap_or_else(|| chrono::Utc::now().naive_utc());

    let event = sqlx::query_as::<_, UsageEvent>(
        r#"
        INSERT INTO usage_events
            (id, subscription_id, metric_name, value, timestamp, idempotency_key, properties)
        VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(&req.subscription_id)
    .bind(&req.metric_name)
    .bind(req.value)
    .bind(timestamp)
    .bind(&req.idempotency_key)
    .bind(&req.properties)
    .fetch_one(pool)
    .await?;

    Ok(event)
}
