pub mod repository;
pub mod schema;
pub mod service;

use crate::db::models::{PricingModel, PricingTier};
use rust_decimal::Decimal;

pub fn calculate_amount(
    pricing_model: &PricingModel,
    base_price: Decimal,
    unit_price: Option<Decimal>,
    tiers: Option<&[PricingTier]>,
    quantity: i32,
) -> Decimal {
    service::calculate_amount(&schema::CalculateAmountRequest {
        pricing_model: pricing_model.clone(),
        base_price,
        unit_price,
        tiers: tiers.map(|tiers| tiers.to_vec()),
        quantity,
    })
}
