use super::schema::{CreatePlanRequest, UpdatePlanRequest};
use async_trait::async_trait;
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait PlansRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn get(&self, id: &str) -> Result<serde_json::Value, BillingError>;
    async fn create(&self, body: &CreatePlanRequest) -> Result<serde_json::Value, BillingError>;
    async fn update(
        &self,
        id: &str,
        body: &UpdatePlanRequest,
    ) -> Result<serde_json::Value, BillingError>;
    async fn delete(&self, id: &str) -> Result<u64, BillingError>;
}

#[derive(Clone)]
pub struct SqlxPlansRepository {
    pool: PgPool,
}

impl SqlxPlansRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PlansRepository for SqlxPlansRepository {
    async fn list(&self) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(p) FROM pricing_plans p ORDER BY p.created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn get(&self, id: &str) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(p) FROM pricing_plans p WHERE p.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("plan", id))
    }

    async fn create(&self, body: &CreatePlanRequest) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO pricing_plans (id, product_id, name, pricing_model, billing_cycle, base_price, unit_price, tiers, usage_metric_name, trial_days, active, created_at, updated_at)
               VALUES (gen_random_uuid()::text, $1, $2, $3::pricing_model, $4::billing_cycle, $5, $6, $7, $8, COALESCE($9, 0), COALESCE($10, true), now(), now())
               RETURNING to_jsonb(pricing_plans.*)"#,
        )
        .bind(body.product_id.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.name.as_ref().and_then(serde_json::Value::as_str))
        .bind(
            body.pricing_model
                .as_ref()
                .and_then(serde_json::Value::as_str)
                .unwrap_or("flat"),
        )
        .bind(
            body.billing_cycle
                .as_ref()
                .and_then(serde_json::Value::as_str)
                .unwrap_or("monthly"),
        )
        .bind(body.base_price.as_ref().and_then(serde_json::Value::as_f64).unwrap_or(0.0))
        .bind(body.unit_price.as_ref().and_then(serde_json::Value::as_f64))
        .bind(body.tiers.as_ref())
        .bind(body.usage_metric_name.as_ref().and_then(serde_json::Value::as_str))
        .bind(
            body.trial_days
                .as_ref()
                .and_then(serde_json::Value::as_i64)
                .map(|value| value as i32),
        )
        .bind(body.active.as_ref().and_then(serde_json::Value::as_bool))
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn update(
        &self,
        id: &str,
        body: &UpdatePlanRequest,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"UPDATE pricing_plans SET
                 name = COALESCE($2, name),
                 pricing_model = COALESCE($3::pricing_model, pricing_model),
                 billing_cycle = COALESCE($4::billing_cycle, billing_cycle),
                 base_price = COALESCE($5, base_price),
                 unit_price = COALESCE($6, unit_price),
                 tiers = COALESCE($7, tiers),
                 usage_metric_name = COALESCE($8, usage_metric_name),
                 trial_days = COALESCE($9, trial_days),
                 active = COALESCE($10, active),
                 updated_at = now()
               WHERE id = $1
               RETURNING to_jsonb(pricing_plans.*)"#,
        )
        .bind(id)
        .bind(body.name.as_ref().and_then(serde_json::Value::as_str))
        .bind(
            body.pricing_model
                .as_ref()
                .and_then(serde_json::Value::as_str),
        )
        .bind(
            body.billing_cycle
                .as_ref()
                .and_then(serde_json::Value::as_str),
        )
        .bind(body.base_price.as_ref().and_then(serde_json::Value::as_f64))
        .bind(body.unit_price.as_ref().and_then(serde_json::Value::as_f64))
        .bind(body.tiers.as_ref())
        .bind(
            body.usage_metric_name
                .as_ref()
                .and_then(serde_json::Value::as_str),
        )
        .bind(
            body.trial_days
                .as_ref()
                .and_then(serde_json::Value::as_i64)
                .map(|value| value as i32),
        )
        .bind(body.active.as_ref().and_then(serde_json::Value::as_bool))
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("plan", id))
    }

    async fn delete(&self, id: &str) -> Result<u64, BillingError> {
        let result = sqlx::query("DELETE FROM pricing_plans WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(BillingError::from)?;

        Ok(result.rows_affected())
    }
}
