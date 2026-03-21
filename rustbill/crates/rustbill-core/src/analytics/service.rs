use super::repository::AnalyticsRepository;
use super::schema::{
    ForecastAnalytics, MonthlyRevenue, OverviewAnalytics, RevenueByProduct, TopCustomer,
};
use crate::error::Result;
use rust_decimal::Decimal;
use serde_json::json;

pub async fn get_overview<R: AnalyticsRepository + ?Sized>(repo: &R) -> Result<OverviewAnalytics> {
    let mrr = repo.overview_mrr().await?.unwrap_or_default();
    let arr = mrr * Decimal::from(12);
    let active_subscriptions = repo.overview_active_subscriptions().await?.unwrap_or(0);
    let new_subscriptions_this_month = repo.overview_new_subscriptions().await?.unwrap_or(0);
    let total_customers = repo.overview_total_customers().await?.unwrap_or(0);
    let total_revenue = repo.overview_total_revenue().await?.unwrap_or_default();
    let outstanding_invoices = repo.overview_outstanding_invoices().await?.unwrap_or(0);
    let overdue_invoices = repo.overview_overdue_invoices().await?.unwrap_or(0);

    let monthly_revenue = repo
        .overview_monthly_revenue_rows()
        .await?
        .into_iter()
        .map(|(month, revenue)| MonthlyRevenue { month, revenue })
        .collect();

    let top_customers = repo
        .overview_top_customer_rows()
        .await?
        .into_iter()
        .map(|(id, name, total_revenue)| TopCustomer {
            id,
            name,
            total_revenue,
        })
        .collect();

    let revenue_by_product = repo
        .overview_revenue_by_product_rows()
        .await?
        .into_iter()
        .map(|(product_id, product_name, revenue)| RevenueByProduct {
            product_id,
            product_name,
            revenue,
        })
        .collect();

    Ok(OverviewAnalytics {
        mrr,
        arr,
        active_subscriptions,
        new_subscriptions_this_month,
        total_customers,
        total_revenue,
        outstanding_invoices,
        overdue_invoices,
        monthly_revenue,
        top_customers,
        revenue_by_product,
    })
}

pub async fn get_forecasting<R: AnalyticsRepository + ?Sized>(
    repo: &R,
) -> Result<ForecastAnalytics> {
    let recent = repo.forecasting_recent_revenue().await?;

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

pub async fn get_reports<R: AnalyticsRepository + ?Sized>(repo: &R) -> Result<serde_json::Value> {
    let paid_count = repo.reports_paid_count().await?.unwrap_or(0);
    let total_paid = repo.reports_total_paid().await?.unwrap_or_default();
    let total_refunded = repo.reports_total_refunded().await?.unwrap_or_default();

    Ok(json!({
        "paidInvoices": paid_count,
        "totalPaid": total_paid.to_string(),
        "totalRefunded": total_refunded.to_string(),
        "netRevenue": (total_paid - total_refunded).to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        overview_mrr: Option<Decimal>,
        overview_active_subscriptions: Option<i64>,
        overview_new_subscriptions: Option<i64>,
        overview_total_customers: Option<i64>,
        overview_total_revenue: Option<Decimal>,
        overview_outstanding_invoices: Option<i64>,
        overview_overdue_invoices: Option<i64>,
        monthly_revenue_rows: Vec<(String, Decimal)>,
        top_customer_rows: Vec<(String, String, Decimal)>,
        revenue_by_product_rows: Vec<(String, String, Decimal)>,
        forecasting_recent_revenue: Vec<Decimal>,
        reports_paid_count: Option<i64>,
        reports_total_paid: Option<Decimal>,
        reports_total_refunded: Option<Decimal>,
    }

    #[derive(Clone, Default)]
    struct StubRepo {
        state: Arc<Mutex<StubState>>,
    }

    #[async_trait]
    impl AnalyticsRepository for StubRepo {
        async fn overview_mrr(&self) -> Result<Option<Decimal>> {
            Ok(self.state.lock().expect("mutex").overview_mrr)
        }

        async fn overview_active_subscriptions(&self) -> Result<Option<i64>> {
            Ok(self
                .state
                .lock()
                .expect("mutex")
                .overview_active_subscriptions)
        }

        async fn overview_new_subscriptions(&self) -> Result<Option<i64>> {
            Ok(self.state.lock().expect("mutex").overview_new_subscriptions)
        }

        async fn overview_total_customers(&self) -> Result<Option<i64>> {
            Ok(self.state.lock().expect("mutex").overview_total_customers)
        }

        async fn overview_total_revenue(&self) -> Result<Option<Decimal>> {
            Ok(self.state.lock().expect("mutex").overview_total_revenue)
        }

        async fn overview_outstanding_invoices(&self) -> Result<Option<i64>> {
            Ok(self
                .state
                .lock()
                .expect("mutex")
                .overview_outstanding_invoices)
        }

        async fn overview_overdue_invoices(&self) -> Result<Option<i64>> {
            Ok(self.state.lock().expect("mutex").overview_overdue_invoices)
        }

        async fn overview_monthly_revenue_rows(&self) -> Result<Vec<(String, Decimal)>> {
            Ok(self
                .state
                .lock()
                .expect("mutex")
                .monthly_revenue_rows
                .clone())
        }

        async fn overview_top_customer_rows(&self) -> Result<Vec<(String, String, Decimal)>> {
            Ok(self.state.lock().expect("mutex").top_customer_rows.clone())
        }

        async fn overview_revenue_by_product_rows(&self) -> Result<Vec<(String, String, Decimal)>> {
            Ok(self
                .state
                .lock()
                .expect("mutex")
                .revenue_by_product_rows
                .clone())
        }

        async fn forecasting_recent_revenue(&self) -> Result<Vec<Decimal>> {
            Ok(self
                .state
                .lock()
                .expect("mutex")
                .forecasting_recent_revenue
                .clone())
        }

        async fn reports_paid_count(&self) -> Result<Option<i64>> {
            Ok(self.state.lock().expect("mutex").reports_paid_count)
        }

        async fn reports_total_paid(&self) -> Result<Option<Decimal>> {
            Ok(self.state.lock().expect("mutex").reports_total_paid)
        }

        async fn reports_total_refunded(&self) -> Result<Option<Decimal>> {
            Ok(self.state.lock().expect("mutex").reports_total_refunded)
        }
    }

    #[tokio::test]
    async fn overview_builds_summary() {
        let repo = StubRepo {
            state: Arc::new(Mutex::new(StubState {
                overview_mrr: Some(Decimal::from(100)),
                overview_active_subscriptions: Some(3),
                overview_new_subscriptions: Some(1),
                overview_total_customers: Some(10),
                overview_total_revenue: Some(Decimal::from(200)),
                overview_outstanding_invoices: Some(2),
                overview_overdue_invoices: Some(1),
                monthly_revenue_rows: vec![("2026-03".to_string(), Decimal::from(50))],
                top_customer_rows: vec![("c1".to_string(), "Acme".to_string(), Decimal::from(25))],
                revenue_by_product_rows: vec![(
                    "p1".to_string(),
                    "Widget".to_string(),
                    Decimal::from(75),
                )],
                ..StubState::default()
            })),
        };

        let overview = get_overview(&repo).await.expect("overview");
        assert_eq!(overview.mrr, Decimal::from(100));
        assert_eq!(overview.arr, Decimal::from(1200));
        assert_eq!(overview.active_subscriptions, 3);
        assert_eq!(overview.monthly_revenue.len(), 1);
    }

    #[tokio::test]
    async fn forecasting_uses_average_monthly() {
        let repo = StubRepo {
            state: Arc::new(Mutex::new(StubState {
                forecasting_recent_revenue: vec![Decimal::from(10), Decimal::from(20)],
                ..StubState::default()
            })),
        };

        let forecast = get_forecasting(&repo).await.expect("forecast");
        assert_eq!(forecast.forecast_3mo, Decimal::from(45));
        assert_eq!(forecast.growth_rate, Decimal::from(100));
    }

    #[tokio::test]
    async fn reports_formats_json() {
        let repo = StubRepo {
            state: Arc::new(Mutex::new(StubState {
                reports_paid_count: Some(4),
                reports_total_paid: Some(Decimal::from(200)),
                reports_total_refunded: Some(Decimal::from(50)),
                ..StubState::default()
            })),
        };

        let report = get_reports(&repo).await.expect("report");
        assert_eq!(report["paidInvoices"], 4);
        assert_eq!(report["netRevenue"], "150");
    }
}
