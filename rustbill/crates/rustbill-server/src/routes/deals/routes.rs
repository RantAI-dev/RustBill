use super::repository::SqlxDealsRepository;
use super::schema::{CreateDealRequest, DealListQuery, DeleteDealResponse, UpdateDealRequest};
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    http::{header::HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
    routing::get,
    Json, Router,
};

const ADMIN_LEGACY_LINK_HEADER: &str = "</api/billing/invoices>; rel=\"successor-version\", </api/billing/subscriptions>; rel=\"successor-version\", </api/billing/usage>; rel=\"successor-version\"";
const V1_LEGACY_LINK_HEADER: &str = "</api/v1/billing/subscriptions>; rel=\"successor-version\", </api/v1/billing/usage>; rel=\"successor-version\", </api/v1/billing/invoices>; rel=\"successor-version\"";
const LEGACY_SUNSET: &str = "Wed, 31 Dec 2026 23:59:59 GMT";

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_admin).post(create_admin))
        .route(
            "/{id}",
            get(get_admin).put(update_admin).delete(remove_admin),
        )
        .layer(axum::middleware::from_fn(add_admin_deprecation_headers))
}

pub fn v1_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_v1).post(create_v1))
        .route("/{id}", get(get_v1).put(update_v1).delete(remove_v1))
        .layer(axum::middleware::from_fn(add_v1_deprecation_headers))
}

fn set_legacy_headers(res: &mut Response, link_header: &'static str) {
    let headers = res.headers_mut();
    headers.insert(
        HeaderName::from_static("deprecation"),
        HeaderValue::from_static("true"),
    );
    headers.insert(
        HeaderName::from_static("sunset"),
        HeaderValue::from_static(LEGACY_SUNSET),
    );
    headers.insert(
        HeaderName::from_static("link"),
        HeaderValue::from_static(link_header),
    );
    headers.insert(
        HeaderName::from_static("x-rustbill-legacy"),
        HeaderValue::from_static("deals"),
    );
}

async fn add_admin_deprecation_headers(req: Request<axum::body::Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    set_legacy_headers(&mut response, ADMIN_LEGACY_LINK_HEADER);
    response
}

async fn add_v1_deprecation_headers(req: Request<axum::body::Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    set_legacy_headers(&mut response, V1_LEGACY_LINK_HEADER);
    response
}

async fn list_admin(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(query): Query<DealListQuery>,
) -> ApiResult<Json<Vec<rustbill_core::db::models::Deal>>> {
    let repo = SqlxDealsRepository::new(state.db.clone());
    let rows = service::list(&repo, &query).await?;
    Ok(Json(rows))
}

async fn list_v1(
    State(state): State<SharedState>,
    Query(query): Query<DealListQuery>,
) -> ApiResult<Json<Vec<rustbill_core::db::models::Deal>>> {
    let repo = SqlxDealsRepository::new(state.db.clone());
    let rows = service::list(&repo, &query).await?;
    Ok(Json(rows))
}

async fn get_admin(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<rustbill_core::db::models::Deal>> {
    let repo = SqlxDealsRepository::new(state.db.clone());
    let row = service::get(&repo, &id).await?;
    Ok(Json(row))
}

async fn get_v1(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Json<rustbill_core::db::models::Deal>> {
    let repo = SqlxDealsRepository::new(state.db.clone());
    let row = service::get(&repo, &id).await?;
    Ok(Json(row))
}

async fn create_admin(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateDealRequest>,
) -> ApiResult<(StatusCode, Json<rustbill_core::db::models::Deal>)> {
    let repo = SqlxDealsRepository::new(state.db.clone());
    let row = service::create(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn create_v1(
    State(state): State<SharedState>,
    Json(body): Json<CreateDealRequest>,
) -> ApiResult<(StatusCode, Json<rustbill_core::db::models::Deal>)> {
    let repo = SqlxDealsRepository::new(state.db.clone());
    let row = service::create(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn update_admin(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateDealRequest>,
) -> ApiResult<Json<rustbill_core::db::models::Deal>> {
    let repo = SqlxDealsRepository::new(state.db.clone());
    let row = service::update(&repo, &id, &body).await?;
    Ok(Json(row))
}

async fn update_v1(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateDealRequest>,
) -> ApiResult<Json<rustbill_core::db::models::Deal>> {
    let repo = SqlxDealsRepository::new(state.db.clone());
    let row = service::update(&repo, &id, &body).await?;
    Ok(Json(row))
}

async fn remove_admin(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteDealResponse>> {
    let repo = SqlxDealsRepository::new(state.db.clone());
    let row = service::delete(&repo, &id).await?;
    Ok(Json(row))
}

async fn remove_v1(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteDealResponse>> {
    let repo = SqlxDealsRepository::new(state.db.clone());
    let row = service::delete(&repo, &id).await?;
    Ok(Json(row))
}
