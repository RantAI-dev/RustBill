use crate::db::models::SubscriptionStatus;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateSubscriptionRequest {
    #[validate(length(min = 1, message = "customer_id is required"))]
    pub customer_id: String,

    #[validate(length(min = 1, message = "plan_id is required"))]
    pub plan_id: String,

    #[serde(default = "default_quantity")]
    pub quantity: i32,

    pub metadata: Option<serde_json::Value>,
    pub stripe_subscription_id: Option<String>,
}

fn default_quantity() -> i32 {
    1
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateSubscriptionRequest {
    pub status: Option<SubscriptionStatus>,
    pub quantity: Option<i32>,
    pub cancel_at_period_end: Option<bool>,
    pub canceled_at: Option<NaiveDateTime>,
    pub metadata: Option<serde_json::Value>,
    pub stripe_subscription_id: Option<String>,

    /// Required for optimistic locking -- must match the current version.
    pub version: i32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ListSubscriptionsFilter {
    pub status: Option<SubscriptionStatus>,
    pub customer_id: Option<String>,
    /// Customer role isolation.
    pub role_customer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SubscriptionView {
    pub id: String,
    pub customer_id: String,
    pub plan_id: String,
    pub status: SubscriptionStatus,
    pub current_period_start: NaiveDateTime,
    pub current_period_end: NaiveDateTime,
    pub canceled_at: Option<NaiveDateTime>,
    pub cancel_at_period_end: bool,
    pub trial_end: Option<NaiveDateTime>,
    pub quantity: i32,
    pub metadata: Option<serde_json::Value>,
    pub stripe_subscription_id: Option<String>,
    pub version: i32,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub customer_name: Option<String>,
    pub plan_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateSubscriptionDraft {
    pub customer_id: String,
    pub plan_id: String,
    pub status: SubscriptionStatus,
    pub current_period_start: NaiveDateTime,
    pub current_period_end: NaiveDateTime,
    pub trial_end: Option<NaiveDateTime>,
    pub quantity: i32,
    pub metadata: Option<serde_json::Value>,
    pub stripe_subscription_id: Option<String>,
}
