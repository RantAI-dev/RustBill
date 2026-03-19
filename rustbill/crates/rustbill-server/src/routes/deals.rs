use super::ApiResult;
use crate::app::SharedState;
use crate::extractors::{AdminUser, ValidatedJson};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    http::{header::HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
    routing::get,
    Json, Router,
};
use rustbill_core::deals::validation::{CreateDealRequest, UpdateDealRequest};

const LEGACY_LINK_HEADER: &str = "</api/billing/invoices>; rel=\"successor-version\", </api/billing/subscriptions>; rel=\"successor-version\", </api/billing/usage>; rel=\"successor-version\"";
const LEGACY_SUNSET: &str = "Wed, 31 Dec 2026 23:59:59 GMT";

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
        .layer(axum::middleware::from_fn(add_deprecation_headers))
}

fn set_legacy_headers(res: &mut Response) {
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
        HeaderValue::from_static(LEGACY_LINK_HEADER),
    );
    headers.insert(
        HeaderName::from_static("x-rustbill-legacy"),
        HeaderValue::from_static("deals"),
    );
}

async fn add_deprecation_headers(req: Request<axum::body::Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    set_legacy_headers(&mut response);
    response
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListParams {
    product_type: Option<String>,
    deal_type: Option<String>,
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = rustbill_core::deals::list_deals(
        &state.db,
        params.product_type.as_deref(),
        params.deal_type.as_deref(),
    )
    .await?;

    let rows = rows
        .into_iter()
        .map(|d| serde_json::to_value(d).expect("deal should serialize"))
        .collect();

    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let deal = rustbill_core::deals::get_deal(&state.db, &id).await?;
    let row = serde_json::to_value(deal).expect("deal should serialize");

    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    ValidatedJson(req): ValidatedJson<CreateDealRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let deal = rustbill_core::deals::create_deal(&state.db, req).await?;
    let row = serde_json::to_value(deal).expect("deal should serialize");

    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<UpdateDealRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let deal = rustbill_core::deals::update_deal(&state.db, &id, req).await?;
    let row = serde_json::to_value(deal).expect("deal should serialize");

    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    rustbill_core::deals::delete_deal(&state.db, &id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
