//! Session authentication middleware: extracts and validates session cookie.

use crate::app::SharedState;
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

/// Middleware that requires a valid session cookie.
pub async fn require_session(
    State(state): State<SharedState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    // Extract session token from cookie
    let token = req
        .headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                if c.starts_with("session=") {
                    Some(c[8..].to_string())
                } else {
                    None
                }
            })
        });

    let token = match token {
        Some(t) if !t.is_empty() => t,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Unauthorized" })),
            )
                .into_response();
        }
    };

    // Validate session
    match rustbill_core::auth::validate_session(&state.db, &token).await {
        Ok(Some(user)) => {
            req.extensions_mut().insert(user);
            next.run(req).await
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Unauthorized" })),
        )
            .into_response(),
    }
}
