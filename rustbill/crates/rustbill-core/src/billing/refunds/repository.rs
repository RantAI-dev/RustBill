use super::schema::{CreateRefundRequest, ListRefundsFilter};
use crate::analytics::sales_ledger::{emit_sales_event, NewSalesEvent, SalesClassification};
use crate::db::models::{Invoice, InvoiceStatus, Payment, Refund, RefundStatus};
use crate::error::Result;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};

#[async_trait]
pub trait RefundRepository {
    async fn list_refunds(&self, filter: &ListRefundsFilter) -> Result<Vec<Refund>>;
    async fn find_payment(&self, payment_id: &str) -> Result<Option<Payment>>;
    async fn non_failed_refund_total_for_payment(&self, payment_id: &str) -> Result<Decimal>;
    async fn create_refund_with_side_effects(
        &self,
        req: &CreateRefundRequest,
        status: RefundStatus,
        processed_at: Option<NaiveDateTime>,
    ) -> Result<Refund>;
    async fn emit_completed_refund_event(
        &self,
        req: &CreateRefundRequest,
        refund: &Refund,
    ) -> Result<()>;
}

pub struct PgRefundRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PgRefundRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RefundRepository for PgRefundRepository<'_> {
    async fn list_refunds(&self, filter: &ListRefundsFilter) -> Result<Vec<Refund>> {
        let rows = sqlx::query_as::<_, Refund>(
            r#"
            SELECT r.*
            FROM refunds r
            JOIN invoices i ON i.id = r.invoice_id
            WHERE r.deleted_at IS NULL
              AND ($1::text IS NULL OR r.invoice_id = $1)
              AND ($2::text IS NULL OR r.payment_id = $2)
              AND ($3::text IS NULL OR i.customer_id = $3)
            ORDER BY r.created_at DESC
            "#,
        )
        .bind(&filter.invoice_id)
        .bind(&filter.payment_id)
        .bind(&filter.role_customer_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows)
    }

    async fn find_payment(&self, payment_id: &str) -> Result<Option<Payment>> {
        let payment = sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE id = $1")
            .bind(payment_id)
            .fetch_optional(self.pool)
            .await?;
        Ok(payment)
    }

    async fn non_failed_refund_total_for_payment(&self, payment_id: &str) -> Result<Decimal> {
        let existing_refunds: Option<Decimal> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0) FROM refunds WHERE payment_id = $1 AND status != 'failed' AND deleted_at IS NULL",
        )
        .bind(payment_id)
        .fetch_one(self.pool)
        .await?;
        Ok(existing_refunds.unwrap_or_default())
    }

    async fn create_refund_with_side_effects(
        &self,
        req: &CreateRefundRequest,
        status: RefundStatus,
        processed_at: Option<NaiveDateTime>,
    ) -> Result<Refund> {
        let mut tx = self.pool.begin().await?;

        let refund = sqlx::query_as::<_, Refund>(
            r#"
            INSERT INTO refunds
                (id, payment_id, invoice_id, amount, reason, status,
                 stripe_refund_id, processed_at)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&req.payment_id)
        .bind(&req.invoice_id)
        .bind(req.amount)
        .bind(&req.reason)
        .bind(&status)
        .bind(&req.stripe_refund_id)
        .bind(processed_at)
        .fetch_one(&mut *tx)
        .await?;

        if status == RefundStatus::Completed {
            recalculate_invoice_status(&mut tx, &req.invoice_id).await?;
        }

        tx.commit().await?;
        Ok(refund)
    }

    async fn emit_completed_refund_event(
        &self,
        req: &CreateRefundRequest,
        refund: &Refund,
    ) -> Result<()> {
        emit_sales_event(
            self.pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "refund.completed",
                classification: SalesClassification::Adjustments,
                amount_subtotal: req.amount,
                amount_tax: Decimal::ZERO,
                amount_total: req.amount,
                currency: "USD",
                customer_id: None,
                subscription_id: None,
                product_id: None,
                invoice_id: Some(&req.invoice_id),
                payment_id: Some(&req.payment_id),
                source_table: "refunds",
                source_id: &refund.id,
                metadata: Some(serde_json::json!({
                    "reason": req.reason,
                })),
            },
        )
        .await?;

        Ok(())
    }
}

async fn recalculate_invoice_status(
    tx: &mut Transaction<'_, Postgres>,
    invoice_id: &str,
) -> Result<()> {
    let invoice =
        sqlx::query_as::<_, Invoice>("SELECT * FROM invoices WHERE id = $1 AND deleted_at IS NULL")
            .bind(invoice_id)
            .fetch_optional(&mut **tx)
            .await?;

    let Some(invoice) = invoice else {
        return Ok(());
    };

    let total_paid: Option<Decimal> =
        sqlx::query_scalar("SELECT COALESCE(SUM(amount), 0) FROM payments WHERE invoice_id = $1")
            .bind(invoice_id)
            .fetch_one(&mut **tx)
            .await?;

    let total_refunded: Option<Decimal> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM refunds WHERE invoice_id = $1 AND status = 'completed' AND deleted_at IS NULL",
    )
    .bind(invoice_id)
    .fetch_one(&mut **tx)
    .await?;

    let net_paid = total_paid.unwrap_or_default() - total_refunded.unwrap_or_default();

    if invoice.status == InvoiceStatus::Paid && net_paid < invoice.total {
        sqlx::query(
            "UPDATE invoices SET status = 'issued', paid_at = NULL, version = version + 1, updated_at = NOW() WHERE id = $1",
        )
        .bind(invoice_id)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}
