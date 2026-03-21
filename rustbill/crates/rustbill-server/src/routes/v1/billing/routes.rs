use super::repository::SqlxBillingRepository;
use super::schema::{
    CreatePaymentMethodRequestV1, CreditsQueryV1, DeletePaymentMethodResponse,
    PaymentMethodCustomerQuery, PaymentMethodSetupRequestV1,
};
use super::service;
use crate::app::SharedState;
use crate::routes::ApiResult;
use axum::{
    extract::{Extension, Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use rustbill_core::auth::api_key::ApiKeyInfo;
use rustbill_core::db::models::SavedPaymentMethod;

pub fn router() -> Router<SharedState> {
    Router::new()
        .nest("/invoices", invoices_router())
        .nest("/one-time-sales", one_time_sales_router())
        .nest("/subscriptions", subscriptions_router())
        .nest("/payment-methods", payment_methods_router())
        .nest("/credits", credits_router())
        .nest("/usage", usage_router())
}

fn invoices_router() -> Router<SharedState> {
    crate::routes::billing::invoices::v1_router()
}

fn one_time_sales_router() -> Router<SharedState> {
    crate::routes::billing::one_time_sales::v1_router()
}

fn subscriptions_router() -> Router<SharedState> {
    crate::routes::billing::subscriptions::v1_router()
}

fn usage_router() -> Router<SharedState> {
    crate::routes::billing::usage::v1_router()
}

fn payment_methods_router() -> Router<SharedState> {
    Router::new()
        .route(
            "/",
            get(list_payment_methods_v1).post(create_payment_method_v1),
        )
        .route("/setup", post(create_payment_method_setup_v1))
        .route("/{id}", delete(delete_payment_method_v1))
        .route("/{id}/default", post(set_default_payment_method_v1))
}

fn credits_router() -> Router<SharedState> {
    Router::new().route("/", get(get_credits_v1))
}

async fn list_payment_methods_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Query(query): Query<PaymentMethodCustomerQuery>,
) -> ApiResult<Json<Vec<SavedPaymentMethod>>> {
    let repo = SqlxBillingRepository::new(
        state.db.clone(),
        state.provider_cache.clone(),
        state.http_client.clone(),
    );
    let methods =
        service::list_payment_methods(&repo, &api_key, query.customer_id.as_deref()).await?;
    Ok(Json(methods))
}

async fn create_payment_method_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Json(body): Json<CreatePaymentMethodRequestV1>,
) -> ApiResult<Json<SavedPaymentMethod>> {
    let repo = SqlxBillingRepository::new(
        state.db.clone(),
        state.provider_cache.clone(),
        state.http_client.clone(),
    );
    let method = service::create_payment_method(&repo, &api_key, &body).await?;
    Ok(Json(method))
}

async fn create_payment_method_setup_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Query(_query): Query<PaymentMethodCustomerQuery>,
    Json(body): Json<PaymentMethodSetupRequestV1>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxBillingRepository::new(
        state.db.clone(),
        state.provider_cache.clone(),
        state.http_client.clone(),
    );
    let response = service::create_payment_method_setup(&repo, &api_key, &body).await?;
    Ok(Json(response))
}

async fn delete_payment_method_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Path(id): Path<String>,
    Query(query): Query<PaymentMethodCustomerQuery>,
) -> ApiResult<Json<DeletePaymentMethodResponse>> {
    let repo = SqlxBillingRepository::new(
        state.db.clone(),
        state.provider_cache.clone(),
        state.http_client.clone(),
    );
    let response =
        service::delete_payment_method(&repo, &api_key, &id, query.customer_id.as_deref()).await?;
    Ok(Json(response))
}

async fn set_default_payment_method_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Path(id): Path<String>,
    Query(query): Query<PaymentMethodCustomerQuery>,
) -> ApiResult<Json<SavedPaymentMethod>> {
    let repo = SqlxBillingRepository::new(
        state.db.clone(),
        state.provider_cache.clone(),
        state.http_client.clone(),
    );
    let method =
        service::set_default_payment_method(&repo, &api_key, &id, query.customer_id.as_deref())
            .await?;
    Ok(Json(method))
}

async fn get_credits_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Query(query): Query<CreditsQueryV1>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxBillingRepository::new(
        state.db.clone(),
        state.provider_cache.clone(),
        state.http_client.clone(),
    );
    let response = service::get_credits(&repo, &api_key, &query).await?;
    Ok(Json(response))
}
