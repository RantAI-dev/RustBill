use super::schema::CreateDunningLogRequest;
use async_trait::async_trait;
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait DunningRepository: Send + Sync {
    async fn list(&self, invoice_id: Option<&str>) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn get(&self, id: &str) -> Result<serde_json::Value, BillingError>;
    async fn create(
        &self,
        body: &CreateDunningLogRequest,
    ) -> Result<serde_json::Value, BillingError>;
}

#[derive(Clone)]
pub struct SqlxDunningRepository {
    pool: PgPool,
}

impl SqlxDunningRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DunningRepository for SqlxDunningRepository {
    async fn list(&self, invoice_id: Option<&str>) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT to_jsonb(d) FROM dunning_log d
               WHERE ($1::text IS NULL OR d.invoice_id = $1)
               ORDER BY d.created_at DESC"#,
        )
        .bind(invoice_id)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn get(&self, id: &str) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(d) FROM dunning_log d WHERE d.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("dunning_log", id))
    }

    async fn create(
        &self,
        body: &CreateDunningLogRequest,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO dunning_log (id, invoice_id, subscription_id, step, scheduled_at, executed_at, notes, created_at)
               VALUES (gen_random_uuid()::text, $1, $2, COALESCE($3::dunning_step, 'reminder'), COALESCE($4::timestamp, now()), $5::timestamp, $6, now())
               RETURNING to_jsonb(dunning_log.*)"#,
        )
        .bind(body.invoice_id.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.subscription_id.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.step.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.scheduled_at.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.executed_at.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.notes.as_ref().and_then(serde_json::Value::as_str))
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }
}
