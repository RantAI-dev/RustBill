use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use rustbill_core::billing::payment_methods;
use rustbill_core::db::models::{PaymentProvider, SavedPaymentMethodType};
use serde::Deserialize;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/setup", post(create_setup_session))
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetupRequest {
    customer_id: String,
    provider: PaymentProvider,
    success_url: Option<String>,
    cancel_url: Option<String>,
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

async fn create_setup_session(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<SetupRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let response = match body.provider {
        PaymentProvider::Stripe => {
            create_stripe_setup_session(
                &state,
                &body.customer_id,
                body.success_url.as_deref(),
                body.cancel_url.as_deref(),
            )
            .await?
        }
        PaymentProvider::Xendit => create_xendit_setup_session(&state, &body.customer_id).await?,
        PaymentProvider::Lemonsqueezy => {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "lemonsqueezy setup sessions are not supported; use LS-managed subscription checkout",
                })),
            ));
        }
    };

    Ok((StatusCode::OK, Json(response)))
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

async fn create_stripe_setup_session(
    state: &SharedState,
    customer_id: &str,
    success_url: Option<&str>,
    cancel_url: Option<&str>,
) -> ApiResult<serde_json::Value> {
    let secret = state.provider_cache.get("stripe_secret_key").await;
    if secret.is_empty() {
        return Err(rustbill_core::error::BillingError::ProviderNotConfigured(
            "stripe".to_string(),
        )
        .into());
    }

    let success = success_url.unwrap_or("https://example.com/billing/payment-methods/success");
    let cancel = cancel_url.unwrap_or("https://example.com/billing/payment-methods/cancel");

    let form = vec![
        ("mode", "setup".to_string()),
        ("success_url", success.to_string()),
        ("cancel_url", cancel.to_string()),
        ("payment_method_types[0]", "card".to_string()),
        ("metadata[customer_id]", customer_id.to_string()),
    ];

    let resp = reqwest::Client::new()
        .post("https://api.stripe.com/v1/checkout/sessions")
        .bearer_auth(secret)
        .form(&form)
        .send()
        .await
        .map_err(|e| {
            rustbill_core::error::BillingError::Internal(anyhow::anyhow!(
                "stripe setup request failed: {e}"
            ))
        })?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.map_err(|e| {
        rustbill_core::error::BillingError::Internal(anyhow::anyhow!(
            "stripe setup parse failed: {e}"
        ))
    })?;

    if !status.is_success() {
        let msg = body["error"]["message"]
            .as_str()
            .unwrap_or("stripe setup failed")
            .to_string();
        return Err(rustbill_core::error::BillingError::BadRequest(msg).into());
    }

    Ok(serde_json::json!({
        "provider": "stripe",
        "customerId": customer_id,
        "setupUrl": body["url"],
        "sessionId": body["id"],
    }))
}

async fn create_xendit_setup_session(
    state: &SharedState,
    customer_id: &str,
) -> ApiResult<serde_json::Value> {
    let secret = state.provider_cache.get("xendit_secret_key").await;
    if secret.is_empty() {
        return Err(rustbill_core::error::BillingError::ProviderNotConfigured(
            "xendit".to_string(),
        )
        .into());
    }

    let body = serde_json::json!({
        "type": "CARD",
        "reusability": "MULTIPLE_USE",
        "metadata": {
            "customer_id": customer_id,
        }
    });

    let resp = reqwest::Client::new()
        .post("https://api.xendit.co/payment_methods")
        .basic_auth(secret, Some(""))
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            rustbill_core::error::BillingError::Internal(anyhow::anyhow!(
                "xendit setup request failed: {e}"
            ))
        })?;

    let status = resp.status();
    let payload: serde_json::Value = resp.json().await.map_err(|e| {
        rustbill_core::error::BillingError::Internal(anyhow::anyhow!(
            "xendit setup parse failed: {e}"
        ))
    })?;

    if !status.is_success() {
        let msg = payload["message"]
            .as_str()
            .unwrap_or("xendit setup failed")
            .to_string();
        return Err(rustbill_core::error::BillingError::BadRequest(msg).into());
    }

    Ok(serde_json::json!({
        "provider": "xendit",
        "customerId": customer_id,
        "setupId": payload["id"],
        "actions": payload["actions"],
    }))
}
