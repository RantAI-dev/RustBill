use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use rustbill_core::billing::payment_methods;
use rustbill_core::db::models::{PaymentProvider, SavedPaymentMethodType};
use serde::Deserialize;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", delete(remove))
        .route("/{id}/default", post(set_default))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomerQuery {
    customer_id: Option<String>,
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(query): Query<CustomerQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let customer_id = query
        .customer_id
        .ok_or_else(|| rustbill_core::error::BillingError::bad_request("customerId is required"))?;
    let methods = payment_methods::list_for_customer(&state.db, &customer_id).await?;
    Ok(Json(serde_json::to_value(methods).map_err(|e| {
        rustbill_core::error::BillingError::Internal(e.into())
    })?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRequest {
    customer_id: String,
    provider: PaymentProvider,
    provider_token: String,
    method_type: SavedPaymentMethodType,
    label: String,
    last_four: Option<String>,
    expiry_month: Option<i32>,
    expiry_year: Option<i32>,
    #[serde(default)]
    set_default: bool,
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let method = payment_methods::create(
        &state.db,
        payment_methods::CreatePaymentMethodRequest {
            customer_id: body.customer_id,
            provider: body.provider,
            provider_token: body.provider_token,
            method_type: body.method_type,
            label: body.label,
            last_four: body.last_four,
            expiry_month: body.expiry_month,
            expiry_year: body.expiry_year,
            set_default: body.set_default,
        },
    )
    .await?;
    Ok(Json(serde_json::to_value(method).map_err(|e| {
        rustbill_core::error::BillingError::Internal(e.into())
    })?))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Query(query): Query<CustomerQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let customer_id = resolve_customer_id(&state.db, &id, query.customer_id.as_deref()).await?;
    payment_methods::remove(&state.db, &customer_id, &id).await?;
    Ok(Json(serde_json::json!({"deleted": true})))
}

async fn set_default(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Query(query): Query<CustomerQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let customer_id = resolve_customer_id(&state.db, &id, query.customer_id.as_deref()).await?;
    let method = payment_methods::set_default(&state.db, &customer_id, &id).await?;
    Ok(Json(serde_json::to_value(method).map_err(|e| {
        rustbill_core::error::BillingError::Internal(e.into())
    })?))
}

async fn resolve_customer_id(
    pool: &sqlx::PgPool,
    method_id: &str,
    provided: Option<&str>,
) -> ApiResult<String> {
    if let Some(customer_id) = provided {
        return Ok(customer_id.to_string());
    }

    let customer_id = sqlx::query_scalar::<_, String>(
        "SELECT customer_id FROM saved_payment_methods WHERE id = $1",
    )
    .bind(method_id)
    .fetch_optional(pool)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::not_found("payment_method", method_id))?;

    Ok(customer_id)
}
