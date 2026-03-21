use super::repository::SqlxStripeWebhookRepository;
use super::service;
use crate::app::SharedState;
use crate::routes::ApiResult;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};

pub fn router() -> Router<SharedState> {
    Router::new().route("/", post(handle_webhook))
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
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    tracing::info!(signature_len = signature.len(), "Received Stripe webhook");

    let secret = state.provider_cache.get("stripe_webhook_secret").await;
    let repo = SqlxStripeWebhookRepository::new(state.db.clone());
    service::handle_webhook(
        &repo,
        &body,
        signature,
        &secret,
        state.email_sender.as_ref(),
    )
    .await?;

    Ok(StatusCode::OK)
}
