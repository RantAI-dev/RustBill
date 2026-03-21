use super::repository::SqlxSubscriptionsRepository;
use super::schema::{
    ChangePlanRequest, CreateSubscriptionRequest, CreateSubscriptionV1Request, LifecycleRequest,
    SubscriptionListParams, UpdateSubscriptionRequest, UpdateSubscriptionV1Request,
};
use super::service;
use crate::app::SharedState;
use crate::extractors::{AdminUser, SessionUser};
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use rustbill_core::db::models::UserRole;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/lifecycle", post(lifecycle))
        .route("/{id}", get(get_one).put(update).delete(remove))
        .route("/{id}/change-plan", post(change_plan))
}

pub fn v1_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_v1).post(create_v1))
        .route("/{id}", get(get_one_v1).put(update_v1))
        .route("/{id}/change-plan", post(change_plan_v1))
}

async fn list(
    State(state): State<SharedState>,
    user: SessionUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let role_customer_id = if user.0.role == UserRole::Customer {
        user.0.customer_id.as_deref()
    } else {
        None
    };

    let repo = SqlxSubscriptionsRepository::new(state.db.clone());
    let rows = service::list_admin(&repo, role_customer_id).await?;
    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxSubscriptionsRepository::new(state.db.clone());
    let row = service::get_admin(&repo, &id).await?;
    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateSubscriptionRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let repo = SqlxSubscriptionsRepository::new(state.db.clone());
    let row = service::create_admin(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateSubscriptionRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxSubscriptionsRepository::new(state.db.clone());
    let row = service::update_admin(&repo, &id, &body).await?;
    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxSubscriptionsRepository::new(state.db.clone());
    let row = service::delete_admin(&repo, &id).await?;
    Ok(Json(row))
}

async fn lifecycle(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<LifecycleRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxSubscriptionsRepository::new(state.db.clone());
    let row = service::lifecycle_admin(&repo, &body).await?;
    Ok(Json(row))
}

async fn change_plan(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<ChangePlanRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxSubscriptionsRepository::new(state.db.clone());
    let row = service::change_plan_admin(&repo, &state.http_client, &id, &body).await?;
    Ok(Json(row))
}

async fn list_v1(
    State(state): State<SharedState>,
    Query(params): Query<SubscriptionListParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let repo = SqlxSubscriptionsRepository::new(state.db.clone());
    let rows = service::list_v1(
        &repo,
        params.status.as_deref(),
        params.customer_id.as_deref(),
    )
    .await?;
    Ok(Json(rows))
}

async fn get_one_v1(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxSubscriptionsRepository::new(state.db.clone());
    let row = service::get_v1(&repo, &id).await?;
    Ok(Json(row))
}

async fn create_v1(
    State(state): State<SharedState>,
    Json(body): Json<CreateSubscriptionV1Request>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let repo = SqlxSubscriptionsRepository::new(state.db.clone());
    let row = service::create_v1(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn update_v1(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSubscriptionV1Request>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxSubscriptionsRepository::new(state.db.clone());
    let row = service::update_v1(&repo, &id, &body).await?;
    Ok(Json(row))
}

async fn change_plan_v1(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<ChangePlanRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxSubscriptionsRepository::new(state.db.clone());
    let row = service::change_plan_v1(&repo, &state.http_client, &id, &body).await?;
    Ok(Json(row))
}
