use crate::app::SharedState;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use rustbill_core::db::models::{PricingPlan, Subscription};
use rustbill_core::error::BillingError;
use serde::Deserialize;

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
        .route("/{id}/change-plan", post(change_plan_v1))
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
           RETURNING to_jsonb(subscriptions.*)"#,
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
             status = COALESCE($3::subscription_status, status),
             metadata = COALESCE($4, metadata),
             updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(subscriptions.*)"#,
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
               RETURNING to_jsonb(usage_events.*)"#,
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

// ---------------------------------------------------------------------------
// Plan Change with Proration (v1 API)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangePlanRequest {
    plan_id: String,
    #[serde(default)]
    quantity: Option<i32>,
    idempotency_key: Option<String>,
}

async fn change_plan_v1(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<ChangePlanRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let now = chrono::Utc::now().naive_utc();

    // Fetch subscription
    let sub: Subscription = sqlx::query_as(
        "SELECT * FROM subscriptions WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(BillingError::from)?
    .ok_or_else(|| BillingError::not_found("subscription", &id))?;

    // Idempotency check
    if let Some(ref key) = body.idempotency_key {
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM invoices WHERE idempotency_key = $1",
        )
        .bind(key)
        .fetch_optional(&state.db)
        .await
        .map_err(BillingError::from)?;
        if existing.is_some() {
            return Ok(Json(serde_json::json!({"message": "already processed"})));
        }
    }

    let old_plan: PricingPlan = sqlx::query_as(
        "SELECT * FROM pricing_plans WHERE id = $1",
    )
    .bind(&sub.plan_id)
    .fetch_one(&state.db)
    .await
    .map_err(BillingError::from)?;

    let new_plan: PricingPlan = sqlx::query_as(
        "SELECT * FROM pricing_plans WHERE id = $1",
    )
    .bind(&body.plan_id)
    .fetch_one(&state.db)
    .await
    .map_err(BillingError::from)?;

    let new_quantity = body.quantity.unwrap_or(sub.quantity);

    // Calculate proration
    let proration = rustbill_core::billing::proration::calculate_proration(
        &old_plan,
        &new_plan,
        sub.quantity,
        new_quantity,
        sub.current_period_start,
        sub.current_period_end,
        now,
    )?;

    // Handle financial result — downgrade deposits credit
    if proration.net < rust_decimal::Decimal::ZERO {
        let currency: String = sqlx::query_scalar(
            "SELECT currency FROM invoices WHERE subscription_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(BillingError::from)?
        .unwrap_or_else(|| "USD".to_string());

        rustbill_core::billing::credits::deposit(
            &state.db,
            &sub.customer_id,
            &currency,
            proration.net.abs(),
            rustbill_core::db::models::CreditReason::Proration,
            &format!("Proration credit: {} → {}", old_plan.name, new_plan.name),
            None,
        )
        .await?;
    }

    // Update subscription plan with optimistic concurrency via version check
    let rows = sqlx::query(
        r#"UPDATE subscriptions
           SET plan_id = $2, quantity = $3, version = version + 1, updated_at = NOW()
           WHERE id = $1 AND version = $4"#,
    )
    .bind(&id)
    .bind(&body.plan_id)
    .bind(new_quantity)
    .bind(sub.version)
    .execute(&state.db)
    .await
    .map_err(BillingError::from)?;

    if rows.rows_affected() == 0 {
        return Err(BillingError::conflict(
            "subscription was modified concurrently; retry the request",
        )
        .into());
    }

    // Emit event (best-effort)
    let _ = rustbill_core::notifications::events::emit_billing_event(
        &state.db,
        &state.http_client,
        rustbill_core::db::models::BillingEventType::SubscriptionPlanChanged,
        "subscription",
        &id,
        Some(&sub.customer_id),
        Some(serde_json::json!({
            "old_plan": old_plan.name,
            "new_plan": new_plan.name,
            "proration_net": proration.net.to_string(),
        })),
    )
    .await;

    let updated: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(s) FROM subscriptions s WHERE s.id = $1",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(BillingError::from)?;

    Ok(Json(updated))
}
