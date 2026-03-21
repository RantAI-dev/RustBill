use crate::db::models::PaymentMethod;
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreatePaymentRequest {
    #[validate(length(min = 1, message = "invoice_id is required"))]
    pub invoice_id: String,

    pub amount: Decimal,
    pub method: PaymentMethod,
    pub reference: Option<String>,
    pub paid_at: Option<NaiveDateTime>,
    pub notes: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
    pub xendit_payment_id: Option<String>,
    pub lemonsqueezy_order_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListPaymentsFilter {
    pub invoice_id: Option<String>,
    /// Customer role isolation -- restrict results to this customer's invoices.
    pub role_customer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PaymentView {
    pub id: String,
    pub invoice_id: String,
    pub amount: Decimal,
    pub method: PaymentMethod,
    pub reference: Option<String>,
    pub paid_at: NaiveDateTime,
    pub notes: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
    pub xendit_payment_id: Option<String>,
    pub lemonsqueezy_order_id: Option<String>,
    pub created_at: NaiveDateTime,
}
