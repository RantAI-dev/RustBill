use crate::error::Result;
use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;

#[async_trait]
pub trait AnalyticsRepository {
    async fn overview_mrr(&self) -> Result<Option<Decimal>>;
    async fn overview_active_subscriptions(&self) -> Result<Option<i64>>;
    async fn overview_new_subscriptions(&self) -> Result<Option<i64>>;
    async fn overview_total_customers(&self) -> Result<Option<i64>>;
    async fn overview_total_revenue(&self) -> Result<Option<Decimal>>;
    async fn overview_outstanding_invoices(&self) -> Result<Option<i64>>;
    async fn overview_overdue_invoices(&self) -> Result<Option<i64>>;
    async fn overview_monthly_revenue_rows(&self) -> Result<Vec<(String, Decimal)>>;
    async fn overview_top_customer_rows(&self) -> Result<Vec<(String, String, Decimal)>>;
    async fn overview_revenue_by_product_rows(&self) -> Result<Vec<(String, String, Decimal)>>;

    async fn forecasting_recent_revenue(&self) -> Result<Vec<Decimal>>;

    async fn reports_paid_count(&self) -> Result<Option<i64>>;
    async fn reports_total_paid(&self) -> Result<Option<Decimal>>;
    async fn reports_total_refunded(&self) -> Result<Option<Decimal>>;
}

pub struct SqlxAnalyticsRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> SqlxAnalyticsRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AnalyticsRepository for SqlxAnalyticsRepository<'_> {
    async fn overview_mrr(&self) -> Result<Option<Decimal>> {
        let mrr = sqlx::query_scalar(
            r#"SELECT COALESCE(SUM(pp.base_price * s.quantity), 0)
            FROM subscriptions s
            JOIN pricing_plans pp ON pp.id = s.plan_id
            WHERE s.status = 'active' AND s.deleted_at IS NULL
            AND pp.billing_cycle = 'monthly'"#,
        )
        .fetch_one(self.pool)
        .await?;
        Ok(mrr)
    }

    async fn overview_active_subscriptions(&self) -> Result<Option<i64>> {
        let rows = sqlx::query_scalar(
            "SELECT COUNT(*) FROM subscriptions WHERE status IN ('active', 'trialing') AND deleted_at IS NULL",
        )
        .fetch_one(self.pool)
        .await?;
        Ok(rows)
    }

    async fn overview_new_subscriptions(&self) -> Result<Option<i64>> {
        let rows = sqlx::query_scalar(
            "SELECT COUNT(*) FROM subscriptions WHERE created_at >= date_trunc('month', CURRENT_DATE) AND deleted_at IS NULL",
        )
        .fetch_one(self.pool)
        .await?;
        Ok(rows)
    }

    async fn overview_total_customers(&self) -> Result<Option<i64>> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM customers")
            .fetch_one(self.pool)
            .await?)
    }

    async fn overview_total_revenue(&self) -> Result<Option<Decimal>> {
        Ok(
            sqlx::query_scalar("SELECT COALESCE(SUM(amount), 0) FROM payments")
                .fetch_one(self.pool)
                .await?,
        )
    }

    async fn overview_outstanding_invoices(&self) -> Result<Option<i64>> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM invoices WHERE status IN ('issued', 'draft') AND deleted_at IS NULL",
        )
        .fetch_one(self.pool)
        .await?)
    }

    async fn overview_overdue_invoices(&self) -> Result<Option<i64>> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM invoices WHERE status = 'overdue' AND deleted_at IS NULL",
        )
        .fetch_one(self.pool)
        .await?)
    }

    async fn overview_monthly_revenue_rows(&self) -> Result<Vec<(String, Decimal)>> {
        let rows = sqlx::query_as::<_, (String, Decimal)>(
            r#"SELECT TO_CHAR(paid_at, 'YYYY-MM') as month, COALESCE(SUM(amount), 0) as revenue
            FROM payments
            WHERE paid_at >= CURRENT_DATE - interval '12 months'
            GROUP BY month ORDER BY month"#,
        )
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    async fn overview_top_customer_rows(&self) -> Result<Vec<(String, String, Decimal)>> {
        let rows = sqlx::query_as::<_, (String, String, Decimal)>(
            r#"SELECT c.id, c.name, COALESCE(SUM(p.amount), 0) as total
            FROM customers c
            LEFT JOIN invoices i ON i.customer_id = c.id
            LEFT JOIN payments p ON p.invoice_id = i.id
            GROUP BY c.id, c.name
            ORDER BY total DESC LIMIT 10"#,
        )
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    async fn overview_revenue_by_product_rows(&self) -> Result<Vec<(String, String, Decimal)>> {
        let rows = sqlx::query_as::<_, (String, String, Decimal)>(
            r#"SELECT p.id, p.name, COALESCE(SUM(d.value), 0) as revenue
            FROM products p
            LEFT JOIN deals d ON d.product_id = p.id
            GROUP BY p.id, p.name
            ORDER BY revenue DESC"#,
        )
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    async fn forecasting_recent_revenue(&self) -> Result<Vec<Decimal>> {
        let recent = sqlx::query_scalar(
            r#"SELECT COALESCE(SUM(amount), 0)
            FROM payments
            WHERE paid_at >= CURRENT_DATE - interval '3 months'
            GROUP BY date_trunc('month', paid_at)
            ORDER BY date_trunc('month', paid_at)"#,
        )
        .fetch_all(self.pool)
        .await?;
        Ok(recent)
    }

    async fn reports_paid_count(&self) -> Result<Option<i64>> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM invoices WHERE status = 'paid' AND deleted_at IS NULL",
        )
        .fetch_one(self.pool)
        .await?)
    }

    async fn reports_total_paid(&self) -> Result<Option<Decimal>> {
        Ok(sqlx::query_scalar(
            "SELECT COALESCE(SUM(total), 0) FROM invoices WHERE status = 'paid' AND deleted_at IS NULL",
        )
        .fetch_one(self.pool)
        .await?)
    }

    async fn reports_total_refunded(&self) -> Result<Option<Decimal>> {
        Ok(sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0) FROM refunds WHERE status = 'completed'",
        )
        .fetch_one(self.pool)
        .await?)
    }
}
