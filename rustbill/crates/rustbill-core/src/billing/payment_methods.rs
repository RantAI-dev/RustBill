use sqlx::PgPool;

use crate::db::models::{PaymentProvider, SavedPaymentMethod, SavedPaymentMethodType};
use crate::error::{BillingError, Result};

pub async fn list_for_customer(
    pool: &PgPool,
    customer_id: &str,
) -> Result<Vec<SavedPaymentMethod>> {
    let methods = sqlx::query_as::<_, SavedPaymentMethod>(
        "SELECT * FROM saved_payment_methods WHERE customer_id = $1 ORDER BY is_default DESC, created_at DESC",
    )
    .bind(customer_id)
    .fetch_all(pool)
    .await?;
    Ok(methods)
}

pub async fn get_default(pool: &PgPool, customer_id: &str) -> Result<Option<SavedPaymentMethod>> {
    let method = sqlx::query_as::<_, SavedPaymentMethod>(
        "SELECT * FROM saved_payment_methods WHERE customer_id = $1 AND is_default = TRUE AND status = 'active'",
    )
    .bind(customer_id)
    .fetch_optional(pool)
    .await?;
    Ok(method)
}

pub struct CreatePaymentMethodRequest {
    pub customer_id: String,
    pub provider: PaymentProvider,
    pub provider_token: String,
    pub method_type: SavedPaymentMethodType,
    pub label: String,
    pub last_four: Option<String>,
    pub expiry_month: Option<i32>,
    pub expiry_year: Option<i32>,
    pub set_default: bool,
}

pub async fn create(pool: &PgPool, req: CreatePaymentMethodRequest) -> Result<SavedPaymentMethod> {
    let mut tx = pool.begin().await?;

    if req.set_default {
        sqlx::query(
            "UPDATE saved_payment_methods SET is_default = FALSE, updated_at = NOW() WHERE customer_id = $1 AND is_default = TRUE",
        )
        .bind(&req.customer_id)
        .execute(&mut *tx)
        .await?;
    }

    let existing_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM saved_payment_methods WHERE customer_id = $1 AND status = 'active'",
    )
    .bind(&req.customer_id)
    .fetch_one(&mut *tx)
    .await?;
    let is_default = req.set_default || existing_count == 0;

    let method = sqlx::query_as::<_, SavedPaymentMethod>(
        r#"INSERT INTO saved_payment_methods
           (id, customer_id, provider, provider_token, method_type, label, last_four, expiry_month, expiry_year, is_default, status)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, $8, $9, 'active')
           RETURNING *"#,
    )
    .bind(&req.customer_id)
    .bind(&req.provider)
    .bind(&req.provider_token)
    .bind(&req.method_type)
    .bind(&req.label)
    .bind(&req.last_four)
    .bind(req.expiry_month)
    .bind(req.expiry_year)
    .bind(is_default)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(method)
}

pub async fn set_default(
    pool: &PgPool,
    customer_id: &str,
    method_id: &str,
) -> Result<SavedPaymentMethod> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        "UPDATE saved_payment_methods SET is_default = FALSE, updated_at = NOW() WHERE customer_id = $1 AND is_default = TRUE",
    )
    .bind(customer_id)
    .execute(&mut *tx)
    .await?;

    let method = sqlx::query_as::<_, SavedPaymentMethod>(
        "UPDATE saved_payment_methods SET is_default = TRUE, updated_at = NOW() WHERE id = $1 AND customer_id = $2 RETURNING *",
    )
    .bind(method_id)
    .bind(customer_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| BillingError::not_found("payment_method", method_id))?;

    tx.commit().await?;
    Ok(method)
}

pub async fn remove(pool: &PgPool, customer_id: &str, method_id: &str) -> Result<()> {
    let result =
        sqlx::query("DELETE FROM saved_payment_methods WHERE id = $1 AND customer_id = $2")
            .bind(method_id)
            .bind(customer_id)
            .execute(pool)
            .await?;

    if result.rows_affected() == 0 {
        return Err(BillingError::not_found("payment_method", method_id));
    }
    Ok(())
}

pub async fn mark_failed(pool: &PgPool, method_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE saved_payment_methods SET status = 'failed', is_default = FALSE, updated_at = NOW() WHERE id = $1",
    )
    .bind(method_id)
    .execute(pool)
    .await?;
    Ok(())
}
