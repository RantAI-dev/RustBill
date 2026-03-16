use crate::db::models::{DealType, ProductType};
use rust_decimal::Decimal;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateDealRequest {
    pub customer_id: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub company: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub contact: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    pub value: Decimal,
    pub product_id: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub product_name: Option<String>,
    pub product_type: Option<ProductType>,
    pub deal_type: DealType,
    pub date: Option<String>,
    pub license_key: Option<String>,
    pub notes: Option<String>,
    pub usage_metric_label: Option<String>,
    pub usage_metric_value: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateDealRequest {
    pub customer_id: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub company: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub contact: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    pub value: Option<Decimal>,
    pub product_id: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub product_name: Option<String>,
    pub product_type: Option<ProductType>,
    pub deal_type: Option<DealType>,
    pub date: Option<String>,
    pub license_key: Option<String>,
    pub notes: Option<String>,
    pub usage_metric_label: Option<String>,
    pub usage_metric_value: Option<i32>,
}
