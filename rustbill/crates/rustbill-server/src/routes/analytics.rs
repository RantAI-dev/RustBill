use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Query, State},
    http::header,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use rust_decimal::Decimal;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/overview", get(overview))
        .route("/forecasting", get(forecasting))
        .route("/reports", get(reports))
        .route("/sales-360/summary", get(sales_360_summary))
        .route("/sales-360/timeseries", get(sales_360_timeseries))
        .route("/sales-360/breakdown", get(sales_360_breakdown))
        .route("/sales-360/reconcile", get(sales_360_reconcile))
        .route("/sales-360/export", get(sales_360_export))
        .route("/sales-360/backfill", post(sales_360_backfill))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sales360Params {
    from: Option<String>,
    to: Option<String>,
    timezone: Option<String>,
    currency: Option<String>,
}

fn parse_window(
    params: &Sales360Params,
) -> (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) {
    use chrono::{Duration, NaiveDate, Utc};
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

async fn sales_360_summary(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<Sales360Params>,
) -> ApiResult<Json<serde_json::Value>> {
    let (from, to) = parse_window(&params);
    let timezone = parse_timezone(&params);
    let currency = parse_currency(&params);

    let rows: Vec<(String, Option<Decimal>, Option<Decimal>, Option<Decimal>)> = sqlx::query_as(
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
    .bind(currency.as_deref())
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let by_currency_rows: Vec<(String, String, Option<Decimal>)> = sqlx::query_as(
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
    .bind(currency.as_deref())
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let available_currencies_rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT currency
        FROM sales_events
        WHERE occurred_at >= $1 AND occurred_at <= $2
        ORDER BY currency ASC
        "#,
    )
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

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

    Ok(Json(serde_json::json!({
        "from": from.to_rfc3339(),
        "to": to.to_rfc3339(),
        "timezone": timezone,
        "currency": currency,
        "availableCurrencies": available_currencies,
        "summary": summary,
        "byCurrency": by_currency,
    })))
}

async fn sales_360_timeseries(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<Sales360Params>,
) -> ApiResult<Json<serde_json::Value>> {
    let (from, to) = parse_window(&params);
    let timezone = parse_timezone(&params);
    let currency = parse_currency(&params);

    let rows: Vec<(String, String, Option<Decimal>)> = sqlx::query_as(
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
    .bind(&timezone)
    .bind(currency.as_deref())
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

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

    Ok(Json(serde_json::json!({
        "from": from.to_rfc3339(),
        "to": to.to_rfc3339(),
        "timezone": timezone,
        "currency": currency,
        "data": data,
    })))
}

async fn sales_360_breakdown(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<Sales360Params>,
) -> ApiResult<Json<serde_json::Value>> {
    let (from, to) = parse_window(&params);
    let timezone = parse_timezone(&params);
    let currency = parse_currency(&params);

    let by_event: Vec<(String, Option<Decimal>)> = sqlx::query_as(
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
    .bind(currency.as_deref())
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let by_customer: Vec<(Option<String>, Option<Decimal>)> = sqlx::query_as(
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
    .bind(currency.as_deref())
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(serde_json::json!({
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
    })))
}

async fn sales_360_backfill(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let result = rustbill_core::analytics::sales_ledger::backfill_sales_events(&state.db).await?;
    Ok(Json(serde_json::json!({
        "success": true,
        "result": result,
    })))
}

async fn sales_360_reconcile(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<Sales360Params>,
) -> ApiResult<Json<serde_json::Value>> {
    let (from, to) = parse_window(&params);
    let timezone = parse_timezone(&params);
    let currency = parse_currency(&params);

    let rows: Vec<(String, Option<Decimal>, Option<Decimal>, i64, i64)> = sqlx::query_as(
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
    .bind(currency.as_deref())
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

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

    Ok(Json(serde_json::json!({
        "from": from.to_rfc3339(),
        "to": to.to_rfc3339(),
        "timezone": timezone,
        "currency": currency,
        "rows": by_classification,
    })))
}

async fn sales_360_export(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<Sales360Params>,
) -> ApiResult<impl IntoResponse> {
    let (from, to) = parse_window(&params);
    let currency = parse_currency(&params);

    let by_event: Vec<(String, String, Option<Decimal>)> = sqlx::query_as(
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
    .bind(currency.as_deref())
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let by_customer: Vec<(Option<String>, String, Option<Decimal>)> = sqlx::query_as(
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
    .bind(currency.as_deref())
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

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

    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"sales-360-export.csv\"",
            ),
        ],
        csv,
    ))
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

    let mrr: (Option<Decimal>,) = sqlx::query_as(
        r#"SELECT SUM(pp.base_price) FROM subscriptions s
           JOIN pricing_plans pp ON pp.id = s.plan_id
           WHERE s.status = 'active' AND s.deleted_at IS NULL"#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let total_revenue: (Option<Decimal>,) = sqlx::query_as("SELECT SUM(amount) FROM payments")
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
        "mrr": decimal_to_i64(mrr.0.unwrap_or_default()),
        "totalRevenue": decimal_to_i64(total_revenue.0.unwrap_or_default()),
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
