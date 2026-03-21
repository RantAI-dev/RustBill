use super::repository::AppCronRepository;
use super::schema::{
    DunningResponse, ExpireLicensesResponse, GenerateInvoicesResponse, LifecycleResponse,
    RunAllResponse,
};
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{extract::State, routing::post, Json, Router};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/run", post(run_all))
        .route("/renew-subscriptions", post(renew_subscriptions))
        .route("/generate-invoices", post(generate_invoices))
        .route("/process-dunning", post(process_dunning))
        .route("/expire-licenses", post(expire_licenses))
}

async fn run_all(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<RunAllResponse>> {
    tracing::info!("Running all cron jobs");
    let repo = AppCronRepository::new(
        state.db.clone(),
        state.email_sender.clone(),
        state.http_client.clone(),
    );
    let response = service::run_all(&repo).await?;
    Ok(Json(response))
}

async fn renew_subscriptions(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<LifecycleResponse>> {
    let repo = AppCronRepository::new(
        state.db.clone(),
        state.email_sender.clone(),
        state.http_client.clone(),
    );
    let response = service::renew_subscriptions(&repo).await?;
    Ok(Json(response))
}

async fn generate_invoices(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<GenerateInvoicesResponse>> {
    let repo = AppCronRepository::new(
        state.db.clone(),
        state.email_sender.clone(),
        state.http_client.clone(),
    );
    let response = service::generate_invoices(&repo).await?;
    Ok(Json(response))
}

async fn process_dunning(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<DunningResponse>> {
    let repo = AppCronRepository::new(
        state.db.clone(),
        state.email_sender.clone(),
        state.http_client.clone(),
    );
    let response = service::process_dunning(&repo).await?;
    Ok(Json(response))
}

async fn expire_licenses(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<ExpireLicensesResponse>> {
    let repo = AppCronRepository::new(
        state.db.clone(),
        state.email_sender.clone(),
        state.http_client.clone(),
    );
    let response = service::expire_licenses(&repo).await?;
    Ok(Json(response))
}
