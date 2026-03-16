//! SessionUser extractor: extracts authenticated user from request extensions.

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use rustbill_core::auth::session::AuthUser;

/// Extractor that provides the authenticated user.
/// Requires the session_auth middleware to have run first.
pub struct SessionUser(pub AuthUser);

impl<S> FromRequestParts<S> for SessionUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthUser>()
            .cloned()
            .map(SessionUser)
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({ "error": "Unauthorized" })),
                )
                    .into_response()
            })
    }
}
