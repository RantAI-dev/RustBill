pub mod auth;
pub mod products;
pub mod customers;
pub mod deals;
pub mod licenses;
pub mod api_keys;
pub mod billing;
pub mod webhooks_inbound;
pub mod v1;
pub mod analytics;
pub mod search;
pub mod settings;

use axum::{http::StatusCode, response::IntoResponse, Json};
use rustbill_core::error::BillingError;

/// Convert BillingError to HTTP response.
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, body) = match self.0 {
            BillingError::NotFound { ref entity, ref id } => (
                StatusCode::NOT_FOUND,
                serde_json::json!({ "error": format!("{entity} {id} not found") }),
            ),
            BillingError::Validation(ref errors) => (
                StatusCode::BAD_REQUEST,
                serde_json::json!({ "error": { "fieldErrors": errors } }),
            ),
            BillingError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                serde_json::json!({ "error": "Unauthorized" }),
            ),
            BillingError::Forbidden => (
                StatusCode::FORBIDDEN,
                serde_json::json!({ "error": "Forbidden" }),
            ),
            BillingError::Conflict(ref msg) => (
                StatusCode::CONFLICT,
                serde_json::json!({ "error": msg }),
            ),
            BillingError::ProviderNotConfigured(ref p) => (
                StatusCode::SERVICE_UNAVAILABLE,
                serde_json::json!({ "error": format!("{p} is not configured") }),
            ),
            BillingError::RateLimited { retry_after } => (
                StatusCode::TOO_MANY_REQUESTS,
                serde_json::json!({ "error": "rate_limited", "retryAfter": retry_after }),
            ),
            BillingError::BadRequest(ref msg) => (
                StatusCode::BAD_REQUEST,
                serde_json::json!({ "error": msg }),
            ),
            BillingError::Database(ref e) => {
                tracing::error!("Database error: {e:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    serde_json::json!({ "error": "An internal error occurred" }),
                )
            }
            BillingError::Internal(ref e) => {
                tracing::error!("Internal error: {e:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    serde_json::json!({ "error": "An internal error occurred" }),
                )
            }
        };
        (status, Json(body)).into_response()
    }
}

/// Wrapper to convert BillingError into an Axum response.
pub struct ApiError(pub BillingError);

impl From<BillingError> for ApiError {
    fn from(e: BillingError) -> Self {
        ApiError(e)
    }
}

/// Result type for route handlers.
pub type ApiResult<T> = std::result::Result<T, ApiError>;
