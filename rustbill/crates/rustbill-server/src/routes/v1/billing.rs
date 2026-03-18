use crate::app::SharedState;
use crate::routes::ApiResult;
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use rustbill_core::auth::api_key::ApiKeyInfo;
use rustbill_core::billing::{credits, payment_methods};
use rustbill_core::db::models::BillingEventType;
use rustbill_core::error::BillingError;
use serde::Deserialize;

pub fn router() -> Router<SharedState> {
    Router::new()
        .nest("/invoices", invoices_router())
        .nest("/subscriptions", subscriptions_router())
        .nest("/payment-methods", payment_methods_router())
        .nest("/credits", credits_router())
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

// ---------------------------------------------------------------------------
// Saved Payment Methods (v1)
// ---------------------------------------------------------------------------

fn payment_methods_router() -> Router<SharedState> {
    Router::new()
        .route(
            "/",
            get(list_payment_methods_v1).post(create_payment_method_v1),
        )
        .route("/setup", post(create_payment_method_setup_v1))
        .route("/{id}", delete(delete_payment_method_v1))
        .route("/{id}/default", post(set_default_payment_method_v1))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaymentMethodCustomerQuery {
    customer_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatePaymentMethodRequestV1 {
    customer_id: String,
    provider: rustbill_core::db::models::PaymentProvider,
    provider_token: String,
    method_type: rustbill_core::db::models::SavedPaymentMethodType,
    label: String,
    last_four: Option<String>,
    expiry_month: Option<i32>,
    expiry_year: Option<i32>,
    #[serde(default)]
    set_default: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaymentMethodSetupRequestV1 {
    customer_id: String,
    provider: rustbill_core::db::models::PaymentProvider,
}

async fn list_payment_methods_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Query(query): Query<PaymentMethodCustomerQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let customer_id = scoped_customer_id(&api_key, query.customer_id.as_deref())?;
    let methods = payment_methods::list_for_customer(&state.db, &customer_id).await?;
    Ok(Json(
        serde_json::to_value(methods).map_err(|e| BillingError::Internal(e.into()))?,
    ))
}

async fn create_payment_method_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Json(body): Json<CreatePaymentMethodRequestV1>,
) -> ApiResult<Json<serde_json::Value>> {
    let customer_id = scoped_customer_id(&api_key, Some(&body.customer_id))?;
    let method = payment_methods::create(
        &state.db,
        payment_methods::CreatePaymentMethodRequest {
            customer_id,
            provider: body.provider,
            provider_token: body.provider_token,
            method_type: body.method_type,
            label: body.label,
            last_four: body.last_four,
            expiry_month: body.expiry_month,
            expiry_year: body.expiry_year,
            set_default: body.set_default,
        },
    )
    .await?;

    Ok(Json(
        serde_json::to_value(method).map_err(|e| BillingError::Internal(e.into()))?,
    ))
}

async fn create_payment_method_setup_v1(
    Extension(api_key): Extension<ApiKeyInfo>,
    Query(_query): Query<PaymentMethodCustomerQuery>,
    Json(body): Json<PaymentMethodSetupRequestV1>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let customer_id = scoped_customer_id(&api_key, Some(&body.customer_id))?;
    Ok((
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "message": "payment method setup session is not implemented yet",
            "customerId": customer_id,
            "provider": body.provider,
        })),
    ))
}

async fn delete_payment_method_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Path(id): Path<String>,
    Query(query): Query<PaymentMethodCustomerQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let customer_id =
        resolve_payment_method_customer_id_v1(&state.db, &id, &api_key, query.customer_id).await?;
    payment_methods::remove(&state.db, &customer_id, &id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn set_default_payment_method_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Path(id): Path<String>,
    Query(query): Query<PaymentMethodCustomerQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let customer_id =
        resolve_payment_method_customer_id_v1(&state.db, &id, &api_key, query.customer_id).await?;
    let method = payment_methods::set_default(&state.db, &customer_id, &id).await?;
    Ok(Json(
        serde_json::to_value(method).map_err(|e| BillingError::Internal(e.into()))?,
    ))
}

async fn resolve_payment_method_customer_id_v1(
    pool: &sqlx::PgPool,
    method_id: &str,
    api_key: &ApiKeyInfo,
    provided_customer_id: Option<String>,
) -> ApiResult<String> {
    let scope_customer_id = scoped_customer_id(api_key, provided_customer_id.as_deref())?;

    if let Some(customer_id) = provided_customer_id {
        return Ok(customer_id);
    }

    let customer_id = sqlx::query_scalar::<_, String>(
        "SELECT customer_id FROM saved_payment_methods WHERE id = $1",
    )
    .bind(method_id)
    .fetch_optional(pool)
    .await
    .map_err(BillingError::from)?
    .ok_or_else(|| BillingError::not_found("payment_method", method_id))?;

    if customer_id != scope_customer_id {
        return Err(BillingError::Forbidden.into());
    }

    Ok(customer_id)
}

// ---------------------------------------------------------------------------
// Credits (v1)
// ---------------------------------------------------------------------------

fn credits_router() -> Router<SharedState> {
    Router::new().route("/", get(get_credits_v1))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreditsQueryV1 {
    customer_id: Option<String>,
    currency: Option<String>,
}

async fn get_credits_v1(
    State(state): State<SharedState>,
    Extension(api_key): Extension<ApiKeyInfo>,
    Query(query): Query<CreditsQueryV1>,
) -> ApiResult<Json<serde_json::Value>> {
    let customer_id = scoped_customer_id(&api_key, query.customer_id.as_deref())?;
    let currency = query.currency.as_deref().unwrap_or("USD");
    let balance = credits::get_balance(&state.db, &customer_id, currency).await?;
    let history = credits::list_credits(&state.db, &customer_id, query.currency.as_deref()).await?;

    Ok(Json(serde_json::json!({
        "balance": balance,
        "currency": currency,
        "history": history,
    })))
}

fn scoped_customer_id(api_key: &ApiKeyInfo, requested: Option<&str>) -> ApiResult<String> {
    let scoped = api_key
        .customer_id
        .as_deref()
        .ok_or(BillingError::Forbidden)?;

    if let Some(requested) = requested {
        if requested != scoped {
            return Err(BillingError::Forbidden.into());
        }
    }

    Ok(scoped.to_string())
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
    let result = rustbill_core::billing::plan_change::change_plan_with_proration(
        &state.db,
        rustbill_core::billing::plan_change::ChangePlanInput {
            subscription_id: &id,
            new_plan_id: &body.plan_id,
            new_quantity: body.quantity,
            idempotency_key: body.idempotency_key.as_deref(),
            now: chrono::Utc::now().naive_utc(),
        },
    )
    .await
    .map_err(BillingError::from)?;

    if !result.already_processed {
        let _ = rustbill_core::notifications::events::emit_billing_event(
            &state.db,
            &state.http_client,
            BillingEventType::SubscriptionPlanChanged,
            "subscription",
            &id,
            Some(&result.customer_id),
            Some(serde_json::json!({
                "old_plan": result.old_plan_name,
                "new_plan": result.new_plan_name,
                "proration_net": result.proration_net.to_string(),
            })),
        )
        .await;
    }

    let payload = if result.already_processed {
        serde_json::to_value(result.invoice)
    } else {
        serde_json::to_value(result.subscription)
    }
    .map_err(|e| BillingError::Internal(e.into()))?;

    Ok(Json(payload))
}
