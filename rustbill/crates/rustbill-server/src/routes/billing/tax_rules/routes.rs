use super::repository::SqlxTaxRulesRepository;
use super::schema::{CreateTaxRuleRequest, UpdateTaxRuleRequest};
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", axum::routing::put(update).delete(remove))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxTaxRulesRepository::new(state.db.clone());
    let rules = service::list(&repo).await?;
    Ok(Json(serde_json::to_value(rules).map_err(|e| {
        rustbill_core::error::BillingError::Internal(e.into())
    })?))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateTaxRuleRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxTaxRulesRepository::new(state.db.clone());
    let rule = service::create(&repo, &body).await?;
    Ok(Json(serde_json::to_value(rule).map_err(|e| {
        rustbill_core::error::BillingError::Internal(e.into())
    })?))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateTaxRuleRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxTaxRulesRepository::new(state.db.clone());
    let rule = service::update(&repo, &id, &body).await?;
    Ok(Json(serde_json::to_value(rule).map_err(|e| {
        rustbill_core::error::BillingError::Internal(e.into())
    })?))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxTaxRulesRepository::new(state.db.clone());
    let result = service::remove(&repo, &id).await?;
    Ok(Json(result))
}
