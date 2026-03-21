use super::schema::CreateUsageEventRequest;
use crate::db::models::UsageEvent;
use crate::error::Result;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use sqlx::PgPool;

#[async_trait]
pub trait UsageRepository: Send + Sync {
    async fn list_usage_events(&self, subscription_id: &str) -> Result<Vec<UsageEvent>>;
    async fn find_usage_event_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<UsageEvent>>;
    async fn create_usage_event(
        &self,
        req: &CreateUsageEventRequest,
        timestamp: NaiveDateTime,
    ) -> Result<UsageEvent>;
}

#[derive(Clone)]
pub struct PgUsageRepository {
    pool: PgPool,
}

impl PgUsageRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl UsageRepository for PgUsageRepository {
    async fn list_usage_events(&self, subscription_id: &str) -> Result<Vec<UsageEvent>> {
        let rows = sqlx::query_as::<_, UsageEvent>(
            r#"
            SELECT * FROM usage_events
            WHERE subscription_id = $1
            ORDER BY timestamp DESC
            "#,
        )
        .bind(subscription_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn find_usage_event_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<UsageEvent>> {
        let event = sqlx::query_as::<_, UsageEvent>(
            "SELECT * FROM usage_events WHERE idempotency_key = $1",
        )
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(event)
    }

    async fn create_usage_event(
        &self,
        req: &CreateUsageEventRequest,
        timestamp: NaiveDateTime,
    ) -> Result<UsageEvent> {
        let event = sqlx::query_as::<_, UsageEvent>(
            r#"
            INSERT INTO usage_events
                (id, subscription_id, metric_name, value, timestamp, idempotency_key, properties)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&req.subscription_id)
        .bind(&req.metric_name)
        .bind(req.value)
        .bind(timestamp)
        .bind(&req.idempotency_key)
        .bind(&req.properties)
        .fetch_one(&self.pool)
        .await?;

        Ok(event)
    }
}
