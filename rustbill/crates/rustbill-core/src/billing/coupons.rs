use crate::db::models::*;
use crate::error::{BillingError, Result};
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use validator::Validate;

// ---- Request types ----

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCouponRequest {
    #[validate(length(min = 1, max = 50, message = "code is required"))]
    pub code: String,

    #[validate(length(min = 1, max = 255, message = "name is required"))]
    pub name: String,

    pub discount_type: DiscountType,
    pub discount_value: Decimal,
    pub currency: Option<String>,
    pub max_redemptions: Option<i32>,
    pub valid_from: NaiveDateTime,
    pub valid_until: Option<NaiveDateTime>,

    #[serde(default = "default_true")]
    pub active: bool,

    pub applies_to: Option<serde_json::Value>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCouponRequest {
    #[validate(length(min = 1, max = 255, message = "name must not be empty"))]
    pub name: Option<String>,

    pub discount_type: Option<DiscountType>,
    pub discount_value: Option<Decimal>,
    pub currency: Option<String>,
    pub max_redemptions: Option<Option<i32>>,
    pub valid_until: Option<Option<NaiveDateTime>>,
    pub active: Option<bool>,
    pub applies_to: Option<Option<serde_json::Value>>,
}

// ---- View type with computed times_redeemed ----

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CouponView {
    pub id: String,
    pub code: String,
    pub name: String,
    pub discount_type: DiscountType,
    pub discount_value: Decimal,
    pub currency: String,
    pub max_redemptions: Option<i32>,
    pub times_redeemed: i32,
    pub valid_from: NaiveDateTime,
    pub valid_until: Option<NaiveDateTime>,
    pub active: bool,
    pub applies_to: Option<serde_json::Value>,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Service functions ----

pub async fn list_coupons(pool: &PgPool) -> Result<Vec<CouponView>> {
    // Auto-deactivate expired coupons first
    sqlx::query(
        r#"
        UPDATE coupons SET active = false, updated_at = NOW()
        WHERE active = true
          AND deleted_at IS NULL
          AND valid_until IS NOT NULL
          AND valid_until < NOW()
        "#,
    )
    .execute(pool)
    .await?;

    let rows = sqlx::query_as::<_, CouponView>(
        r#"
        SELECT
            c.id, c.code, c.name, c.discount_type, c.discount_value,
            c.currency, c.max_redemptions,
            COALESCE((SELECT COUNT(*)::int FROM subscription_discounts sd WHERE sd.coupon_id = c.id), 0) AS times_redeemed,
            c.valid_from, c.valid_until, c.active, c.applies_to,
            c.deleted_at, c.created_at, c.updated_at
        FROM coupons c
        WHERE c.deleted_at IS NULL
        ORDER BY c.created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn get_coupon(pool: &PgPool, id: &str) -> Result<Coupon> {
    sqlx::query_as::<_, Coupon>(
        "SELECT * FROM coupons WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| BillingError::not_found("coupon", id))
}

pub async fn create_coupon(pool: &PgPool, req: CreateCouponRequest) -> Result<Coupon> {
    req.validate().map_err(BillingError::from_validation)?;

    let currency = req.currency.clone().unwrap_or_else(|| "USD".to_string());

    // Check for duplicate code
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM coupons WHERE code = $1 AND deleted_at IS NULL",
    )
    .bind(&req.code)
    .fetch_optional(pool)
    .await?;

    if existing.is_some() {
        return Err(BillingError::conflict(format!(
            "coupon code '{}' already exists",
            req.code
        )));
    }

    let row = sqlx::query_as::<_, Coupon>(
        r#"
        INSERT INTO coupons
            (id, code, name, discount_type, discount_value, currency,
             max_redemptions, times_redeemed, valid_from, valid_until, active, applies_to)
        VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, 0, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(&req.code)
    .bind(&req.name)
    .bind(&req.discount_type)
    .bind(req.discount_value)
    .bind(&currency)
    .bind(req.max_redemptions)
    .bind(req.valid_from)
    .bind(req.valid_until)
    .bind(req.active)
    .bind(&req.applies_to)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn update_coupon(pool: &PgPool, id: &str, req: UpdateCouponRequest) -> Result<Coupon> {
    req.validate().map_err(BillingError::from_validation)?;

    let _existing = get_coupon(pool, id).await?;

    let row = sqlx::query_as::<_, Coupon>(
        r#"
        UPDATE coupons SET
            name           = COALESCE($2, name),
            discount_type  = COALESCE($3, discount_type),
            discount_value = COALESCE($4, discount_value),
            currency       = COALESCE($5, currency),
            max_redemptions = COALESCE($6, max_redemptions),
            valid_until    = COALESCE($7, valid_until),
            active         = COALESCE($8, active),
            applies_to     = COALESCE($9, applies_to),
            updated_at     = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.discount_type)
    .bind(req.discount_value)
    .bind(&req.currency)
    .bind(req.max_redemptions.flatten())
    .bind(req.valid_until.flatten())
    .bind(req.active)
    .bind(req.applies_to.as_ref().and_then(|o| o.as_ref()))
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn delete_coupon(pool: &PgPool, id: &str) -> Result<()> {
    let result = sqlx::query(
        "UPDATE coupons SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(BillingError::not_found("coupon", id));
    }
    Ok(())
}

/// Apply a coupon to a subscription by creating a subscription_discount record.
pub async fn apply_coupon(
    pool: &PgPool,
    subscription_id: &str,
    coupon_id: &str,
    expires_at: Option<NaiveDateTime>,
) -> Result<SubscriptionDiscount> {
    // Validate subscription exists
    let _sub = crate::billing::subscriptions::get_subscription(pool, subscription_id).await?;

    // Validate coupon exists and is active
    let coupon = get_coupon(pool, coupon_id).await?;
    if !coupon.active {
        return Err(BillingError::bad_request("coupon is not active"));
    }

    // Check max redemptions
    if let Some(max) = coupon.max_redemptions {
        if coupon.times_redeemed >= max {
            return Err(BillingError::bad_request(
                "coupon has reached max redemptions",
            ));
        }
    }

    // Check not already applied
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM subscription_discounts WHERE subscription_id = $1 AND coupon_id = $2",
    )
    .bind(subscription_id)
    .bind(coupon_id)
    .fetch_optional(pool)
    .await?;

    if existing.is_some() {
        return Err(BillingError::conflict(
            "coupon already applied to this subscription",
        ));
    }

    let mut tx = pool.begin().await?;

    let sd = sqlx::query_as::<_, SubscriptionDiscount>(
        r#"
        INSERT INTO subscription_discounts
            (id, subscription_id, coupon_id, applied_at, expires_at)
        VALUES (gen_random_uuid()::text, $1, $2, NOW(), $3)
        RETURNING *
        "#,
    )
    .bind(subscription_id)
    .bind(coupon_id)
    .bind(expires_at)
    .fetch_one(&mut *tx)
    .await?;

    // Increment times_redeemed
    sqlx::query("UPDATE coupons SET times_redeemed = times_redeemed + 1 WHERE id = $1")
        .bind(coupon_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(sd)
}
