use super::repository::SqlxPaymentMethodRepository;
use super::schema::{
    CreatePaymentMethodRequest, CustomerQuery, DeletePaymentMethodResponse,
    SetupPaymentMethodRequest, SetupPaymentMethodResponse, UnsupportedSetupResponse,
};
use super::service::{self, PaymentMethodSetupGateway};
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use rustbill_core::db::models::PaymentProvider;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/setup", post(create_setup_session))
        .route("/{id}", delete(remove))
        .route("/{id}/default", post(set_default))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(query): Query<CustomerQuery>,
) -> ApiResult<Json<Vec<rustbill_core::db::models::SavedPaymentMethod>>> {
    let customer_id = match query.customer_id {
        Some(customer_id) => customer_id,
        None => {
            return Err(
                rustbill_core::error::BillingError::bad_request("customerId is required").into(),
            )
        }
    };
    let repo = SqlxPaymentMethodRepository::new(state.db.clone());
    let methods = service::list_for_customer(&repo, &customer_id).await?;
    Ok(Json(methods))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreatePaymentMethodRequest>,
) -> ApiResult<Json<rustbill_core::db::models::SavedPaymentMethod>> {
    let repo = SqlxPaymentMethodRepository::new(state.db.clone());
    let method = service::create(&repo, &body).await?;
    Ok(Json(method))
}

async fn create_setup_session(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<SetupPaymentMethodRequest>,
) -> ApiResult<(StatusCode, Json<SetupResponse>)> {
    if matches!(body.provider, PaymentProvider::Lemonsqueezy) {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(SetupResponse::Unsupported(UnsupportedSetupResponse {
                error:
                    "lemonsqueezy setup sessions are not supported; use LS-managed subscription checkout"
                        .to_string(),
            })),
        ));
    }

    let gateway = HttpPaymentMethodSetupGateway::new(state.clone());
    let response = service::create_setup_session(&gateway, &body).await?;
    Ok((StatusCode::OK, Json(SetupResponse::Success(response))))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Query(query): Query<CustomerQuery>,
) -> ApiResult<Json<DeletePaymentMethodResponse>> {
    let repo = SqlxPaymentMethodRepository::new(state.db.clone());
    let deleted = service::remove(&repo, &id, query.customer_id.as_deref()).await?;
    Ok(Json(deleted))
}

async fn set_default(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Query(query): Query<CustomerQuery>,
) -> ApiResult<Json<rustbill_core::db::models::SavedPaymentMethod>> {
    let repo = SqlxPaymentMethodRepository::new(state.db.clone());
    let method = service::set_default(&repo, &id, query.customer_id.as_deref()).await?;
    Ok(Json(method))
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(untagged)]
enum SetupResponse {
    Success(SetupPaymentMethodResponse),
    Unsupported(UnsupportedSetupResponse),
}

#[derive(Clone)]
struct HttpPaymentMethodSetupGateway {
    state: SharedState,
}

impl HttpPaymentMethodSetupGateway {
    fn new(state: SharedState) -> Self {
        Self { state }
    }
}

fn error_message(value: &serde_json::Value, key: &str, fallback: &str) -> String {
    match value
        .get(key)
        .and_then(|entry| entry.get("message"))
        .and_then(serde_json::Value::as_str)
    {
        Some(message) => message.to_string(),
        None => fallback.to_string(),
    }
}

fn xendit_error_message(value: &serde_json::Value, fallback: &str) -> String {
    match value.get("message").and_then(serde_json::Value::as_str) {
        Some(message) => message.to_string(),
        None => fallback.to_string(),
    }
}

#[async_trait::async_trait]
impl PaymentMethodSetupGateway for HttpPaymentMethodSetupGateway {
    async fn create_stripe_setup_session(
        &self,
        customer_id: &str,
        success_url: &str,
        cancel_url: &str,
    ) -> Result<SetupPaymentMethodResponse, rustbill_core::error::BillingError> {
        let secret = self.state.provider_cache.get("stripe_secret_key").await;
        if secret.is_empty() {
            return Err(rustbill_core::error::BillingError::ProviderNotConfigured(
                "stripe".to_string(),
            ));
        }

        let form = vec![
            ("mode", "setup".to_string()),
            ("success_url", success_url.to_string()),
            ("cancel_url", cancel_url.to_string()),
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
            let msg = error_message(&body, "error", "stripe setup failed");
            return Err(rustbill_core::error::BillingError::BadRequest(msg));
        }

        Ok(SetupPaymentMethodResponse {
            provider: PaymentProvider::Stripe,
            customer_id: customer_id.to_string(),
            setup_url: body
                .get("url")
                .and_then(serde_json::Value::as_str)
                .map(std::borrow::ToOwned::to_owned),
            session_id: body
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(std::borrow::ToOwned::to_owned),
            setup_id: None,
            actions: None,
        })
    }

    async fn create_xendit_setup_session(
        &self,
        customer_id: &str,
    ) -> Result<SetupPaymentMethodResponse, rustbill_core::error::BillingError> {
        let secret = self.state.provider_cache.get("xendit_secret_key").await;
        if secret.is_empty() {
            return Err(rustbill_core::error::BillingError::ProviderNotConfigured(
                "xendit".to_string(),
            ));
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
            let msg = xendit_error_message(&payload, "xendit setup failed");
            return Err(rustbill_core::error::BillingError::BadRequest(msg));
        }

        Ok(SetupPaymentMethodResponse {
            provider: PaymentProvider::Xendit,
            customer_id: customer_id.to_string(),
            setup_url: None,
            session_id: None,
            setup_id: payload
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(std::borrow::ToOwned::to_owned),
            actions: payload.get("actions").cloned(),
        })
    }
}
