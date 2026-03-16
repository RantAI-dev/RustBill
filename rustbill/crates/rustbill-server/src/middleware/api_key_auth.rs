//! API key authentication middleware for public v1 API.

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use crate::app::SharedState;
use rustbill_core::auth::api_key::ApiKeyInfo;

/// Middleware that requires a valid API key in Authorization: Bearer header.
pub async fn require_api_key(
    State(state): State<SharedState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let key = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string());

    let key = match key {
        Some(k) if !k.is_empty() => k,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Missing or invalid API key" })),
            ).into_response();
        }
    };

    match rustbill_core::auth::api_key::verify_api_key(&state.db, &key).await {
        Ok(Some(info)) => {
            req.extensions_mut().insert(info);
            next.run(req).await
        }
        _ => {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Invalid API key" })),
            ).into_response()
        }
    }
}
