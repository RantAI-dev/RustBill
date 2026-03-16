//! Analytics: overview, forecasting, reports.

use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewAnalytics {
    pub mrr: Decimal,
    pub arr: Decimal,
    pub active_subscriptions: i64,
    pub new_subscriptions_this_month: i64,
    pub total_customers: i64,
    pub total_revenue: Decimal,
    pub outstanding_invoices: i64,
    pub overdue_invoices: i64,
    pub monthly_revenue: Vec<MonthlyRevenue>,
    pub top_customers: Vec<TopCustomer>,
    pub revenue_by_product: Vec<RevenueByProduct>,
}

#[derive(Debug, Serialize)]
pub struct MonthlyRevenue {
    pub month: String,
    pub revenue: Decimal,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopCustomer {
    pub id: String,
    pub name: String,
    pub total_revenue: Decimal,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevenueByProduct {
    pub product_id: String,
    pub product_name: String,
    pub revenue: Decimal,
}

pub async fn get_overview(pool: &PgPool) -> crate::error::Result<OverviewAnalytics> {
    // MRR from active subscriptions
    let mrr: Option<Decimal> = sqlx::query_scalar(
        r#"SELECT COALESCE(SUM(pp.base_price * s.quantity), 0)
        FROM subscriptions s
        JOIN pricing_plans pp ON pp.id = s.plan_id
        WHERE s.status = 'active' AND s.deleted_at IS NULL
        AND pp.billing_cycle = 'monthly'"#
    )
    .fetch_one(pool)
    .await?;

    let mrr = mrr.unwrap_or_default();
    let arr = mrr * Decimal::from(12);

    let active_subscriptions: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subscriptions WHERE status IN ('active', 'trialing') AND deleted_at IS NULL"
    )
    .fetch_one(pool)
    .await?;

    let new_subs: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subscriptions WHERE created_at >= date_trunc('month', CURRENT_DATE) AND deleted_at IS NULL"
    )
    .fetch_one(pool)
    .await?;

    let total_customers: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) FROM customers")
        .fetch_one(pool)
        .await?;

    let total_revenue: Option<Decimal> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM payments"
    )
    .fetch_one(pool)
    .await?;

    let outstanding: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM invoices WHERE status IN ('issued', 'draft') AND deleted_at IS NULL"
    )
    .fetch_one(pool)
    .await?;

    let overdue: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM invoices WHERE status = 'overdue' AND deleted_at IS NULL"
    )
    .fetch_one(pool)
    .await?;

    // Monthly revenue (last 12 months)
    let monthly: Vec<MonthlyRevenue> = sqlx::query_as::<_, (String, Decimal)>(
        r#"SELECT TO_CHAR(paid_at, 'YYYY-MM') as month, COALESCE(SUM(amount), 0) as revenue
        FROM payments
        WHERE paid_at >= CURRENT_DATE - interval '12 months'
        GROUP BY month ORDER BY month"#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(month, revenue)| MonthlyRevenue { month, revenue })
    .collect();

    // Top customers
    let top_customers: Vec<TopCustomer> = sqlx::query_as::<_, (String, String, Decimal)>(
        r#"SELECT c.id, c.name, COALESCE(SUM(p.amount), 0) as total
        FROM customers c
        LEFT JOIN invoices i ON i.customer_id = c.id
        LEFT JOIN payments p ON p.invoice_id = i.id
        GROUP BY c.id, c.name
        ORDER BY total DESC LIMIT 10"#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(id, name, total_revenue)| TopCustomer { id, name, total_revenue })
    .collect();

    // Revenue by product
    let by_product: Vec<RevenueByProduct> = sqlx::query_as::<_, (String, String, Decimal)>(
        r#"SELECT p.id, p.name, COALESCE(SUM(d.value), 0) as revenue
        FROM products p
        LEFT JOIN deals d ON d.product_id = p.id
        GROUP BY p.id, p.name
        ORDER BY revenue DESC"#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(product_id, product_name, revenue)| RevenueByProduct { product_id, product_name, revenue })
    .collect();

    Ok(OverviewAnalytics {
        mrr,
        arr,
        active_subscriptions: active_subscriptions.unwrap_or(0),
        new_subscriptions_this_month: new_subs.unwrap_or(0),
        total_customers: total_customers.unwrap_or(0),
        total_revenue: total_revenue.unwrap_or_default(),
        outstanding_invoices: outstanding.unwrap_or(0),
        overdue_invoices: overdue.unwrap_or(0),
        monthly_revenue: monthly,
        top_customers,
        revenue_by_product: by_product,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastAnalytics {
    pub forecast_3mo: Decimal,
    pub forecast_6mo: Decimal,
    pub forecast_12mo: Decimal,
    pub growth_rate: Decimal,
}

pub async fn get_forecasting(pool: &PgPool) -> crate::error::Result<ForecastAnalytics> {
    // Simple linear forecast based on last 3 months revenue trend
    let recent: Vec<Decimal> = sqlx::query_scalar(
        r#"SELECT COALESCE(SUM(amount), 0)
        FROM payments
        WHERE paid_at >= CURRENT_DATE - interval '3 months'
        GROUP BY date_trunc('month', paid_at)
        ORDER BY date_trunc('month', paid_at)"#
    )
    .fetch_all(pool)
    .await?;

    let avg_monthly = if recent.is_empty() {
        Decimal::ZERO
    } else {
        recent.iter().sum::<Decimal>() / Decimal::from(recent.len() as i64)
    };

    let growth_rate = if recent.len() >= 2 {
        let first = recent[0];
        let last = recent[recent.len() - 1];
        if first > Decimal::ZERO {
            ((last - first) / first * Decimal::from(100)).round_dp(2)
        } else {
            Decimal::ZERO
        }
    } else {
        Decimal::ZERO
    };

    Ok(ForecastAnalytics {
        forecast_3mo: (avg_monthly * Decimal::from(3)).round_dp(2),
        forecast_6mo: (avg_monthly * Decimal::from(6)).round_dp(2),
        forecast_12mo: (avg_monthly * Decimal::from(12)).round_dp(2),
        growth_rate,
    })
}

pub async fn get_reports(pool: &PgPool) -> crate::error::Result<serde_json::Value> {
    // Summary report
    let paid_count: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM invoices WHERE status = 'paid' AND deleted_at IS NULL"
    )
    .fetch_one(pool)
    .await?;

    let total_paid: Option<Decimal> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total), 0) FROM invoices WHERE status = 'paid' AND deleted_at IS NULL"
    )
    .fetch_one(pool)
    .await?;

    let total_refunded: Option<Decimal> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM refunds WHERE status = 'completed'"
    )
    .fetch_one(pool)
    .await?;

    Ok(serde_json::json!({
        "paidInvoices": paid_count.unwrap_or(0),
        "totalPaid": total_paid.unwrap_or_default().to_string(),
        "totalRefunded": total_refunded.unwrap_or_default().to_string(),
        "netRevenue": (total_paid.unwrap_or_default() - total_refunded.unwrap_or_default()).to_string(),
    }))
}
