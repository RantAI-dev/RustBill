use super::repository::SqlxCreditsRepository;
use super::schema::{AdjustRequest, AdjustUpdateRequest, CreditQuery};
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/adjust", axum::routing::post(adjust))
        .route(
            "/adjust/{id}",
            axum::routing::put(update_adjustment).delete(delete_adjustment),
        )
        .route("/{customer_id}", axum::routing::get(get_customer_credits))
}

async fn adjust(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<AdjustRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxCreditsRepository::new(state.db.clone());
    let credit = service::adjust(&repo, &body).await?;
    Ok(Json(serde_json::to_value(credit).map_err(|e| {
        rustbill_core::error::BillingError::Internal(e.into())
    })?))
}

async fn update_adjustment(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<AdjustUpdateRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxCreditsRepository::new(state.db.clone());
    let credit = service::update_adjustment(&repo, &id, &body).await?;
    Ok(Json(serde_json::to_value(credit).map_err(|e| {
        rustbill_core::error::BillingError::Internal(e.into())
    })?))
}

async fn delete_adjustment(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxCreditsRepository::new(state.db.clone());
    let result = service::delete_adjustment(&repo, &id).await?;
    Ok(Json(result))
}

async fn get_customer_credits(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(customer_id): Path<String>,
    Query(query): Query<CreditQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxCreditsRepository::new(state.db.clone());
    let result =
        service::get_customer_credits(&repo, &customer_id, query.currency.as_deref()).await?;
    Ok(Json(result))
}
