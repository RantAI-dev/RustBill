use super::schema::CreateRefundRequest;
use async_trait::async_trait;
use rust_decimal::Decimal;
use rustbill_core::analytics::sales_ledger::{
    emit_sales_event, NewSalesEvent, SalesClassification,
};
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait RefundRepository: Send + Sync {
    async fn list_admin(&self) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn get_admin(&self, id: &str) -> Result<serde_json::Value, BillingError>;
    async fn create_admin(
        &self,
        body: &CreateRefundRequest,
        amount: f64,
    ) -> Result<serde_json::Value, BillingError>;
    async fn update_admin(
        &self,
        id: &str,
        status: Option<&str>,
    ) -> Result<serde_json::Value, BillingError>;
    async fn delete_admin(&self, id: &str) -> Result<u64, BillingError>;
    async fn find_completed_event(
        &self,
        id: &str,
    ) -> Result<Option<(String, String)>, BillingError>;
    async fn emit_created_event(
        &self,
        row: &serde_json::Value,
        amount: Decimal,
    ) -> Result<(), BillingError>;
    async fn emit_completed_event(
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
        completed_event: Option<(String, String)>,
    ) -> Result<(), BillingError>;
}

#[derive(Clone)]
pub struct SqlxRefundRepository {
    pool: PgPool,
}

impl SqlxRefundRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn value_string<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(serde_json::Value::as_str)
}

#[async_trait]
impl RefundRepository for SqlxRefundRepository {
    async fn list_admin(&self) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(r) FROM refunds r WHERE r.deleted_at IS NULL ORDER BY r.created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn get_admin(&self, id: &str) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(r) FROM refunds r WHERE r.id = $1 AND r.deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("refund", id))
    }

    async fn create_admin(
        &self,
        body: &CreateRefundRequest,
        amount: f64,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO refunds (id, payment_id, invoice_id, amount, reason, status, stripe_refund_id, created_at)
               VALUES (gen_random_uuid()::text, $1, COALESCE($2, (SELECT invoice_id FROM payments WHERE id = $1)), $3, $4, 'pending', $5, now())
               RETURNING to_jsonb(refunds.*)"#,
        )
        .bind(body.payment_id.as_deref())
        .bind(body.invoice_id.as_deref())
        .bind(amount)
        .bind(body.reason.as_deref())
        .bind(body.stripe_refund_id.as_deref())
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
            r#"UPDATE refunds SET
                  status = COALESCE($2::refund_status, status),
                  processed_at = CASE WHEN $2::refund_status = 'completed' THEN now() ELSE processed_at END
               WHERE id = $1 AND deleted_at IS NULL
               RETURNING to_jsonb(refunds.*)"#,
        )
        .bind(id)
        .bind(status)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("refund", id))
    }

    async fn delete_admin(&self, id: &str) -> Result<u64, BillingError> {
        let result = sqlx::query(
            "UPDATE refunds SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(result.rows_affected())
    }

    async fn find_completed_event(
        &self,
        id: &str,
    ) -> Result<Option<(String, String)>, BillingError> {
        sqlx::query_as::<_, (String, String)>(
            r#"SELECT id, event_type
               FROM sales_events
               WHERE source_table = 'refunds'
                 AND source_id = $1
                 AND event_type = 'refund.completed'
               ORDER BY created_at DESC
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
        let refund_id = value_string(row, "id").unwrap_or_default();
        let Some(invoice_id) = value_string(row, "invoice_id") else {
            return Ok(());
        };
        let Some(payment_id) = value_string(row, "payment_id") else {
            return Ok(());
        };
        let Some(reason) = value_string(row, "reason") else {
            return Ok(());
        };

        if let Err(err) = emit_sales_event(
            &self.pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "refund.created",
                classification: SalesClassification::Adjustments,
                amount_subtotal: amount,
                amount_tax: Decimal::ZERO,
                amount_total: amount,
                currency: "USD",
                customer_id: None,
                subscription_id: None,
                product_id: None,
                invoice_id: Some(invoice_id),
                payment_id: Some(payment_id),
                source_table: "refunds",
                source_id: refund_id,
                metadata: Some(serde_json::json!({
                    "reason": reason,
                    "status": "pending",
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, refund_id, "failed to emit refund.created");
        }

        Ok(())
    }

    async fn emit_completed_event(
        &self,
        id: &str,
        row: &serde_json::Value,
        amount: Decimal,
    ) -> Result<(), BillingError> {
        let reason = value_string(row, "reason").unwrap_or_default();
        let invoice_id = value_string(row, "invoice_id");
        let payment_id = value_string(row, "payment_id");

        if let Err(err) = emit_sales_event(
            &self.pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "refund.completed",
                classification: SalesClassification::Adjustments,
                amount_subtotal: amount,
                amount_tax: Decimal::ZERO,
                amount_total: amount,
                currency: "USD",
                customer_id: None,
                subscription_id: None,
                product_id: None,
                invoice_id,
                payment_id,
                source_table: "refunds",
                source_id: id,
                metadata: Some(serde_json::json!({
                    "reason": reason,
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, refund_id = %id, "failed to emit refund.completed");
        }

        Ok(())
    }

    async fn emit_reversal_event(
        &self,
        id: &str,
        before: &serde_json::Value,
        amount: Decimal,
        completed_event: Option<(String, String)>,
    ) -> Result<(), BillingError> {
        let mut metadata = serde_json::json!({
            "trigger": "refund_delete",
            "reason": "refund_removed",
        });
        if let Some((event_id, event_type)) = completed_event {
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
                event_type: "refund.reversal",
                classification: SalesClassification::Adjustments,
                amount_subtotal: -amount,
                amount_tax: Decimal::ZERO,
                amount_total: -amount,
                currency: "USD",
                customer_id: None,
                subscription_id: None,
                product_id: None,
                invoice_id: value_string(before, "invoice_id"),
                payment_id: value_string(before, "payment_id"),
                source_table: "refund_revisions",
                source_id: id,
                metadata: Some(metadata),
            },
        )
        .await
        {
            tracing::warn!(error = %err, refund_id = %id, "failed to emit refund.reversal");
        }

        Ok(())
    }
}
