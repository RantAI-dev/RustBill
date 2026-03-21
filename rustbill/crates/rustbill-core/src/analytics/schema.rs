use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
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

#[derive(Debug, Serialize, Clone)]
pub struct MonthlyRevenue {
    pub month: String,
    pub revenue: Decimal,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TopCustomer {
    pub id: String,
    pub name: String,
    pub total_revenue: Decimal,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RevenueByProduct {
    pub product_id: String,
    pub product_name: String,
    pub revenue: Decimal,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ForecastAnalytics {
    pub forecast_3mo: Decimal,
    pub forecast_6mo: Decimal,
    pub forecast_12mo: Decimal,
    pub growth_rate: Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct ReportSummary {
    pub paid_invoices: i64,
    pub total_paid: String,
    pub total_refunded: String,
    pub net_revenue: String,
}
