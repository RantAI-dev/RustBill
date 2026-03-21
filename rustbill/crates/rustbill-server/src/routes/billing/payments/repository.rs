use super::schema::{CreatePaymentRequest, UpdatePaymentRequest};
use async_trait::async_trait;
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait PaymentsRepository: Send + Sync {
    async fn list(&self, customer_id: Option<&str>)
        -> Result<Vec<serde_json::Value>, BillingError>;
    async fn get(&self, id: &str) -> Result<serde_json::Value, BillingError>;
    async fn create(&self, body: &CreatePaymentRequest) -> Result<serde_json::Value, BillingError>;
    async fn update(
        &self,
        id: &str,
        body: &UpdatePaymentRequest,
    ) -> Result<serde_json::Value, BillingError>;
    async fn delete(&self, id: &str) -> Result<u64, BillingError>;
}

#[derive(Clone)]
pub struct SqlxPaymentsRepository {
    pool: PgPool,
}

impl SqlxPaymentsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PaymentsRepository for SqlxPaymentsRepository {
    async fn list(
        &self,
        customer_id: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT to_jsonb(p) FROM payments p
               JOIN invoices i ON i.id = p.invoice_id
               WHERE ($1::text IS NULL OR i.customer_id = $1)
               ORDER BY p.created_at DESC"#,
        )
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn get(&self, id: &str) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(p) FROM payments p WHERE p.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("payment", id))
    }

    async fn create(&self, body: &CreatePaymentRequest) -> Result<serde_json::Value, BillingError> {
        let row = sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO payments (id, invoice_id, amount, method, reference, paid_at, notes, stripe_payment_intent_id, xendit_payment_id, lemonsqueezy_order_id, created_at)
               VALUES (gen_random_uuid()::text, $1, $2, COALESCE($3::payment_method, 'manual'), $4, COALESCE($5::timestamp, now()), $6, $7, $8, $9, now())
               RETURNING to_jsonb(payments.*)"#,
        )
        .bind(body.invoice_id.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.amount.as_ref().and_then(serde_json::Value::as_f64).unwrap_or(0.0))
        .bind(body.method.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.reference.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.paid_at.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.notes.as_ref().and_then(serde_json::Value::as_str))
        .bind(
            body.stripe_payment_intent_id
                .as_ref()
                .and_then(serde_json::Value::as_str),
        )
        .bind(body.xendit_payment_id.as_ref().and_then(serde_json::Value::as_str))
        .bind(
            body.lemonsqueezy_order_id
                .as_ref()
                .and_then(serde_json::Value::as_str),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(row)
    }

    async fn update(
        &self,
        id: &str,
        body: &UpdatePaymentRequest,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"UPDATE payments SET
                 reference = COALESCE($2, reference),
                 notes = COALESCE($3, notes)
               WHERE id = $1
               RETURNING to_jsonb(payments.*)"#,
        )
        .bind(id)
        .bind(body.reference.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.notes.as_ref().and_then(serde_json::Value::as_str))
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("payment", id))
    }

    async fn delete(&self, id: &str) -> Result<u64, BillingError> {
        let result = sqlx::query("DELETE FROM payments WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(BillingError::from)?;

        Ok(result.rows_affected())
    }
}
