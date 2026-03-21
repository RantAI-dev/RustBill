use super::schema::CheckoutResult;
use async_trait::async_trait;
use rustbill_core::error::BillingError;
use std::sync::Arc;

#[async_trait]
pub trait CheckoutRepository: Send + Sync {
    async fn create_checkout(
        &self,
        invoice_id: &str,
        provider: &str,
        origin: &str,
    ) -> Result<CheckoutResult, BillingError>;
}

#[derive(Clone)]
pub struct SqlxCheckoutRepository {
    state: Arc<crate::app::AppState>,
}

impl SqlxCheckoutRepository {
    pub fn new(state: crate::app::SharedState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl CheckoutRepository for SqlxCheckoutRepository {
    async fn create_checkout(
        &self,
        invoice_id: &str,
        provider: &str,
        origin: &str,
    ) -> Result<CheckoutResult, BillingError> {
        let provider_keys: &[&str] = match provider {
            "stripe" => &["stripe_secret_key"],
            "xendit" => &["xendit_secret_key"],
            "lemonsqueezy" => &["lemonsqueezy_api_key", "lemonsqueezy_store_id"],
            _ => &[],
        };

        let settings = self
            .state
            .provider_cache
            .get_provider_settings(provider_keys)
            .await;
        let result = rustbill_core::billing::checkout::create_checkout(
            &self.state.db,
            &self.state.http_client,
            &settings,
            invoice_id,
            provider,
            origin,
        )
        .await?;

        Ok(CheckoutResult {
            invoice_id: invoice_id.to_string(),
            provider: result.provider,
            checkout_url: result.checkout_url,
        })
    }
}
