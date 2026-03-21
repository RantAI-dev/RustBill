use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCouponRequest {
    pub code: Option<Value>,
    pub name: Option<Value>,
    pub discount_type: Option<Value>,
    pub discount_value: Option<Value>,
    pub currency: Option<Value>,
    pub max_redemptions: Option<Value>,
    pub valid_from: Option<Value>,
    pub valid_until: Option<Value>,
    pub active: Option<Value>,
    pub applies_to: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCouponRequest {
    pub code: Option<Value>,
    pub name: Option<Value>,
    pub discount_type: Option<Value>,
    pub discount_value: Option<Value>,
    pub currency: Option<Value>,
    pub max_redemptions: Option<Value>,
    pub valid_until: Option<Value>,
    pub active: Option<Value>,
    pub applies_to: Option<Value>,
}
