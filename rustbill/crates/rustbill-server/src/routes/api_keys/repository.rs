use async_trait::async_trait;
use rustbill_core::error::{BillingError, Result};
use sqlx::PgPool;

#[async_trait]
pub trait ApiKeysRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<serde_json::Value>>;
    async fn create(
        &self,
        name: &str,
        customer_id: Option<&str>,
        key_prefix: &str,
        key_hash: &str,
    ) -> Result<serde_json::Value>;
    async fn revoke(&self, id: &str) -> Result<u64>;
}

#[derive(Clone)]
pub struct SqlxApiKeysRepository {
    pool: PgPool,
}

impl SqlxApiKeysRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ApiKeysRepository for SqlxApiKeysRepository {
    async fn list(&self) -> Result<Vec<serde_json::Value>> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT to_jsonb(k) - 'key_hash' FROM api_keys k
               ORDER BY k.created_at DESC"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn create(
        &self,
        name: &str,
        customer_id: Option<&str>,
        key_prefix: &str,
        key_hash: &str,
    ) -> Result<serde_json::Value> {
        if let Some(customer_id) = customer_id {
            sqlx::query_scalar::<_, serde_json::Value>(
                r#"INSERT INTO api_keys (id, name, customer_id, key_prefix, key_hash, created_at)
                   VALUES (gen_random_uuid()::text, $1, $2, $3, $4, now())
                   RETURNING to_jsonb(api_keys.*) - 'key_hash'"#,
            )
            .bind(name)
            .bind(customer_id)
            .bind(key_prefix)
            .bind(key_hash)
            .fetch_one(&self.pool)
            .await
            .map_err(BillingError::from)
        } else {
            sqlx::query_scalar::<_, serde_json::Value>(
                r#"INSERT INTO api_keys (id, name, key_prefix, key_hash, created_at)
                   VALUES (gen_random_uuid()::text, $1, $2, $3, now())
                   RETURNING to_jsonb(api_keys.*) - 'key_hash'"#,
            )
            .bind(name)
            .bind(key_prefix)
            .bind(key_hash)
            .fetch_one(&self.pool)
            .await
            .map_err(BillingError::from)
        }
    }

    async fn revoke(&self, id: &str) -> Result<u64> {
        let result = sqlx::query(
            "UPDATE api_keys SET status = 'revoked' WHERE id = $1 AND status = 'active'",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(result.rows_affected())
    }
}
