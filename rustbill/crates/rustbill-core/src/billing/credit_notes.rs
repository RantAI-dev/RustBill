use crate::db::models::*;
use crate::error::{BillingError, Result};
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use validator::Validate;

// ---- Request types ----

#[derive(Debug, Deserialize, Validate)]
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

#[derive(Debug, Deserialize, Validate)]
pub struct CreditNoteItemInput {
    #[validate(length(min = 1, message = "description is required"))]
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCreditNoteRequest {
    pub reason: Option<String>,
    pub status: Option<CreditNoteStatus>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListCreditNotesFilter {
    pub invoice_id: Option<String>,
    /// Customer role isolation.
    pub role_customer_id: Option<String>,
}

// ---- View type ----

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

// ---- Service functions ----

pub async fn list_credit_notes(
    pool: &PgPool,
    filter: &ListCreditNotesFilter,
) -> Result<Vec<CreditNoteView>> {
    let rows = sqlx::query_as::<_, CreditNoteView>(
        r#"
        SELECT *
        FROM credit_notes
        WHERE deleted_at IS NULL
          AND ($1::text IS NULL OR invoice_id = $1)
          AND ($2::text IS NULL OR customer_id = $2)
        ORDER BY created_at DESC
        "#,
    )
    .bind(&filter.invoice_id)
    .bind(&filter.role_customer_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn get_credit_note(pool: &PgPool, id: &str) -> Result<CreditNote> {
    sqlx::query_as::<_, CreditNote>(
        "SELECT * FROM credit_notes WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| BillingError::not_found("credit_note", id))
}

pub async fn create_credit_note(
    pool: &PgPool,
    req: CreateCreditNoteRequest,
) -> Result<CreditNote> {
    req.validate().map_err(BillingError::from_validation)?;

    if req.items.is_empty() {
        return Err(BillingError::bad_request(
            "at least one credit note item is required",
        ));
    }

    let mut tx = pool.begin().await?;

    // Generate credit note number
    let credit_note_number: String = sqlx::query_scalar(
        "SELECT 'CN-' || LPAD(nextval('credit_note_number_seq')::text, 8, '0')",
    )
    .fetch_one(&mut *tx)
    .await?;

    // Compute total amount from items
    let amount: Decimal = req
        .items
        .iter()
        .map(|i| (i.quantity * i.unit_price).round_dp(2))
        .sum();

    let status = req.status.clone().unwrap_or(CreditNoteStatus::Draft);

    let issued_at = if status == CreditNoteStatus::Issued {
        Some(chrono::Utc::now().naive_utc())
    } else {
        None
    };

    let cn = sqlx::query_as::<_, CreditNote>(
        r#"
        INSERT INTO credit_notes
            (id, credit_note_number, invoice_id, customer_id, reason, amount, status, issued_at)
        VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(&credit_note_number)
    .bind(&req.invoice_id)
    .bind(&req.customer_id)
    .bind(&req.reason)
    .bind(amount)
    .bind(&status)
    .bind(issued_at)
    .fetch_one(&mut *tx)
    .await?;

    // Insert items
    for item in &req.items {
        let item_amount = (item.quantity * item.unit_price).round_dp(2);
        sqlx::query(
            r#"
            INSERT INTO credit_note_items
                (id, credit_note_id, description, quantity, unit_price, amount)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5)
            "#,
        )
        .bind(&cn.id)
        .bind(&item.description)
        .bind(item.quantity)
        .bind(item.unit_price)
        .bind(item_amount)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(cn)
}

pub async fn update_credit_note(
    pool: &PgPool,
    id: &str,
    req: UpdateCreditNoteRequest,
) -> Result<CreditNote> {
    req.validate().map_err(BillingError::from_validation)?;

    let _existing = get_credit_note(pool, id).await?;

    let issued_at = if req.status == Some(CreditNoteStatus::Issued) {
        Some(chrono::Utc::now().naive_utc())
    } else {
        None
    };

    let row = sqlx::query_as::<_, CreditNote>(
        r#"
        UPDATE credit_notes SET
            reason    = COALESCE($2, reason),
            status    = COALESCE($3, status),
            issued_at = COALESCE($4, issued_at),
            updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.reason)
    .bind(&req.status)
    .bind(issued_at)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn delete_credit_note(pool: &PgPool, id: &str) -> Result<()> {
    let result = sqlx::query(
        "UPDATE credit_notes SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(BillingError::not_found("credit_note", id));
    }
    Ok(())
}
