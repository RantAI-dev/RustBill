use super::repository::{PgProviderSettingsRepository, ProviderSettingsRepository};
use super::schema::{
    LsProviderInfo, ProviderInfo, ProviderSettings, ProviderStatus, TaxProviderInfo,
    XenditProviderInfo,
};
use crate::error::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const CACHE_TTL: Duration = Duration::from_secs(60);

pub struct ProviderSettingsCache {
    cache: RwLock<HashMap<String, CacheEntry>>,
    repo: Arc<dyn ProviderSettingsRepository>,
}

struct CacheEntry {
    value: String,
    expires_at: Instant,
}

impl ProviderSettingsCache {
    pub fn new(pool: PgPool) -> Arc<Self> {
        let repo = Arc::new(PgProviderSettingsRepository::new(pool));
        Arc::new(Self {
            cache: RwLock::new(HashMap::new()),
            repo,
        })
    }

    pub async fn get(&self, key: &str) -> String {
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(key) {
                if entry.expires_at > Instant::now() {
                    return entry.value.clone();
                }
            }
        }

        let db_value = self.repo.get_setting_value(key).await.ok().flatten();
        if let Some(ref value) = db_value {
            let mut cache = self.cache.write().await;
            cache.insert(
                key.to_string(),
                CacheEntry {
                    value: value.clone(),
                    expires_at: Instant::now() + CACHE_TTL,
                },
            );
            return value.clone();
        }

        let env_key = key.to_uppercase();
        std::env::var(&env_key).unwrap_or_default()
    }

    pub async fn get_provider_settings(&self, keys: &[&str]) -> ProviderSettings {
        let mut values = HashMap::new();
        for key in keys {
            let value = self.get(key).await;
            if !value.is_empty() {
                values.insert((*key).to_string(), value);
            }
        }
        ProviderSettings::new(values)
    }

    pub async fn save(&self, key: &str, value: &str, sensitive: bool) -> Result<()> {
        self.repo.save_setting_value(key, value, sensitive).await?;
        let mut cache = self.cache.write().await;
        cache.remove(key);
        Ok(())
    }

    pub async fn get_status(&self) -> ProviderStatus {
        let stripe_key = self.get("stripe_secret_key").await;
        let stripe_webhook = self.get("stripe_webhook_secret").await;
        let xendit_key = self.get("xendit_secret_key").await;
        let xendit_token = self.get("xendit_webhook_token").await;
        let ls_key = self.get("lemonsqueezy_api_key").await;
        let ls_store = self.get("lemonsqueezy_store_id").await;
        let ls_webhook = self.get("lemonsqueezy_webhook_secret").await;
        let external_tax_provider = self.get("external_tax_provider").await;
        let taxjar_api_key = self.get("taxjar_api_key").await;

        ProviderStatus {
            stripe: ProviderInfo {
                configured: !stripe_key.is_empty(),
                secret_key: mask_value(&stripe_key),
                webhook_secret: mask_value(&stripe_webhook),
            },
            xendit: XenditProviderInfo {
                configured: !xendit_key.is_empty(),
                secret_key: mask_value(&xendit_key),
                webhook_token: mask_value(&xendit_token),
            },
            lemonsqueezy: LsProviderInfo {
                configured: !ls_key.is_empty() && !ls_store.is_empty(),
                api_key: mask_value(&ls_key),
                store_id: ls_store,
                webhook_secret: mask_value(&ls_webhook),
            },
            tax: TaxProviderInfo {
                configured: !external_tax_provider.is_empty(),
                external_provider: external_tax_provider,
                taxjar_api_key: mask_value(&taxjar_api_key),
            },
        }
    }

    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

fn mask_value(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    if value.len() <= 8 {
        return "••••••••".to_string();
    }
    format!("••••••••{}", &value[value.len() - 4..])
}
