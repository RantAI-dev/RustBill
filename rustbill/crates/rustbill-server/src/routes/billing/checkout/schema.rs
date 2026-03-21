use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutQuery {
    pub invoice_id: String,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutResponse {
    pub invoice_id: String,
    pub provider: String,
    pub checkout_url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckoutResult {
    pub invoice_id: String,
    pub provider: String,
    pub checkout_url: String,
}
