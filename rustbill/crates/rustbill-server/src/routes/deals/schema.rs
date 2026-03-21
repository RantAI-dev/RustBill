use rust_decimal::Decimal;
use rustbill_core::db::models::{DealType, ProductType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DealMutationRequest {
    #[serde(alias = "customer_id")]
    pub customer_id: Option<String>,
    #[serde(alias = "name")]
    pub company: Option<String>,
    pub contact: Option<String>,
    pub email: Option<String>,
    pub value: Option<Decimal>,
    #[serde(alias = "product_id")]
    pub product_id: Option<String>,
    #[serde(alias = "product_name")]
    pub product_name: Option<String>,
    #[serde(alias = "product_type", alias = "type")]
    pub product_type: Option<ProductType>,
    #[serde(alias = "deal_type")]
    pub deal_type: Option<DealType>,
    pub date: Option<String>,
    #[serde(alias = "license_key")]
    pub license_key: Option<String>,
    pub notes: Option<String>,
    #[serde(alias = "usage_metric_label")]
    pub usage_metric_label: Option<String>,
    #[serde(alias = "usage_metric_value")]
    pub usage_metric_value: Option<i32>,
    #[serde(default, alias = "auto_create_invoice")]
    pub auto_create_invoice: Option<bool>,
}

pub type CreateDealRequest = DealMutationRequest;
pub type UpdateDealRequest = DealMutationRequest;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DealListQuery {
    #[serde(alias = "product_type")]
    pub product_type: Option<String>,
    #[serde(alias = "deal_type")]
    pub deal_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DeleteDealResponse {
    pub success: bool,
}
