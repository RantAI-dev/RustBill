//! Cron secret authentication for scheduled task endpoints.

use crate::app::SharedState;
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

/// Middleware that requires cron secret OR valid admin session.
pub async fn require_cron_or_admin(
    State(state): State<SharedState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    // Check cron secret
    let cron_secret = req
        .headers()
        .get("x-cron-secret")
        .or_else(|| req.headers().get("authorization"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.strip_prefix("Bearer ").unwrap_or(s).to_string());

    if let Some(ref expected) = state.config.auth.cron_secret {
        if let Some(ref provided) = cron_secret {
            if provided == expected {
                return next.run(req).await;
            }
        }
    } else {
        // No cron secret configured — allow in dev mode
        return next.run(req).await;
    }

    // Fall back to session auth check
    let token = req
        .headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix("session=").map(String::from)
            })
        });

    if let Some(token) = token {
        if let Ok(Some(user)) = rustbill_core::auth::validate_session(&state.db, &token).await {
            if user.role == rustbill_core::db::models::UserRole::Admin {
                return next.run(req).await;
            }
        }
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "Unauthorized" })),
    )
        .into_response()
}
