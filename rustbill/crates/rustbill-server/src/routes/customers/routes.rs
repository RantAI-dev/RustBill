use super::repository::SqlxCustomerRepository;
use super::schema::{CreateCustomerRequest, DeleteCustomerResponse, UpdateCustomerRequest};
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
        .route("/", get(list_admin).post(create_admin))
        .route(
            "/{id}",
            get(get_admin).put(update_admin).delete(remove_admin),
        )
}

pub fn v1_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_v1).post(create_v1))
        .route("/{id}", get(get_v1).put(update_v1).delete(remove_v1))
}

async fn list_admin(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<rustbill_core::db::models::Customer>>> {
    let repo = SqlxCustomerRepository::new(state.db.clone());
    let rows = service::list(&repo).await?;
    Ok(Json(rows))
}

async fn list_v1(
    State(state): State<SharedState>,
) -> ApiResult<Json<Vec<rustbill_core::db::models::Customer>>> {
    let repo = SqlxCustomerRepository::new(state.db.clone());
    let rows = service::list(&repo).await?;
    Ok(Json(rows))
}

async fn get_admin(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<rustbill_core::db::models::Customer>> {
    let repo = SqlxCustomerRepository::new(state.db.clone());
    let row = service::get(&repo, &id).await?;
    Ok(Json(row))
}

async fn get_v1(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Json<rustbill_core::db::models::Customer>> {
    let repo = SqlxCustomerRepository::new(state.db.clone());
    let row = service::get(&repo, &id).await?;
    Ok(Json(row))
}

async fn create_admin(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateCustomerRequest>,
) -> ApiResult<(StatusCode, Json<rustbill_core::db::models::Customer>)> {
    let repo = SqlxCustomerRepository::new(state.db.clone());
    let row = service::create(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn create_v1(
    State(state): State<SharedState>,
    Json(body): Json<CreateCustomerRequest>,
) -> ApiResult<(StatusCode, Json<rustbill_core::db::models::Customer>)> {
    let repo = SqlxCustomerRepository::new(state.db.clone());
    let row = service::create(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn update_admin(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateCustomerRequest>,
) -> ApiResult<Json<rustbill_core::db::models::Customer>> {
    let repo = SqlxCustomerRepository::new(state.db.clone());
    let row = service::update(&repo, &id, &body).await?;
    Ok(Json(row))
}

async fn update_v1(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateCustomerRequest>,
) -> ApiResult<Json<rustbill_core::db::models::Customer>> {
    let repo = SqlxCustomerRepository::new(state.db.clone());
    let row = service::update(&repo, &id, &body).await?;
    Ok(Json(row))
}

async fn remove_admin(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteCustomerResponse>> {
    let repo = SqlxCustomerRepository::new(state.db.clone());
    let row = service::delete(&repo, &id).await?;
    Ok(Json(row))
}

async fn remove_v1(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteCustomerResponse>> {
    let repo = SqlxCustomerRepository::new(state.db.clone());
    let row = service::delete(&repo, &id).await?;
    Ok(Json(row))
}
