pub mod repository;
pub mod sales_ledger;
pub mod schema;
pub mod service;

use crate::error::Result;
use repository::SqlxAnalyticsRepository;
use sqlx::PgPool;

pub use schema::{
    ForecastAnalytics, MonthlyRevenue, OverviewAnalytics, RevenueByProduct, TopCustomer,
};

pub async fn get_overview(pool: &PgPool) -> Result<schema::OverviewAnalytics> {
    let repo = SqlxAnalyticsRepository::new(pool);
    service::get_overview(&repo).await
}

pub async fn get_forecasting(pool: &PgPool) -> Result<schema::ForecastAnalytics> {
    let repo = SqlxAnalyticsRepository::new(pool);
    service::get_forecasting(&repo).await
}

pub async fn get_reports(pool: &PgPool) -> Result<serde_json::Value> {
    let repo = SqlxAnalyticsRepository::new(pool);
    service::get_reports(&repo).await
}
