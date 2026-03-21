use crate::db::models::DiscountType;
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateCouponRequest {
    #[validate(length(min = 1, max = 50, message = "code is required"))]
    pub code: String,

    #[validate(length(min = 1, max = 255, message = "name is required"))]
    pub name: String,

    pub discount_type: DiscountType,
    pub discount_value: Decimal,
    pub currency: Option<String>,
    pub max_redemptions: Option<i32>,
    pub valid_from: NaiveDateTime,
    pub valid_until: Option<NaiveDateTime>,

    #[serde(default = "default_true")]
    pub active: bool,

    pub applies_to: Option<serde_json::Value>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateCouponRequest {
    #[validate(length(min = 1, max = 255, message = "name must not be empty"))]
    pub name: Option<String>,

    pub discount_type: Option<DiscountType>,
    pub discount_value: Option<Decimal>,
    pub currency: Option<String>,
    pub max_redemptions: Option<Option<i32>>,
    pub valid_until: Option<Option<NaiveDateTime>>,
    pub active: Option<bool>,
    pub applies_to: Option<Option<serde_json::Value>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplyCouponRequest {
    pub subscription_id: String,
    pub coupon_id: String,
    pub expires_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CouponView {
    pub id: String,
    pub code: String,
    pub name: String,
    pub discount_type: DiscountType,
    pub discount_value: Decimal,
    pub currency: String,
    pub max_redemptions: Option<i32>,
    pub times_redeemed: i32,
    pub valid_from: NaiveDateTime,
    pub valid_until: Option<NaiveDateTime>,
    pub active: bool,
    pub applies_to: Option<serde_json::Value>,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
