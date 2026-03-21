use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjustRequest {
    pub customer_id: String,
    pub currency: String,
    pub amount: rust_decimal::Decimal,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjustUpdateRequest {
    pub amount: rust_decimal::Decimal,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditQuery {
    pub currency: Option<String>,
}
