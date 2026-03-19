use crate::analytics::sales_ledger::{emit_sales_event, NewSalesEvent, SalesClassification};
use crate::db::models::*;
use crate::error::{BillingError, Result};
use crate::notifications::email::EmailSender;
use crate::notifications::send;
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use validator::Validate;

// ---- Request types ----

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePaymentRequest {
    #[validate(length(min = 1, message = "invoice_id is required"))]
    pub invoice_id: String,

    pub amount: Decimal,
    pub method: PaymentMethod,
    pub reference: Option<String>,
    pub paid_at: Option<NaiveDateTime>,
    pub notes: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
    pub xendit_payment_id: Option<String>,
    pub lemonsqueezy_order_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListPaymentsFilter {
    pub invoice_id: Option<String>,
    /// Customer role isolation -- restrict results to this customer's invoices.
    pub role_customer_id: Option<String>,
}

// ---- View type ----

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PaymentView {
    pub id: String,
    pub invoice_id: String,
    pub amount: Decimal,
    pub method: PaymentMethod,
    pub reference: Option<String>,
    pub paid_at: NaiveDateTime,
    pub notes: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
    pub xendit_payment_id: Option<String>,
    pub lemonsqueezy_order_id: Option<String>,
    pub created_at: NaiveDateTime,
}

// ---- Service functions ----

pub async fn list_payments(pool: &PgPool, filter: &ListPaymentsFilter) -> Result<Vec<PaymentView>> {
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
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn create_payment(pool: &PgPool, req: CreatePaymentRequest) -> Result<Payment> {
    let (payment, _invoice, _became_paid) = create_payment_inner(pool, req).await?;
    Ok(payment)
}

async fn create_payment_inner(
    pool: &PgPool,
    req: CreatePaymentRequest,
) -> Result<(Payment, Invoice, bool)> {
    req.validate().map_err(BillingError::from_validation)?;

    let mut tx = pool.begin().await?;

    // Validate invoice exists and is not void/paid
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

    // Idempotency check on Stripe payment intent ID
    if let Some(ref stripe_id) = req.stripe_payment_intent_id {
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT id FROM payments WHERE stripe_payment_intent_id = $1")
                .bind(stripe_id)
                .fetch_optional(&mut *tx)
                .await?;

        if let Some((existing_id,)) = existing {
            // Return existing payment (idempotent)
            let p = sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE id = $1")
                .bind(&existing_id)
                .fetch_one(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok((p, invoice, false));
        }
    }

    // Idempotency check on Xendit payment ID
    if let Some(ref xendit_id) = req.xendit_payment_id {
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT id FROM payments WHERE xendit_payment_id = $1")
                .bind(xendit_id)
                .fetch_optional(&mut *tx)
                .await?;

        if let Some((existing_id,)) = existing {
            let p = sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE id = $1")
                .bind(&existing_id)
                .fetch_one(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok((p, invoice, false));
        }
    }

    let paid_at = req
        .paid_at
        .unwrap_or_else(|| chrono::Utc::now().naive_utc());

    // Insert payment
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

    // Check if invoice is fully paid
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

        // Deposit overpayment as credit to customer wallet
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

    Ok((payment, invoice, invoice_became_paid))
}

/// Create a payment and send email notifications.
pub async fn create_payment_with_notification(
    pool: &PgPool,
    req: CreatePaymentRequest,
    email_sender: Option<&EmailSender>,
) -> Result<Payment> {
    let method_str = format!("{:?}", req.method).to_lowercase();
    let (payment, invoice, invoice_became_paid) = create_payment_inner(pool, req).await?;

    // Send payment received notification
    let pool_clone = pool.clone();
    let email_sender_cloned = email_sender.cloned();
    let customer_id = invoice.customer_id.clone();
    let amount_str = payment.amount.to_string();
    let method = method_str;
    tokio::spawn(async move {
        send::notify_payment_received(
            email_sender_cloned.as_ref(),
            &pool_clone,
            &customer_id,
            &amount_str,
            &method,
        )
        .await;
    });

    // If invoice became fully paid, send invoice paid notification
    if invoice_became_paid {
        let pool_clone2 = pool.clone();
        let email_sender_cloned2 = email_sender.cloned();
        let customer_id2 = invoice.customer_id.clone();
        let inv_number = invoice.invoice_number.clone();
        let total_str = invoice.total.to_string();
        let currency = invoice.currency.clone();
        tokio::spawn(async move {
            send::notify_invoice_paid(
                email_sender_cloned2.as_ref(),
                &pool_clone2,
                &customer_id2,
                &inv_number,
                &total_str,
                &currency,
            )
            .await;
        });
    }

    Ok(payment)
}
