use async_trait::async_trait;
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait EventsRepository: Send + Sync {
    async fn list(
        &self,
        event_type: Option<&str>,
        resource_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn count(
        &self,
        event_type: Option<&str>,
        resource_id: Option<&str>,
    ) -> Result<i64, BillingError>;
    async fn get(&self, id: &str) -> Result<serde_json::Value, BillingError>;
}

#[derive(Clone)]
pub struct SqlxEventsRepository {
    pool: PgPool,
}

impl SqlxEventsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EventsRepository for SqlxEventsRepository {
    async fn list(
        &self,
        event_type: Option<&str>,
        resource_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT to_jsonb(e) FROM billing_events e
               WHERE ($1::text IS NULL OR e.event_type = $1::billing_event_type)
                 AND ($2::text IS NULL OR e.resource_id = $2)
               ORDER BY e.created_at DESC
               LIMIT $3 OFFSET $4"#,
        )
        .bind(event_type)
        .bind(resource_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn count(
        &self,
        event_type: Option<&str>,
        resource_id: Option<&str>,
    ) -> Result<i64, BillingError> {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM billing_events e
               WHERE ($1::text IS NULL OR e.event_type = $1::billing_event_type)
                 AND ($2::text IS NULL OR e.resource_id = $2)"#,
        )
        .bind(event_type)
        .bind(resource_id)
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn get(&self, id: &str) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(e) FROM billing_events e WHERE e.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("event", id))
    }
}
