use crate::app::SharedState;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use rustbill_core::db::models::Product;

use super::repository::SqlxProductsRepository;
use super::schema::ProductListItem;
use super::service;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list))
        .route("/{id}", get(get_one))
}

async fn list(State(state): State<SharedState>) -> ApiResult<Json<Vec<ProductListItem>>> {
    let repo = SqlxProductsRepository::new(state.db.clone());
    let products = service::list(&repo).await?;
    Ok(Json(products))
}

async fn get_one(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Product>> {
    let repo = SqlxProductsRepository::new(state.db.clone());
    let product = service::get(&repo, &id).await?;
    Ok(Json(product))
}
