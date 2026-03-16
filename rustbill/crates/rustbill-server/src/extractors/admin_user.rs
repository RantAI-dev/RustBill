//! AdminUser extractor: requires admin role.

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use rustbill_core::auth::session::AuthUser;
use rustbill_core::db::models::UserRole;

/// Extractor that requires admin role.
pub struct AdminUser(pub AuthUser);

impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = parts
            .extensions
            .get::<AuthUser>()
            .cloned()
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({ "error": "Unauthorized" })),
                ).into_response()
            })?;

        if user.role != UserRole::Admin {
            return Err((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({ "error": "Forbidden" })),
            ).into_response());
        }

        Ok(AdminUser(user))
    }
}
