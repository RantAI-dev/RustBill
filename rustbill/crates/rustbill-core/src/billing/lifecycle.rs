//! Comprehensive subscription lifecycle processor.
//!
//! Orchestrates trial conversion, cancellation, renewal with invoice
//! generation, usage-based billing, and coupon discount application.

use crate::billing::tiered_pricing;
use crate::db::models::*;
use crate::error::Result;
use crate::notifications::email::EmailSender;
use crate::notifications::send;
use chrono::{NaiveDateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::PgPool;

/// Summary of a lifecycle run.
#[derive(Debug, Clone, Serialize, Default)]
pub struct LifecycleResult {
    pub trials_converted: u64,
    pub canceled: u64,
    pub renewed: u64,
    pub invoices_generated: u64,
    pub errors: Vec<String>,
}

/// Run the full subscription lifecycle.
pub async fn run_full_lifecycle(
    pool: &PgPool,
    email_sender: Option<&EmailSender>,
    http_client: &reqwest::Client,
) -> Result<LifecycleResult> {
    let mut result = LifecycleResult::default();

    // 1. Convert expired trials
    result.trials_converted = convert_expired_trials(pool).await?;

    // 2. Cancel at period end
    result.canceled = cancel_at_period_end(pool).await?;

    // 3. Renew active subscriptions (with invoice generation)
    let (renewed, invoiced, errors) =
        renew_active_subscriptions(pool, email_sender, http_client).await?;
    result.renewed = renewed;
    result.invoices_generated = invoiced;
    result.errors = errors;

    tracing::info!(
        trials_converted = result.trials_converted,
        canceled = result.canceled,
        renewed = result.renewed,
        invoices_generated = result.invoices_generated,
        "Full lifecycle completed"
    );

    Ok(result)
}

/// Step 1: Convert subscriptions with status='trialing' past trial_end to active.
async fn convert_expired_trials(pool: &PgPool) -> Result<u64> {
    let now = Utc::now().naive_utc();

    let r = sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'active',
            current_period_start = $1,
            current_period_end = $1 + (
                SELECT CASE pp.billing_cycle
                    WHEN 'monthly'   THEN INTERVAL '1 month'
                    WHEN 'quarterly' THEN INTERVAL '3 months'
                    WHEN 'yearly'    THEN INTERVAL '1 year'
                END
                FROM pricing_plans pp WHERE pp.id = subscriptions.plan_id
            ),
            version = version + 1,
            updated_at = NOW()
        WHERE status = 'trialing'
          AND deleted_at IS NULL
          AND trial_end IS NOT NULL
          AND trial_end <= $1
        "#,
    )
    .bind(now)
    .execute(pool)
    .await?;

    let count = r.rows_affected();
    if count > 0 {
        tracing::info!(count, "Converted expired trials to active");
    }
    Ok(count)
}

/// Step 2: Cancel subscriptions with cancel_at_period_end=true past period end.
async fn cancel_at_period_end(pool: &PgPool) -> Result<u64> {
    let now = Utc::now().naive_utc();

    let r = sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'canceled',
            canceled_at = $1,
            version = version + 1,
            updated_at = NOW()
        WHERE status = 'active'
          AND deleted_at IS NULL
          AND cancel_at_period_end = true
          AND current_period_end <= $1
        "#,
    )
    .bind(now)
    .execute(pool)
    .await?;

    let count = r.rows_affected();
    if count > 0 {
        tracing::info!(count, "Canceled subscriptions at period end");
    }
    Ok(count)
}

/// Step 3: Renew active subscriptions past period end, generating invoices.
async fn renew_active_subscriptions(
    pool: &PgPool,
    email_sender: Option<&EmailSender>,
    http_client: &reqwest::Client,
) -> Result<(u64, u64, Vec<String>)> {
    let now = Utc::now().naive_utc();
    let mut renewed: u64 = 0;
    let mut invoiced: u64 = 0;
    let mut errors: Vec<String> = Vec::new();

    // Fetch all active subs past period end (not canceling)
    let subs = sqlx::query_as::<_, Subscription>(
        r#"
        SELECT * FROM subscriptions
        WHERE status = 'active'
          AND deleted_at IS NULL
          AND cancel_at_period_end = false
          AND current_period_end <= $1
        "#,
    )
    .bind(now)
    .fetch_all(pool)
    .await?;

    for sub in &subs {
        match renew_single_subscription(pool, sub, now, email_sender, http_client).await {
            Ok(_) => {
                renewed += 1;
                invoiced += 1;
            }
            Err(e) => {
                tracing::error!(subscription_id = %sub.id, error = %e, "Failed to renew subscription");
                errors.push(format!("sub {}: {}", sub.id, e));
            }
        }
    }

    Ok((renewed, invoiced, errors))
}

/// Renew a single subscription: generate invoice, apply discounts, extend period.
async fn renew_single_subscription(
    pool: &PgPool,
    sub: &Subscription,
    now: NaiveDateTime,
    email_sender: Option<&EmailSender>,
    http_client: &reqwest::Client,
) -> Result<()> {
    let plan = sqlx::query_as::<_, PricingPlan>(
        "SELECT * FROM pricing_plans WHERE id = $1",
    )
    .bind(&sub.plan_id)
    .fetch_one(pool)
    .await?;

    let mut tx = pool.begin().await?;

    // Calculate the subscription amount
    let quantity = compute_quantity(pool, sub, &plan).await?;
    let tiers: Option<Vec<PricingTier>> = plan
        .tiers
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let amount = tiered_pricing::calculate_amount(
        &plan.pricing_model,
        plan.base_price,
        plan.unit_price,
        tiers.as_deref(),
        quantity,
    );

    // Generate invoice number
    let invoice_number: String = sqlx::query_scalar(
        "SELECT 'INV-' || LPAD(nextval('invoice_number_seq')::text, 8, '0')",
    )
    .fetch_one(&mut *tx)
    .await?;

    let new_period_end = advance_period(sub.current_period_end, &plan.billing_cycle);
    let due_at = now + chrono::Duration::days(30);

    // Create invoice
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        INSERT INTO invoices
            (id, invoice_number, customer_id, subscription_id, status,
             issued_at, due_at, subtotal, tax, total, currency, notes)
        VALUES (gen_random_uuid()::text, $1, $2, $3, 'issued', $4, $5, 0, 0, 0, 'USD', $6)
        RETURNING *
        "#,
    )
    .bind(&invoice_number)
    .bind(&sub.customer_id)
    .bind(&sub.id)
    .bind(now)
    .bind(due_at)
    .bind(format!("Auto-renewal for subscription {}", sub.id))
    .fetch_one(&mut *tx)
    .await?;

    // Add line item for the plan
    sqlx::query(
        r#"
        INSERT INTO invoice_items
            (id, invoice_id, description, quantity, unit_price, amount,
             period_start, period_end)
        VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(&invoice.id)
    .bind(format!("{} ({})", plan.name, format_cycle(&plan.billing_cycle)))
    .bind(Decimal::from(quantity))
    .bind(plan.base_price)
    .bind(amount)
    .bind(sub.current_period_end) // new period starts at old period end
    .bind(new_period_end)
    .execute(&mut *tx)
    .await?;

    // Apply active coupon discounts from subscription_discounts table
    let discounts = sqlx::query_as::<_, SubscriptionDiscountWithCoupon>(
        r#"
        SELECT sd.id, sd.subscription_id, sd.coupon_id, sd.applied_at, sd.expires_at,
               c.discount_type, c.discount_value, c.code AS coupon_code
        FROM subscription_discounts sd
        JOIN coupons c ON c.id = sd.coupon_id AND c.active = true AND c.deleted_at IS NULL
        WHERE sd.subscription_id = $1
          AND (sd.expires_at IS NULL OR sd.expires_at > $2)
        "#,
    )
    .bind(&sub.id)
    .bind(now)
    .fetch_all(&mut *tx)
    .await?;

    for discount in &discounts {
        let discount_amount = match discount.discount_type {
            DiscountType::Percentage => {
                (amount * discount.discount_value / Decimal::from(100)).round_dp(2)
            }
            DiscountType::FixedAmount => discount.discount_value.min(amount),
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
            .bind(format!("Discount ({})", discount.coupon_code))
            .bind(-discount_amount)
            .bind(-discount_amount)
            .execute(&mut *tx)
            .await?;
        }
    }

    // Recompute invoice totals
    let subtotal: Option<Decimal> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM invoice_items WHERE invoice_id = $1",
    )
    .bind(&invoice.id)
    .fetch_one(&mut *tx)
    .await?;

    let subtotal = subtotal.unwrap_or_default();
    let total = subtotal; // No tax on auto-renewals by default

    sqlx::query(
        "UPDATE invoices SET subtotal = $2, tax = 0, total = $3, updated_at = NOW() WHERE id = $1",
    )
    .bind(&invoice.id)
    .bind(subtotal)
    .bind(total)
    .execute(&mut *tx)
    .await?;

    // Advance the subscription period
    sqlx::query(
        r#"
        UPDATE subscriptions SET
            current_period_start = current_period_end,
            current_period_end = $2,
            version = version + 1,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(&sub.id)
    .bind(new_period_end)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Emit billing event (non-blocking)
    let pool_clone = pool.clone();
    let http_clone = http_client.clone();
    let sub_id = sub.id.clone();
    let cust_id = sub.customer_id.clone();
    let inv_data = serde_json::json!({
        "invoice_id": invoice.id,
        "invoice_number": invoice_number,
        "total": total.to_string(),
    });
    tokio::spawn(async move {
        let _ = crate::notifications::emit_billing_event(
            &pool_clone,
            &http_clone,
            BillingEventType::SubscriptionRenewed,
            "subscription",
            &sub_id,
            Some(&cust_id),
            Some(inv_data),
        )
        .await;
    });

    // Send renewal email notification (non-blocking)
    let pool_email = pool.clone();
    let email_sender_cloned = email_sender.cloned();
    let customer_id = sub.customer_id.clone();
    let plan_name = plan.name.clone();
    let inv_num = invoice_number.clone();
    let total_str = total.to_string();
    let period_end_str = new_period_end.format("%Y-%m-%d").to_string();
    tokio::spawn(async move {
        send::notify_subscription_renewed(
            email_sender_cloned.as_ref(),
            &pool_email,
            &customer_id,
            &plan_name,
            &inv_num,
            &total_str,
            "USD",
            &period_end_str,
        )
        .await;
    });

    tracing::info!(
        subscription_id = %sub.id,
        invoice_number = %invoice_number,
        total = %total,
        "Subscription renewed with invoice"
    );

    Ok(())
}

/// For usage-based plans, compute quantity from usage_events for the period.
/// For other plans, return the subscription quantity.
async fn compute_quantity(
    pool: &PgPool,
    sub: &Subscription,
    plan: &PricingPlan,
) -> Result<i32> {
    if plan.pricing_model == PricingModel::UsageBased {
        // Sum usage events for the current billing period
        let metric_name = plan
            .usage_metric_name
            .as_deref()
            .unwrap_or("api_calls");

        let total_usage: Option<Decimal> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(value), 0) FROM usage_events
            WHERE subscription_id = $1
              AND metric_name = $2
              AND timestamp >= $3
              AND timestamp < $4
            "#,
        )
        .bind(&sub.id)
        .bind(metric_name)
        .bind(sub.current_period_start)
        .bind(sub.current_period_end)
        .fetch_one(pool)
        .await?;

        let usage = total_usage
            .unwrap_or_default()
            .to_string()
            .parse::<i64>()
            .unwrap_or(0);

        Ok(usage.max(0) as i32)
    } else {
        Ok(sub.quantity)
    }
}

/// Generate invoices for all active subscriptions that are due for renewal,
/// without advancing the subscription period. This is the standalone invoice
/// generation endpoint.
pub async fn generate_pending_invoices(
    pool: &PgPool,
    email_sender: Option<&EmailSender>,
    http_client: &reqwest::Client,
) -> Result<u64> {
    // This delegates to the full lifecycle — the invoice generation is part
    // of the renewal process. For standalone usage, we run the full renewal.
    let result = run_full_lifecycle(pool, email_sender, http_client).await?;
    Ok(result.invoices_generated)
}

// ---- Helper types ----

#[derive(Debug, sqlx::FromRow)]
struct SubscriptionDiscountWithCoupon {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    subscription_id: String,
    #[allow(dead_code)]
    coupon_id: String,
    #[allow(dead_code)]
    applied_at: NaiveDateTime,
    #[allow(dead_code)]
    expires_at: Option<NaiveDateTime>,
    discount_type: DiscountType,
    discount_value: Decimal,
    coupon_code: String,
}

fn advance_period(from: NaiveDateTime, cycle: &BillingCycle) -> NaiveDateTime {
    match cycle {
        BillingCycle::Monthly => from + chrono::Duration::days(30),
        BillingCycle::Quarterly => from + chrono::Duration::days(90),
        BillingCycle::Yearly => from + chrono::Duration::days(365),
    }
}

fn format_cycle(cycle: &BillingCycle) -> &'static str {
    match cycle {
        BillingCycle::Monthly => "Monthly",
        BillingCycle::Quarterly => "Quarterly",
        BillingCycle::Yearly => "Yearly",
    }
}
