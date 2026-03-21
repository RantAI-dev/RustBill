use super::schema::CalculateAmountRequest;
use crate::db::models::PricingModel;
use rust_decimal::Decimal;

pub fn calculate_amount(req: &CalculateAmountRequest) -> Decimal {
    match req.pricing_model {
        PricingModel::Flat => req.base_price,

        PricingModel::PerUnit | PricingModel::UsageBased => {
            let up = req.unit_price.unwrap_or(req.base_price);
            up * Decimal::from(req.quantity)
        }

        PricingModel::Tiered => {
            let Some(tiers) = req.tiers.as_deref() else {
                // No tiers defined -- fall back to flat
                return req.base_price;
            };

            if tiers.is_empty() {
                return req.base_price;
            }

            let mut remaining = req.quantity as i64;
            let mut total = Decimal::ZERO;
            let mut prev_upper = 0i64;

            for tier in tiers {
                if remaining <= 0 {
                    break;
                }

                let tier_upper = tier.up_to.unwrap_or(i64::MAX);
                let tier_size = tier_upper - prev_upper;
                let units_in_tier = remaining.min(tier_size);

                let tier_price = Decimal::try_from(tier.price).unwrap_or_default();
                total += tier_price * Decimal::from(units_in_tier);

                remaining -= units_in_tier;
                prev_upper = tier_upper;
            }

            total
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{PricingModel, PricingTier};

    fn request(
        pricing_model: PricingModel,
        base_price: Decimal,
        unit_price: Option<Decimal>,
        tiers: Option<Vec<PricingTier>>,
        quantity: i32,
    ) -> CalculateAmountRequest {
        CalculateAmountRequest {
            pricing_model,
            base_price,
            unit_price,
            tiers,
            quantity,
        }
    }

    #[test]
    fn flat_pricing() {
        let amount = calculate_amount(&request(
            PricingModel::Flat,
            Decimal::from(100),
            None,
            None,
            5,
        ));
        assert_eq!(amount, Decimal::from(100));
    }

    #[test]
    fn per_unit_pricing() {
        let amount = calculate_amount(&request(
            PricingModel::PerUnit,
            Decimal::from(0),
            Some(Decimal::from(10)),
            None,
            5,
        ));
        assert_eq!(amount, Decimal::from(50));
    }

    #[test]
    fn usage_based_pricing() {
        let amount = calculate_amount(&request(
            PricingModel::UsageBased,
            Decimal::from(7),
            None,
            None,
            3,
        ));
        assert_eq!(amount, Decimal::from(21));
    }

    #[test]
    fn tiered_pricing() {
        let tiers = vec![
            PricingTier {
                up_to: Some(10),
                price: 5.0,
            },
            PricingTier {
                up_to: Some(20),
                price: 3.0,
            },
            PricingTier {
                up_to: None,
                price: 1.0,
            },
        ];

        let amount = calculate_amount(&request(
            PricingModel::Tiered,
            Decimal::ZERO,
            None,
            Some(tiers),
            15,
        ));
        assert_eq!(amount, Decimal::from(65));
    }

    #[test]
    fn tiered_pricing_all_tiers() {
        let tiers = vec![
            PricingTier {
                up_to: Some(10),
                price: 5.0,
            },
            PricingTier {
                up_to: Some(20),
                price: 3.0,
            },
            PricingTier {
                up_to: None,
                price: 1.0,
            },
        ];

        let amount = calculate_amount(&request(
            PricingModel::Tiered,
            Decimal::ZERO,
            None,
            Some(tiers),
            25,
        ));
        assert_eq!(amount, Decimal::from(85));
    }

    #[test]
    fn tiered_pricing_falls_back_to_flat_without_tiers() {
        let amount = calculate_amount(&request(
            PricingModel::Tiered,
            Decimal::from(42),
            None,
            None,
            10,
        ));
        assert_eq!(amount, Decimal::from(42));
    }
}
