use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::error::Result;

#[derive(Debug, Clone, Copy)]
pub enum SalesClassification {
    Bookings,
    Billings,
    Collections,
    Adjustments,
    Recurring,
}

impl SalesClassification {
    fn as_str(self) -> &'static str {
        match self {
            Self::Bookings => "bookings",
            Self::Billings => "billings",
            Self::Collections => "collections",
            Self::Adjustments => "adjustments",
            Self::Recurring => "recurring",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewSalesEvent<'a> {
    pub occurred_at: DateTime<Utc>,
    pub event_type: &'a str,
    pub classification: SalesClassification,
    pub amount_subtotal: Decimal,
    pub amount_tax: Decimal,
    pub amount_total: Decimal,
    pub currency: &'a str,
    pub customer_id: Option<&'a str>,
    pub subscription_id: Option<&'a str>,
    pub product_id: Option<&'a str>,
    pub invoice_id: Option<&'a str>,
    pub payment_id: Option<&'a str>,
    pub source_table: &'a str,
    pub source_id: &'a str,
    pub metadata: Option<serde_json::Value>,
}

pub async fn emit_sales_event(pool: &PgPool, event: NewSalesEvent<'_>) -> Result<()> {
    sqlx::query("SELECT ensure_sales_events_partitions(date_trunc('month', $1)::date, 4, 1)")
        .bind(event.occurred_at)
        .execute(pool)
        .await?;

    let mut tx = pool.begin().await?;

    let inserted_key: Option<i32> = sqlx::query_scalar(
        r#"
        INSERT INTO sales_event_idempotency_keys (source_table, source_id, event_type)
        VALUES ($1, $2, $3)
        ON CONFLICT (source_table, source_id, event_type) DO NOTHING
        RETURNING 1
        "#,
    )
    .bind(event.source_table)
    .bind(event.source_id)
    .bind(event.event_type)
    .fetch_optional(&mut *tx)
    .await?;

    if inserted_key.is_none() {
        tx.commit().await?;
        return Ok(());
    }

    sqlx::query(
        r#"
        INSERT INTO sales_events
            (id, occurred_at, event_type, classification,
             amount_subtotal, amount_tax, amount_total, currency,
             customer_id, subscription_id, product_id, invoice_id, payment_id,
             source_table, source_id, metadata)
        VALUES
            (gen_random_uuid()::text, $1, $2, $3,
             $4, $5, $6, $7,
             $8, $9, $10, $11, $12,
             $13, $14, $15)
        "#,
    )
    .bind(event.occurred_at)
    .bind(event.event_type)
    .bind(event.classification.as_str())
    .bind(event.amount_subtotal)
    .bind(event.amount_tax)
    .bind(event.amount_total)
    .bind(event.currency)
    .bind(event.customer_id)
    .bind(event.subscription_id)
    .bind(event.product_id)
    .bind(event.invoice_id)
    .bind(event.payment_id)
    .bind(event.source_table)
    .bind(event.source_id)
    .bind(event.metadata)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct BackfillResult {
    pub deal_created: u64,
    pub invoice_issued: u64,
    pub payment_collected: u64,
    pub credit_note_created: u64,
    pub refund_completed: u64,
    pub subscription_created: u64,
}

pub async fn backfill_sales_events(pool: &PgPool) -> Result<BackfillResult> {
    let deal_created = sqlx::query(
        r#"
        WITH candidates AS (
            SELECT
                d.id,
                COALESCE(d.created_at, NOW()) AS occurred_at,
                COALESCE(d.value, 0) AS amount,
                d.customer_id,
                d.product_id,
                d.deal_type,
                d.product_type
            FROM deals d
        ), inserted_keys AS (
            INSERT INTO sales_event_idempotency_keys (source_table, source_id, event_type)
            SELECT 'deals', c.id, 'deal.created'
            FROM candidates c
            ON CONFLICT (source_table, source_id, event_type) DO NOTHING
            RETURNING source_id
        )
        INSERT INTO sales_events
            (id, occurred_at, event_type, classification,
             amount_subtotal, amount_tax, amount_total, currency,
             customer_id, subscription_id, product_id, invoice_id, payment_id,
             source_table, source_id, metadata)
        SELECT
            gen_random_uuid()::text,
            c.occurred_at,
            'deal.created',
            'bookings',
            c.amount,
            0,
            c.amount,
            'USD',
            c.customer_id,
            NULL,
            c.product_id,
            NULL,
            NULL,
            'deals',
            c.id,
            jsonb_build_object('deal_type', c.deal_type, 'product_type', c.product_type)
        FROM candidates c
        JOIN inserted_keys k ON k.source_id = c.id
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();

    let invoice_issued = sqlx::query(
        r#"
        WITH candidates AS (
            SELECT
                i.id,
                COALESCE(i.issued_at, i.created_at, NOW()) AS occurred_at,
                COALESCE(i.subtotal, 0) AS subtotal,
                COALESCE(i.tax, 0) AS tax,
                COALESCE(i.total, 0) AS total,
                COALESCE(i.currency, 'USD') AS currency,
                i.customer_id,
                i.subscription_id,
                i.status
            FROM invoices i
            WHERE i.deleted_at IS NULL
              AND i.status <> 'void'
        ), inserted_keys AS (
            INSERT INTO sales_event_idempotency_keys (source_table, source_id, event_type)
            SELECT 'invoices', c.id, 'invoice.issued'
            FROM candidates c
            ON CONFLICT (source_table, source_id, event_type) DO NOTHING
            RETURNING source_id
        )
        INSERT INTO sales_events
            (id, occurred_at, event_type, classification,
             amount_subtotal, amount_tax, amount_total, currency,
             customer_id, subscription_id, product_id, invoice_id, payment_id,
             source_table, source_id, metadata)
        SELECT
            gen_random_uuid()::text,
            c.occurred_at,
            'invoice.issued',
            'billings',
            c.subtotal,
            c.tax,
            c.total,
            c.currency,
            c.customer_id,
            c.subscription_id,
            NULL,
            c.id,
            NULL,
            'invoices',
            c.id,
            jsonb_build_object('status', c.status)
        FROM candidates c
        JOIN inserted_keys k ON k.source_id = c.id
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();

    let payment_collected = sqlx::query(
        r#"
        WITH candidates AS (
            SELECT
                p.id,
                COALESCE(p.paid_at, p.created_at, NOW()) AS occurred_at,
                COALESCE(p.amount, 0) AS amount,
                COALESCE(i.currency, 'USD') AS currency,
                i.customer_id,
                i.subscription_id,
                i.id AS invoice_id,
                p.method
            FROM payments p
            JOIN invoices i ON i.id = p.invoice_id
        ), inserted_keys AS (
            INSERT INTO sales_event_idempotency_keys (source_table, source_id, event_type)
            SELECT 'payments', c.id, 'payment.collected'
            FROM candidates c
            ON CONFLICT (source_table, source_id, event_type) DO NOTHING
            RETURNING source_id
        )
        INSERT INTO sales_events
            (id, occurred_at, event_type, classification,
             amount_subtotal, amount_tax, amount_total, currency,
             customer_id, subscription_id, product_id, invoice_id, payment_id,
             source_table, source_id, metadata)
        SELECT
            gen_random_uuid()::text,
            c.occurred_at,
            'payment.collected',
            'collections',
            c.amount,
            0,
            c.amount,
            c.currency,
            c.customer_id,
            c.subscription_id,
            NULL,
            c.invoice_id,
            c.id,
            'payments',
            c.id,
            jsonb_build_object('method', c.method)
        FROM candidates c
        JOIN inserted_keys k ON k.source_id = c.id
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();

    let credit_note_created = sqlx::query(
        r#"
        WITH candidates AS (
            SELECT
                cn.id,
                COALESCE(cn.issued_at, cn.created_at, NOW()) AS occurred_at,
                COALESCE(cn.amount, 0) AS amount,
                COALESCE(i.currency, 'USD') AS currency,
                cn.customer_id,
                i.subscription_id,
                cn.invoice_id,
                cn.status,
                cn.reason
            FROM credit_notes cn
            LEFT JOIN invoices i ON i.id = cn.invoice_id
            WHERE cn.deleted_at IS NULL
        ), inserted_keys AS (
            INSERT INTO sales_event_idempotency_keys (source_table, source_id, event_type)
            SELECT 'credit_notes', c.id, 'credit_note.created'
            FROM candidates c
            ON CONFLICT (source_table, source_id, event_type) DO NOTHING
            RETURNING source_id
        )
        INSERT INTO sales_events
            (id, occurred_at, event_type, classification,
             amount_subtotal, amount_tax, amount_total, currency,
             customer_id, subscription_id, product_id, invoice_id, payment_id,
             source_table, source_id, metadata)
        SELECT
            gen_random_uuid()::text,
            c.occurred_at,
            'credit_note.created',
            'adjustments',
            c.amount,
            0,
            c.amount,
            c.currency,
            c.customer_id,
            c.subscription_id,
            NULL,
            c.invoice_id,
            NULL,
            'credit_notes',
            c.id,
            jsonb_build_object('status', c.status, 'reason', c.reason)
        FROM candidates c
        JOIN inserted_keys k ON k.source_id = c.id
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();

    let refund_completed = sqlx::query(
        r#"
        WITH candidates AS (
            SELECT
                r.id,
                COALESCE(r.processed_at, r.created_at, NOW()) AS occurred_at,
                COALESCE(r.amount, 0) AS amount,
                COALESCE(i.currency, 'USD') AS currency,
                i.customer_id,
                i.subscription_id,
                r.invoice_id,
                r.payment_id,
                r.status,
                r.reason
            FROM refunds r
            LEFT JOIN invoices i ON i.id = r.invoice_id
            WHERE r.deleted_at IS NULL
              AND r.status = 'completed'
        ), inserted_keys AS (
            INSERT INTO sales_event_idempotency_keys (source_table, source_id, event_type)
            SELECT 'refunds', c.id, 'refund.completed'
            FROM candidates c
            ON CONFLICT (source_table, source_id, event_type) DO NOTHING
            RETURNING source_id
        )
        INSERT INTO sales_events
            (id, occurred_at, event_type, classification,
             amount_subtotal, amount_tax, amount_total, currency,
             customer_id, subscription_id, product_id, invoice_id, payment_id,
             source_table, source_id, metadata)
        SELECT
            gen_random_uuid()::text,
            c.occurred_at,
            'refund.completed',
            'adjustments',
            c.amount,
            0,
            c.amount,
            c.currency,
            c.customer_id,
            c.subscription_id,
            NULL,
            c.invoice_id,
            c.payment_id,
            'refunds',
            c.id,
            jsonb_build_object('status', c.status, 'reason', c.reason)
        FROM candidates c
        JOIN inserted_keys k ON k.source_id = c.id
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();

    let subscription_created = sqlx::query(
        r#"
        WITH candidates AS (
            SELECT
                s.id,
                COALESCE(s.created_at, NOW()) AS occurred_at,
                s.customer_id,
                s.status,
                s.plan_id,
                s.quantity
            FROM subscriptions s
            WHERE s.deleted_at IS NULL
        ), inserted_keys AS (
            INSERT INTO sales_event_idempotency_keys (source_table, source_id, event_type)
            SELECT 'subscriptions', c.id, 'subscription.created'
            FROM candidates c
            ON CONFLICT (source_table, source_id, event_type) DO NOTHING
            RETURNING source_id
        )
        INSERT INTO sales_events
            (id, occurred_at, event_type, classification,
             amount_subtotal, amount_tax, amount_total, currency,
             customer_id, subscription_id, product_id, invoice_id, payment_id,
             source_table, source_id, metadata)
        SELECT
            gen_random_uuid()::text,
            c.occurred_at,
            'subscription.created',
            'recurring',
            0,
            0,
            0,
            'USD',
            c.customer_id,
            c.id,
            NULL,
            NULL,
            NULL,
            'subscriptions',
            c.id,
            jsonb_build_object('status', c.status, 'plan_id', c.plan_id, 'quantity', c.quantity)
        FROM candidates c
        JOIN inserted_keys k ON k.source_id = c.id
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(BackfillResult {
        deal_created,
        invoice_issued,
        payment_collected,
        credit_note_created,
        refund_completed,
        subscription_created,
    })
}
