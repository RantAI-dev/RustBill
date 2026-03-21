use rustbill_core::db::models::{PaymentProvider, SavedPaymentMethodType};
use serde::{Deserialize, Serialize};

pub const DEFAULT_STRIPE_SETUP_SUCCESS_URL: &str =
    "https://example.com/billing/payment-methods/success";
pub const DEFAULT_STRIPE_SETUP_CANCEL_URL: &str =
    "https://example.com/billing/payment-methods/cancel";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerQuery {
    pub customer_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupPaymentMethodRequest {
    pub customer_id: String,
    pub provider: PaymentProvider,
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetupPaymentMethodResponse {
    pub provider: PaymentProvider,
    pub customer_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DeletePaymentMethodResponse {
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UnsupportedSetupResponse {
    pub error: String,
}
