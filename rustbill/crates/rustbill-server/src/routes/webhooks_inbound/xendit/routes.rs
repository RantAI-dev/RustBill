use super::repository::SqlxXenditWebhookRepository;
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
    let callback_token = headers
        .get("x-callback-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    tracing::info!(
        has_token = !callback_token.is_empty(),
        "Received Xendit webhook"
    );

    let expected_token = state.provider_cache.get("xendit_webhook_token").await;
    let repo = SqlxXenditWebhookRepository::new(state.db.clone());
    service::handle_webhook(
        &repo,
        &body,
        callback_token,
        &expected_token,
        state.email_sender.as_ref(),
    )
    .await?;

    Ok(StatusCode::OK)
}
