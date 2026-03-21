use crate::db::models::{PricingModel, PricingTier};
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct CalculateAmountRequest {
    pub pricing_model: PricingModel,
    pub base_price: Decimal,
    pub unit_price: Option<Decimal>,
    pub tiers: Option<Vec<PricingTier>>,
    pub quantity: i32,
}
