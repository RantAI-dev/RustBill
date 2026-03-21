use super::repository::ProviderSettingsRepository;
use super::schema::UpdateProvidersRequest;
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{extract::State, routing::get, Json, Router};

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
    let repo = ProviderSettingsRepository::new(state.provider_cache.clone());
    let result = service::get_providers(&repo).await?;
    Ok(Json(result))
}

async fn update_providers(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<UpdateProvidersRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = ProviderSettingsRepository::new(state.provider_cache.clone());
    let result = service::update_providers(&repo, &body).await?;
    Ok(Json(result))
}
