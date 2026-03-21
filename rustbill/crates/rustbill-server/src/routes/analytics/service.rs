use super::repository::AnalyticsRepository;
use super::schema::{ForecastParams, ReportParams, Sales360Params};
use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rustbill_core::error::BillingError;

fn parse_window(params: &Sales360Params) -> (chrono::DateTime<Utc>, chrono::DateTime<Utc>) {
    use chrono::Duration;
    let default_to = Utc::now();
    let default_from = default_to - Duration::days(30);

    let from = params
        .from
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|naive| chrono::DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
        .unwrap_or(default_from);

    let to = params
        .to
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(23, 59, 59))
        .map(|naive| chrono::DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
        .unwrap_or(default_to);

    (from, to)
}

fn parse_timezone(params: &Sales360Params) -> String {
    params
        .timezone
        .as_deref()
        .map(str::trim)
        .filter(|tz| !tz.is_empty())
        .unwrap_or("UTC")
        .to_string()
}

fn parse_currency(params: &Sales360Params) -> Option<String> {
    params
        .currency
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(|c| c.to_ascii_uppercase())
}

fn decimal_to_i64(d: Decimal) -> i64 {
    d.to_i64().unwrap_or(0)
}

fn decimal_to_f64(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(0.0)
}

pub async fn overview<R: AnalyticsRepository>(repo: &R) -> Result<serde_json::Value, BillingError> {
    let total_customers = repo.total_customers().await?;
    let active_subscriptions = repo.active_subscriptions().await?;
    let mrr = repo.mrr().await?;
    let total_revenue = repo.total_revenue().await?;
    let active_licenses = repo.active_licenses().await?;

    Ok(serde_json::json!({
        "totalCustomers": total_customers.0,
        "activeSubscriptions": active_subscriptions.0,
        "mrr": decimal_to_i64(mrr.0.unwrap_or_default()),
        "totalRevenue": decimal_to_i64(total_revenue.0.unwrap_or_default()),
        "activeLicenses": active_licenses.0,
    }))
}

pub async fn forecasting<R: AnalyticsRepository>(
    repo: &R,
    _params: &ForecastParams,
) -> Result<serde_json::Value, BillingError> {
    let now = chrono::Utc::now();
    let current_month = now.month0() as i32;
    let current_year = now.year();

    let monthly_deals = repo.monthly_deals().await?;

    let mut actuals_map = std::collections::HashMap::new();
    for (month_num, year_num, total) in &monthly_deals {
        let key = format!("{}-{}", *year_num as i32, *month_num as i32);
        let val = total.map(decimal_to_i64).unwrap_or(0);
        actuals_map.insert(key, val);
    }

    let annual_target = repo.annual_target().await?;
    let annual_target_val = decimal_to_i64(annual_target.0.unwrap_or_default());
    let monthly_target = annual_target_val / 12;

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

    let mut forecast_data = Vec::new();
    for (idx, month_name) in [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .iter()
    .enumerate()
    {
        let month_idx = idx as i32 + 1;
        let actual = actuals_map
            .get(&format!("{}-{}", current_year, month_idx))
            .copied();
        let is_past = idx as i32 <= current_month;
        let growth_factor = 1.0 + (idx as f64 - current_month as f64) * 0.05;
        let forecast = (avg_recent as f64 * growth_factor.max(0.8)).round() as i64;

        forecast_data.push(serde_json::json!({
            "month": month_name,
            "actual": if is_past { actual.map(serde_json::Value::from) } else { None }.unwrap_or(serde_json::Value::Null),
            "forecast": forecast,
            "target": monthly_target,
        }));
    }

    let quarterly_invoices = repo.quarterly_invoices().await?;
    let mut quarters: std::collections::HashMap<String, (i64, i64, i64)> =
        std::collections::HashMap::new();
    for q in 1..=4 {
        quarters.insert(format!("Q{}", q), (0, 0, 0));
    }
    for (quarter, status, total) in &quarterly_invoices {
        let q_key = format!("Q{}", *quarter as i32);
        let val = decimal_to_i64(total.unwrap_or_default());
        if let Some(entry) = quarters.get_mut(&q_key) {
            if status == "paid" {
                entry.0 += val;
            }
            entry.1 += val;
            entry.2 += val;
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

    let overdue_invoices = repo.overdue_invoices().await?;
    let at_risk_subs = repo.at_risk_subscriptions().await?;
    let low_health = repo.low_health_customers().await?;
    let mut risk_factors: Vec<serde_json::Value> = Vec::new();

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

    let annual_projected: i64 = forecast_data
        .iter()
        .map(|d| d["forecast"].as_i64().unwrap_or(0))
        .sum();
    let scenarios = serde_json::json!([
        { "name": "Conservative", "probability": 85, "revenue": (annual_projected as f64 * 0.9).round() as i64, "color": "chart-4" },
        { "name": "Base Case", "probability": 65, "revenue": annual_projected, "color": "accent" },
        { "name": "Optimistic", "probability": 40, "revenue": (annual_projected as f64 * 1.15).round() as i64, "color": "chart-1" },
    ]);

    let current_quarter_idx = current_month / 3;
    let current_q_key = format!("Q{}", current_quarter_idx + 1);
    let (committed, _best_case, projected) =
        quarters.get(&current_q_key).copied().unwrap_or((0, 0, 0));
    let _ = committed;
    let quarter_target = annual_target_val / 4;

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

    let deal_coverage = if quarter_target > 0 {
        ((projected as f64 / quarter_target as f64) * 10.0).round() / 10.0
    } else {
        0.0
    };

    let at_risk_revenue: i64 = risk_factors
        .iter()
        .map(|r| {
            let impact_str = r["impact"].as_str().unwrap_or("$0");
            let digits: String = impact_str.chars().filter(|c| c.is_ascii_digit()).collect();
            digits.parse::<i64>().unwrap_or(0)
        })
        .sum();

    Ok(serde_json::json!({
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
    }))
}

pub async fn reports<R: AnalyticsRepository>(
    repo: &R,
    _params: &ReportParams,
) -> Result<serde_json::Value, BillingError> {
    let monthly_deals = repo.reports_monthly_deals().await?;
    let total_customers = repo.reports_total_customers().await?;
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

    let product_type_revenue = repo.product_type_revenue().await?;
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

    let recent_invoices = repo.recent_invoices().await?;
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

    Ok(serde_json::json!({
        "conversionData": conversion_data,
        "sourceData": source_data,
        "reports": reports,
        "yoyChange": yoy_change,
    }))
}

pub async fn sales_360_summary<R: AnalyticsRepository>(
    repo: &R,
    params: &Sales360Params,
) -> Result<serde_json::Value, BillingError> {
    let (from, to) = parse_window(params);
    let timezone = parse_timezone(params);
    let currency = parse_currency(params);

    let rows = repo
        .sales_360_summary_rows(from, to, currency.as_deref())
        .await?;
    let by_currency_rows = repo
        .sales_360_by_currency_rows(from, to, currency.as_deref())
        .await?;
    let available_currencies_rows = repo.sales_360_available_currencies(from, to).await?;

    let mut summary = serde_json::Map::new();
    for (classification, subtotal, tax, total) in rows {
        summary.insert(
            classification,
            serde_json::json!({
                "subtotal": decimal_to_i64(subtotal.unwrap_or_default()),
                "tax": decimal_to_i64(tax.unwrap_or_default()),
                "total": decimal_to_i64(total.unwrap_or_default()),
            }),
        );
    }

    let mut by_currency: std::collections::BTreeMap<
        String,
        serde_json::Map<String, serde_json::Value>,
    > = std::collections::BTreeMap::new();
    for (row_currency, classification, total) in by_currency_rows {
        let entry = by_currency.entry(row_currency).or_default();
        entry.insert(
            classification,
            serde_json::Value::from(decimal_to_i64(total.unwrap_or_default())),
        );
    }

    let available_currencies: Vec<String> = available_currencies_rows
        .into_iter()
        .map(|(value,)| value)
        .collect();

    Ok(serde_json::json!({
        "from": from.to_rfc3339(),
        "to": to.to_rfc3339(),
        "timezone": timezone,
        "currency": currency,
        "availableCurrencies": available_currencies,
        "summary": summary,
        "byCurrency": by_currency,
    }))
}

pub async fn sales_360_timeseries<R: AnalyticsRepository>(
    repo: &R,
    params: &Sales360Params,
) -> Result<serde_json::Value, BillingError> {
    let (from, to) = parse_window(params);
    let timezone = parse_timezone(params);
    let currency = parse_currency(params);

    let rows = repo
        .sales_360_timeseries_rows(from, to, &timezone, currency.as_deref())
        .await?;

    let mut grouped: std::collections::BTreeMap<
        String,
        serde_json::Map<String, serde_json::Value>,
    > = std::collections::BTreeMap::new();

    for (day, classification, total) in rows {
        let entry = grouped.entry(day.clone()).or_insert_with(|| {
            let mut base = serde_json::Map::new();
            base.insert("day".to_string(), serde_json::Value::String(day));
            base
        });
        entry.insert(
            classification,
            serde_json::Value::from(decimal_to_i64(total.unwrap_or_default())),
        );
    }

    let data: Vec<serde_json::Value> = grouped
        .into_values()
        .map(serde_json::Value::Object)
        .collect();

    Ok(serde_json::json!({
        "from": from.to_rfc3339(),
        "to": to.to_rfc3339(),
        "timezone": timezone,
        "currency": currency,
        "data": data,
    }))
}

pub async fn sales_360_breakdown<R: AnalyticsRepository>(
    repo: &R,
    params: &Sales360Params,
) -> Result<serde_json::Value, BillingError> {
    let (from, to) = parse_window(params);
    let timezone = parse_timezone(params);
    let currency = parse_currency(params);

    let by_event = repo
        .sales_360_breakdown_event_rows(from, to, currency.as_deref())
        .await?;
    let by_customer = repo
        .sales_360_breakdown_customer_rows(from, to, currency.as_deref())
        .await?;

    Ok(serde_json::json!({
        "from": from.to_rfc3339(),
        "to": to.to_rfc3339(),
        "timezone": timezone,
        "currency": currency,
        "byEventType": by_event.into_iter().map(|(event_type, total)| serde_json::json!({
            "eventType": event_type,
            "total": decimal_to_i64(total.unwrap_or_default()),
        })).collect::<Vec<_>>(),
        "byCustomer": by_customer.into_iter().map(|(customer_id, total)| serde_json::json!({
            "customerId": customer_id,
            "total": decimal_to_i64(total.unwrap_or_default()),
        })).collect::<Vec<_>>(),
    }))
}

pub async fn sales_360_backfill<R: AnalyticsRepository>(
    repo: &R,
) -> Result<serde_json::Value, BillingError> {
    let result = repo.backfill_sales_events().await?;
    Ok(serde_json::json!({
        "success": true,
        "result": serde_json::to_value(result).map_err(|e| BillingError::Internal(e.into()))?,
    }))
}

pub async fn sales_360_reconcile<R: AnalyticsRepository>(
    repo: &R,
    params: &Sales360Params,
) -> Result<serde_json::Value, BillingError> {
    let (from, to) = parse_window(params);
    let timezone = parse_timezone(params);
    let currency = parse_currency(params);

    let rows = repo
        .sales_360_reconcile_rows(from, to, currency.as_deref())
        .await?;

    let mut by_classification: std::collections::BTreeMap<String, serde_json::Value> =
        std::collections::BTreeMap::new();

    for (classification, ledger_total, source_total, event_count, missing_sources) in rows {
        let ledger_total = decimal_to_i64(ledger_total.unwrap_or_default());
        let source_total = decimal_to_i64(source_total.unwrap_or_default());
        let delta = ledger_total - source_total;
        let status = if delta == 0 && missing_sources == 0 {
            "ok"
        } else {
            "drift"
        };

        by_classification.insert(
            classification,
            serde_json::json!({
                "ledgerTotal": ledger_total,
                "sourceTotal": source_total,
                "delta": delta,
                "eventCount": event_count,
                "missingSources": missing_sources,
                "status": status,
            }),
        );
    }

    for classification in [
        "bookings",
        "billings",
        "collections",
        "adjustments",
        "recurring",
    ] {
        by_classification
            .entry(classification.to_string())
            .or_insert_with(|| {
                serde_json::json!({
                    "ledgerTotal": 0,
                    "sourceTotal": 0,
                    "delta": 0,
                    "eventCount": 0,
                    "missingSources": 0,
                    "status": "ok",
                })
            });
    }

    Ok(serde_json::json!({
        "from": from.to_rfc3339(),
        "to": to.to_rfc3339(),
        "timezone": timezone,
        "currency": currency,
        "rows": by_classification,
    }))
}

pub async fn sales_360_export<R: AnalyticsRepository>(
    repo: &R,
    params: &Sales360Params,
) -> Result<String, BillingError> {
    let (from, to) = parse_window(params);
    let currency = parse_currency(params);

    let by_event = repo
        .sales_360_export_event_rows(from, to, currency.as_deref())
        .await?;
    let by_customer = repo
        .sales_360_export_customer_rows(from, to, currency.as_deref())
        .await?;

    let mut csv = String::from("section,key,currency,total\n");
    for (event_type, row_currency, total) in by_event {
        csv.push_str(&format!(
            "event_type,{},{},{}\n",
            event_type,
            row_currency,
            decimal_to_i64(total.unwrap_or_default())
        ));
    }
    for (customer_id, row_currency, total) in by_customer {
        csv.push_str(&format!(
            "customer,{},{},{}\n",
            customer_id.unwrap_or_else(|| "unknown".to_string()),
            row_currency,
            decimal_to_i64(total.unwrap_or_default())
        ));
    }

    Ok(csv)
}
