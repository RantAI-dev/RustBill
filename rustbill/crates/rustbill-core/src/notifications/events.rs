//! Compatibility wrapper for billing event emission.

use super::repository::PgNotificationsRepository;
use super::service;
use crate::db::models::BillingEventType;
use sqlx::PgPool;

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
    let repo = PgNotificationsRepository::new(pool);
    service::emit_billing_event(
        &repo,
        http,
        service::build_emit_request(event_type, resource_type, resource_id, customer_id, data),
    )
    .await
}
