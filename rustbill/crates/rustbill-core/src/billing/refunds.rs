use crate::analytics::sales_ledger::{emit_sales_event, NewSalesEvent, SalesClassification};
use crate::db::models::*;
use crate::error::{BillingError, Result};
use rust_decimal::Decimal;
use serde::Deserialize;
use sqlx::PgPool;
use validator::Validate;

// ---- Request types ----

#[derive(Debug, Deserialize, Validate)]
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

#[derive(Debug, Deserialize, Default)]
pub struct ListRefundsFilter {
    pub invoice_id: Option<String>,
    pub payment_id: Option<String>,
    /// Customer role isolation.
    pub role_customer_id: Option<String>,
}

// ---- Service functions ----

pub async fn list_refunds(pool: &PgPool, filter: &ListRefundsFilter) -> Result<Vec<Refund>> {
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
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn create_refund(pool: &PgPool, req: CreateRefundRequest) -> Result<Refund> {
    req.validate().map_err(BillingError::from_validation)?;

    let mut tx = pool.begin().await?;

    // Validate payment exists
    let payment = sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE id = $1")
        .bind(&req.payment_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| BillingError::not_found("payment", &req.payment_id))?;

    // Check that total refunds for this payment don't exceed payment amount
    let existing_refunds: Option<Decimal> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM refunds WHERE payment_id = $1 AND status != 'failed' AND deleted_at IS NULL",
    )
    .bind(&req.payment_id)
    .fetch_one(&mut *tx)
    .await?;

    let total_after = existing_refunds.unwrap_or_default() + req.amount;
    if total_after > payment.amount {
        return Err(BillingError::bad_request(format!(
            "refund total ({total_after}) would exceed payment amount ({})",
            payment.amount
        )));
    }

    let status = req.status.clone().unwrap_or(RefundStatus::Pending);
    let processed_at = if status == RefundStatus::Completed {
        Some(chrono::Utc::now().naive_utc())
    } else {
        None
    };

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

    // If refund is completed, recalculate invoice paid status
    if status == RefundStatus::Completed {
        recalculate_invoice_status(&mut tx, &req.invoice_id).await?;
    }

    tx.commit().await?;

    if status == RefundStatus::Completed {
        if let Err(err) = emit_sales_event(
            pool,
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
        .await
        {
            tracing::warn!(error = %err, refund_id = %refund.id, "failed to emit sales event refund.completed");
        }
    }

    Ok(refund)
}

// ---- Internal helpers ----

async fn recalculate_invoice_status(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
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

    // If previously paid but now net paid < total, revert to issued
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
