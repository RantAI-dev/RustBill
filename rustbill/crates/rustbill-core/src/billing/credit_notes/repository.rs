use super::schema::{
    CreateCreditNoteDraft, CreditNoteItemDraft, CreditNoteView, ListCreditNotesFilter,
    UpdateCreditNoteDraft,
};
use crate::analytics::sales_ledger::{emit_sales_event, NewSalesEvent, SalesClassification};
use crate::db::models::{CreditNote, CreditNoteItem};
use crate::error::Result;
use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};

#[async_trait]
pub trait CreditNotesRepository: Send + Sync {
    async fn list_credit_notes(
        &self,
        filter: &ListCreditNotesFilter,
    ) -> Result<Vec<CreditNoteView>>;
    async fn get_credit_note(&self, id: &str) -> Result<Option<CreditNote>>;
    async fn create_credit_note(&self, draft: &CreateCreditNoteDraft) -> Result<CreditNote>;
    async fn update_credit_note(
        &self,
        id: &str,
        draft: &UpdateCreditNoteDraft,
    ) -> Result<CreditNote>;
    async fn delete_credit_note(&self, id: &str) -> Result<u64>;
}

#[derive(Clone)]
pub struct PgCreditNotesRepository {
    pool: PgPool,
}

impl PgCreditNotesRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl CreditNotesRepository for PgCreditNotesRepository {
    async fn list_credit_notes(
        &self,
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
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn get_credit_note(&self, id: &str) -> Result<Option<CreditNote>> {
        let note = sqlx::query_as::<_, CreditNote>(
            "SELECT * FROM credit_notes WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(note)
    }

    async fn create_credit_note(&self, draft: &CreateCreditNoteDraft) -> Result<CreditNote> {
        let mut tx = self.pool.begin().await?;

        let credit_note_number: String = sqlx::query_scalar(
            "SELECT 'CN-' || LPAD(nextval('credit_note_number_seq')::text, 8, '0')",
        )
        .fetch_one(&mut *tx)
        .await?;

        let note = sqlx::query_as::<_, CreditNote>(
            r#"
            INSERT INTO credit_notes
                (id, credit_note_number, invoice_id, customer_id, reason, amount, status, issued_at)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&credit_note_number)
        .bind(&draft.invoice_id)
        .bind(&draft.customer_id)
        .bind(&draft.reason)
        .bind(draft.amount)
        .bind(&draft.status)
        .bind(draft.issued_at)
        .fetch_one(&mut *tx)
        .await?;

        for item in &draft.items {
            insert_credit_note_item(&mut tx, &note.id, item).await?;
        }

        tx.commit().await?;

        if let Err(err) = emit_sales_event(
            &self.pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "credit_note.created",
                classification: SalesClassification::Adjustments,
                amount_subtotal: note.amount,
                amount_tax: Decimal::ZERO,
                amount_total: note.amount,
                currency: "USD",
                customer_id: Some(&note.customer_id),
                subscription_id: None,
                product_id: None,
                invoice_id: Some(&note.invoice_id),
                payment_id: None,
                source_table: "credit_notes",
                source_id: &note.id,
                metadata: Some(serde_json::json!({
                    "status": note.status,
                    "reason": note.reason,
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, credit_note_id = %note.id, "failed to emit sales event credit_note.created");
        }

        Ok(note)
    }

    async fn update_credit_note(
        &self,
        id: &str,
        draft: &UpdateCreditNoteDraft,
    ) -> Result<CreditNote> {
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
        .bind(&draft.reason)
        .bind(&draft.status)
        .bind(draft.issued_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn delete_credit_note(&self, id: &str) -> Result<u64> {
        let result = sqlx::query(
            "UPDATE credit_notes SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

async fn insert_credit_note_item(
    tx: &mut Transaction<'_, Postgres>,
    credit_note_id: &str,
    item: &CreditNoteItemDraft,
) -> Result<CreditNoteItem> {
    let row = sqlx::query_as::<_, CreditNoteItem>(
        r#"
        INSERT INTO credit_note_items
            (id, credit_note_id, description, quantity, unit_price, amount)
        VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(credit_note_id)
    .bind(&item.description)
    .bind(item.quantity)
    .bind(item.unit_price)
    .bind(item.amount)
    .fetch_one(&mut **tx)
    .await?;

    Ok(row)
}
