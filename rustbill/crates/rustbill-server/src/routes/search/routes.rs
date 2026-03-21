use super::repository::SqlxSearchRepository;
use super::schema::SearchParams;
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new().route("/", get(search))
}

async fn search(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<SearchParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxSearchRepository::new(state.db.clone());
    let result = service::search(&repo, &params).await?;
    Ok(Json(result))
}
