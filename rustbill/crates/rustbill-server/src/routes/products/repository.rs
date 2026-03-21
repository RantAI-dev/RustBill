use super::schema::{CreateProductRequest, ProductListItem, UpdateProductRequest};
use async_trait::async_trait;
use rust_decimal::Decimal;
use rustbill_core::db::models::{Product, ProductType};
use rustbill_core::error::{BillingError, Result};
use sqlx::PgPool;

#[async_trait]
pub trait ProductsRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<ProductListItem>>;
    async fn get(&self, id: &str) -> Result<Product>;
    async fn create(&self, body: &CreateProductRequest) -> Result<Product>;
    async fn update(&self, id: &str, body: &UpdateProductRequest) -> Result<Product>;
    async fn delete(&self, id: &str) -> Result<u64>;
}

#[derive(Clone)]
pub struct SqlxProductsRepository {
    pool: PgPool,
}

impl SqlxProductsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProductsRepository for SqlxProductsRepository {
    async fn list(&self) -> Result<Vec<ProductListItem>> {
        let rows = sqlx::query_as::<_, Product>("SELECT * FROM products ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(BillingError::from)?;

        let mut results = Vec::with_capacity(rows.len());
        for product in rows {
            let revenue: Decimal = sqlx::query_scalar(
                "SELECT COALESCE(SUM(value), 0) FROM deals WHERE product_id = $1",
            )
            .bind(&product.id)
            .fetch_one(&self.pool)
            .await
            .map_err(BillingError::from)?;

            let this_month: Decimal = sqlx::query_scalar(
                "SELECT COALESCE(SUM(value), 0) FROM deals WHERE product_id = $1 AND created_at >= date_trunc('month', CURRENT_DATE)",
            )
            .bind(&product.id)
            .fetch_one(&self.pool)
            .await
            .map_err(BillingError::from)?;

            let last_month: Decimal = sqlx::query_scalar(
                "SELECT COALESCE(SUM(value), 0) FROM deals WHERE product_id = $1 AND created_at >= date_trunc('month', CURRENT_DATE) - interval '1 month' AND created_at < date_trunc('month', CURRENT_DATE)",
            )
            .bind(&product.id)
            .fetch_one(&self.pool)
            .await
            .map_err(BillingError::from)?;

            let change = if last_month > Decimal::ZERO {
                ((this_month - last_month) / last_month * Decimal::from(100)).round_dp(2)
            } else if this_month > Decimal::ZERO {
                Decimal::from(100)
            } else {
                Decimal::ZERO
            };

            let (active_licenses, total_licenses) = if product.product_type == ProductType::Licensed
            {
                let active: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM licenses WHERE product_id = $1 AND status = 'active'",
                )
                .bind(&product.id)
                .fetch_one(&self.pool)
                .await
                .map_err(BillingError::from)?;

                let total: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM licenses WHERE product_id = $1")
                        .bind(&product.id)
                        .fetch_one(&self.pool)
                        .await
                        .map_err(BillingError::from)?;

                (Some(active), Some(total))
            } else {
                (None, None)
            };

            results.push(ProductListItem {
                product,
                revenue: revenue.to_string(),
                change: change.to_string(),
                active_licenses,
                total_licenses,
            });
        }

        Ok(results)
    }

    async fn get(&self, id: &str) -> Result<Product> {
        sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(BillingError::from)?
            .ok_or_else(|| BillingError::not_found("product", id))
    }

    async fn create(&self, body: &CreateProductRequest) -> Result<Product> {
        sqlx::query_as::<_, Product>(
            r#"
            INSERT INTO products (id, name, product_type, revenue, target, change,
                units_sold, active_licenses, total_licenses,
                mau, dau, free_users, paid_users, churn_rate,
                api_calls, active_developers, avg_latency)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#,
        )
        .bind(&body.name)
        .bind(&body.product_type)
        .bind(body.revenue.unwrap_or_default())
        .bind(body.target.unwrap_or_default())
        .bind(body.change.unwrap_or_default())
        .bind(body.units_sold)
        .bind(body.active_licenses)
        .bind(body.total_licenses)
        .bind(body.mau)
        .bind(body.dau)
        .bind(body.free_users)
        .bind(body.paid_users)
        .bind(body.churn_rate)
        .bind(body.api_calls)
        .bind(body.active_developers)
        .bind(body.avg_latency)
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn update(&self, id: &str, body: &UpdateProductRequest) -> Result<Product> {
        sqlx::query_as::<_, Product>(
            r#"
            UPDATE products SET
                name = COALESCE($2, name),
                product_type = COALESCE($3, product_type),
                revenue = COALESCE($4, revenue),
                target = COALESCE($5, target),
                change = COALESCE($6, change),
                units_sold = COALESCE($7, units_sold),
                active_licenses = COALESCE($8, active_licenses),
                total_licenses = COALESCE($9, total_licenses),
                mau = COALESCE($10, mau),
                dau = COALESCE($11, dau),
                free_users = COALESCE($12, free_users),
                paid_users = COALESCE($13, paid_users),
                churn_rate = COALESCE($14, churn_rate),
                api_calls = COALESCE($15, api_calls),
                active_developers = COALESCE($16, active_developers),
                avg_latency = COALESCE($17, avg_latency),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&body.name)
        .bind(&body.product_type)
        .bind(body.revenue)
        .bind(body.target)
        .bind(body.change)
        .bind(body.units_sold)
        .bind(body.active_licenses)
        .bind(body.total_licenses)
        .bind(body.mau)
        .bind(body.dau)
        .bind(body.free_users)
        .bind(body.paid_users)
        .bind(body.churn_rate)
        .bind(body.api_calls)
        .bind(body.active_developers)
        .bind(body.avg_latency)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("product", id))
    }

    async fn delete(&self, id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM products WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(BillingError::from)?;

        Ok(result.rows_affected())
    }
}
