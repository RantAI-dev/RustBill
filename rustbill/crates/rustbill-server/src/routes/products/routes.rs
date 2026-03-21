use super::repository::SqlxProductsRepository;
use super::schema::{CreateProductRequest, ProductListItem, UpdateProductRequest};
use super::service;
use crate::app::SharedState;
use crate::extractors::{AdminUser, ValidatedJson};
use crate::routes::ApiResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use rustbill_core::db::models::Product;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<ProductListItem>>> {
    let repo = SqlxProductsRepository::new(state.db.clone());
    let rows = service::list(&repo).await?;
    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Product>> {
    let repo = SqlxProductsRepository::new(state.db.clone());
    let product = service::get(&repo, &id).await?;
    Ok(Json(product))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    ValidatedJson(body): ValidatedJson<CreateProductRequest>,
) -> ApiResult<(StatusCode, Json<Product>)> {
    let repo = SqlxProductsRepository::new(state.db.clone());
    let product = service::create(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(product)))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    ValidatedJson(body): ValidatedJson<UpdateProductRequest>,
) -> ApiResult<Json<Product>> {
    let repo = SqlxProductsRepository::new(state.db.clone());
    let product = service::update(&repo, &id, &body).await?;
    Ok(Json(product))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxProductsRepository::new(state.db.clone());
    let result = service::delete(&repo, &id).await?;
    Ok(Json(result))
}
