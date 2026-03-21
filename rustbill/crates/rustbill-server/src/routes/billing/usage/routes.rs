use super::repository::SqlxUsageRepository;
use super::schema::{
    CreateUsageEventRequest, ListUsageParams, ListUsageV1Params, UpdateUsageEventRequest,
    UsageRecordInput,
};
use super::service;
use crate::app::SharedState;
use crate::extractors::{AdminUser, SessionUser};
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use rustbill_core::db::models::UserRole;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(record))
        .route("/{id}", axum::routing::put(update).delete(remove))
        .route("/{subscription_id}/summary", get(summary))
}

pub fn v1_router() -> Router<SharedState> {
    Router::new().route("/", get(list_v1).post(record_v1))
}

async fn list(
    State(state): State<SharedState>,
    user: SessionUser,
    Query(params): Query<ListUsageParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let role_customer_id = if user.0.role == UserRole::Customer {
        user.0.customer_id.as_deref()
    } else {
        None
    };
    let repo = SqlxUsageRepository::new(state.db.clone());
    let rows = service::list_admin(
        &repo,
        params.subscription_id.as_deref(),
        params.metric_name.as_deref(),
        role_customer_id,
    )
    .await?;
    Ok(Json(rows))
}

async fn record(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateUsageEventRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let repo = SqlxUsageRepository::new(state.db.clone());
    let row = service::record_admin(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn summary(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(subscription_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxUsageRepository::new(state.db.clone());
    let row = service::summary_admin(&repo, &subscription_id).await?;
    Ok(Json(row))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateUsageEventRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxUsageRepository::new(state.db.clone());
    let row = service::update_admin(&repo, &id, &body).await?;
    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxUsageRepository::new(state.db.clone());
    let row = service::remove_admin(&repo, &id).await?;
    Ok(Json(row))
}

async fn list_v1(
    State(state): State<SharedState>,
    Query(params): Query<ListUsageV1Params>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let repo = SqlxUsageRepository::new(state.db.clone());
    let rows = service::list_v1(
        &repo,
        params.subscription_id.as_deref(),
        params.metric.as_deref(),
    )
    .await?;
    Ok(Json(rows))
}

async fn record_v1(
    State(state): State<SharedState>,
    Json(body): Json<UsageRecordInput>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let repo = SqlxUsageRepository::new(state.db.clone());
    let row = service::record_v1(&repo, body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}
