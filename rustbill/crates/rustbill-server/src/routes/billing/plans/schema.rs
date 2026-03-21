use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlanRequest {
    pub product_id: Option<Value>,
    pub name: Option<Value>,
    pub pricing_model: Option<Value>,
    pub billing_cycle: Option<Value>,
    pub base_price: Option<Value>,
    pub unit_price: Option<Value>,
    pub tiers: Option<Value>,
    pub usage_metric_name: Option<Value>,
    pub trial_days: Option<Value>,
    pub active: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePlanRequest {
    pub name: Option<Value>,
    pub pricing_model: Option<Value>,
    pub billing_cycle: Option<Value>,
    pub base_price: Option<Value>,
    pub unit_price: Option<Value>,
    pub tiers: Option<Value>,
    pub usage_metric_name: Option<Value>,
    pub trial_days: Option<Value>,
    pub active: Option<Value>,
}
