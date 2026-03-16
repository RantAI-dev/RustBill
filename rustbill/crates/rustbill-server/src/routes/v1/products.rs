use crate::app::SharedState;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list))
        .route("/{id}", get(get_one))
}

async fn list(State(state): State<SharedState>) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let products = rustbill_core::products::list_products(&state.db).await?;
    Ok(Json(products))
}

async fn get_one(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let product = rustbill_core::products::get_product(&state.db, &id).await?;
    Ok(Json(serde_json::to_value(product).unwrap()))
}
