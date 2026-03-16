use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new().route("/", get(get_checkout_url))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckoutParams {
    invoice_id: String,
    provider: Option<String>,
}

async fn get_checkout_url(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<CheckoutParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let provider = params.provider.as_deref().unwrap_or("stripe");

    // Determine the origin for redirect URLs
    let origin = std::env::var("PUBLIC_URL").unwrap_or_else(|_| {
        format!(
            "http://{}:{}",
            state.config.server.host, state.config.server.port
        )
    });

    // Build provider settings based on the selected provider
    let setting_keys: &[&str] = match provider {
        "stripe" => &["stripe_secret_key"],
        "xendit" => &["xendit_secret_key"],
        "lemonsqueezy" => &["lemonsqueezy_api_key", "lemonsqueezy_store_id"],
        _ => &[],
    };
    let settings = state
        .provider_cache
        .get_provider_settings(setting_keys)
        .await;

    let result = rustbill_core::billing::checkout::create_checkout(
        &state.db,
        &state.http_client,
        &settings,
        &params.invoice_id,
        provider,
        &origin,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "invoiceId": params.invoice_id,
        "provider": result.provider,
        "checkoutUrl": result.checkout_url,
    })))
}
