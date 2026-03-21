use super::repository::SqlxCreditNoteRepository;
use super::schema::{CreateCreditNoteRequest, UpdateCreditNoteRequest};
use super::service;
use crate::app::SharedState;
use crate::extractors::{AdminUser, SessionUser};
use crate::routes::ApiResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use rustbill_core::db::models::UserRole;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
}

async fn list(
    State(state): State<SharedState>,
    user: SessionUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let role_customer_id = if user.0.role == UserRole::Customer {
        user.0.customer_id.clone()
    } else {
        None
    };

    let repo = SqlxCreditNoteRepository::new(state.db.clone());
    let rows = service::list(&repo, role_customer_id.as_deref()).await?;
    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxCreditNoteRepository::new(state.db.clone());
    let row = service::get_admin(&repo, &id).await?;
    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateCreditNoteRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let repo = SqlxCreditNoteRepository::new(state.db.clone());
    let row = service::create_admin(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateCreditNoteRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxCreditNoteRepository::new(state.db.clone());
    let row = service::update_admin(&repo, &id, &body).await?;
    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxCreditNoteRepository::new(state.db.clone());
    let row = service::delete_admin(&repo, &id).await?;
    Ok(Json(row))
}
