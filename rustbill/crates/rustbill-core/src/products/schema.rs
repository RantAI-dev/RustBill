use crate::db::models::{ProductType, Trend};
use rust_decimal::Decimal;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct CreateProductRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub product_type: ProductType,
    pub revenue: Option<Decimal>,
    pub target: Option<Decimal>,
    pub change: Option<Decimal>,
    pub units_sold: Option<i32>,
    pub active_licenses: Option<i32>,
    pub total_licenses: Option<i32>,
    pub mau: Option<i32>,
    pub dau: Option<i32>,
    pub free_users: Option<i32>,
    pub paid_users: Option<i32>,
    pub churn_rate: Option<Decimal>,
    pub api_calls: Option<i32>,
    pub active_developers: Option<i32>,
    pub avg_latency: Option<Decimal>,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct UpdateProductRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    pub product_type: Option<ProductType>,
    pub revenue: Option<Decimal>,
    pub target: Option<Decimal>,
    pub change: Option<Decimal>,
    pub units_sold: Option<i32>,
    pub active_licenses: Option<i32>,
    pub total_licenses: Option<i32>,
    pub mau: Option<i32>,
    pub dau: Option<i32>,
    pub free_users: Option<i32>,
    pub paid_users: Option<i32>,
    pub churn_rate: Option<Decimal>,
    pub api_calls: Option<i32>,
    pub active_developers: Option<i32>,
    pub avg_latency: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct ProductMetrics {
    pub revenue: Decimal,
    pub change: Decimal,
    pub active_licenses: Option<i64>,
    pub total_licenses: Option<i64>,
    pub trend: Trend,
}
