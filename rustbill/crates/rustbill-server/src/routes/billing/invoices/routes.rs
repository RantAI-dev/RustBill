use super::repository::SqlxInvoiceRepository;
use super::schema::{
    CreateInvoiceRequest, InvoiceItemInput, InvoiceListParams, UpdateInvoiceRequest,
};
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
        .route("/{id}/items", get(list_items).post(add_item))
        .route("/{id}/pdf", get(get_pdf))
}

pub fn v1_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_v1))
        .route("/{id}", get(get_one_v1))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let repo = SqlxInvoiceRepository::new(state.db.clone());
    let rows = service::list_admin(&repo).await?;
    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxInvoiceRepository::new(state.db.clone());
    let row = service::get_admin(&repo, &id).await?;
    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateInvoiceRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let repo = SqlxInvoiceRepository::new(state.db.clone());
    let row = service::create_admin(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateInvoiceRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxInvoiceRepository::new(state.db.clone());
    let row = service::update_admin(&repo, &id, &body).await?;
    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxInvoiceRepository::new(state.db.clone());
    let row = service::delete_admin(&repo, &id).await?;
    Ok(Json(row))
}

async fn list_items(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let repo = SqlxInvoiceRepository::new(state.db.clone());
    let rows = service::list_items(&repo, &id).await?;
    Ok(Json(rows))
}

async fn add_item(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<InvoiceItemInput>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let repo = SqlxInvoiceRepository::new(state.db.clone());
    let row = service::add_item(&repo, &id, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn get_pdf(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let repo = SqlxInvoiceRepository::new(state.db.clone());
    let (pdf_bytes, invoice_number) = service::get_pdf(&repo, &id).await?;

    let filename = invoice_number
        .map(|n| format!("invoice-{n}.pdf"))
        .unwrap_or_else(|| format!("invoice-{id}.pdf"));

    let headers = [
        (header::CONTENT_TYPE, "application/pdf".to_string()),
        (
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{filename}\""),
        ),
    ];

    Ok((headers, pdf_bytes))
}

async fn list_v1(
    State(state): State<SharedState>,
    Query(params): Query<InvoiceListParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let repo = SqlxInvoiceRepository::new(state.db.clone());
    let rows = service::list_v1(
        &repo,
        params.status.as_deref(),
        params.customer_id.as_deref(),
    )
    .await?;
    Ok(Json(rows))
}

async fn get_one_v1(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxInvoiceRepository::new(state.db.clone());
    let row = service::get_v1(&repo, &id).await?;
    Ok(Json(row))
}
