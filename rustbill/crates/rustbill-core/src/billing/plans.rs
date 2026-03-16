use crate::db::models::*;
use crate::error::{BillingError, Result};
use rust_decimal::Decimal;
use serde::Deserialize;
use sqlx::PgPool;
use validator::Validate;

// ---- Request types ----

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePlanRequest {
    pub product_id: Option<String>,

    #[validate(length(min = 1, max = 255, message = "name is required"))]
    pub name: String,

    pub pricing_model: PricingModel,
    pub billing_cycle: BillingCycle,
    pub base_price: Decimal,
    pub unit_price: Option<Decimal>,
    pub tiers: Option<Vec<PricingTier>>,
    pub usage_metric_name: Option<String>,

    #[serde(default)]
    pub trial_days: i32,

    #[serde(default = "default_true")]
    pub active: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePlanRequest {
    pub product_id: Option<Option<String>>,

    #[validate(length(min = 1, max = 255, message = "name must not be empty"))]
    pub name: Option<String>,

    pub pricing_model: Option<PricingModel>,
    pub billing_cycle: Option<BillingCycle>,
    pub base_price: Option<Decimal>,
    pub unit_price: Option<Option<Decimal>>,
    pub tiers: Option<Option<Vec<PricingTier>>>,
    pub usage_metric_name: Option<Option<String>>,
    pub trial_days: Option<i32>,
    pub active: Option<bool>,
}

// ---- View type (plan + product info) ----

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct PlanView {
    pub id: String,
    pub product_id: Option<String>,
    pub name: String,
    pub pricing_model: PricingModel,
    pub billing_cycle: BillingCycle,
    pub base_price: Decimal,
    pub unit_price: Option<Decimal>,
    pub tiers: Option<serde_json::Value>,
    pub usage_metric_name: Option<String>,
    pub trial_days: i32,
    pub active: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    // Joined fields
    pub product_name: Option<String>,
    pub product_type: Option<ProductType>,
}

// ---- Service functions ----

pub async fn list_plans(pool: &PgPool) -> Result<Vec<PlanView>> {
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
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn get_plan(pool: &PgPool, id: &str) -> Result<PricingPlan> {
    sqlx::query_as::<_, PricingPlan>("SELECT * FROM pricing_plans WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| BillingError::not_found("plan", id))
}

pub async fn create_plan(pool: &PgPool, req: CreatePlanRequest) -> Result<PricingPlan> {
    req.validate().map_err(BillingError::from_validation)?;

    let tiers_json = req
        .tiers
        .as_ref()
        .map(|t| serde_json::to_value(t).unwrap());

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
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn update_plan(pool: &PgPool, id: &str, req: UpdatePlanRequest) -> Result<PricingPlan> {
    req.validate().map_err(BillingError::from_validation)?;

    // Ensure plan exists
    let _existing = get_plan(pool, id).await?;

    let tiers_json: Option<Option<serde_json::Value>> = req.tiers.map(|opt| {
        opt.map(|t| serde_json::to_value(t).unwrap())
    });

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
    .bind(&req.product_id.flatten())
    .bind(&req.name)
    .bind(&req.pricing_model)
    .bind(&req.billing_cycle)
    .bind(req.base_price)
    .bind(req.unit_price.flatten())
    .bind(tiers_json.flatten())
    .bind(req.usage_metric_name.flatten())
    .bind(req.trial_days)
    .bind(req.active)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn delete_plan(pool: &PgPool, id: &str) -> Result<()> {
    let result = sqlx::query("DELETE FROM pricing_plans WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(BillingError::not_found("plan", id));
    }
    Ok(())
}
