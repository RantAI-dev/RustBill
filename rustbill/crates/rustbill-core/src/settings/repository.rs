use crate::error::Result;
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
pub trait ProviderSettingsRepository: Send + Sync {
    async fn get_setting_value(&self, key: &str) -> Result<Option<String>>;
    async fn save_setting_value(&self, key: &str, value: &str, sensitive: bool) -> Result<()>;
}

#[derive(Clone)]
pub struct PgProviderSettingsRepository {
    pool: PgPool,
}

impl PgProviderSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProviderSettingsRepository for PgProviderSettingsRepository {
    async fn get_setting_value(&self, key: &str) -> Result<Option<String>> {
        let value = sqlx::query_scalar("SELECT value FROM system_settings WHERE key = $1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(value)
    }

    async fn save_setting_value(&self, key: &str, value: &str, sensitive: bool) -> Result<()> {
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
        Ok(())
    }
}
