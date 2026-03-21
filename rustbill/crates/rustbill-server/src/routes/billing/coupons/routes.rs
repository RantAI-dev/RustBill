use super::repository::SqlxCouponsRepository;
use super::schema::{CreateCouponRequest, UpdateCouponRequest};
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let repo = SqlxCouponsRepository::new(state.db.clone());
    let rows = service::list(&repo).await?;
    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxCouponsRepository::new(state.db.clone());
    let row = service::get(&repo, &id).await?;
    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateCouponRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let repo = SqlxCouponsRepository::new(state.db.clone());
    let row = service::create(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateCouponRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxCouponsRepository::new(state.db.clone());
    let row = service::update(&repo, &id, &body).await?;
    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxCouponsRepository::new(state.db.clone());
    let row = service::delete(&repo, &id).await?;
    Ok(Json(row))
}
