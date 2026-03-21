use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProrationLineItem {
    pub description: String,
    pub amount: Decimal,
    pub period_start: NaiveDateTime,
    pub period_end: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProrationResult {
    pub credit_amount: Decimal,
    pub charge_amount: Decimal,
    pub net: Decimal,
    pub line_items: Vec<ProrationLineItem>,
}
