use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaxRuleRequest {
    pub country: String,
    pub region: Option<String>,
    pub tax_name: String,
    pub rate: Decimal,
    pub inclusive: bool,
    pub product_category: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaxRuleRequest {
    pub tax_name: String,
    pub rate: Decimal,
    pub inclusive: bool,
}
