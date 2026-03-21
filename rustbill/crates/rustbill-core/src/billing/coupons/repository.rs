use super::schema::{CouponView, CreateCouponRequest, UpdateCouponRequest};
use crate::db::models::{Coupon, Subscription, SubscriptionDiscount};
use crate::error::Result;
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
pub trait CouponsRepository: Send + Sync {
    async fn list_coupons(&self) -> Result<Vec<CouponView>>;
    async fn get_coupon(&self, id: &str) -> Result<Option<Coupon>>;
    async fn find_coupon_by_code(&self, code: &str) -> Result<Option<Coupon>>;
    async fn create_coupon(&self, req: &CreateCouponRequest) -> Result<Coupon>;
    async fn update_coupon(&self, id: &str, req: &UpdateCouponRequest) -> Result<Coupon>;
    async fn delete_coupon(&self, id: &str) -> Result<u64>;
    async fn find_subscription(&self, id: &str) -> Result<Option<Subscription>>;
    async fn subscription_has_coupon(&self, subscription_id: &str, coupon_id: &str)
        -> Result<bool>;
    async fn apply_coupon(
        &self,
        subscription: &Subscription,
        coupon: &Coupon,
        expires_at: Option<chrono::NaiveDateTime>,
    ) -> Result<SubscriptionDiscount>;
}

#[derive(Clone)]
pub struct PgCouponsRepository {
    pool: PgPool,
}

impl PgCouponsRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl CouponsRepository for PgCouponsRepository {
    async fn list_coupons(&self) -> Result<Vec<CouponView>> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE coupons SET active = false, updated_at = NOW()
            WHERE active = true
              AND deleted_at IS NULL
              AND valid_until IS NOT NULL
              AND valid_until < NOW()
            "#,
        )
        .execute(&mut *tx)
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
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(rows)
    }

    async fn get_coupon(&self, id: &str) -> Result<Option<Coupon>> {
        let row = sqlx::query_as::<_, Coupon>(
            "SELECT * FROM coupons WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn find_coupon_by_code(&self, code: &str) -> Result<Option<Coupon>> {
        let row = sqlx::query_as::<_, Coupon>(
            "SELECT * FROM coupons WHERE code = $1 AND deleted_at IS NULL",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_coupon(&self, req: &CreateCouponRequest) -> Result<Coupon> {
        let currency = req.currency.clone().unwrap_or_else(|| "USD".to_string());

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
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn update_coupon(&self, id: &str, req: &UpdateCouponRequest) -> Result<Coupon> {
        let row = sqlx::query_as::<_, Coupon>(
            r#"
            UPDATE coupons SET
                name            = COALESCE($2, name),
                discount_type   = COALESCE($3, discount_type),
                discount_value  = COALESCE($4, discount_value),
                currency        = COALESCE($5, currency),
                max_redemptions = COALESCE($6, max_redemptions),
                valid_until     = COALESCE($7, valid_until),
                active          = COALESCE($8, active),
                applies_to      = COALESCE($9, applies_to),
                updated_at      = NOW()
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
        .bind(req.applies_to.as_ref().and_then(|v| v.as_ref()))
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn delete_coupon(&self, id: &str) -> Result<u64> {
        let result = sqlx::query(
            "UPDATE coupons SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn find_subscription(&self, id: &str) -> Result<Option<Subscription>> {
        let row = sqlx::query_as::<_, Subscription>(
            "SELECT * FROM subscriptions WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn subscription_has_coupon(
        &self,
        subscription_id: &str,
        coupon_id: &str,
    ) -> Result<bool> {
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM subscription_discounts WHERE subscription_id = $1 AND coupon_id = $2",
        )
        .bind(subscription_id)
        .bind(coupon_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(existing.is_some())
    }

    async fn apply_coupon(
        &self,
        subscription: &Subscription,
        coupon: &Coupon,
        expires_at: Option<chrono::NaiveDateTime>,
    ) -> Result<SubscriptionDiscount> {
        let mut tx = self.pool.begin().await?;

        let sd = sqlx::query_as::<_, SubscriptionDiscount>(
            r#"
            INSERT INTO subscription_discounts
                (id, subscription_id, coupon_id, applied_at, expires_at)
            VALUES (gen_random_uuid()::text, $1, $2, NOW(), $3)
            RETURNING *
            "#,
        )
        .bind(&subscription.id)
        .bind(&coupon.id)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE coupons SET times_redeemed = times_redeemed + 1 WHERE id = $1")
            .bind(&coupon.id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(sd)
    }
}
