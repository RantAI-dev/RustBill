//! ValidatedJson extractor: deserializes + validates request body.

use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;

/// Extracts and validates a JSON request body.
pub struct ValidatedJson<T>(pub T);

impl<S, T> axum::extract::FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(
        req: axum::extract::Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await.map_err(|e: JsonRejection| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            ).into_response()
        })?;

        value.validate().map_err(|e| {
            let errors: Vec<serde_json::Value> = e
                .field_errors()
                .into_iter()
                .flat_map(|(field, errs)| {
                    errs.iter().map(move |err| {
                        serde_json::json!({
                            "field": field,
                            "message": err.message.as_ref().map(|m| m.to_string()).unwrap_or_else(|| format!("invalid value for {field}"))
                        })
                    })
                })
                .collect();

            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": { "fieldErrors": errors } })),
            ).into_response()
        })?;

        Ok(ValidatedJson(value))
    }
}
