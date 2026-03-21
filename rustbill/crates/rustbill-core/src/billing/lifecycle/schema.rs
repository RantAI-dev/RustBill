use crate::analytics::sales_ledger::SalesClassification;
use crate::db::models::{Invoice, Subscription};
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct LifecycleResult {
    pub trials_converted: u64,
    pub canceled: u64,
    pub pre_generated: u64,
    pub renewed: u64,
    pub invoices_generated: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RenewalInvoiceOutput {
    pub invoice: Invoice,
    pub final_amount_due: Decimal,
    pub total: Decimal,
    pub invoice_number: String,
    pub plan_name: String,
    pub new_period_end: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct SubscriptionTransition {
    pub before: Subscription,
    pub after: Subscription,
}

#[derive(Debug, Clone)]
pub struct SalesEventSpec {
    pub event_type: &'static str,
    pub classification: SalesClassification,
    pub amount_subtotal: Decimal,
    pub amount_tax: Decimal,
    pub amount_total: Decimal,
    pub currency: String,
    pub customer_id: Option<String>,
    pub subscription_id: Option<String>,
    pub invoice_id: Option<String>,
    pub payment_id: Option<String>,
    pub source_table: &'static str,
    pub source_id: String,
    pub metadata: Option<serde_json::Value>,
}
