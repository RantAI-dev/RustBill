use super::repository::SqlxOneTimeSalesRepository;
use super::schema::{CreateOneTimeSaleRequest, OneTimeSalesListParams, UpdateOneTimeSaleRequest};
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use rustbill_core::auth::api_key::ApiKeyInfo;
use rustbill_core::error::BillingError;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
}

pub fn v1_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_v1).post(create_v1))
        .route("/{id}", get(get_one_v1).put(update_v1).delete(remove_v1))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let repo = SqlxOneTimeSalesRepository::new(state.db.clone());
    let rows = service::list_admin(&repo).await?;
    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxOneTimeSalesRepository::new(state.db.clone());
    let row = service::get_admin(&repo, &id).await?;
    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateOneTimeSaleRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let repo = SqlxOneTimeSalesRepository::new(state.db.clone());
    let row = service::create_admin(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateOneTimeSaleRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxOneTimeSalesRepository::new(state.db.clone());
    let row = service::update_admin(&repo, &id, &body).await?;
    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxOneTimeSalesRepository::new(state.db.clone());
    let row = service::delete_admin(&repo, &id).await?;
    Ok(Json(row))
}

async fn list_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Query(params): Query<OneTimeSalesListParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let customer_id = scoped_customer_id(&api_key, params.customer_id.as_deref())?;
    let repo = SqlxOneTimeSalesRepository::new(state.db.clone());
    let rows = service::list_scoped(&repo, params.status.as_deref(), &customer_id).await?;
    Ok(Json(rows))
}

async fn get_one_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let customer_id = api_key
        .customer_id
        .as_deref()
        .ok_or(BillingError::Forbidden)?
        .to_string();
    let repo = SqlxOneTimeSalesRepository::new(state.db.clone());
    let row = service::get_scoped(&repo, &id, &customer_id).await?;
    Ok(Json(row))
}

async fn create_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Json(mut body): Json<CreateOneTimeSaleRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let customer_id = scoped_customer_id(&api_key, Some(&body.customer_id))?;
    body.customer_id = customer_id.clone();
    let repo = SqlxOneTimeSalesRepository::new(state.db.clone());
    let row = service::create_scoped(&repo, &customer_id, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn update_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Path(id): Path<String>,
    Json(body): Json<UpdateOneTimeSaleRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let customer_id = api_key
        .customer_id
        .as_deref()
        .ok_or(BillingError::Forbidden)?
        .to_string();
    let repo = SqlxOneTimeSalesRepository::new(state.db.clone());
    let row = service::update_scoped(&repo, &id, &customer_id, &body).await?;
    Ok(Json(row))
}

async fn remove_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let customer_id = api_key
        .customer_id
        .as_deref()
        .ok_or(BillingError::Forbidden)?
        .to_string();
    let repo = SqlxOneTimeSalesRepository::new(state.db.clone());
    let row = service::delete_scoped(&repo, &id, &customer_id).await?;
    Ok(Json(row))
}

fn scoped_customer_id(api_key: &ApiKeyInfo, requested: Option<&str>) -> ApiResult<String> {
    let scoped = api_key
        .customer_id
        .as_deref()
        .ok_or(BillingError::Forbidden)?;

    if let Some(requested) = requested {
        if requested != scoped {
            return Err(BillingError::Forbidden.into());
        }
    }

    Ok(scoped.to_string())
}
