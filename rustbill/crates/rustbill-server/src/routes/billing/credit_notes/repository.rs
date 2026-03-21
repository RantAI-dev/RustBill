use super::schema::CreateCreditNoteRequest;
use async_trait::async_trait;
use rust_decimal::Decimal;
use rustbill_core::analytics::sales_ledger::{
    emit_sales_event, NewSalesEvent, SalesClassification,
};
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait CreditNoteRepository: Send + Sync {
    async fn list_admin(
        &self,
        role_customer_id: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn get_admin(&self, id: &str) -> Result<serde_json::Value, BillingError>;
    async fn create_admin(
        &self,
        body: &CreateCreditNoteRequest,
        amount: f64,
    ) -> Result<serde_json::Value, BillingError>;
    async fn update_admin(
        &self,
        id: &str,
        status: Option<&str>,
    ) -> Result<serde_json::Value, BillingError>;
    async fn delete_admin(&self, id: &str) -> Result<u64, BillingError>;
    async fn find_prior_event(&self, id: &str) -> Result<Option<(String, String)>, BillingError>;
    async fn emit_created_event(
        &self,
        row: &serde_json::Value,
        amount: Decimal,
    ) -> Result<(), BillingError>;
    async fn emit_issued_event(
        &self,
        id: &str,
        row: &serde_json::Value,
        amount: Decimal,
    ) -> Result<(), BillingError>;
    async fn emit_reversal_event(
        &self,
        id: &str,
        before: &serde_json::Value,
        amount: Decimal,
        prior_event: Option<(String, String)>,
    ) -> Result<(), BillingError>;
}

#[derive(Clone)]
pub struct SqlxCreditNoteRepository {
    pool: PgPool,
}

impl SqlxCreditNoteRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn value_string<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(serde_json::Value::as_str)
}

#[async_trait]
impl CreditNoteRepository for SqlxCreditNoteRepository {
    async fn list_admin(
        &self,
        role_customer_id: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT to_jsonb(cn) FROM credit_notes cn
               WHERE cn.deleted_at IS NULL
                 AND ($1::text IS NULL OR cn.customer_id = $1)
               ORDER BY cn.created_at DESC"#,
        )
        .bind(role_customer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn get_admin(&self, id: &str) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(cn) FROM credit_notes cn WHERE cn.id = $1 AND cn.deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("credit_note", id))
    }

    async fn create_admin(
        &self,
        body: &CreateCreditNoteRequest,
        amount: f64,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO credit_notes (id, credit_note_number, invoice_id, customer_id, amount, reason, status, created_at, updated_at)
               VALUES (gen_random_uuid()::text, 'CN-' || LPAD((extract(epoch from now()) * 1000)::bigint::text, 14, '0'), $1, $2, $3, $4, 'draft', now(), now())
               RETURNING to_jsonb(credit_notes.*)"#,
        )
        .bind(body.invoice_id.as_deref())
        .bind(body.customer_id.as_deref())
        .bind(amount)
        .bind(body.reason.as_deref())
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn update_admin(
        &self,
        id: &str,
        status: Option<&str>,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"UPDATE credit_notes SET
                 status = COALESCE($2::credit_note_status, status),
                 updated_at = now()
               WHERE id = $1
               RETURNING to_jsonb(credit_notes.*)"#,
        )
        .bind(id)
        .bind(status)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("credit_note", id))
    }

    async fn delete_admin(&self, id: &str) -> Result<u64, BillingError> {
        let result = sqlx::query(
            "UPDATE credit_notes SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(result.rows_affected())
    }

    async fn find_prior_event(&self, id: &str) -> Result<Option<(String, String)>, BillingError> {
        sqlx::query_as::<_, (String, String)>(
            r#"SELECT id, event_type
               FROM sales_events
               WHERE source_table = 'credit_notes'
                 AND source_id = $1
                 AND amount_total > 0
                 AND event_type IN ('credit_note.created', 'credit_note.issued')
               ORDER BY occurred_at DESC, created_at DESC
               LIMIT 1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn emit_created_event(
        &self,
        row: &serde_json::Value,
        amount: Decimal,
    ) -> Result<(), BillingError> {
        let credit_note_id = value_string(row, "id").unwrap_or_default();
        let Some(invoice_id) = value_string(row, "invoice_id") else {
            return Ok(());
        };
        let Some(customer_id) = value_string(row, "customer_id") else {
            return Ok(());
        };
        let Some(reason) = value_string(row, "reason") else {
            return Ok(());
        };

        if let Err(err) = emit_sales_event(
            &self.pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "credit_note.created",
                classification: SalesClassification::Adjustments,
                amount_subtotal: amount,
                amount_tax: Decimal::ZERO,
                amount_total: amount,
                currency: "USD",
                customer_id: Some(customer_id),
                subscription_id: None,
                product_id: None,
                invoice_id: Some(invoice_id),
                payment_id: None,
                source_table: "credit_notes",
                source_id: credit_note_id,
                metadata: Some(serde_json::json!({
                    "status": "draft",
                    "reason": reason,
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, credit_note_id, "failed to emit credit_note.created");
        }

        Ok(())
    }

    async fn emit_issued_event(
        &self,
        id: &str,
        row: &serde_json::Value,
        amount: Decimal,
    ) -> Result<(), BillingError> {
        let reason = value_string(row, "reason").unwrap_or_default();

        if let Err(err) = emit_sales_event(
            &self.pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "credit_note.issued",
                classification: SalesClassification::Adjustments,
                amount_subtotal: amount,
                amount_tax: Decimal::ZERO,
                amount_total: amount,
                currency: "USD",
                customer_id: value_string(row, "customer_id"),
                subscription_id: None,
                product_id: None,
                invoice_id: value_string(row, "invoice_id"),
                payment_id: None,
                source_table: "credit_notes",
                source_id: id,
                metadata: Some(serde_json::json!({
                    "reason": reason,
                    "status": "issued",
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, credit_note_id = %id, "failed to emit credit_note.issued");
        }

        Ok(())
    }

    async fn emit_reversal_event(
        &self,
        id: &str,
        before: &serde_json::Value,
        amount: Decimal,
        prior_event: Option<(String, String)>,
    ) -> Result<(), BillingError> {
        let mut metadata = serde_json::json!({
            "trigger": "credit_note_delete",
            "reason": "credit_note_removed",
        });
        if let Some((event_id, event_type)) = prior_event {
            if let Some(map) = metadata.as_object_mut() {
                map.insert(
                    "reversal_of_event_id".to_string(),
                    serde_json::json!(event_id),
                );
                map.insert(
                    "reversal_of_event_type".to_string(),
                    serde_json::json!(event_type),
                );
            }
        }

        if let Err(err) = emit_sales_event(
            &self.pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "credit_note.reversal",
                classification: SalesClassification::Adjustments,
                amount_subtotal: -amount,
                amount_tax: Decimal::ZERO,
                amount_total: -amount,
                currency: "USD",
                customer_id: value_string(before, "customer_id"),
                subscription_id: None,
                product_id: None,
                invoice_id: value_string(before, "invoice_id"),
                payment_id: None,
                source_table: "credit_note_revisions",
                source_id: id,
                metadata: Some(metadata),
            },
        )
        .await
        {
            tracing::warn!(error = %err, credit_note_id = %id, "failed to emit credit_note.reversal");
        }

        Ok(())
    }
}
