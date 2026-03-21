use rustbill_core::db::models::Product;
pub use rustbill_core::products::validation::{CreateProductRequest, UpdateProductRequest};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProductListItem {
    #[serde(flatten)]
    pub product: Product,
    pub revenue: String,
    pub change: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "activeLicenses")]
    pub active_licenses: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "totalLicenses")]
    pub total_licenses: Option<i64>,
}
