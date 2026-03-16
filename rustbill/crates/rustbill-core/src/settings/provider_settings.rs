//! Payment provider settings: DB storage with in-memory cache + env fallback.

use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const CACHE_TTL: Duration = Duration::from_secs(60);

pub struct ProviderSettingsCache {
    cache: RwLock<HashMap<String, CacheEntry>>,
    pool: PgPool,
}

struct CacheEntry {
    value: String,
    expires_at: Instant,
}

/// A snapshot of provider settings for use in payment operations.
#[derive(Debug, Clone, Default)]
pub struct ProviderSettings {
    settings: HashMap<String, String>,
}

impl ProviderSettings {
    pub fn get(&self, key: &str) -> Option<String> {
        self.settings.get(key).cloned()
    }
}

impl ProviderSettingsCache {
    pub fn new(pool: PgPool) -> Arc<Self> {
        Arc::new(Self {
            cache: RwLock::new(HashMap::new()),
            pool,
        })
    }

    /// Get a single setting value: cache -> DB -> env fallback.
    pub async fn get(&self, key: &str) -> String {
        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(key) {
                if entry.expires_at > Instant::now() {
                    return entry.value.clone();
                }
            }
        }

        // Try DB
        let db_value: Option<String> =
            sqlx::query_scalar("SELECT value FROM system_settings WHERE key = $1")
                .bind(key)
                .fetch_optional(&self.pool)
                .await
                .ok()
                .flatten();

        if let Some(ref val) = db_value {
            let mut cache = self.cache.write().await;
            cache.insert(
                key.to_string(),
                CacheEntry {
                    value: val.clone(),
                    expires_at: Instant::now() + CACHE_TTL,
                },
            );
            return val.clone();
        }

        // Env fallback
        let env_key = key.to_uppercase();
        let env_val = std::env::var(&env_key).unwrap_or_default();
        env_val
    }

    /// Get all settings for a provider as a ProviderSettings snapshot.
    pub async fn get_provider_settings(&self, keys: &[&str]) -> ProviderSettings {
        let mut settings = HashMap::new();
        for key in keys {
            let val = self.get(key).await;
            if !val.is_empty() {
                settings.insert(key.to_string(), val);
            }
        }
        ProviderSettings { settings }
    }

    /// Save a setting to DB and invalidate cache.
    pub async fn save(&self, key: &str, value: &str, sensitive: bool) -> crate::error::Result<()> {
        sqlx::query(
            r#"INSERT INTO system_settings (key, value, sensitive, updated_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (key) DO UPDATE SET value = $2, sensitive = $3, updated_at = NOW()"#,
        )
        .bind(key)
        .bind(value)
        .bind(sensitive)
        .execute(&self.pool)
        .await?;

        let mut cache = self.cache.write().await;
        cache.remove(key);
        Ok(())
    }

    /// Get provider status with masked values for display.
    pub async fn get_status(&self) -> ProviderStatus {
        let stripe_key = self.get("stripe_secret_key").await;
        let stripe_webhook = self.get("stripe_webhook_secret").await;
        let xendit_key = self.get("xendit_secret_key").await;
        let xendit_token = self.get("xendit_webhook_token").await;
        let ls_key = self.get("lemonsqueezy_api_key").await;
        let ls_store = self.get("lemonsqueezy_store_id").await;
        let ls_webhook = self.get("lemonsqueezy_webhook_secret").await;

        ProviderStatus {
            stripe: ProviderInfo {
                configured: !stripe_key.is_empty(),
                secret_key: mask_value(&stripe_key),
                webhook_secret: mask_value(&stripe_webhook),
            },
            xendit: ProviderInfo {
                configured: !xendit_key.is_empty(),
                secret_key: mask_value(&xendit_key),
                webhook_secret: mask_value(&xendit_token),
            },
            lemonsqueezy: LsProviderInfo {
                configured: !ls_key.is_empty() && !ls_store.is_empty(),
                api_key: mask_value(&ls_key),
                store_id: ls_store,
                webhook_secret: mask_value(&ls_webhook),
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

#[derive(Debug, serde::Serialize)]
pub struct ProviderStatus {
    pub stripe: ProviderInfo,
    pub xendit: ProviderInfo,
    pub lemonsqueezy: LsProviderInfo,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub configured: bool,
    pub secret_key: String,
    pub webhook_secret: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LsProviderInfo {
    pub configured: bool,
    pub api_key: String,
    pub store_id: String,
    pub webhook_secret: String,
}
