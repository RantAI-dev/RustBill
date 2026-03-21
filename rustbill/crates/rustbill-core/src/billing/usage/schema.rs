use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateUsageEventRequest {
    #[validate(length(min = 1, message = "subscription_id is required"))]
    pub subscription_id: String,

    #[validate(length(min = 1, message = "metric_name is required"))]
    pub metric_name: String,

    pub value: Decimal,
    pub timestamp: Option<NaiveDateTime>,
    pub idempotency_key: Option<String>,
    pub properties: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ListUsageEventsFilter {
    pub subscription_id: Option<String>,
    /// Customer role isolation.
    pub role_customer_id: Option<String>,
}
