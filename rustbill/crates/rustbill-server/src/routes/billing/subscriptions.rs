use crate::app::SharedState;
use crate::extractors::{AdminUser, SessionUser};
use crate::routes::ApiResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use rustbill_core::db::models::{BillingEventType, UserRole};
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
    let metadata = merged_subscription_metadata(&body)?;

    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"INSERT INTO subscriptions (id, customer_id, plan_id, status, current_period_start, current_period_end, quantity, metadata, cancel_at_period_end, version, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, 'active', now(), now() + interval '1 month', COALESCE($3, 1), $4, false, 1, now(), now())
           RETURNING to_jsonb(subscriptions.*)"#,
    )
    .bind(body["customerId"].as_str())
    .bind(body["planId"].as_str())
    .bind(body["quantity"].as_i64().map(|v| v as i32))
    .bind(metadata)
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
    let metadata = merged_subscription_metadata_optional(&body)?;

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
    .bind(metadata)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "subscription".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

fn merged_subscription_metadata(
    body: &serde_json::Value,
) -> Result<serde_json::Value, BillingError> {
    let base = body
        .get("metadata")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    merge_pre_renewal_days(base, body)
}

fn merged_subscription_metadata_optional(
    body: &serde_json::Value,
) -> Result<Option<serde_json::Value>, BillingError> {
    if body.get("metadata").is_none() && body.get("preRenewalInvoiceDays").is_none() {
        return Ok(None);
    }

    let base = body
        .get("metadata")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    Ok(Some(merge_pre_renewal_days(base, body)?))
}

fn merge_pre_renewal_days(
    mut metadata: serde_json::Value,
    body: &serde_json::Value,
) -> Result<serde_json::Value, BillingError> {
    let Some(days) = body.get("preRenewalInvoiceDays").and_then(|v| v.as_i64()) else {
        return Ok(metadata);
    };

    if !(0..=90).contains(&days) {
        return Err(BillingError::BadRequest(
            "preRenewalInvoiceDays must be between 0 and 90".to_string(),
        ));
    }

    if !metadata.is_object() {
        metadata = serde_json::json!({});
    }

    if let Some(obj) = metadata.as_object_mut() {
        obj.insert("preRenewalInvoiceDays".to_string(), serde_json::json!(days));
    }

    Ok(metadata)
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
        "cancel" => "canceled",
        "renew" => "active",
        _ => {
            return Err(rustbill_core::error::BillingError::BadRequest(format!(
                "Unknown lifecycle action: {action}"
            ))
            .into());
        }
    };

    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"UPDATE subscriptions SET status = $2::subscription_status, updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(subscriptions.*)"#,
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
