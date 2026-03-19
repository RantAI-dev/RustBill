use crate::analytics::sales_ledger::{emit_sales_event, NewSalesEvent, SalesClassification};
use crate::db::models::*;
use crate::error::{BillingError, Result};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use validator::Validate;

// ---- Request types ----

#[derive(Debug, Deserialize, Validate)]
pub struct CreateSubscriptionRequest {
    #[validate(length(min = 1, message = "customer_id is required"))]
    pub customer_id: String,

    #[validate(length(min = 1, message = "plan_id is required"))]
    pub plan_id: String,

    #[serde(default = "default_quantity")]
    pub quantity: i32,

    pub metadata: Option<serde_json::Value>,
    pub stripe_subscription_id: Option<String>,
}

fn default_quantity() -> i32 {
    1
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateSubscriptionRequest {
    pub status: Option<SubscriptionStatus>,
    pub quantity: Option<i32>,
    pub cancel_at_period_end: Option<bool>,
    pub canceled_at: Option<NaiveDateTime>,
    pub metadata: Option<serde_json::Value>,
    pub stripe_subscription_id: Option<String>,

    /// Required for optimistic locking -- must match the current version.
    pub version: i32,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListSubscriptionsFilter {
    pub status: Option<SubscriptionStatus>,
    pub customer_id: Option<String>,
    /// Customer role isolation.
    pub role_customer_id: Option<String>,
}

// ---- View type (subscription + joined names) ----

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SubscriptionView {
    pub id: String,
    pub customer_id: String,
    pub plan_id: String,
    pub status: SubscriptionStatus,
    pub current_period_start: NaiveDateTime,
    pub current_period_end: NaiveDateTime,
    pub canceled_at: Option<NaiveDateTime>,
    pub cancel_at_period_end: bool,
    pub trial_end: Option<NaiveDateTime>,
    pub quantity: i32,
    pub metadata: Option<serde_json::Value>,
    pub stripe_subscription_id: Option<String>,
    pub version: i32,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    // Joined
    pub customer_name: Option<String>,
    pub plan_name: Option<String>,
}

// ---- Service functions ----

pub async fn list_subscriptions(
    pool: &PgPool,
    filter: &ListSubscriptionsFilter,
) -> Result<Vec<SubscriptionView>> {
    let rows = sqlx::query_as::<_, SubscriptionView>(
        r#"
        SELECT
            s.*,
            c.name AS customer_name,
            pp.name AS plan_name
        FROM subscriptions s
        LEFT JOIN customers c ON c.id = s.customer_id
        LEFT JOIN pricing_plans pp ON pp.id = s.plan_id
        WHERE s.deleted_at IS NULL
          AND ($1::subscription_status IS NULL OR s.status = $1)
          AND ($2::text IS NULL OR s.customer_id = $2)
          AND ($3::text IS NULL OR s.customer_id = $3)
        ORDER BY s.created_at DESC
        "#,
    )
    .bind(&filter.status)
    .bind(&filter.customer_id)
    .bind(&filter.role_customer_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn get_subscription(pool: &PgPool, id: &str) -> Result<Subscription> {
    sqlx::query_as::<_, Subscription>(
        "SELECT * FROM subscriptions WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| BillingError::not_found("subscription", id))
}

pub async fn create_subscription(
    pool: &PgPool,
    req: CreateSubscriptionRequest,
) -> Result<Subscription> {
    req.validate().map_err(BillingError::from_validation)?;

    // Fetch the plan to compute period dates and trial
    let plan = crate::billing::plans::get_plan(pool, &req.plan_id).await?;

    let now = Utc::now().naive_utc();
    let (status, trial_end, period_start, period_end) = if plan.trial_days > 0 {
        let trial_end = now + chrono::Duration::days(plan.trial_days as i64);
        (
            SubscriptionStatus::Trialing,
            Some(trial_end),
            now,
            trial_end,
        )
    } else {
        let period_end = advance_period(now, &plan.billing_cycle);
        (SubscriptionStatus::Active, None, now, period_end)
    };

    let row = sqlx::query_as::<_, Subscription>(
        r#"
        INSERT INTO subscriptions
            (id, customer_id, plan_id, status, current_period_start, current_period_end,
             trial_end, quantity, metadata, stripe_subscription_id, version,
             cancel_at_period_end)
        VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, $8, $9, 1, false)
        RETURNING *
        "#,
    )
    .bind(&req.customer_id)
    .bind(&req.plan_id)
    .bind(&status)
    .bind(period_start)
    .bind(period_end)
    .bind(trial_end)
    .bind(req.quantity)
    .bind(&req.metadata)
    .bind(&req.stripe_subscription_id)
    .fetch_one(pool)
    .await?;

    if let Err(err) = emit_sales_event(
        pool,
        NewSalesEvent {
            occurred_at: chrono::Utc::now(),
            event_type: "subscription.created",
            classification: SalesClassification::Recurring,
            amount_subtotal: rust_decimal::Decimal::ZERO,
            amount_tax: rust_decimal::Decimal::ZERO,
            amount_total: rust_decimal::Decimal::ZERO,
            currency: "USD",
            customer_id: Some(&row.customer_id),
            subscription_id: Some(&row.id),
            product_id: None,
            invoice_id: None,
            payment_id: None,
            source_table: "subscriptions",
            source_id: &row.id,
            metadata: Some(serde_json::json!({
                "status": row.status,
                "plan_id": row.plan_id,
                "quantity": row.quantity,
            })),
        },
    )
    .await
    {
        tracing::warn!(error = %err, subscription_id = %row.id, "failed to emit sales event subscription.created");
    }

    Ok(row)
}

pub async fn update_subscription(
    pool: &PgPool,
    id: &str,
    req: UpdateSubscriptionRequest,
) -> Result<Subscription> {
    let row = sqlx::query_as::<_, Subscription>(
        r#"
        UPDATE subscriptions SET
            status              = COALESCE($2, status),
            quantity            = COALESCE($3, quantity),
            cancel_at_period_end = COALESCE($4, cancel_at_period_end),
            canceled_at         = COALESCE($5, canceled_at),
            metadata            = COALESCE($6, metadata),
            stripe_subscription_id = COALESCE($7, stripe_subscription_id),
            version             = version + 1,
            updated_at          = NOW()
        WHERE id = $1
          AND deleted_at IS NULL
          AND version = $8
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.status)
    .bind(req.quantity)
    .bind(req.cancel_at_period_end)
    .bind(req.canceled_at)
    .bind(&req.metadata)
    .bind(&req.stripe_subscription_id)
    .bind(req.version)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        BillingError::conflict(format!(
            "subscription {id} was modified concurrently (version mismatch)"
        ))
    })?;

    Ok(row)
}

pub async fn delete_subscription(pool: &PgPool, id: &str) -> Result<()> {
    let result = sqlx::query(
        "UPDATE subscriptions SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(BillingError::not_found("subscription", id));
    }
    Ok(())
}

/// Run the subscription lifecycle:
///  1. Trialing subs past trial_end -> activate
///  2. Active subs with cancel_at_period_end past period_end -> cancel
///  3. Active subs past period_end -> renew (advance period dates)
pub async fn run_lifecycle(pool: &PgPool) -> Result<u64> {
    let now = Utc::now().naive_utc();
    let mut processed: u64 = 0;

    // 1. Trial -> Active
    let r = sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'active',
            current_period_end = current_period_end + (
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
    processed += r.rows_affected();

    // 2. Cancel at period end
    let r = sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'canceled',
            canceled_at = NOW(),
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
    processed += r.rows_affected();

    // 3. Renew active subs
    let r = sqlx::query(
        r#"
        UPDATE subscriptions
        SET current_period_start = current_period_end,
            current_period_end = current_period_end + (
                SELECT CASE pp.billing_cycle
                    WHEN 'monthly'   THEN INTERVAL '1 month'
                    WHEN 'quarterly' THEN INTERVAL '3 months'
                    WHEN 'yearly'    THEN INTERVAL '1 year'
                END
                FROM pricing_plans pp WHERE pp.id = subscriptions.plan_id
            ),
            version = version + 1,
            updated_at = NOW()
        WHERE status = 'active'
          AND deleted_at IS NULL
          AND cancel_at_period_end = false
          AND current_period_end <= $1
        "#,
    )
    .bind(now)
    .execute(pool)
    .await?;
    processed += r.rows_affected();

    tracing::info!(processed, "subscription lifecycle completed");
    Ok(processed)
}

// ---- Helpers ----

/// Advance a period by one billing cycle using calendar-month semantics.
/// Monthly: Jan 15 -> Feb 15 -> Mar 15 (not +30 days).
pub fn advance_period(from: NaiveDateTime, cycle: &BillingCycle) -> NaiveDateTime {
    let date = from.date();
    let time = from.time();
    let months = match cycle {
        BillingCycle::Monthly => 1,
        BillingCycle::Quarterly => 3,
        BillingCycle::Yearly => 12,
    };
    let new_date = date
        .checked_add_months(chrono::Months::new(months))
        .unwrap_or_else(|| date + chrono::Duration::days(months as i64 * 30));
    new_date.and_time(time)
}
