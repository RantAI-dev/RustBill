use async_trait::async_trait;
use rustbill_core::error::BillingError;
use rustbill_core::settings::provider_settings::{ProviderSettingsCache, ProviderStatus};
use std::sync::Arc;

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get_status(&self) -> ProviderStatus;

    async fn save(&self, key: &str, value: &str, sensitive: bool) -> Result<(), BillingError>;
}

#[derive(Clone)]
pub struct ProviderSettingsRepository {
    cache: Arc<ProviderSettingsCache>,
}

impl ProviderSettingsRepository {
    pub fn new(cache: Arc<ProviderSettingsCache>) -> Self {
        Self { cache }
    }
}

#[async_trait]
impl SettingsRepository for ProviderSettingsRepository {
    async fn get_status(&self) -> ProviderStatus {
        self.cache.get_status().await
    }

    async fn save(&self, key: &str, value: &str, sensitive: bool) -> Result<(), BillingError> {
        self.cache.save(key, value, sensitive).await
    }
}
