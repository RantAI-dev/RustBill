use super::schema::{CreateCouponRequest, UpdateCouponRequest};
use async_trait::async_trait;
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait CouponsRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn get(&self, id: &str) -> Result<serde_json::Value, BillingError>;
    async fn create(&self, body: &CreateCouponRequest) -> Result<serde_json::Value, BillingError>;
    async fn update(
        &self,
        id: &str,
        body: &UpdateCouponRequest,
    ) -> Result<serde_json::Value, BillingError>;
    async fn delete(&self, id: &str) -> Result<u64, BillingError>;
}

#[derive(Clone)]
pub struct SqlxCouponsRepository {
    pool: PgPool,
}

impl SqlxCouponsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CouponsRepository for SqlxCouponsRepository {
    async fn list(&self) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(c) FROM coupons c WHERE c.deleted_at IS NULL ORDER BY c.created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn get(&self, id: &str) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(c) FROM coupons c WHERE c.id = $1 AND c.deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("coupon", id))
    }

    async fn create(&self, body: &CreateCouponRequest) -> Result<serde_json::Value, BillingError> {
        let code = body.code.as_ref().and_then(serde_json::Value::as_str);
        let name = body
            .name
            .as_ref()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| code.unwrap_or("Untitled"));

        sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO coupons (id, code, name, discount_type, discount_value, currency, max_redemptions, times_redeemed, valid_from, valid_until, active, applies_to, created_at, updated_at)
               VALUES (gen_random_uuid()::text, $1, $2, $3::discount_type, $4::numeric, COALESCE($5, 'USD'), $6, 0, COALESCE($7::timestamp, now()), $8::timestamp, COALESCE($9, true), $10::jsonb, now(), now())
               RETURNING to_jsonb(coupons.*)"#,
        )
        .bind(code)
        .bind(name)
        .bind(body.discount_type.as_ref().and_then(serde_json::Value::as_str))
        .bind(
            body.discount_value
                .as_ref()
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
        )
        .bind(body.currency.as_ref().and_then(serde_json::Value::as_str))
        .bind(
            body.max_redemptions
                .as_ref()
                .and_then(serde_json::Value::as_i64)
                .map(|value| value as i32),
        )
        .bind(body.valid_from.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.valid_until.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.active.as_ref().and_then(serde_json::Value::as_bool))
        .bind(body.applies_to.as_ref())
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn update(
        &self,
        id: &str,
        body: &UpdateCouponRequest,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"UPDATE coupons SET
                 code = COALESCE($2, code),
                 name = COALESCE($3, name),
                 discount_type = COALESCE($4::discount_type, discount_type),
                 discount_value = COALESCE($5::numeric, discount_value),
                 currency = COALESCE($6, currency),
                 max_redemptions = COALESCE($7, max_redemptions),
                 valid_until = COALESCE($8::timestamp, valid_until),
                 active = COALESCE($9, active),
                 applies_to = COALESCE($10::jsonb, applies_to),
                 updated_at = now()
               WHERE id = $1 AND deleted_at IS NULL
               RETURNING to_jsonb(coupons.*)"#,
        )
        .bind(id)
        .bind(body.code.as_ref().and_then(serde_json::Value::as_str))
        .bind(body.name.as_ref().and_then(serde_json::Value::as_str))
        .bind(
            body.discount_type
                .as_ref()
                .and_then(serde_json::Value::as_str),
        )
        .bind(
            body.discount_value
                .as_ref()
                .and_then(serde_json::Value::as_f64),
        )
        .bind(body.currency.as_ref().and_then(serde_json::Value::as_str))
        .bind(
            body.max_redemptions
                .as_ref()
                .and_then(serde_json::Value::as_i64)
                .map(|value| value as i32),
        )
        .bind(
            body.valid_until
                .as_ref()
                .and_then(serde_json::Value::as_str),
        )
        .bind(body.active.as_ref().and_then(serde_json::Value::as_bool))
        .bind(body.applies_to.as_ref())
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("coupon", id))
    }

    async fn delete(&self, id: &str) -> Result<u64, BillingError> {
        let result = sqlx::query("UPDATE coupons SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(BillingError::from)?;

        Ok(result.rows_affected())
    }
}
