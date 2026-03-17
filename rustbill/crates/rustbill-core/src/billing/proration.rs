use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

use crate::db::models::{PricingModel, PricingPlan};
use crate::error::{BillingError, Result};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProrationLineItem {
    pub description: String,
    pub amount: Decimal,
    pub period_start: NaiveDateTime,
    pub period_end: NaiveDateTime,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProrationResult {
    pub credit_amount: Decimal,
    pub charge_amount: Decimal,
    pub net: Decimal,
    pub line_items: Vec<ProrationLineItem>,
}

/// Calculate proration for a mid-cycle plan or quantity change.
/// Returns error for usage-based plans (must wait for next cycle).
pub fn calculate_proration(
    old_plan: &PricingPlan,
    new_plan: &PricingPlan,
    old_quantity: i32,
    new_quantity: i32,
    period_start: NaiveDateTime,
    period_end: NaiveDateTime,
    now: NaiveDateTime,
) -> Result<ProrationResult> {
    if old_plan.pricing_model == PricingModel::UsageBased
        || new_plan.pricing_model == PricingModel::UsageBased
    {
        return Err(BillingError::bad_request(
            "mid-cycle plan changes are not supported for usage-based plans; schedule the change for the next billing cycle",
        ));
    }

    let total_seconds = (period_end - period_start).num_seconds() as f64;
    if total_seconds <= 0.0 {
        return Err(BillingError::bad_request("invalid period: end must be after start"));
    }
    let remaining_seconds = (period_end - now).num_seconds().max(0) as f64;
    let ratio = Decimal::from_f64(remaining_seconds / total_seconds)
        .unwrap_or(Decimal::ZERO);

    let old_amount = plan_amount(old_plan, old_quantity);
    let new_amount = plan_amount(new_plan, new_quantity);

    let credit = (old_amount * ratio).round_dp(2);
    let charge = (new_amount * ratio).round_dp(2);
    let net = charge - credit;

    let mut line_items = Vec::new();

    if credit > Decimal::ZERO {
        line_items.push(ProrationLineItem {
            description: format!("Credit: {} (unused portion)", old_plan.name),
            amount: -credit,
            period_start: now,
            period_end,
        });
    }

    if charge > Decimal::ZERO {
        line_items.push(ProrationLineItem {
            description: format!("Charge: {} (remaining portion)", new_plan.name),
            amount: charge,
            period_start: now,
            period_end,
        });
    }

    Ok(ProrationResult {
        credit_amount: credit,
        charge_amount: charge,
        net,
        line_items,
    })
}

fn plan_amount(plan: &PricingPlan, quantity: i32) -> Decimal {
    match plan.pricing_model {
        PricingModel::Flat => plan.base_price,
        PricingModel::PerUnit => {
            plan.unit_price.unwrap_or(plan.base_price) * Decimal::from(quantity)
        }
        PricingModel::Tiered => {
            crate::billing::tiered_pricing::calculate_amount(
                &plan.pricing_model,
                plan.base_price,
                plan.unit_price,
                plan.tiers.as_ref().and_then(|t| {
                    serde_json::from_value::<Vec<crate::db::models::PricingTier>>(t.clone()).ok()
                }).as_deref(),
                quantity,
            )
        }
        PricingModel::UsageBased => Decimal::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::BillingCycle;
    use chrono::NaiveDate;

    fn make_plan(name: &str, model: PricingModel, base: f64, unit: Option<f64>) -> PricingPlan {
        PricingPlan {
            id: "plan-1".to_string(),
            product_id: None,
            name: name.to_string(),
            pricing_model: model,
            billing_cycle: BillingCycle::Monthly,
            base_price: Decimal::from_f64(base).unwrap(),
            unit_price: unit.map(|u| Decimal::from_f64(u).unwrap()),
            tiers: None,
            usage_metric_name: None,
            trial_days: 0,
            active: true,
            created_at: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn test_upgrade_mid_cycle() {
        let old = make_plan("Pro", PricingModel::Flat, 100.0, None);
        let new = make_plan("Enterprise", PricingModel::Flat, 200.0, None);
        let start = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let now = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap().and_hms_opt(0, 0, 0).unwrap();

        let result = calculate_proration(&old, &new, 1, 1, start, end, now).unwrap();
        assert!(result.net > Decimal::ZERO);
        assert_eq!(result.line_items.len(), 2);
        assert!(result.line_items[0].amount < Decimal::ZERO);
        assert!(result.line_items[1].amount > Decimal::ZERO);
    }

    #[test]
    fn test_downgrade_produces_credit() {
        let old = make_plan("Enterprise", PricingModel::Flat, 200.0, None);
        let new = make_plan("Pro", PricingModel::Flat, 100.0, None);
        let start = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let now = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap().and_hms_opt(0, 0, 0).unwrap();

        let result = calculate_proration(&old, &new, 1, 1, start, end, now).unwrap();
        assert!(result.net < Decimal::ZERO);
    }

    #[test]
    fn test_usage_based_rejected() {
        let old = make_plan("Usage", PricingModel::UsageBased, 0.0, Some(0.01));
        let new = make_plan("Pro", PricingModel::Flat, 100.0, None);
        let start = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let now = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap().and_hms_opt(0, 0, 0).unwrap();

        let result = calculate_proration(&old, &new, 1, 1, start, end, now);
        assert!(result.is_err());
    }

    #[test]
    fn test_quantity_change() {
        let plan = make_plan("Per Seat", PricingModel::PerUnit, 10.0, Some(10.0));
        let start = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let now = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap().and_hms_opt(0, 0, 0).unwrap();

        let result = calculate_proration(&plan, &plan, 5, 10, start, end, now).unwrap();
        assert!(result.net > Decimal::ZERO);
    }
}
