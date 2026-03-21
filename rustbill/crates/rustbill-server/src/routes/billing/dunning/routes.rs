use super::repository::SqlxDunningRepository;
use super::schema::{CreateDunningLogRequest, DunningListParams};
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<DunningListParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let repo = SqlxDunningRepository::new(state.db.clone());
    let rows = service::list(&repo, &params).await?;
    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxDunningRepository::new(state.db.clone());
    let row = service::get(&repo, &id).await?;
    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateDunningLogRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let repo = SqlxDunningRepository::new(state.db.clone());
    let row = service::create(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}
