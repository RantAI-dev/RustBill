use super::schema::{CreateUsageEventRequest, UpdateUsageEventRequest};
use async_trait::async_trait;
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait UsageRepository: Send + Sync {
    async fn list_admin(
        &self,
        subscription_id: Option<&str>,
        metric_name: Option<&str>,
        customer_id: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn record_admin(
        &self,
        req: &CreateUsageEventRequest,
    ) -> Result<serde_json::Value, BillingError>;
    async fn summary_admin(
        &self,
        subscription_id: &str,
    ) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn update_admin(
        &self,
        id: &str,
        req: &UpdateUsageEventRequest,
    ) -> Result<serde_json::Value, BillingError>;
    async fn remove_admin(&self, id: &str) -> Result<u64, BillingError>;

    async fn list_v1(
        &self,
        subscription_id: Option<&str>,
        metric: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn record_v1(
        &self,
        req: &CreateUsageEventRequest,
    ) -> Result<serde_json::Value, BillingError>;
}

#[derive(Clone)]
pub struct SqlxUsageRepository {
    pool: PgPool,
}

impl SqlxUsageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UsageRepository for SqlxUsageRepository {
    async fn list_admin(
        &self,
        subscription_id: Option<&str>,
        metric_name: Option<&str>,
        customer_id: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT to_jsonb(u) FROM usage_events u
               JOIN subscriptions s ON s.id = u.subscription_id
               WHERE ($1::text IS NULL OR u.subscription_id = $1)
                 AND ($2::text IS NULL OR u.metric_name = $2)
                 AND ($3::text IS NULL OR s.customer_id = $3)
               ORDER BY u.timestamp DESC"#,
        )
        .bind(subscription_id)
        .bind(metric_name)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn record_admin(
        &self,
        req: &CreateUsageEventRequest,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO usage_events (id, subscription_id, metric_name, value, timestamp, idempotency_key, properties)
               VALUES (gen_random_uuid()::text, $1, $2, $3, COALESCE($4::timestamp, now()), $5, $6)
               RETURNING to_jsonb(usage_events.*)"#,
        )
        .bind(req.subscription_id.as_deref())
        .bind(req.metric_name.as_deref())
        .bind(req.normalized_value())
        .bind(req.timestamp.as_deref())
        .bind(req.idempotency_key.as_deref())
        .bind(req.normalized_properties())
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn summary_admin(
        &self,
        subscription_id: &str,
    ) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT jsonb_build_object(
                 'metricName', u.metric_name,
                 'totalValue', SUM(u.value),
                 'recordCount', COUNT(*)
               )
               FROM usage_events u
               WHERE u.subscription_id = $1
               GROUP BY u.metric_name"#,
        )
        .bind(subscription_id)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn update_admin(
        &self,
        id: &str,
        req: &UpdateUsageEventRequest,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"UPDATE usage_events SET
                 metric_name = COALESCE($2, metric_name),
                 value = COALESCE($3, value),
                 timestamp = COALESCE($4::timestamp, timestamp),
                 properties = COALESCE($5, properties)
               WHERE id = $1
               RETURNING to_jsonb(usage_events.*)"#,
        )
        .bind(id)
        .bind(req.metric_name.as_deref())
        .bind(req.value)
        .bind(req.timestamp.as_deref())
        .bind(req.properties.clone())
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("usage_event", id))
    }

    async fn remove_admin(&self, id: &str) -> Result<u64, BillingError> {
        let result = sqlx::query("DELETE FROM usage_events WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(BillingError::from)?;
        Ok(result.rows_affected())
    }

    async fn list_v1(
        &self,
        subscription_id: Option<&str>,
        metric: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT to_jsonb(u) FROM usage_events u
               WHERE ($1::text IS NULL OR u.subscription_id = $1)
                 AND ($2::text IS NULL OR u.metric_name = $2)
               ORDER BY u.timestamp DESC"#,
        )
        .bind(subscription_id)
        .bind(metric)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn record_v1(
        &self,
        req: &CreateUsageEventRequest,
    ) -> Result<serde_json::Value, BillingError> {
        self.record_admin(req).await
    }
}
