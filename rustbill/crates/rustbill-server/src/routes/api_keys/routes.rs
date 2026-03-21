use super::repository::SqlxApiKeysRepository;
use super::schema::CreateApiKeyRequest;
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", delete(revoke))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let repo = SqlxApiKeysRepository::new(state.db.clone());
    let rows = service::list(&repo).await?;
    Ok(Json(rows))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateApiKeyRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let repo = SqlxApiKeysRepository::new(state.db.clone());
    let row = service::create(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn revoke(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxApiKeysRepository::new(state.db.clone());
    let row = service::revoke(&repo, &id).await?;
    Ok(Json(row))
}
