use chrono::NaiveDateTime;
use rust_decimal::Decimal;

use crate::billing::proration::ProrationResult;
use crate::db::models::{Invoice, PricingPlan, Subscription};

pub struct ChangePlanInput<'a> {
    pub subscription_id: &'a str,
    pub new_plan_id: &'a str,
    pub new_quantity: Option<i32>,
    pub idempotency_key: Option<&'a str>,
    pub now: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct ChangePlanOutput {
    pub subscription: Subscription,
    pub invoice: Option<Invoice>,
    pub already_processed: bool,
    pub proration_net: Decimal,
    pub old_plan_name: String,
    pub new_plan_name: String,
    pub customer_id: String,
}

#[derive(Debug, Clone)]
pub struct ChangePlanWork {
    pub subscription: Subscription,
    pub old_plan: PricingPlan,
    pub new_plan: PricingPlan,
    pub new_quantity: i32,
    pub currency: String,
    pub proration: ProrationResult,
    pub now: NaiveDateTime,
    pub idempotency_key: Option<String>,
}
