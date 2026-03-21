use async_trait::async_trait;
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait WebhooksRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn get(&self, id: &str) -> Result<serde_json::Value, BillingError>;
    async fn create(
        &self,
        url: Option<&str>,
        description: Option<&str>,
        events: &serde_json::Value,
        secret: &str,
    ) -> Result<serde_json::Value, BillingError>;
    async fn update(
        &self,
        id: &str,
        url: Option<&str>,
        description: Option<&str>,
        events: Option<&serde_json::Value>,
        status: Option<&str>,
    ) -> Result<serde_json::Value, BillingError>;
    async fn delete(&self, id: &str) -> Result<u64, BillingError>;
}

#[derive(Clone)]
pub struct SqlxWebhooksRepository {
    pool: PgPool,
}

impl SqlxWebhooksRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WebhooksRepository for SqlxWebhooksRepository {
    async fn list(&self) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(w) FROM webhook_endpoints w ORDER BY w.created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn get(&self, id: &str) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(w) FROM webhook_endpoints w WHERE w.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("webhook_endpoint", id))
    }

    async fn create(
        &self,
        url: Option<&str>,
        description: Option<&str>,
        events: &serde_json::Value,
        secret: &str,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO webhook_endpoints (id, url, description, events, secret, status, created_at, updated_at)
               VALUES (gen_random_uuid()::text, $1, $2, $3, $4, 'active', now(), now())
               RETURNING to_jsonb(webhook_endpoints.*)"#,
        )
        .bind(url)
        .bind(description)
        .bind(events)
        .bind(secret)
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn update(
        &self,
        id: &str,
        url: Option<&str>,
        description: Option<&str>,
        events: Option<&serde_json::Value>,
        status: Option<&str>,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"UPDATE webhook_endpoints SET
                 url = COALESCE($2, url),
                 description = COALESCE($3, description),
                 events = COALESCE($4, events),
                 status = COALESCE($5::webhook_status, status),
                 updated_at = now()
               WHERE id = $1
               RETURNING to_jsonb(webhook_endpoints.*)"#,
        )
        .bind(id)
        .bind(url)
        .bind(description)
        .bind(events)
        .bind(status)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("webhook_endpoint", id))
    }

    async fn delete(&self, id: &str) -> Result<u64, BillingError> {
        let result = sqlx::query("DELETE FROM webhook_endpoints WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(BillingError::from)?;

        Ok(result.rows_affected())
    }
}
