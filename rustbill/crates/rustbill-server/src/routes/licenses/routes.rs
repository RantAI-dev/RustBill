use super::repository::SqlxLicensesRepository;
use super::schema::{
    CreateLicenseRequest, DeactivateLicenseQuery, ListLicensesQuery, UpdateLicenseRequest,
    VerifyLicenseRequest,
};
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};

/// Public routes — no session required.
pub fn public_router() -> Router<SharedState> {
    Router::new().route("/verify", post(verify))
}

/// Admin routes — session required (applied by caller).
pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/keypair", get(get_keypair).post(create_keypair))
        .route("/{key}", put(update).delete(remove))
        .route("/{key}/sign", post(sign))
        .route("/{key}/export", get(export))
        .route(
            "/{key}/activations",
            get(list_activations).delete(deactivate),
        )
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<ListLicensesQuery>,
) -> ApiResult<Json<Vec<rustbill_core::db::models::License>>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let rows = service::list(&repo, params.status.as_deref()).await?;
    Ok(Json(rows))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateLicenseRequest>,
) -> ApiResult<(StatusCode, Json<rustbill_core::db::models::License>)> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let row = service::create_legacy(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(key): Path<String>,
    Json(body): Json<UpdateLicenseRequest>,
) -> ApiResult<Json<rustbill_core::db::models::License>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let row = service::update_legacy(&repo, &key, &body).await?;
    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(key): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let row = service::remove_license(&repo, &key).await?;
    Ok(Json(row))
}

async fn verify(
    State(state): State<SharedState>,
    Json(body): Json<VerifyLicenseRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let row = service::verify_legacy(&repo, &body).await?;
    Ok(Json(row))
}

async fn sign(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(key): Path<String>,
) -> ApiResult<Json<super::schema::SignLicenseResponse>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let response = service::sign_license(&repo, &key).await?;
    Ok(Json(response))
}

async fn export(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(key): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let file_content = service::export_license_file(&repo, &key).await?;

    let filename = format!("license-{}.lic", key);
    let headers = [
        (
            axum::http::header::CONTENT_TYPE,
            "application/octet-stream".to_string(),
        ),
        (
            axum::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        ),
    ];

    Ok((headers, file_content))
}

async fn list_activations(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(key): Path<String>,
) -> ApiResult<Json<Vec<rustbill_core::db::models::LicenseActivation>>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let rows = service::list_activations(&repo, &key).await?;
    Ok(Json(rows))
}

async fn deactivate(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(key): Path<String>,
    Query(params): Query<DeactivateLicenseQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let response = service::deactivate(&repo, &key, &params.device_id).await?;
    Ok(Json(response))
}

async fn get_keypair(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<super::schema::KeypairStatusResponse>> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let response = service::get_keypair(&repo).await?;
    Ok(Json(response))
}

async fn create_keypair(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<(StatusCode, Json<super::schema::KeypairCreateResponse>)> {
    let repo = SqlxLicensesRepository::new(state.db.clone());
    let response = service::create_keypair(&repo).await?;
    Ok((StatusCode::CREATED, Json(response)))
}
