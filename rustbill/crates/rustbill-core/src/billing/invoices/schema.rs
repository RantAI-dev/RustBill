use crate::db::models::InvoiceStatus;
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateInvoiceRequest {
    #[validate(length(min = 1, message = "customer_id is required"))]
    pub customer_id: String,

    pub subscription_id: Option<String>,
    pub due_at: Option<NaiveDateTime>,
    pub currency: Option<String>,
    pub notes: Option<String>,
    pub coupon_code: Option<String>,
    pub tax_rate: Option<Decimal>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateInvoiceRequest {
    pub status: Option<InvoiceStatus>,
    pub due_at: Option<NaiveDateTime>,
    pub notes: Option<String>,
    pub stripe_invoice_id: Option<String>,
    pub xendit_invoice_id: Option<String>,
    pub lemonsqueezy_order_id: Option<String>,

    /// Required for optimistic locking.
    pub version: i32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ListInvoicesFilter {
    pub status: Option<InvoiceStatus>,
    pub customer_id: Option<String>,
    /// When set, restrict results to invoices belonging to this customer (role isolation).
    pub role_customer_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct AddInvoiceItemRequest {
    #[validate(length(min = 1, message = "description is required"))]
    pub description: String,

    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub period_start: Option<NaiveDateTime>,
    pub period_end: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct InvoiceView {
    pub id: String,
    pub invoice_number: String,
    pub customer_id: String,
    pub subscription_id: Option<String>,
    pub status: InvoiceStatus,
    pub issued_at: Option<NaiveDateTime>,
    pub due_at: Option<NaiveDateTime>,
    pub paid_at: Option<NaiveDateTime>,
    pub subtotal: Decimal,
    pub tax: Decimal,
    pub total: Decimal,
    pub currency: String,
    pub notes: Option<String>,
    pub stripe_invoice_id: Option<String>,
    pub xendit_invoice_id: Option<String>,
    pub lemonsqueezy_order_id: Option<String>,
    pub version: i32,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub customer_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InvoiceItemDraft {
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub amount: Decimal,
    pub period_start: Option<NaiveDateTime>,
    pub period_end: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct CreateInvoiceDraft {
    pub invoice_number: String,
    pub customer_id: String,
    pub subscription_id: Option<String>,
    pub due_at: Option<NaiveDateTime>,
    pub currency: String,
    pub notes: Option<String>,
    pub subtotal: Decimal,
    pub tax: Decimal,
    pub total: Decimal,
    pub line_items: Vec<InvoiceItemDraft>,
    pub coupon_id_to_increment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateInvoiceDraft {
    pub status: Option<InvoiceStatus>,
    pub due_at: Option<NaiveDateTime>,
    pub notes: Option<String>,
    pub stripe_invoice_id: Option<String>,
    pub xendit_invoice_id: Option<String>,
    pub lemonsqueezy_order_id: Option<String>,
    pub version: i32,
    pub issued_at: Option<NaiveDateTime>,
}
