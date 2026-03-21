pub mod repository;
pub mod schema;
pub mod service;

pub use schema::{ProrationLineItem, ProrationResult};

use crate::db::models::PricingPlan;
use crate::error::Result;
use chrono::NaiveDateTime;

pub fn calculate_proration(
    old_plan: &PricingPlan,
    new_plan: &PricingPlan,
    old_quantity: i32,
    new_quantity: i32,
    period_start: NaiveDateTime,
    period_end: NaiveDateTime,
    now: NaiveDateTime,
) -> Result<ProrationResult> {
    service::calculate_proration(
        old_plan,
        new_plan,
        old_quantity,
        new_quantity,
        period_start,
        period_end,
        now,
    )
}
