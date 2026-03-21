use super::schema::{CreateProductRequest, ProductMetrics, UpdateProductRequest};
use crate::db::models::{Product, ProductType};
use crate::error::{BillingError, Result};
use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;

#[async_trait]
pub trait ProductsRepository {
    async fn list_products(&self) -> Result<Vec<Product>>;
    async fn product_metrics(
        &self,
        product_id: &str,
        product_type: &ProductType,
    ) -> Result<ProductMetrics>;
    async fn get_product(&self, id: &str) -> Result<Product>;
    async fn create_product(&self, req: CreateProductRequest) -> Result<Product>;
    async fn update_product(&self, id: &str, req: UpdateProductRequest) -> Result<Product>;
    async fn delete_product(&self, id: &str) -> Result<u64>;
}

pub struct PgProductsRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PgProductsRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProductsRepository for PgProductsRepository<'_> {
    async fn list_products(&self) -> Result<Vec<Product>> {
        let rows = sqlx::query_as::<_, Product>("SELECT * FROM products ORDER BY created_at DESC")
            .fetch_all(self.pool)
            .await?;
        Ok(rows)
    }

    async fn product_metrics(
        &self,
        product_id: &str,
        product_type: &ProductType,
    ) -> Result<ProductMetrics> {
        let revenue: Option<Decimal> =
            sqlx::query_scalar("SELECT COALESCE(SUM(value), 0) FROM deals WHERE product_id = $1")
                .bind(product_id)
                .fetch_one(self.pool)
                .await?;

        let this_month: Option<Decimal> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(value), 0) FROM deals WHERE product_id = $1 AND created_at >= date_trunc('month', CURRENT_DATE)",
        )
        .bind(product_id)
        .fetch_one(self.pool)
        .await?;

        let last_month: Option<Decimal> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(value), 0) FROM deals WHERE product_id = $1 AND created_at >= date_trunc('month', CURRENT_DATE) - interval '1 month' AND created_at < date_trunc('month', CURRENT_DATE)",
        )
        .bind(product_id)
        .fetch_one(self.pool)
        .await?;

        let this_month = this_month.unwrap_or_default();
        let last_month = last_month.unwrap_or_default();
        let change = if last_month > Decimal::ZERO {
            ((this_month - last_month) / last_month * Decimal::from(100)).round_dp(2)
        } else if this_month > Decimal::ZERO {
            Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        let (active_licenses, total_licenses) = if *product_type == ProductType::Licensed {
            let active: Option<i64> = sqlx::query_scalar(
                "SELECT COUNT(*) FROM licenses WHERE product_id = $1 AND status = 'active'",
            )
            .bind(product_id)
            .fetch_one(self.pool)
            .await?;

            let total: Option<i64> =
                sqlx::query_scalar("SELECT COUNT(*) FROM licenses WHERE product_id = $1")
                    .bind(product_id)
                    .fetch_one(self.pool)
                    .await?;

            (Some(active.unwrap_or(0)), Some(total.unwrap_or(0)))
        } else {
            (None, None)
        };

        Ok(ProductMetrics {
            revenue: revenue.unwrap_or_default(),
            change,
            active_licenses,
            total_licenses,
            trend: crate::db::models::Trend::Stable,
        })
    }

    async fn get_product(&self, id: &str) -> Result<Product> {
        sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?
            .ok_or_else(|| BillingError::not_found("product", id))
    }

    async fn create_product(&self, req: CreateProductRequest) -> Result<Product> {
        let row = sqlx::query_as::<_, Product>(
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
        .bind(&req.name)
        .bind(&req.product_type)
        .bind(req.revenue.unwrap_or_default())
        .bind(req.target.unwrap_or_default())
        .bind(req.change.unwrap_or_default())
        .bind(req.units_sold)
        .bind(req.active_licenses)
        .bind(req.total_licenses)
        .bind(req.mau)
        .bind(req.dau)
        .bind(req.free_users)
        .bind(req.paid_users)
        .bind(req.churn_rate)
        .bind(req.api_calls)
        .bind(req.active_developers)
        .bind(req.avg_latency)
        .fetch_one(self.pool)
        .await?;

        Ok(row)
    }

    async fn update_product(&self, id: &str, req: UpdateProductRequest) -> Result<Product> {
        let row = sqlx::query_as::<_, Product>(
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
        .bind(&req.name)
        .bind(&req.product_type)
        .bind(req.revenue)
        .bind(req.target)
        .bind(req.change)
        .bind(req.units_sold)
        .bind(req.active_licenses)
        .bind(req.total_licenses)
        .bind(req.mau)
        .bind(req.dau)
        .bind(req.free_users)
        .bind(req.paid_users)
        .bind(req.churn_rate)
        .bind(req.api_calls)
        .bind(req.active_developers)
        .bind(req.avg_latency)
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| BillingError::not_found("product", id))?;

        Ok(row)
    }

    async fn delete_product(&self, id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM products WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
