use crate::db::models::{BillingCycle, PricingModel, PricingTier, ProductType};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
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

#[derive(Debug, Clone, Deserialize, Validate)]
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

#[derive(Debug, Clone, Serialize, FromRow)]
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
    pub product_name: Option<String>,
    pub product_type: Option<ProductType>,
}
