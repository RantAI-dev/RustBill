use super::schema::CreatePaymentMethodRequestV1;
use async_trait::async_trait;
use rust_decimal::Decimal;
use rustbill_core::billing::{credits as core_credits, payment_methods as core_payment_methods};
use rustbill_core::db::models::{CustomerCredit, SavedPaymentMethod};
use rustbill_core::error::BillingError;
use rustbill_core::settings::provider_settings::ProviderSettingsCache;
use sqlx::PgPool;
use std::sync::Arc;

#[async_trait]
pub trait BillingRepository: Send + Sync {
    async fn list_payment_methods(
        &self,
        customer_id: &str,
    ) -> std::result::Result<Vec<SavedPaymentMethod>, BillingError>;
    async fn create_payment_method(
        &self,
        req: &CreatePaymentMethodRequestV1,
    ) -> std::result::Result<SavedPaymentMethod, BillingError>;
    async fn find_payment_method_customer_id(
        &self,
        method_id: &str,
    ) -> std::result::Result<Option<String>, BillingError>;
    async fn remove_payment_method(
        &self,
        customer_id: &str,
        method_id: &str,
    ) -> std::result::Result<(), BillingError>;
    async fn set_default_payment_method(
        &self,
        customer_id: &str,
        method_id: &str,
    ) -> std::result::Result<SavedPaymentMethod, BillingError>;
    async fn create_stripe_setup_session(
        &self,
        customer_id: &str,
        success_url: &str,
        cancel_url: &str,
    ) -> std::result::Result<serde_json::Value, BillingError>;
    async fn create_xendit_setup_session(
        &self,
        customer_id: &str,
    ) -> std::result::Result<serde_json::Value, BillingError>;
    async fn get_credit_balance(
        &self,
        customer_id: &str,
        currency: &str,
    ) -> std::result::Result<Decimal, BillingError>;
    async fn list_credits(
        &self,
        customer_id: &str,
        currency: Option<&str>,
    ) -> std::result::Result<Vec<CustomerCredit>, BillingError>;
}

#[derive(Clone)]
pub struct SqlxBillingRepository {
    pool: PgPool,
    provider_cache: Arc<ProviderSettingsCache>,
    http_client: reqwest::Client,
}

impl SqlxBillingRepository {
    pub fn new(
        pool: PgPool,
        provider_cache: Arc<ProviderSettingsCache>,
        http_client: reqwest::Client,
    ) -> Self {
        Self {
            pool,
            provider_cache,
            http_client,
        }
    }
}

#[async_trait]
impl BillingRepository for SqlxBillingRepository {
    async fn list_payment_methods(
        &self,
        customer_id: &str,
    ) -> std::result::Result<Vec<SavedPaymentMethod>, BillingError> {
        core_payment_methods::list_for_customer(&self.pool, customer_id).await
    }

    async fn create_payment_method(
        &self,
        req: &CreatePaymentMethodRequestV1,
    ) -> std::result::Result<SavedPaymentMethod, BillingError> {
        core_payment_methods::create(
            &self.pool,
            core_payment_methods::CreatePaymentMethodRequest {
                customer_id: req.customer_id.clone(),
                provider: req.provider.clone(),
                provider_token: req.provider_token.clone(),
                method_type: req.method_type.clone(),
                label: req.label.clone(),
                last_four: req.last_four.clone(),
                expiry_month: req.expiry_month,
                expiry_year: req.expiry_year,
                set_default: req.set_default,
            },
        )
        .await
    }

    async fn find_payment_method_customer_id(
        &self,
        method_id: &str,
    ) -> std::result::Result<Option<String>, BillingError> {
        sqlx::query_scalar::<_, String>(
            "SELECT customer_id FROM saved_payment_methods WHERE id = $1",
        )
        .bind(method_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn remove_payment_method(
        &self,
        customer_id: &str,
        method_id: &str,
    ) -> std::result::Result<(), BillingError> {
        core_payment_methods::remove(&self.pool, customer_id, method_id).await
    }

    async fn set_default_payment_method(
        &self,
        customer_id: &str,
        method_id: &str,
    ) -> std::result::Result<SavedPaymentMethod, BillingError> {
        core_payment_methods::set_default(&self.pool, customer_id, method_id).await
    }

    async fn create_stripe_setup_session(
        &self,
        customer_id: &str,
        success_url: &str,
        cancel_url: &str,
    ) -> std::result::Result<serde_json::Value, BillingError> {
        let secret = self.provider_cache.get("stripe_secret_key").await;
        if secret.is_empty() {
            return Err(BillingError::ProviderNotConfigured("stripe".to_string()));
        }

        let form = vec![
            ("mode", "setup".to_string()),
            ("success_url", success_url.to_string()),
            ("cancel_url", cancel_url.to_string()),
            ("payment_method_types[0]", "card".to_string()),
            ("metadata[customer_id]", customer_id.to_string()),
        ];

        let response = self
            .http_client
            .post("https://api.stripe.com/v1/checkout/sessions")
            .bearer_auth(secret)
            .form(&form)
            .send()
            .await
            .map_err(|error| {
                BillingError::Internal(anyhow::anyhow!("stripe setup request failed: {error}"))
            })?;

        let status = response.status();
        let body: serde_json::Value = response.json().await.map_err(|error| {
            BillingError::Internal(anyhow::anyhow!("stripe setup parse failed: {error}"))
        })?;

        if !status.is_success() {
            let msg = body
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("stripe setup failed");
            return Err(BillingError::BadRequest(msg.to_string()));
        }

        Ok(serde_json::json!({
            "provider": "stripe",
            "customerId": customer_id,
            "setupUrl": body.get("url").cloned().unwrap_or(serde_json::Value::Null),
            "sessionId": body.get("id").cloned().unwrap_or(serde_json::Value::Null),
        }))
    }

    async fn create_xendit_setup_session(
        &self,
        customer_id: &str,
    ) -> std::result::Result<serde_json::Value, BillingError> {
        let secret = self.provider_cache.get("xendit_secret_key").await;
        if secret.is_empty() {
            return Err(BillingError::ProviderNotConfigured("xendit".to_string()));
        }

        let body = serde_json::json!({
            "type": "CARD",
            "reusability": "MULTIPLE_USE",
            "metadata": {
                "customer_id": customer_id,
            }
        });

        let response = self
            .http_client
            .post("https://api.xendit.co/payment_methods")
            .basic_auth(secret, Some(""))
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                BillingError::Internal(anyhow::anyhow!("xendit setup request failed: {error}"))
            })?;

        let status = response.status();
        let payload: serde_json::Value = response.json().await.map_err(|error| {
            BillingError::Internal(anyhow::anyhow!("xendit setup parse failed: {error}"))
        })?;

        if !status.is_success() {
            let msg = payload
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("xendit setup failed");
            return Err(BillingError::BadRequest(msg.to_string()));
        }

        Ok(serde_json::json!({
            "provider": "xendit",
            "customerId": customer_id,
            "setupId": payload.get("id").cloned().unwrap_or(serde_json::Value::Null),
            "actions": payload.get("actions").cloned().unwrap_or(serde_json::Value::Null),
        }))
    }

    async fn get_credit_balance(
        &self,
        customer_id: &str,
        currency: &str,
    ) -> std::result::Result<Decimal, BillingError> {
        core_credits::get_balance(&self.pool, customer_id, currency).await
    }

    async fn list_credits(
        &self,
        customer_id: &str,
        currency: Option<&str>,
    ) -> std::result::Result<Vec<CustomerCredit>, BillingError> {
        core_credits::list_credits(&self.pool, customer_id, currency).await
    }
}
