use crate::db::models::{Customer, Invoice};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CheckoutResult {
    pub checkout_url: String,
    pub provider: String,
}

#[derive(Debug, Clone)]
pub struct CheckoutRequest {
    pub invoice_id: String,
    pub provider: String,
    pub origin: String,
}

#[derive(Debug, Clone)]
pub struct CheckoutContext {
    pub invoice: Invoice,
    pub customer: Customer,
    pub success_url: String,
    pub cancel_url: String,
}
