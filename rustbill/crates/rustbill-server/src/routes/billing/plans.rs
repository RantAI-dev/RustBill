use axum::{extract::{Path, State}, http::StatusCode, routing::{delete, get, post, put}, Json, Router};
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(p) FROM billing_plans p ORDER BY p.created_at DESC",
    )
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
        "SELECT to_jsonb(p) FROM billing_plans p WHERE p.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "plan".into(),
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
        r#"INSERT INTO billing_plans (id, product_id, name, slug, interval, interval_count, amount, currency, trial_days, metadata, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, $8, $9, now(), now())
           RETURNING to_jsonb(billing_plans)"#,
    )
    .bind(body["productId"].as_str())
    .bind(body["name"].as_str())
    .bind(body["slug"].as_str())
    .bind(body["interval"].as_str())
    .bind(body["intervalCount"].as_i64().unwrap_or(1) as i32)
    .bind(body["amount"].as_i64().unwrap_or(0))
    .bind(body["currency"].as_str().unwrap_or("USD"))
    .bind(body["trialDays"].as_i64().map(|v| v as i32))
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
        r#"UPDATE billing_plans SET
             name = COALESCE($2, name),
             amount = COALESCE($3, amount),
             metadata = COALESCE($4, metadata),
             updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(billing_plans)"#,
    )
    .bind(&id)
    .bind(body["name"].as_str())
    .bind(body["amount"].as_i64())
    .bind(body.get("metadata"))
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "plan".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM billing_plans WHERE id = $1")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "plan".into(),
            id,
        }.into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
