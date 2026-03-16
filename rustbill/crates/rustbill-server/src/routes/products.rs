use axum::{extract::{Path, State}, http::StatusCode, routing::{delete, get, post, put}, Json, Router};
use crate::app::SharedState;
use crate::extractors::{AdminUser, ValidatedJson};
use rustbill_core::products::validation::{CreateProductRequest, UpdateProductRequest};
use super::ApiResult;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let products = rustbill_core::products::list_products(&state.db).await?;
    Ok(Json(products))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let product = rustbill_core::products::get_product(&state.db, &id).await?;
    Ok(Json(serde_json::to_value(product).unwrap()))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    ValidatedJson(req): ValidatedJson<CreateProductRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let product = rustbill_core::products::create_product(&state.db, req).await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(product).unwrap())))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<UpdateProductRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let product = rustbill_core::products::update_product(&state.db, &id, req).await?;
    Ok(Json(serde_json::to_value(product).unwrap()))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    rustbill_core::products::delete_product(&state.db, &id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}
