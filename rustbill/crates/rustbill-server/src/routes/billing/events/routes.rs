use super::repository::SqlxEventsRepository;
use super::schema::EventsListParams;
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list))
        .route("/{id}", get(get_one))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<EventsListParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxEventsRepository::new(state.db.clone());
    let body = service::list(&repo, &params).await?;
    Ok(Json(body))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxEventsRepository::new(state.db.clone());
    let row = service::get(&repo, &id).await?;
    Ok(Json(row))
}
