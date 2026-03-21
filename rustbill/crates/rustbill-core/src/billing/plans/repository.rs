use super::schema::{CreatePlanRequest, PlanView, UpdatePlanRequest};
use crate::db::models::PricingPlan;
use crate::error::{BillingError, Result};
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
pub trait PlansRepository: Send + Sync {
    async fn list_plans(&self) -> Result<Vec<PlanView>>;
    async fn get_plan(&self, id: &str) -> Result<Option<PricingPlan>>;
    async fn create_plan(&self, req: &CreatePlanRequest) -> Result<PricingPlan>;
    async fn update_plan(&self, id: &str, req: &UpdatePlanRequest) -> Result<PricingPlan>;
    async fn delete_plan(&self, id: &str) -> Result<u64>;
}

#[derive(Clone)]
pub struct PgPlansRepository {
    pool: PgPool,
}

impl PgPlansRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl PlansRepository for PgPlansRepository {
    async fn list_plans(&self) -> Result<Vec<PlanView>> {
        let rows = sqlx::query_as::<_, PlanView>(
            r#"
            SELECT
                pp.id, pp.product_id, pp.name, pp.pricing_model, pp.billing_cycle,
                pp.base_price, pp.unit_price, pp.tiers, pp.usage_metric_name,
                pp.trial_days, pp.active, pp.created_at, pp.updated_at,
                p.name AS product_name,
                p.product_type AS product_type
            FROM pricing_plans pp
            LEFT JOIN products p ON p.id = pp.product_id
            ORDER BY pp.created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn get_plan(&self, id: &str) -> Result<Option<PricingPlan>> {
        let plan = sqlx::query_as::<_, PricingPlan>("SELECT * FROM pricing_plans WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(plan)
    }

    async fn create_plan(&self, req: &CreatePlanRequest) -> Result<PricingPlan> {
        let tiers_json = serialize_tiers(&req.tiers)?;

        let row = sqlx::query_as::<_, PricingPlan>(
            r#"
            INSERT INTO pricing_plans
                (id, product_id, name, pricing_model, billing_cycle, base_price,
                 unit_price, tiers, usage_metric_name, trial_days, active)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(&req.product_id)
        .bind(&req.name)
        .bind(&req.pricing_model)
        .bind(&req.billing_cycle)
        .bind(req.base_price)
        .bind(req.unit_price)
        .bind(&tiers_json)
        .bind(&req.usage_metric_name)
        .bind(req.trial_days)
        .bind(req.active)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn update_plan(&self, id: &str, req: &UpdatePlanRequest) -> Result<PricingPlan> {
        let tiers_json = serialize_update_tiers(&req.tiers)?;

        let row = sqlx::query_as::<_, PricingPlan>(
            r#"
            UPDATE pricing_plans SET
                product_id   = COALESCE($2, product_id),
                name         = COALESCE($3, name),
                pricing_model = COALESCE($4, pricing_model),
                billing_cycle = COALESCE($5, billing_cycle),
                base_price   = COALESCE($6, base_price),
                unit_price   = COALESCE($7, unit_price),
                tiers        = COALESCE($8, tiers),
                usage_metric_name = COALESCE($9, usage_metric_name),
                trial_days   = COALESCE($10, trial_days),
                active       = COALESCE($11, active),
                updated_at   = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(req.product_id.clone().flatten())
        .bind(&req.name)
        .bind(&req.pricing_model)
        .bind(&req.billing_cycle)
        .bind(req.base_price)
        .bind(req.unit_price.flatten())
        .bind(tiers_json)
        .bind(req.usage_metric_name.clone().flatten())
        .bind(req.trial_days)
        .bind(req.active)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn delete_plan(&self, id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM pricing_plans WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}

fn serialize_tiers(
    tiers: &Option<Vec<crate::db::models::PricingTier>>,
) -> Result<Option<serde_json::Value>> {
    match tiers {
        Some(tiers) => Ok(Some(serde_json::to_value(tiers).map_err(|e| {
            BillingError::Internal(anyhow::anyhow!("failed to serialize plan tiers: {e}"))
        })?)),
        None => Ok(None),
    }
}

fn serialize_update_tiers(
    tiers: &Option<Option<Vec<crate::db::models::PricingTier>>>,
) -> Result<Option<serde_json::Value>> {
    match tiers {
        Some(Some(tiers)) => Ok(Some(serde_json::to_value(tiers).map_err(|e| {
            BillingError::Internal(anyhow::anyhow!("failed to serialize plan tiers: {e}"))
        })?)),
        Some(None) | None => Ok(None),
    }
}
