use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use rust_decimal::Decimal;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/overview", get(overview))
        .route("/forecasting", get(forecasting))
        .route("/reports", get(reports))
}

async fn overview(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let total_customers: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM customers")
        .fetch_one(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    let active_subscriptions: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM subscriptions WHERE status = 'active'")
            .fetch_one(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?;

    let mrr: (Option<i64>,) = sqlx::query_as(
        r#"SELECT SUM(bp.amount) FROM subscriptions s
           JOIN billing_plans bp ON bp.id = s.plan_id
           WHERE s.status = 'active'"#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let total_revenue: (Option<i64>,) =
        sqlx::query_as("SELECT SUM(amount) FROM payments WHERE status = 'succeeded'")
            .fetch_one(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?;

    let active_licenses: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM licenses WHERE status = 'active'")
            .fetch_one(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(serde_json::json!({
        "totalCustomers": total_customers.0,
        "activeSubscriptions": active_subscriptions.0,
        "mrr": mrr.0.unwrap_or(0),
        "totalRevenue": total_revenue.0.unwrap_or(0),
        "activeLicenses": active_licenses.0,
    })))
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct ForecastParams {
    months: Option<i32>,
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

async fn forecasting(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(_params): Query<ForecastParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let now = chrono::Utc::now();
    let current_month = now.month0() as i32; // 0-indexed
    let current_year = now.year();

    // 1. Monthly actuals — deal values grouped by month for current year
    let monthly_deals: Vec<(f64, f64, Option<Decimal>)> = sqlx::query_as(
        r#"SELECT
             extract(month from to_date(date, 'Mon DD, YYYY'))::float8 as month_num,
             extract(year from to_date(date, 'Mon DD, YYYY'))::float8 as year_num,
             SUM(value) as total
           FROM deals
           GROUP BY month_num, year_num
           ORDER BY year_num, month_num"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    // Build actuals map: "year-month" -> value
    let mut actuals_map = std::collections::HashMap::new();
    for (month_num, year_num, total) in &monthly_deals {
        let key = format!("{}-{}", *year_num as i32, *month_num as i32);
        let val = total.map(|d| decimal_to_i64(d)).unwrap_or(0);
        actuals_map.insert(key, val);
    }

    // Monthly target from products
    let annual_target: (Option<Decimal>,) = sqlx::query_as("SELECT SUM(target) FROM products")
        .fetch_one(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;
    let annual_target_val = decimal_to_i64(annual_target.0.unwrap_or_default());
    let monthly_target = annual_target_val / 12;

    // Recent average for forecast projection (last 6 months)
    let mut recent_actuals: Vec<i64> = Vec::new();
    for i in 0..6 {
        let mut m = current_month - i;
        let mut y = current_year;
        if m < 0 {
            m += 12;
            y -= 1;
        }
        let key = format!("{}-{}", y, m + 1);
        if let Some(&val) = actuals_map.get(&key) {
            if val > 0 {
                recent_actuals.push(val);
            }
        }
    }
    let avg_recent = if recent_actuals.is_empty() {
        monthly_target
    } else {
        recent_actuals.iter().sum::<i64>() / recent_actuals.len() as i64
    };

    // 2. Build forecast data (12 months)
    let mut forecast_data = Vec::new();
    for (idx, month_name) in MONTHS.iter().enumerate() {
        let month_idx = idx as i32 + 1;
        let actual = actuals_map
            .get(&format!("{}-{}", current_year, month_idx))
            .copied();
        let is_past = idx as i32 <= current_month;

        // Forecast: growing projection from average
        let growth_factor = 1.0 + (idx as f64 - current_month as f64) * 0.05;
        let forecast = (avg_recent as f64 * growth_factor.max(0.8)).round() as i64;

        forecast_data.push(serde_json::json!({
            "month": month_name,
            "actual": if is_past { actual.map(serde_json::Value::from) } else { None }.unwrap_or(serde_json::Value::Null),
            "forecast": forecast,
            "target": monthly_target,
        }));
    }

    // 3. Quarterly breakdown from invoice data
    let quarterly_invoices: Vec<(f64, String, Option<Decimal>)> = sqlx::query_as(
        r#"SELECT
             extract(quarter from created_at)::float8 as quarter,
             status::text as status,
             SUM(total) as total
           FROM invoices
           WHERE deleted_at IS NULL
           GROUP BY quarter, status"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let mut quarters: std::collections::HashMap<String, (i64, i64, i64)> =
        std::collections::HashMap::new();
    for q in 1..=4 {
        quarters.insert(format!("Q{}", q), (0, 0, 0)); // committed, bestCase, projected
    }
    for (quarter, status, total) in &quarterly_invoices {
        let q_key = format!("Q{}", *quarter as i32);
        let val = decimal_to_i64(total.unwrap_or_default());
        if let Some(entry) = quarters.get_mut(&q_key) {
            if status == "paid" {
                entry.0 += val; // committed
            }
            entry.1 += val; // bestCase
            entry.2 += val; // projected
        }
    }

    let quarterly_forecast: Vec<serde_json::Value> = (1..=4)
        .map(|q| {
            let q_key = format!("Q{}", q);
            let (committed, best_case, projected) =
                quarters.get(&q_key).copied().unwrap_or((0, 0, 0));
            serde_json::json!({
                "quarter": q_key,
                "committed": committed,
                "bestCase": std::cmp::max(best_case, (committed as f64 * 1.2) as i64),
                "projected": std::cmp::max(projected, (committed as f64 * 1.5) as i64),
            })
        })
        .collect();

    // 4. Risk factors
    let mut risk_factors: Vec<serde_json::Value> = Vec::new();

    // Overdue invoices
    let overdue_invoices: Vec<(String, String, Decimal, Option<String>)> = sqlx::query_as(
        r#"SELECT i.id, i.invoice_number, i.total, c.name
           FROM invoices i
           LEFT JOIN customers c ON i.customer_id = c.id
           WHERE i.status = 'overdue' AND i.deleted_at IS NULL"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    if !overdue_invoices.is_empty() {
        let total_overdue: i64 = overdue_invoices
            .iter()
            .map(|(_, _, t, _)| decimal_to_i64(*t))
            .sum();
        let deals_list: Vec<String> = overdue_invoices
            .iter()
            .map(|(_, inv_num, _, name)| {
                format!("{} ({})", inv_num, name.as_deref().unwrap_or("Unknown"))
            })
            .collect();
        risk_factors.push(serde_json::json!({
            "id": "overdue",
            "title": "Overdue Invoices",
            "description": format!("{} invoice(s) past due date", overdue_invoices.len()),
            "impact": format!("-${}", total_overdue),
            "severity": if total_overdue > 1000 { "high" } else { "medium" },
            "deals": deals_list,
        }));
    }

    // At-risk subscriptions (past_due or cancel_at_period_end)
    let at_risk_subs: Vec<(String, Option<String>)> = sqlx::query_as(
        r#"SELECT s.id, c.name
           FROM subscriptions s
           LEFT JOIN customers c ON s.customer_id = c.id
           WHERE (s.status = 'past_due' OR s.cancel_at_period_end = true)
             AND s.deleted_at IS NULL"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    if !at_risk_subs.is_empty() {
        let estimated_impact = at_risk_subs.len() as i64 * ((avg_recent as f64 * 0.1) as i64);
        let deals_list: Vec<String> = at_risk_subs
            .iter()
            .map(|(_, name)| name.as_deref().unwrap_or("Unknown").to_string())
            .collect();
        risk_factors.push(serde_json::json!({
            "id": "past-due-subs",
            "title": "Past-Due Subscriptions",
            "description": format!("{} subscription(s) with payment issues", at_risk_subs.len()),
            "impact": format!("-${}", estimated_impact),
            "severity": "high",
            "deals": deals_list,
        }));
    }

    // Low-health customers (health_score < 60)
    let low_health: Vec<(String, String, i32)> = sqlx::query_as(
        r#"SELECT id, name, health_score
           FROM customers
           WHERE health_score < 60"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    if !low_health.is_empty() {
        let estimated_impact = low_health.len() as i64 * 50000;
        let deals_list: Vec<String> = low_health
            .iter()
            .map(|(_, name, score)| format!("{} ({}%)", name, score))
            .collect();
        risk_factors.push(serde_json::json!({
            "id": "low-health",
            "title": "Customer Health Decline",
            "description": format!("{} customer(s) with health score below 60", low_health.len()),
            "impact": format!("-${}", estimated_impact),
            "severity": "medium",
            "deals": deals_list,
        }));
    }

    if risk_factors.is_empty() {
        risk_factors.push(serde_json::json!({
            "id": "none",
            "title": "No Significant Risks",
            "description": "All metrics are within normal ranges",
            "impact": "$0",
            "severity": "low",
            "deals": [],
        }));
    }

    // 5. Scenarios
    let annual_projected: i64 = forecast_data
        .iter()
        .map(|d| d["forecast"].as_i64().unwrap_or(0))
        .sum();
    let scenarios = serde_json::json!([
        { "name": "Conservative", "probability": 85, "revenue": (annual_projected as f64 * 0.9).round() as i64, "color": "chart-4" },
        { "name": "Base Case", "probability": 65, "revenue": annual_projected, "color": "accent" },
        { "name": "Optimistic", "probability": 40, "revenue": (annual_projected as f64 * 1.15).round() as i64, "color": "chart-1" },
    ]);

    // 6. KPIs
    let current_quarter_idx = current_month / 3;
    let current_q_key = format!("Q{}", current_quarter_idx + 1);
    let (committed, _best_case, projected) =
        quarters.get(&current_q_key).copied().unwrap_or((0, 0, 0));
    let _ = committed; // suppress unused warning
    let quarter_target = annual_target_val / 4;

    // Forecast accuracy — compare forecast vs actuals for past months
    let forecast_accuracy: serde_json::Value = {
        if recent_actuals.len() < 2 {
            serde_json::Value::Null
        } else {
            let mut total_error = 0.0f64;
            let mut compared = 0;
            for i in 0..=current_month as usize {
                let key = format!("{}-{}", current_year, i + 1);
                if let Some(&actual) = actuals_map.get(&key) {
                    if actual > 0 {
                        let forecast_val = forecast_data
                            .get(i)
                            .and_then(|d| d["forecast"].as_i64())
                            .unwrap_or(0);
                        let error = (forecast_val - actual).unsigned_abs() as f64 / actual as f64;
                        total_error += error;
                        compared += 1;
                    }
                }
            }
            if compared == 0 {
                serde_json::Value::Null
            } else {
                serde_json::json!(((1.0 - total_error / compared as f64) * 100.0).round() as i64)
            }
        }
    };

    // Deal coverage: pipeline value / quarterly target
    let deal_coverage = if quarter_target > 0 {
        ((projected as f64 / quarter_target as f64) * 10.0).round() / 10.0
    } else {
        0.0
    };

    // At-risk revenue: sum of impact values from risk factors
    let at_risk_revenue: i64 = risk_factors
        .iter()
        .map(|r| {
            let impact_str = r["impact"].as_str().unwrap_or("$0");
            // Extract digits from the impact string
            let digits: String = impact_str.chars().filter(|c| c.is_ascii_digit()).collect();
            digits.parse::<i64>().unwrap_or(0)
        })
        .sum();

    Ok(Json(serde_json::json!({
        "forecastData": forecast_data,
        "quarterlyForecast": quarterly_forecast,
        "riskFactors": risk_factors,
        "scenarios": scenarios,
        "kpis": {
            "currentQuarterForecast": projected,
            "quarterTarget": quarter_target,
            "forecastAccuracy": forecast_accuracy,
            "dealCoverage": deal_coverage,
            "atRiskRevenue": at_risk_revenue,
        },
    })))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct ReportParams {
    report_type: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

async fn reports(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(_params): Query<ReportParams>,
) -> ApiResult<Json<serde_json::Value>> {
    // 1. Conversion data — monthly deal count / customer ratio
    let monthly_deals: Vec<(String, f64, i64)> = sqlx::query_as(
        r#"SELECT
             to_char(to_date(date, 'Mon DD, YYYY'), 'Mon') as month,
             extract(month from to_date(date, 'Mon DD, YYYY'))::float8 as month_num,
             COUNT(*) as deal_count
           FROM deals
           GROUP BY month, month_num
           ORDER BY month_num"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let total_customers: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM customers")
        .fetch_one(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;
    let customer_count = std::cmp::max(total_customers.0, 1);

    let conversion_data: Vec<serde_json::Value> = monthly_deals
        .iter()
        .map(|(month, _month_num, deal_count)| {
            let rate = ((*deal_count as f64 / customer_count as f64) * 100.0).round() as i64;
            serde_json::json!({
                "month": month,
                "rate": rate,
            })
        })
        .collect();

    // 2. Revenue by product type
    let product_type_revenue: Vec<(String, Option<Decimal>)> = sqlx::query_as(
        r#"SELECT product_type::text, SUM(revenue) as revenue
           FROM products
           GROUP BY product_type"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let total_revenue: f64 = product_type_revenue
        .iter()
        .map(|(_, r)| decimal_to_f64(r.unwrap_or_default()))
        .sum();

    let type_labels: std::collections::HashMap<&str, &str> = [
        ("licensed", "Licensed Products"),
        ("saas", "AI Chat Platform"),
        ("api", "AI Chat API"),
    ]
    .into_iter()
    .collect();

    let type_colors: std::collections::HashMap<&str, &str> = [
        ("licensed", "oklch(0.7 0.18 220)"),
        ("saas", "oklch(0.75 0.18 55)"),
        ("api", "oklch(0.65 0.2 25)"),
    ]
    .into_iter()
    .collect();

    let source_data: Vec<serde_json::Value> = product_type_revenue
        .iter()
        .map(|(ptype, revenue)| {
            let rev = decimal_to_f64(revenue.unwrap_or_default());
            let pct = if total_revenue > 0.0 {
                ((rev / total_revenue) * 100.0).round() as i64
            } else {
                0
            };
            serde_json::json!({
                "name": type_labels.get(ptype.as_str()).unwrap_or(&ptype.as_str()),
                "value": pct,
                "color": type_colors.get(ptype.as_str()).unwrap_or(&"oklch(0.5 0 0)"),
            })
        })
        .collect();

    // 3. YoY change
    let yoy_change = if conversion_data.len() >= 2 {
        let first_rate = conversion_data
            .first()
            .and_then(|v| v["rate"].as_i64())
            .unwrap_or(0);
        let last_rate = conversion_data
            .last()
            .and_then(|v| v["rate"].as_i64())
            .unwrap_or(0);
        format!("+{}%", (last_rate - first_rate).unsigned_abs())
    } else {
        "+0%".to_string()
    };

    // 4. Recent reports — last 10 invoices as report entries
    let recent_invoices: Vec<(
        String,
        String,
        String,
        Decimal,
        NaiveDateTime,
        Option<String>,
    )> = sqlx::query_as(
        r#"SELECT i.id, i.invoice_number, i.status::text, i.total, i.created_at, c.name
               FROM invoices i
               LEFT JOIN customers c ON i.customer_id = c.id
               WHERE i.deleted_at IS NULL
               ORDER BY i.created_at DESC
               LIMIT 10"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let reports: Vec<serde_json::Value> = recent_invoices
        .iter()
        .map(
            |(id, inv_number, status, _total, created_at, customer_name)| {
                let cust = customer_name.as_deref().unwrap_or("Unknown");
                let date_str = created_at.format("%b %d, %Y").to_string();
                let report_status = if status == "draft" {
                    "generating"
                } else {
                    "ready"
                };
                serde_json::json!({
                    "id": id,
                    "name": format!("Invoice {} — {}", inv_number, cust),
                    "type": "Invoice",
                    "date": date_str,
                    "status": report_status,
                })
            },
        )
        .collect();

    Ok(Json(serde_json::json!({
        "conversionData": conversion_data,
        "sourceData": source_data,
        "reports": reports,
        "yoyChange": yoy_change,
    })))
}

/// Convert Decimal to i64 by truncating
fn decimal_to_i64(d: Decimal) -> i64 {
    use rust_decimal::prelude::ToPrimitive;
    d.to_i64().unwrap_or(0)
}

/// Convert Decimal to f64
fn decimal_to_f64(d: Decimal) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    d.to_f64().unwrap_or(0.0)
}

// Need chrono traits for month/year access
use chrono::{Datelike, NaiveDateTime};
