use crate::db::models::CreditNoteStatus;
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateCreditNoteRequest {
    #[validate(length(min = 1, message = "invoice_id is required"))]
    pub invoice_id: String,

    #[validate(length(min = 1, message = "customer_id is required"))]
    pub customer_id: String,

    #[validate(length(min = 1, message = "reason is required"))]
    pub reason: String,

    pub status: Option<CreditNoteStatus>,
    pub items: Vec<CreditNoteItemInput>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreditNoteItemInput {
    #[validate(length(min = 1, message = "description is required"))]
    pub description: String,

    pub quantity: Decimal,
    pub unit_price: Decimal,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateCreditNoteRequest {
    pub reason: Option<String>,
    pub status: Option<CreditNoteStatus>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ListCreditNotesFilter {
    pub invoice_id: Option<String>,
    /// Customer role isolation.
    pub role_customer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CreditNoteView {
    pub id: String,
    pub credit_note_number: String,
    pub invoice_id: String,
    pub customer_id: String,
    pub reason: String,
    pub amount: Decimal,
    pub status: CreditNoteStatus,
    pub issued_at: Option<NaiveDateTime>,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct CreditNoteItemDraft {
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub amount: Decimal,
}

#[derive(Debug, Clone)]
pub struct CreateCreditNoteDraft {
    pub invoice_id: String,
    pub customer_id: String,
    pub reason: String,
    pub amount: Decimal,
    pub status: CreditNoteStatus,
    pub issued_at: Option<NaiveDateTime>,
    pub items: Vec<CreditNoteItemDraft>,
}

#[derive(Debug, Clone)]
pub struct UpdateCreditNoteDraft {
    pub reason: Option<String>,
    pub status: Option<CreditNoteStatus>,
    pub issued_at: Option<NaiveDateTime>,
}
