use crate::db::models::CreditReason;
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct CreditBalanceRequest {
    pub customer_id: String,
    pub currency: String,
}

#[derive(Debug, Clone)]
pub struct ListCreditsRequest {
    pub customer_id: String,
    pub currency: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreditAdjustmentRequest {
    pub customer_id: String,
    pub currency: String,
    pub amount: Decimal,
    pub reason: CreditReason,
    pub description: String,
    pub invoice_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApplyCreditRequest {
    pub customer_id: String,
    pub invoice_id: String,
    pub currency: String,
    pub max_amount: Decimal,
}
