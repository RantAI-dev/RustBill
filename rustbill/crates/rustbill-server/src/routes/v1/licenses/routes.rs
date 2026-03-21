use super::repository::SqlxLicensesRepository;
use super::schema::{
    CreateLicenseRequest, ListLicensesQuery, UpdateLicenseRequest, VerifyLicenseRequest,
};
use super::service;
use crate::app::SharedState;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/verify", post(verify))
        .route("/{key}", get(get_one).put(update).delete(remove))
        .route("/{key}/activations", get(list_activations))
}

async fn list(
    State(state): State<SharedState>,
    Query(params): Query<ListLicensesQuery>,
) -> ApiResult<Json<Vec<rustbill_core::db::models::License>>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let rows = service::list(&repo, params.status.as_deref()).await?;
    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    Path(key): Path<String>,
) -> ApiResult<Json<rustbill_core::db::models::License>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let row = service::get_one(&repo, &key).await?;
    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    Json(body): Json<CreateLicenseRequest>,
) -> ApiResult<(StatusCode, Json<rustbill_core::db::models::License>)> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let row = service::create(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    Path(key): Path<String>,
    Json(body): Json<UpdateLicenseRequest>,
) -> ApiResult<Json<rustbill_core::db::models::License>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let row = service::update(&repo, &key, &body).await?;
    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    Path(key): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let row = service::remove(&repo, &key).await?;
    Ok(Json(row))
}

async fn verify(
    State(state): State<SharedState>,
    Json(body): Json<VerifyLicenseRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let row = service::verify(&repo, &body).await?;
    Ok(Json(row))
}

async fn list_activations(
    State(state): State<SharedState>,
    Path(key): Path<String>,
) -> ApiResult<Json<Vec<rustbill_core::db::models::LicenseActivation>>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let rows = service::list_activations(&repo, &key).await?;
    Ok(Json(rows))
}
