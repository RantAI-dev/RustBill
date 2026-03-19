use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{extract::State, routing::get, Json, Router};
use rustbill_core::error::BillingError;

pub fn router() -> Router<SharedState> {
    Router::new().route(
        "/payment-providers",
        get(get_providers).put(update_providers),
    )
}

async fn get_providers(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let status = state.provider_cache.get_status().await;
    Ok(Json(
        serde_json::to_value(status).map_err(|e| BillingError::Internal(e.into()))?,
    ))
}

async fn update_providers(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let provider = body["provider"]
        .as_str()
        .ok_or_else(|| BillingError::BadRequest("provider is required".to_string()))?;
    let settings = body["settings"]
        .as_object()
        .ok_or_else(|| BillingError::BadRequest("settings must be an object".to_string()))?;

    let key_map: Vec<(&str, &str, bool)> = match provider {
        "stripe" => vec![
            ("secretKey", "stripe_secret_key", true),
            ("webhookSecret", "stripe_webhook_secret", true),
        ],
        "xendit" => vec![
            ("secretKey", "xendit_secret_key", true),
            ("webhookToken", "xendit_webhook_token", true),
        ],
        "lemonsqueezy" => vec![
            ("apiKey", "lemonsqueezy_api_key", true),
            ("storeId", "lemonsqueezy_store_id", false),
            ("webhookSecret", "lemonsqueezy_webhook_secret", true),
        ],
        "tax" => vec![
            ("externalProvider", "external_tax_provider", false),
            ("taxjarApiKey", "taxjar_api_key", true),
        ],
        _ => {
            return Err(BillingError::BadRequest(format!("Unknown provider: {provider}")).into());
        }
    };

    for (field, key, sensitive) in key_map {
        let Some(value) = settings.get(field).and_then(|v| v.as_str()) else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        state.provider_cache.save(key, value, sensitive).await?;
    }

    let status = state.provider_cache.get_status().await;
    Ok(Json(
        serde_json::to_value(status).map_err(|e| BillingError::Internal(e.into()))?,
    ))
}
