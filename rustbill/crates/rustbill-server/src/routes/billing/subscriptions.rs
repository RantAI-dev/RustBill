use crate::app::SharedState;
use crate::extractors::{AdminUser, SessionUser};
use crate::routes::ApiResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use rustbill_core::db::models::{PricingPlan, Subscription, UserRole};
use rustbill_core::error::BillingError;
use serde::Deserialize;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/lifecycle", post(lifecycle))
        .route("/{id}", get(get_one).put(update).delete(remove))
        .route("/{id}/change-plan", post(change_plan))
}

async fn list(
    State(state): State<SharedState>,
    user: SessionUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let role_customer_id = if user.0.role == UserRole::Customer {
        user.0.customer_id.clone()
    } else {
        None
    };

    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(s) FROM subscriptions s
           WHERE ($1::text IS NULL OR s.customer_id = $1)
           ORDER BY s.created_at DESC"#,
    )
    .bind(&role_customer_id)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
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

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
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

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
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

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query(
        "UPDATE subscriptions SET status = 'canceled', updated_at = now() WHERE id = $1",
    )
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "subscription".into(),
            id,
        }
        .into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Handle subscription lifecycle events (pause, resume, cancel, renew).
async fn lifecycle(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let subscription_id = body["subscriptionId"].as_str().unwrap_or_default();
    let action = body["action"].as_str().unwrap_or_default();

    let new_status = match action {
        "pause" => "paused",
        "resume" => "active",
        "cancel" => "cancelled",
        "renew" => "active",
        _ => {
            return Err(rustbill_core::error::BillingError::BadRequest(format!(
                "Unknown lifecycle action: {action}"
            ))
            .into());
        }
    };

    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"UPDATE subscriptions SET status = $2, updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(subscriptions)"#,
    )
    .bind(subscription_id)
    .bind(new_status)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "subscription".into(),
        id: subscription_id.to_string(),
    })?;

    Ok(Json(row))
}

// ---------------------------------------------------------------------------
// Plan Change with Proration
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangePlanRequest {
    plan_id: String,
    #[serde(default)]
    quantity: Option<i32>,
    idempotency_key: Option<String>,
}

async fn change_plan(
    State(state): State<SharedState>,
    _user: AdminUser,
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
    // For proration.net > 0 (upgrade), invoice creation will be added later
    // as it requires the full invoice number sequence.

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
