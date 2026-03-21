use crate::db::models::{PaymentProvider, SavedPaymentMethodType};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePaymentMethodRequest {
    pub customer_id: String,
    pub provider: PaymentProvider,
    pub provider_token: String,
    pub method_type: SavedPaymentMethodType,
    pub label: String,
    pub last_four: Option<String>,
    pub expiry_month: Option<i32>,
    pub expiry_year: Option<i32>,
    #[serde(default)]
    pub set_default: bool,
}

#[derive(Debug, Clone)]
pub struct CreatePaymentMethodDraft {
    pub customer_id: String,
    pub provider: PaymentProvider,
    pub provider_token: String,
    pub method_type: SavedPaymentMethodType,
    pub label: String,
    pub last_four: Option<String>,
    pub expiry_month: Option<i32>,
    pub expiry_year: Option<i32>,
    pub is_default: bool,
    pub clear_existing_default: bool,
}
