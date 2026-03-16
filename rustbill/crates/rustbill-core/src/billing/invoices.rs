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
pub struct CreateInvoiceRequest {
    #[validate(length(min = 1, message = "customer_id is required"))]
    pub customer_id: String,

    pub subscription_id: Option<String>,
    pub due_at: Option<NaiveDateTime>,
    pub currency: Option<String>,
    pub notes: Option<String>,
    pub coupon_code: Option<String>,
    pub tax_rate: Option<Decimal>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateInvoiceRequest {
    pub status: Option<InvoiceStatus>,
    pub due_at: Option<NaiveDateTime>,
    pub notes: Option<String>,
    pub stripe_invoice_id: Option<String>,
    pub xendit_invoice_id: Option<String>,
    pub lemonsqueezy_order_id: Option<String>,

    /// Required for optimistic locking.
    pub version: i32,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListInvoicesFilter {
    pub status: Option<InvoiceStatus>,
    pub customer_id: Option<String>,
    /// When set, restrict results to invoices belonging to this customer (role isolation).
    pub role_customer_id: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AddInvoiceItemRequest {
    #[validate(length(min = 1, message = "description is required"))]
    pub description: String,

    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub period_start: Option<NaiveDateTime>,
    pub period_end: Option<NaiveDateTime>,
}

// ---- View type ----

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct InvoiceView {
    pub id: String,
    pub invoice_number: String,
    pub customer_id: String,
    pub subscription_id: Option<String>,
    pub status: InvoiceStatus,
    pub issued_at: Option<NaiveDateTime>,
    pub due_at: Option<NaiveDateTime>,
    pub paid_at: Option<NaiveDateTime>,
    pub subtotal: Decimal,
    pub tax: Decimal,
    pub total: Decimal,
    pub currency: String,
    pub notes: Option<String>,
    pub stripe_invoice_id: Option<String>,
    pub xendit_invoice_id: Option<String>,
    pub lemonsqueezy_order_id: Option<String>,
    pub version: i32,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    // Joined
    pub customer_name: Option<String>,
}

// ---- Service functions ----

pub async fn list_invoices(pool: &PgPool, filter: &ListInvoicesFilter) -> Result<Vec<InvoiceView>> {
    let rows = sqlx::query_as::<_, InvoiceView>(
        r#"
        SELECT
            i.*,
            c.name AS customer_name
        FROM invoices i
        LEFT JOIN customers c ON c.id = i.customer_id
        WHERE i.deleted_at IS NULL
          AND ($1::invoice_status IS NULL OR i.status = $1)
          AND ($2::text IS NULL OR i.customer_id = $2)
          AND ($3::text IS NULL OR i.customer_id = $3)
        ORDER BY i.created_at DESC
        "#,
    )
    .bind(&filter.status)
    .bind(&filter.customer_id)
    .bind(&filter.role_customer_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn get_invoice(pool: &PgPool, id: &str) -> Result<Invoice> {
    sqlx::query_as::<_, Invoice>("SELECT * FROM invoices WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| BillingError::not_found("invoice", id))
}

pub async fn create_invoice(pool: &PgPool, req: CreateInvoiceRequest) -> Result<Invoice> {
    req.validate().map_err(BillingError::from_validation)?;

    let currency = req.currency.clone().unwrap_or_else(|| "USD".to_string());
    let tax_rate = req.tax_rate.unwrap_or_default();

    let mut tx = pool.begin().await?;

    // Generate invoice number from DB sequence
    let invoice_number: String =
        sqlx::query_scalar("SELECT 'INV-' || LPAD(nextval('invoice_number_seq')::text, 8, '0')")
            .fetch_one(&mut *tx)
            .await?;

    // Insert the invoice in draft state with zero totals initially
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        INSERT INTO invoices
            (id, invoice_number, customer_id, subscription_id, status,
             due_at, subtotal, tax, total, currency, notes)
        VALUES (gen_random_uuid()::text, $1, $2, $3, 'draft', $4, 0, 0, 0, $5, $6)
        RETURNING *
        "#,
    )
    .bind(&invoice_number)
    .bind(&req.customer_id)
    .bind(&req.subscription_id)
    .bind(req.due_at)
    .bind(&currency)
    .bind(&req.notes)
    .fetch_one(&mut *tx)
    .await?;

    // Auto-generate line items from subscription plan if subscription_id is provided
    if let Some(ref sub_id) = req.subscription_id {
        let sub = sqlx::query_as::<_, Subscription>(
            "SELECT * FROM subscriptions WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(sub_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| BillingError::not_found("subscription", sub_id.as_str()))?;

        let plan = sqlx::query_as::<_, PricingPlan>("SELECT * FROM pricing_plans WHERE id = $1")
            .bind(&sub.plan_id)
            .fetch_one(&mut *tx)
            .await?;

        // Calculate amount using tiered pricing helper
        let tiers: Option<Vec<PricingTier>> = plan
            .tiers
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let amount = crate::billing::tiered_pricing::calculate_amount(
            &plan.pricing_model,
            plan.base_price,
            plan.unit_price,
            tiers.as_deref(),
            sub.quantity,
        );

        sqlx::query(
            r#"
            INSERT INTO invoice_items
                (id, invoice_id, description, quantity, unit_price, amount,
                 period_start, period_end)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&invoice.id)
        .bind(format!(
            "{} ({})",
            plan.name,
            format_cycle(&plan.billing_cycle)
        ))
        .bind(Decimal::from(sub.quantity))
        .bind(plan.base_price)
        .bind(amount)
        .bind(sub.current_period_start)
        .bind(sub.current_period_end)
        .execute(&mut *tx)
        .await?;
    }

    // Apply coupon discount if provided
    if let Some(ref coupon_code) = req.coupon_code {
        let coupon = sqlx::query_as::<_, Coupon>(
            "SELECT * FROM coupons WHERE code = $1 AND active = true AND deleted_at IS NULL",
        )
        .bind(coupon_code)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            BillingError::bad_request(format!("coupon '{coupon_code}' not found or inactive"))
        })?;

        // Check redemption limits
        if let Some(max) = coupon.max_redemptions {
            if coupon.times_redeemed >= max {
                return Err(BillingError::bad_request(
                    "coupon has reached max redemptions",
                ));
            }
        }

        // Compute line-item subtotal so far
        let line_subtotal: Option<Decimal> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0) FROM invoice_items WHERE invoice_id = $1",
        )
        .bind(&invoice.id)
        .fetch_one(&mut *tx)
        .await?;

        let sub = line_subtotal.unwrap_or_default();
        let discount_amount = match coupon.discount_type {
            DiscountType::Percentage => {
                (sub * coupon.discount_value / Decimal::from(100)).round_dp(2)
            }
            DiscountType::FixedAmount => coupon.discount_value.min(sub),
        };

        if discount_amount > Decimal::ZERO {
            sqlx::query(
                r#"
                INSERT INTO invoice_items
                    (id, invoice_id, description, quantity, unit_price, amount)
                VALUES (gen_random_uuid()::text, $1, $2, 1, $3, $4)
                "#,
            )
            .bind(&invoice.id)
            .bind(format!("Discount ({})", coupon.code))
            .bind(-discount_amount)
            .bind(-discount_amount)
            .execute(&mut *tx)
            .await?;

            // Increment times_redeemed
            sqlx::query("UPDATE coupons SET times_redeemed = times_redeemed + 1 WHERE id = $1")
                .bind(&coupon.id)
                .execute(&mut *tx)
                .await?;
        }
    }

    // Recompute totals
    let subtotal: Option<Decimal> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM invoice_items WHERE invoice_id = $1",
    )
    .bind(&invoice.id)
    .fetch_one(&mut *tx)
    .await?;

    let subtotal = subtotal.unwrap_or_default();
    let tax = (subtotal * tax_rate / Decimal::from(100)).round_dp(2);
    let total = subtotal + tax;

    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        UPDATE invoices SET subtotal = $2, tax = $3, total = $4, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(&invoice.id)
    .bind(subtotal)
    .bind(tax)
    .bind(total)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(invoice)
}

/// Create an invoice and send email notification to the customer.
pub async fn create_invoice_with_notification(
    pool: &PgPool,
    req: CreateInvoiceRequest,
    email_sender: Option<&EmailSender>,
) -> Result<Invoice> {
    let customer_id = req.customer_id.clone();
    let invoice = create_invoice(pool, req).await?;

    // Send email notification (non-blocking)
    let pool_clone = pool.clone();
    let email_sender_cloned = email_sender.cloned();
    let inv_number = invoice.invoice_number.clone();
    let total_str = invoice.total.to_string();
    let currency = invoice.currency.clone();
    tokio::spawn(async move {
        send::notify_invoice_created(
            email_sender_cloned.as_ref(),
            &pool_clone,
            &customer_id,
            &inv_number,
            &total_str,
            &currency,
        )
        .await;
    });

    Ok(invoice)
}

/// Update invoice and send notification if status changes to Paid.
pub async fn update_invoice_with_notification(
    pool: &PgPool,
    id: &str,
    req: UpdateInvoiceRequest,
    email_sender: Option<&EmailSender>,
) -> Result<Invoice> {
    let is_marking_paid = req.status == Some(InvoiceStatus::Paid);
    let is_marking_issued = req.status == Some(InvoiceStatus::Issued);
    let invoice = update_invoice(pool, id, req).await?;

    if is_marking_paid {
        let pool_clone = pool.clone();
        let email_sender_cloned = email_sender.cloned();
        let customer_id = invoice.customer_id.clone();
        let inv_number = invoice.invoice_number.clone();
        let total_str = invoice.total.to_string();
        let currency = invoice.currency.clone();
        tokio::spawn(async move {
            send::notify_invoice_paid(
                email_sender_cloned.as_ref(),
                &pool_clone,
                &customer_id,
                &inv_number,
                &total_str,
                &currency,
            )
            .await;
        });
    }

    if is_marking_issued {
        let pool_clone = pool.clone();
        let email_sender_cloned = email_sender.cloned();
        let customer_id = invoice.customer_id.clone();
        let inv_number = invoice.invoice_number.clone();
        let total_str = invoice.total.to_string();
        let currency = invoice.currency.clone();
        let due_date = invoice
            .due_at
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "N/A".to_string());
        tokio::spawn(async move {
            send::notify_invoice_issued(
                email_sender_cloned.as_ref(),
                &pool_clone,
                &customer_id,
                &inv_number,
                &total_str,
                &currency,
                &due_date,
            )
            .await;
        });
    }

    Ok(invoice)
}

pub async fn update_invoice(pool: &PgPool, id: &str, req: UpdateInvoiceRequest) -> Result<Invoice> {
    let now = chrono::Utc::now().naive_utc();

    // If status is being set to "issued", set issued_at
    let issued_at = if req.status == Some(InvoiceStatus::Issued) {
        Some(now)
    } else {
        None
    };

    let row = sqlx::query_as::<_, Invoice>(
        r#"
        UPDATE invoices SET
            status              = COALESCE($2, status),
            due_at              = COALESCE($3, due_at),
            notes               = COALESCE($4, notes),
            stripe_invoice_id   = COALESCE($5, stripe_invoice_id),
            xendit_invoice_id   = COALESCE($6, xendit_invoice_id),
            lemonsqueezy_order_id = COALESCE($7, lemonsqueezy_order_id),
            issued_at           = COALESCE($8, issued_at),
            version             = version + 1,
            updated_at          = NOW()
        WHERE id = $1
          AND deleted_at IS NULL
          AND version = $9
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.status)
    .bind(req.due_at)
    .bind(&req.notes)
    .bind(&req.stripe_invoice_id)
    .bind(&req.xendit_invoice_id)
    .bind(&req.lemonsqueezy_order_id)
    .bind(issued_at)
    .bind(req.version)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        BillingError::conflict(format!(
            "invoice {id} was modified concurrently (version mismatch)"
        ))
    })?;

    Ok(row)
}

pub async fn delete_invoice(pool: &PgPool, id: &str) -> Result<()> {
    let result = sqlx::query(
        "UPDATE invoices SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(BillingError::not_found("invoice", id));
    }
    Ok(())
}

// ---- Invoice Items ----

pub async fn add_invoice_item(
    pool: &PgPool,
    invoice_id: &str,
    req: AddInvoiceItemRequest,
) -> Result<InvoiceItem> {
    req.validate().map_err(BillingError::from_validation)?;

    // Ensure invoice exists
    let _inv = get_invoice(pool, invoice_id).await?;

    let amount = (req.quantity * req.unit_price).round_dp(2);

    let mut tx = pool.begin().await?;

    let item = sqlx::query_as::<_, InvoiceItem>(
        r#"
        INSERT INTO invoice_items
            (id, invoice_id, description, quantity, unit_price, amount, period_start, period_end)
        VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(invoice_id)
    .bind(&req.description)
    .bind(req.quantity)
    .bind(req.unit_price)
    .bind(amount)
    .bind(req.period_start)
    .bind(req.period_end)
    .fetch_one(&mut *tx)
    .await?;

    // Recompute invoice totals
    recompute_invoice_totals(&mut tx, invoice_id).await?;

    tx.commit().await?;
    Ok(item)
}

pub async fn list_invoice_items(pool: &PgPool, invoice_id: &str) -> Result<Vec<InvoiceItem>> {
    let rows = sqlx::query_as::<_, InvoiceItem>(
        "SELECT * FROM invoice_items WHERE invoice_id = $1 ORDER BY id",
    )
    .bind(invoice_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

// ---- Internal helpers ----

async fn recompute_invoice_totals(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    invoice_id: &str,
) -> Result<()> {
    let subtotal: Option<Decimal> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM invoice_items WHERE invoice_id = $1",
    )
    .bind(invoice_id)
    .fetch_one(&mut **tx)
    .await?;

    let subtotal = subtotal.unwrap_or_default();

    // Keep existing tax ratio
    let (old_subtotal, old_tax): (Decimal, Decimal) =
        sqlx::query_as("SELECT subtotal, tax FROM invoices WHERE id = $1")
            .bind(invoice_id)
            .fetch_one(&mut **tx)
            .await?;

    let tax = if old_subtotal > Decimal::ZERO {
        (subtotal * old_tax / old_subtotal).round_dp(2)
    } else {
        Decimal::ZERO
    };
    let total = subtotal + tax;

    sqlx::query(
        "UPDATE invoices SET subtotal = $2, tax = $3, total = $4, updated_at = NOW() WHERE id = $1",
    )
    .bind(invoice_id)
    .bind(subtotal)
    .bind(tax)
    .bind(total)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

fn format_cycle(cycle: &BillingCycle) -> &'static str {
    match cycle {
        BillingCycle::Monthly => "Monthly",
        BillingCycle::Quarterly => "Quarterly",
        BillingCycle::Yearly => "Yearly",
    }
}
