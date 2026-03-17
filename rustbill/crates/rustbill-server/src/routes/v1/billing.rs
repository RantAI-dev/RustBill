use crate::app::SharedState;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .nest("/invoices", invoices_router())
        .nest("/subscriptions", subscriptions_router())
        .nest("/usage", usage_router())
}

// ---------------------------------------------------------------------------
// Invoices
// ---------------------------------------------------------------------------

fn invoices_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_invoices))
        .route("/{id}", get(get_invoice))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvoiceListParams {
    status: Option<String>,
    customer_id: Option<String>,
}

async fn list_invoices(
    State(state): State<SharedState>,
    Query(params): Query<InvoiceListParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(i) FROM invoices i
           WHERE ($1::text IS NULL OR i.status = $1)
             AND ($2::text IS NULL OR i.customer_id = $2)
           ORDER BY i.created_at DESC"#,
    )
    .bind(&params.status)
    .bind(&params.customer_id)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows))
}

async fn get_invoice(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let invoice = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(i) FROM invoices i WHERE i.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "invoice".into(),
        id: id.clone(),
    })?;

    // Fetch items and payments for the invoice
    let items = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(li) FROM invoice_items li WHERE li.invoice_id = $1 ORDER BY li.id",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let payments = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(p) FROM payments p WHERE p.invoice_id = $1 ORDER BY p.created_at",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    // Merge items and payments into the invoice object
    let mut result = invoice;
    if let Some(obj) = result.as_object_mut() {
        obj.insert("items".to_string(), serde_json::json!(items));
        obj.insert("payments".to_string(), serde_json::json!(payments));
    }

    Ok(Json(result))
}

// ---------------------------------------------------------------------------
// Subscriptions
// ---------------------------------------------------------------------------

fn subscriptions_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_subscriptions).post(create_subscription))
        .route("/{id}", get(get_subscription).put(update_subscription))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubscriptionListParams {
    status: Option<String>,
    customer_id: Option<String>,
}

async fn list_subscriptions(
    State(state): State<SharedState>,
    Query(params): Query<SubscriptionListParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(s) FROM subscriptions s
           WHERE ($1::text IS NULL OR s.status = $1)
             AND ($2::text IS NULL OR s.customer_id = $2)
           ORDER BY s.created_at DESC"#,
    )
    .bind(&params.status)
    .bind(&params.customer_id)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows))
}

async fn get_subscription(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(s) FROM subscriptions s WHERE s.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "subscription".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

async fn create_subscription(
    State(state): State<SharedState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"INSERT INTO subscriptions (id, customer_id, plan_id, status, current_period_start, current_period_end, quantity, metadata, cancel_at_period_end, version, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, 'active', now(), now() + interval '1 month', COALESCE($3, 1), $4, false, 1, now(), now())
           RETURNING to_jsonb(subscriptions)"#,
    )
    .bind(body["customerId"].as_str())
    .bind(body["planId"].as_str())
    .bind(body["quantity"].as_i64().map(|v| v as i32))
    .bind(body.get("metadata").unwrap_or(&serde_json::json!({})))
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok((StatusCode::CREATED, Json(row)))
}

async fn update_subscription(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"UPDATE subscriptions SET
             plan_id = COALESCE($2, plan_id),
             status = COALESCE($3, status),
             metadata = COALESCE($4, metadata),
             updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(subscriptions)"#,
    )
    .bind(&id)
    .bind(body["planId"].as_str())
    .bind(body["status"].as_str())
    .bind(body.get("metadata"))
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "subscription".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

// ---------------------------------------------------------------------------
// Usage
// ---------------------------------------------------------------------------

fn usage_router() -> Router<SharedState> {
    Router::new().route("/", get(list_usage).post(record_usage))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageListParams {
    subscription_id: Option<String>,
    metric: Option<String>,
}

async fn list_usage(
    State(state): State<SharedState>,
    Query(params): Query<UsageListParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(u) FROM usage_events u
           WHERE ($1::text IS NULL OR u.subscription_id = $1)
             AND ($2::text IS NULL OR u.metric_name = $2)
           ORDER BY u.timestamp DESC"#,
    )
    .bind(&params.subscription_id)
    .bind(&params.metric)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows))
}

async fn record_usage(
    State(state): State<SharedState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    // Support batch: accept either a single object or an array
    let events = if body.is_array() {
        body.as_array().unwrap().clone()
    } else {
        vec![body]
    };

    let mut results = Vec::with_capacity(events.len());

    for event in &events {
        let row = sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO usage_events (id, subscription_id, metric_name, value, timestamp, idempotency_key, properties)
               VALUES (gen_random_uuid()::text, $1, $2, $3, COALESCE($4::timestamp, now()), $5, $6)
               RETURNING to_jsonb(usage_events)"#,
        )
        .bind(event["subscriptionId"].as_str())
        .bind(event["metricName"].as_str())
        .bind(event["value"].as_f64().unwrap_or(1.0))
        .bind(event["timestamp"].as_str())
        .bind(event["idempotencyKey"].as_str())
        .bind(event.get("properties").unwrap_or(&serde_json::json!({})))
        .fetch_one(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

        results.push(row);
    }

    // Return single object if single event, array if batch
    let response = if results.len() == 1 {
        results.into_iter().next().unwrap()
    } else {
        serde_json::json!(results)
    };

    Ok((StatusCode::CREATED, Json(response)))
}
