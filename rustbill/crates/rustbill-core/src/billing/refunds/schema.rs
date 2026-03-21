use crate::db::models::RefundStatus;
use rust_decimal::Decimal;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct CreateRefundRequest {
    #[validate(length(min = 1, message = "payment_id is required"))]
    pub payment_id: String,

    #[validate(length(min = 1, message = "invoice_id is required"))]
    pub invoice_id: String,

    pub amount: Decimal,

    #[validate(length(min = 1, message = "reason is required"))]
    pub reason: String,

    pub status: Option<RefundStatus>,
    pub stripe_refund_id: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ListRefundsFilter {
    pub invoice_id: Option<String>,
    pub payment_id: Option<String>,
    /// Customer role isolation.
    pub role_customer_id: Option<String>,
}
