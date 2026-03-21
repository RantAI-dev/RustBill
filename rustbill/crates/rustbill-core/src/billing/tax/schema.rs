use crate::db::models::TaxRule;
use rust_decimal::Decimal;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaxResult {
    pub rate: Decimal,
    pub amount: Decimal,
    pub name: String,
    pub inclusive: bool,
}

impl TaxResult {
    pub fn zero() -> Self {
        Self {
            rate: Decimal::ZERO,
            amount: Decimal::ZERO,
            name: String::new(),
            inclusive: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResolveTaxRequest {
    pub country: String,
    pub region: Option<String>,
    pub product_category: Option<String>,
    pub subtotal: Decimal,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateTaxRuleRequest {
    #[validate(length(min = 1, message = "country is required"))]
    pub country: String,
    pub region: Option<String>,
    #[validate(length(min = 1, message = "tax_name is required"))]
    pub tax_name: String,
    pub rate: Decimal,
    pub inclusive: bool,
    pub product_category: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateTaxRuleRequest {
    #[validate(length(min = 1, message = "tax_name is required"))]
    pub tax_name: String,
    pub rate: Decimal,
    pub inclusive: bool,
}

pub fn calculate_tax(subtotal: Decimal, rule: &TaxRule) -> TaxResult {
    let amount = if rule.inclusive {
        let divisor = Decimal::ONE + rule.rate;
        (subtotal * rule.rate / divisor).round_dp(2)
    } else {
        (subtotal * rule.rate).round_dp(2)
    };

    TaxResult {
        rate: rule.rate,
        amount,
        name: rule.tax_name.clone(),
        inclusive: rule.inclusive,
    }
}
