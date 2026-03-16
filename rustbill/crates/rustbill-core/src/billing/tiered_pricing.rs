use crate::db::models::{PricingModel, PricingTier};
use rust_decimal::Decimal;

/// Calculate the total amount for a given pricing model and quantity.
///
/// - **Flat**: returns `base_price` regardless of quantity.
/// - **PerUnit**: returns `unit_price * quantity` (falls back to `base_price` if no unit_price).
/// - **Tiered**: iterates through tiers, charging each tier's price for the units within that
///   tier's range, then sums the result.
/// - **UsageBased**: same as PerUnit (billed after usage is aggregated).
pub fn calculate_amount(
    pricing_model: &PricingModel,
    base_price: Decimal,
    unit_price: Option<Decimal>,
    tiers: Option<&[PricingTier]>,
    quantity: i32,
) -> Decimal {
    match pricing_model {
        PricingModel::Flat => base_price,

        PricingModel::PerUnit | PricingModel::UsageBased => {
            let up = unit_price.unwrap_or(base_price);
            up * Decimal::from(quantity)
        }

        PricingModel::Tiered => {
            let Some(tiers) = tiers else {
                // No tiers defined -- fall back to flat
                return base_price;
            };

            if tiers.is_empty() {
                return base_price;
            }

            let mut remaining = quantity as i64;
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

    #[test]
    fn flat_pricing() {
        let amount = calculate_amount(
            &PricingModel::Flat,
            Decimal::from(100),
            None,
            None,
            5,
        );
        assert_eq!(amount, Decimal::from(100));
    }

    #[test]
    fn per_unit_pricing() {
        let amount = calculate_amount(
            &PricingModel::PerUnit,
            Decimal::from(0),
            Some(Decimal::from(10)),
            None,
            5,
        );
        assert_eq!(amount, Decimal::from(50));
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

        // 10 units at $5 + 5 units at $3 = 50 + 15 = 65
        let amount = calculate_amount(
            &PricingModel::Tiered,
            Decimal::ZERO,
            None,
            Some(&tiers),
            15,
        );
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

        // 10*5 + 10*3 + 5*1 = 50 + 30 + 5 = 85
        let amount = calculate_amount(
            &PricingModel::Tiered,
            Decimal::ZERO,
            None,
            Some(&tiers),
            25,
        );
        assert_eq!(amount, Decimal::from(85));
    }
}
