use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use rustbill_core::analytics::sales_ledger::BackfillResult;
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait AnalyticsRepository: Send + Sync {
    async fn total_customers(&self) -> Result<(i64,), BillingError>;
    async fn active_subscriptions(&self) -> Result<(i64,), BillingError>;
    async fn mrr(&self) -> Result<(Option<Decimal>,), BillingError>;
    async fn total_revenue(&self) -> Result<(Option<Decimal>,), BillingError>;
    async fn active_licenses(&self) -> Result<(i64,), BillingError>;

    async fn monthly_deals(&self) -> Result<Vec<(f64, f64, Option<Decimal>)>, BillingError>;
    async fn annual_target(&self) -> Result<(Option<Decimal>,), BillingError>;
    async fn quarterly_invoices(&self)
        -> Result<Vec<(f64, String, Option<Decimal>)>, BillingError>;
    async fn overdue_invoices(
        &self,
    ) -> Result<Vec<(String, String, Decimal, Option<String>)>, BillingError>;
    async fn at_risk_subscriptions(&self) -> Result<Vec<(String, Option<String>)>, BillingError>;
    async fn low_health_customers(&self) -> Result<Vec<(String, String, i32)>, BillingError>;

    async fn reports_monthly_deals(&self) -> Result<Vec<(String, f64, i64)>, BillingError>;
    async fn reports_total_customers(&self) -> Result<(i64,), BillingError>;
    async fn product_type_revenue(&self) -> Result<Vec<(String, Option<Decimal>)>, BillingError>;
    async fn recent_invoices(
        &self,
    ) -> Result<
        Vec<(
            String,
            String,
            String,
            Decimal,
            NaiveDateTime,
            Option<String>,
        )>,
        BillingError,
    >;

    async fn sales_360_summary_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(String, Option<Decimal>, Option<Decimal>, Option<Decimal>)>, BillingError>;
    async fn sales_360_by_currency_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(String, String, Option<Decimal>)>, BillingError>;
    async fn sales_360_available_currencies(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<(String,)>, BillingError>;
    async fn sales_360_timeseries_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        timezone: &str,
        currency: Option<&str>,
    ) -> Result<Vec<(String, String, Option<Decimal>)>, BillingError>;
    async fn sales_360_breakdown_event_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(String, Option<Decimal>)>, BillingError>;
    async fn sales_360_breakdown_customer_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(Option<String>, Option<Decimal>)>, BillingError>;
    async fn sales_360_reconcile_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(String, Option<Decimal>, Option<Decimal>, i64, i64)>, BillingError>;
    async fn sales_360_export_event_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(String, String, Option<Decimal>)>, BillingError>;
    async fn sales_360_export_customer_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(Option<String>, String, Option<Decimal>)>, BillingError>;

    async fn backfill_sales_events(&self) -> Result<BackfillResult, BillingError>;
}

#[derive(Clone)]
pub struct SqlxAnalyticsRepository {
    pool: PgPool,
}

impl SqlxAnalyticsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AnalyticsRepository for SqlxAnalyticsRepository {
    async fn total_customers(&self) -> Result<(i64,), BillingError> {
        sqlx::query_as("SELECT COUNT(*) FROM customers")
            .fetch_one(&self.pool)
            .await
            .map_err(BillingError::from)
    }

    async fn active_subscriptions(&self) -> Result<(i64,), BillingError> {
        sqlx::query_as("SELECT COUNT(*) FROM subscriptions WHERE status = 'active'")
            .fetch_one(&self.pool)
            .await
            .map_err(BillingError::from)
    }

    async fn mrr(&self) -> Result<(Option<Decimal>,), BillingError> {
        sqlx::query_as(
            r#"SELECT SUM(pp.base_price) FROM subscriptions s
               JOIN pricing_plans pp ON pp.id = s.plan_id
               WHERE s.status = 'active' AND s.deleted_at IS NULL"#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn total_revenue(&self) -> Result<(Option<Decimal>,), BillingError> {
        sqlx::query_as("SELECT SUM(amount) FROM payments")
            .fetch_one(&self.pool)
            .await
            .map_err(BillingError::from)
    }

    async fn active_licenses(&self) -> Result<(i64,), BillingError> {
        sqlx::query_as("SELECT COUNT(*) FROM licenses WHERE status = 'active'")
            .fetch_one(&self.pool)
            .await
            .map_err(BillingError::from)
    }

    async fn monthly_deals(&self) -> Result<Vec<(f64, f64, Option<Decimal>)>, BillingError> {
        sqlx::query_as(
            r#"SELECT
                 extract(month from to_date(date, 'Mon DD, YYYY'))::float8 as month_num,
                 extract(year from to_date(date, 'Mon DD, YYYY'))::float8 as year_num,
                 SUM(value) as total
               FROM deals
               GROUP BY month_num, year_num
               ORDER BY year_num, month_num"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn annual_target(&self) -> Result<(Option<Decimal>,), BillingError> {
        sqlx::query_as("SELECT SUM(target) FROM products")
            .fetch_one(&self.pool)
            .await
            .map_err(BillingError::from)
    }

    async fn quarterly_invoices(
        &self,
    ) -> Result<Vec<(f64, String, Option<Decimal>)>, BillingError> {
        sqlx::query_as(
            r#"SELECT
                 extract(quarter from created_at)::float8 as quarter,
                 status::text as status,
                 SUM(total) as total
               FROM invoices
               WHERE deleted_at IS NULL
               GROUP BY quarter, status"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn overdue_invoices(
        &self,
    ) -> Result<Vec<(String, String, Decimal, Option<String>)>, BillingError> {
        sqlx::query_as(
            r#"SELECT i.id, i.invoice_number, i.total, c.name
               FROM invoices i
               LEFT JOIN customers c ON i.customer_id = c.id
               WHERE i.status = 'overdue' AND i.deleted_at IS NULL"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn at_risk_subscriptions(&self) -> Result<Vec<(String, Option<String>)>, BillingError> {
        sqlx::query_as(
            r#"SELECT s.id, c.name
               FROM subscriptions s
               LEFT JOIN customers c ON s.customer_id = c.id
               WHERE (s.status = 'past_due' OR s.cancel_at_period_end = true)
                 AND s.deleted_at IS NULL"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn low_health_customers(&self) -> Result<Vec<(String, String, i32)>, BillingError> {
        sqlx::query_as(
            r#"SELECT id, name, health_score
               FROM customers
               WHERE health_score < 60"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn reports_monthly_deals(&self) -> Result<Vec<(String, f64, i64)>, BillingError> {
        sqlx::query_as(
            r#"SELECT
                 to_char(to_date(date, 'Mon DD, YYYY'), 'Mon') as month,
                 extract(month from to_date(date, 'Mon DD, YYYY'))::float8 as month_num,
                 COUNT(*) as deal_count
               FROM deals
               GROUP BY month, month_num
               ORDER BY month_num"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn reports_total_customers(&self) -> Result<(i64,), BillingError> {
        sqlx::query_as("SELECT COUNT(*) FROM customers")
            .fetch_one(&self.pool)
            .await
            .map_err(BillingError::from)
    }

    async fn product_type_revenue(&self) -> Result<Vec<(String, Option<Decimal>)>, BillingError> {
        sqlx::query_as(
            r#"SELECT product_type::text, SUM(revenue) as revenue
               FROM products
               GROUP BY product_type"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn recent_invoices(
        &self,
    ) -> Result<
        Vec<(
            String,
            String,
            String,
            Decimal,
            NaiveDateTime,
            Option<String>,
        )>,
        BillingError,
    > {
        sqlx::query_as(
            r#"SELECT i.id, i.invoice_number, i.status::text, i.total, i.created_at, c.name
               FROM invoices i
               LEFT JOIN customers c ON i.customer_id = c.id
               WHERE i.deleted_at IS NULL
               ORDER BY i.created_at DESC
               LIMIT 10"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn sales_360_summary_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(String, Option<Decimal>, Option<Decimal>, Option<Decimal>)>, BillingError>
    {
        sqlx::query_as(
            r#"
            SELECT
              classification,
              COALESCE(SUM(amount_subtotal), 0) AS subtotal,
              COALESCE(SUM(amount_tax), 0) AS tax,
              COALESCE(SUM(amount_total), 0) AS total
            FROM sales_events
            WHERE occurred_at >= $1 AND occurred_at <= $2
              AND ($3::text IS NULL OR currency = $3)
            GROUP BY classification
            ORDER BY classification
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(currency)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn sales_360_by_currency_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(String, String, Option<Decimal>)>, BillingError> {
        sqlx::query_as(
            r#"
            SELECT currency, classification, COALESCE(SUM(amount_total), 0) AS total
            FROM sales_events
            WHERE occurred_at >= $1 AND occurred_at <= $2
              AND ($3::text IS NULL OR currency = $3)
            GROUP BY currency, classification
            ORDER BY currency ASC, classification ASC
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(currency)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn sales_360_available_currencies(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<(String,)>, BillingError> {
        sqlx::query_as(
            r#"
            SELECT DISTINCT currency
            FROM sales_events
            WHERE occurred_at >= $1 AND occurred_at <= $2
            ORDER BY currency ASC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn sales_360_timeseries_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        timezone: &str,
        currency: Option<&str>,
    ) -> Result<Vec<(String, String, Option<Decimal>)>, BillingError> {
        sqlx::query_as(
            r#"
            SELECT
              to_char(date_trunc('day', timezone($3, occurred_at)), 'YYYY-MM-DD') AS day,
              classification,
              COALESCE(SUM(amount_total), 0) AS total
            FROM sales_events
            WHERE occurred_at >= $1 AND occurred_at <= $2
              AND ($4::text IS NULL OR currency = $4)
            GROUP BY day, classification
            ORDER BY day ASC, classification ASC
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(timezone)
        .bind(currency)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn sales_360_breakdown_event_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(String, Option<Decimal>)>, BillingError> {
        sqlx::query_as(
            r#"
            SELECT event_type, COALESCE(SUM(amount_total), 0) AS total
            FROM sales_events
            WHERE occurred_at >= $1 AND occurred_at <= $2
              AND ($3::text IS NULL OR currency = $3)
            GROUP BY event_type
            ORDER BY total DESC
            LIMIT 20
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(currency)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn sales_360_breakdown_customer_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(Option<String>, Option<Decimal>)>, BillingError> {
        sqlx::query_as(
            r#"
            SELECT customer_id, COALESCE(SUM(amount_total), 0) AS total
            FROM sales_events
            WHERE occurred_at >= $1 AND occurred_at <= $2
              AND ($3::text IS NULL OR currency = $3)
            GROUP BY customer_id
            ORDER BY total DESC
            LIMIT 20
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(currency)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn sales_360_reconcile_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(String, Option<Decimal>, Option<Decimal>, i64, i64)>, BillingError> {
        sqlx::query_as(
            r#"
            SELECT
              se.classification,
              COALESCE(SUM(se.amount_total), 0) AS ledger_total,
              COALESCE(SUM(
                CASE
                  WHEN se.source_table = 'deals' THEN COALESCE((SELECT d.value FROM deals d WHERE d.id = se.source_id), 0)
                  WHEN se.source_table = 'invoices' THEN COALESCE((SELECT i.total FROM invoices i WHERE i.id = se.source_id AND i.deleted_at IS NULL), 0)
                  WHEN se.source_table = 'payments' THEN COALESCE((SELECT p.amount FROM payments p WHERE p.id = se.source_id), 0)
                  WHEN se.source_table = 'credit_notes' THEN COALESCE((SELECT cn.amount FROM credit_notes cn WHERE cn.id = se.source_id AND cn.deleted_at IS NULL), 0)
                  WHEN se.source_table = 'refunds' THEN COALESCE((SELECT r.amount FROM refunds r WHERE r.id = se.source_id AND r.deleted_at IS NULL), 0)
                  WHEN se.source_table = 'subscriptions' THEN 0
                  ELSE 0
                END
              ), 0) AS source_total,
              COUNT(*)::bigint AS event_count,
              SUM(
                CASE
                  WHEN se.source_table = 'deals' AND NOT EXISTS (SELECT 1 FROM deals d WHERE d.id = se.source_id) THEN 1
                  WHEN se.source_table = 'invoices' AND NOT EXISTS (SELECT 1 FROM invoices i WHERE i.id = se.source_id AND i.deleted_at IS NULL) THEN 1
                  WHEN se.source_table = 'payments' AND NOT EXISTS (SELECT 1 FROM payments p WHERE p.id = se.source_id) THEN 1
                  WHEN se.source_table = 'credit_notes' AND NOT EXISTS (SELECT 1 FROM credit_notes cn WHERE cn.id = se.source_id AND cn.deleted_at IS NULL) THEN 1
                  WHEN se.source_table = 'refunds' AND NOT EXISTS (SELECT 1 FROM refunds r WHERE r.id = se.source_id AND r.deleted_at IS NULL) THEN 1
                  WHEN se.source_table = 'subscriptions' AND NOT EXISTS (SELECT 1 FROM subscriptions s WHERE s.id = se.source_id AND s.deleted_at IS NULL) THEN 1
                  ELSE 0
                END
              )::bigint AS missing_sources
            FROM sales_events se
            WHERE se.occurred_at >= $1 AND se.occurred_at <= $2
              AND ($3::text IS NULL OR se.currency = $3)
            GROUP BY se.classification
            ORDER BY se.classification ASC
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(currency)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn sales_360_export_event_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(String, String, Option<Decimal>)>, BillingError> {
        sqlx::query_as(
            r#"
            SELECT event_type, currency, COALESCE(SUM(amount_total), 0) AS total
            FROM sales_events
            WHERE occurred_at >= $1 AND occurred_at <= $2
              AND ($3::text IS NULL OR currency = $3)
            GROUP BY event_type, currency
            ORDER BY total DESC
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(currency)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn sales_360_export_customer_rows(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        currency: Option<&str>,
    ) -> Result<Vec<(Option<String>, String, Option<Decimal>)>, BillingError> {
        sqlx::query_as(
            r#"
            SELECT customer_id, currency, COALESCE(SUM(amount_total), 0) AS total
            FROM sales_events
            WHERE occurred_at >= $1 AND occurred_at <= $2
              AND ($3::text IS NULL OR currency = $3)
            GROUP BY customer_id, currency
            ORDER BY total DESC
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(currency)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn backfill_sales_events(&self) -> Result<BackfillResult, BillingError> {
        rustbill_core::analytics::sales_ledger::backfill_sales_events(&self.pool).await
    }
}
