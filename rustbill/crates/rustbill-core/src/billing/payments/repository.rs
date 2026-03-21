use super::schema::{CreatePaymentRequest, ListPaymentsFilter, PaymentView};
use crate::analytics::sales_ledger::{emit_sales_event, NewSalesEvent, SalesClassification};
use crate::db::models::{CreditReason, Invoice, InvoiceStatus, Payment};
use crate::error::{BillingError, Result};
use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct PaymentCreateOutcome {
    pub payment: Payment,
    pub invoice: Invoice,
    pub invoice_became_paid: bool,
}

#[async_trait]
pub trait PaymentsRepository: Send + Sync {
    async fn list_payments(&self, filter: &ListPaymentsFilter) -> Result<Vec<PaymentView>>;
    async fn create_payment(&self, req: &CreatePaymentRequest) -> Result<PaymentCreateOutcome>;
}

#[derive(Clone)]
pub struct PgPaymentsRepository {
    pool: PgPool,
}

impl PgPaymentsRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl PaymentsRepository for PgPaymentsRepository {
    async fn list_payments(&self, filter: &ListPaymentsFilter) -> Result<Vec<PaymentView>> {
        let rows = sqlx::query_as::<_, PaymentView>(
            r#"
            SELECT p.*
            FROM payments p
            JOIN invoices i ON i.id = p.invoice_id
            WHERE ($1::text IS NULL OR p.invoice_id = $1)
              AND ($2::text IS NULL OR i.customer_id = $2)
            ORDER BY p.created_at DESC
            "#,
        )
        .bind(&filter.invoice_id)
        .bind(&filter.role_customer_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn create_payment(&self, req: &CreatePaymentRequest) -> Result<PaymentCreateOutcome> {
        create_payment_with_pool(&self.pool, req).await
    }
}

async fn create_payment_with_pool(
    pool: &PgPool,
    req: &CreatePaymentRequest,
) -> Result<PaymentCreateOutcome> {
    let mut tx = pool.begin().await?;

    let invoice =
        sqlx::query_as::<_, Invoice>("SELECT * FROM invoices WHERE id = $1 AND deleted_at IS NULL")
            .bind(&req.invoice_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| BillingError::not_found("invoice", &req.invoice_id))?;

    if invoice.status == InvoiceStatus::Void {
        return Err(BillingError::bad_request("cannot pay a voided invoice"));
    }
    if invoice.status == InvoiceStatus::Paid {
        return Err(BillingError::bad_request("invoice is already fully paid"));
    }

    if let Some(ref stripe_id) = req.stripe_payment_intent_id {
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT id FROM payments WHERE stripe_payment_intent_id = $1")
                .bind(stripe_id)
                .fetch_optional(&mut *tx)
                .await?;

        if let Some((existing_id,)) = existing {
            let payment = sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE id = $1")
                .bind(&existing_id)
                .fetch_one(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(PaymentCreateOutcome {
                payment,
                invoice,
                invoice_became_paid: false,
            });
        }
    }

    if let Some(ref xendit_id) = req.xendit_payment_id {
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT id FROM payments WHERE xendit_payment_id = $1")
                .bind(xendit_id)
                .fetch_optional(&mut *tx)
                .await?;

        if let Some((existing_id,)) = existing {
            let payment = sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE id = $1")
                .bind(&existing_id)
                .fetch_one(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(PaymentCreateOutcome {
                payment,
                invoice,
                invoice_became_paid: false,
            });
        }
    }

    let paid_at = req
        .paid_at
        .unwrap_or_else(|| chrono::Utc::now().naive_utc());

    let payment = sqlx::query_as::<_, Payment>(
        r#"
        INSERT INTO payments
            (id, invoice_id, amount, method, reference, paid_at, notes,
             stripe_payment_intent_id, xendit_payment_id, lemonsqueezy_order_id)
        VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(&req.invoice_id)
    .bind(req.amount)
    .bind(&req.method)
    .bind(&req.reference)
    .bind(paid_at)
    .bind(&req.notes)
    .bind(&req.stripe_payment_intent_id)
    .bind(&req.xendit_payment_id)
    .bind(&req.lemonsqueezy_order_id)
    .fetch_one(&mut *tx)
    .await?;

    let total_paid: Option<Decimal> =
        sqlx::query_scalar("SELECT COALESCE(SUM(amount), 0) FROM payments WHERE invoice_id = $1")
            .bind(&req.invoice_id)
            .fetch_one(&mut *tx)
            .await?;

    let total_refunded: Option<Decimal> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM refunds WHERE invoice_id = $1 AND status = 'completed'",
    )
    .bind(&req.invoice_id)
    .fetch_one(&mut *tx)
    .await?;

    let net_paid = total_paid.unwrap_or_default() - total_refunded.unwrap_or_default();

    if net_paid >= invoice.amount_due {
        sqlx::query(
            "UPDATE invoices SET status = 'paid', paid_at = $2, version = version + 1, updated_at = NOW() WHERE id = $1",
        )
        .bind(&req.invoice_id)
        .bind(paid_at)
        .execute(&mut *tx)
        .await?;

        if net_paid > invoice.amount_due {
            let excess = net_paid - invoice.amount_due;
            if let Err(e) = crate::billing::credits::deposit_in_tx(
                &mut tx,
                &invoice.customer_id,
                &invoice.currency,
                excess,
                CreditReason::Overpayment,
                &format!("Overpayment on invoice {}", invoice.invoice_number),
                Some(&invoice.id),
            )
            .await
            {
                tracing::warn!("Failed to deposit overpayment credit: {e}");
            }
        }
    }

    let invoice_became_paid = net_paid >= invoice.amount_due;

    tx.commit().await?;

    if let Err(err) = emit_sales_event(
        pool,
        NewSalesEvent {
            occurred_at: chrono::Utc::now(),
            event_type: "payment.collected",
            classification: SalesClassification::Collections,
            amount_subtotal: payment.amount,
            amount_tax: Decimal::ZERO,
            amount_total: payment.amount,
            currency: &invoice.currency,
            customer_id: Some(&invoice.customer_id),
            subscription_id: invoice.subscription_id.as_deref(),
            product_id: None,
            invoice_id: Some(&invoice.id),
            payment_id: Some(&payment.id),
            source_table: "payments",
            source_id: &payment.id,
            metadata: Some(serde_json::json!({
                "method": payment.method,
                "reference": payment.reference,
            })),
        },
    )
    .await
    {
        tracing::warn!(error = %err, payment_id = %payment.id, "failed to emit sales event payment.collected");
    }

    if invoice_became_paid {
        if let Err(err) = emit_sales_event(
            pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "invoice.paid",
                classification: SalesClassification::Collections,
                amount_subtotal: invoice.subtotal,
                amount_tax: invoice.tax,
                amount_total: invoice.total,
                currency: &invoice.currency,
                customer_id: Some(&invoice.customer_id),
                subscription_id: invoice.subscription_id.as_deref(),
                product_id: None,
                invoice_id: Some(&invoice.id),
                payment_id: Some(&payment.id),
                source_table: "invoices",
                source_id: &invoice.id,
                metadata: Some(serde_json::json!({
                    "trigger": "payment",
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, invoice_id = %invoice.id, "failed to emit sales event invoice.paid");
        }
    }

    Ok(PaymentCreateOutcome {
        payment,
        invoice,
        invoice_became_paid,
    })
}
