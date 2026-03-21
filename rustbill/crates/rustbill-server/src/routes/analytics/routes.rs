use super::repository::SqlxAnalyticsRepository;
use super::schema::{ForecastParams, ReportParams, Sales360Params};
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Query, State},
    http::header,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/overview", get(overview))
        .route("/forecasting", get(forecasting))
        .route("/reports", get(reports))
        .route("/sales-360/summary", get(sales_360_summary))
        .route("/sales-360/timeseries", get(sales_360_timeseries))
        .route("/sales-360/breakdown", get(sales_360_breakdown))
        .route("/sales-360/reconcile", get(sales_360_reconcile))
        .route("/sales-360/export", get(sales_360_export))
        .route("/sales-360/backfill", post(sales_360_backfill))
}

async fn overview(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxAnalyticsRepository::new(state.db.clone());
    let result = service::overview(&repo).await?;
    Ok(Json(result))
}

async fn forecasting(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<ForecastParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxAnalyticsRepository::new(state.db.clone());
    let result = service::forecasting(&repo, &params).await?;
    Ok(Json(result))
}

async fn reports(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<ReportParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxAnalyticsRepository::new(state.db.clone());
    let result = service::reports(&repo, &params).await?;
    Ok(Json(result))
}

async fn sales_360_summary(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<Sales360Params>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxAnalyticsRepository::new(state.db.clone());
    let result = service::sales_360_summary(&repo, &params).await?;
    Ok(Json(result))
}

async fn sales_360_timeseries(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<Sales360Params>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxAnalyticsRepository::new(state.db.clone());
    let result = service::sales_360_timeseries(&repo, &params).await?;
    Ok(Json(result))
}

async fn sales_360_breakdown(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<Sales360Params>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxAnalyticsRepository::new(state.db.clone());
    let result = service::sales_360_breakdown(&repo, &params).await?;
    Ok(Json(result))
}

async fn sales_360_reconcile(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<Sales360Params>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxAnalyticsRepository::new(state.db.clone());
    let result = service::sales_360_reconcile(&repo, &params).await?;
    Ok(Json(result))
}

async fn sales_360_export(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<Sales360Params>,
) -> ApiResult<impl IntoResponse> {
    let repo = SqlxAnalyticsRepository::new(state.db.clone());
    let csv = service::sales_360_export(&repo, &params).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"sales-360-export.csv\"",
            ),
        ],
        csv,
    ))
}

async fn sales_360_backfill(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxAnalyticsRepository::new(state.db.clone());
    let result = service::sales_360_backfill(&repo).await?;
    Ok(Json(result))
}
