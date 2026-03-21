use super::repository::LemonSqueezyWebhookRepository;
use rustbill_core::error::BillingError;

pub async fn handle_webhook<R: LemonSqueezyWebhookRepository>(
    _repo: &R,
) -> Result<axum::http::StatusCode, BillingError> {
    tracing::warn!("LemonSqueezy webhook handler is currently stubbed out");
    Ok(axum::http::StatusCode::OK)
}
