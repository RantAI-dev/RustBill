use super::repository::SqlxLemonSqueezyWebhookRepository;
use super::service;
use crate::app::SharedState;
use axum::{extract::State, http::StatusCode, routing::post, Router};

pub fn router() -> Router<SharedState> {
    // TODO: restore full webhook handler once the async Send-bound issue is resolved
    Router::new().route("/", post(handle_webhook_stub))
}

async fn handle_webhook_stub(State(_state): State<SharedState>) -> StatusCode {
    let repo = SqlxLemonSqueezyWebhookRepository;
    let _ = service::handle_webhook(&repo).await;
    StatusCode::OK
}
