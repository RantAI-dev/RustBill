use axum::{extract::{Path, State}, http::StatusCode, routing::{delete, get, post, put}, Json, Router};
use crate::app::SharedState;
use crate::extractors::{AdminUser, SessionUser};
use crate::routes::ApiResult;
use rustbill_core::db::models::UserRole;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/lifecycle", post(lifecycle))
        .route("/{id}", get(get_one).put(update).delete(remove))
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
        r#"INSERT INTO subscriptions (id, customer_id, plan_id, status, current_period_start, current_period_end, metadata, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, 'active', now(), now() + interval '1 month', $3, now(), now())
           RETURNING to_jsonb(subscriptions)"#,
    )
    .bind(body["customerId"].as_str())
    .bind(body["planId"].as_str())
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
    let result = sqlx::query("UPDATE subscriptions SET status = 'cancelled', updated_at = now() WHERE id = $1")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "subscription".into(),
            id,
        }.into());
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
            return Err(rustbill_core::error::BillingError::BadRequest(
                format!("Unknown lifecycle action: {action}"),
            ).into());
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
