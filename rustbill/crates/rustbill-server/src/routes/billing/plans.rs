use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

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
        "SELECT to_jsonb(p) FROM pricing_plans p ORDER BY p.created_at DESC",
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
        "SELECT to_jsonb(p) FROM pricing_plans p WHERE p.id = $1",
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
        r#"INSERT INTO pricing_plans (id, product_id, name, pricing_model, billing_cycle, base_price, unit_price, tiers, usage_metric_name, trial_days, active, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3::pricing_model, $4::billing_cycle, $5, $6, $7, $8, COALESCE($9, 0), COALESCE($10, true), now(), now())
           RETURNING to_jsonb(pricing_plans.*)"#,
    )
    .bind(body["productId"].as_str())
    .bind(body["name"].as_str())
    .bind(body["pricingModel"].as_str().unwrap_or("flat"))
    .bind(body["billingCycle"].as_str().unwrap_or("monthly"))
    .bind(body["basePrice"].as_f64().unwrap_or(0.0))
    .bind(body["unitPrice"].as_f64())
    .bind(body.get("tiers"))
    .bind(body["usageMetricName"].as_str())
    .bind(body["trialDays"].as_i64().map(|v| v as i32))
    .bind(body["active"].as_bool())
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
        r#"UPDATE pricing_plans SET
             name = COALESCE($2, name),
             pricing_model = COALESCE($3::pricing_model, pricing_model),
             billing_cycle = COALESCE($4::billing_cycle, billing_cycle),
             base_price = COALESCE($5, base_price),
             unit_price = COALESCE($6, unit_price),
             tiers = COALESCE($7, tiers),
             usage_metric_name = COALESCE($8, usage_metric_name),
             trial_days = COALESCE($9, trial_days),
             active = COALESCE($10, active),
             updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(pricing_plans.*)"#,
    )
    .bind(&id)
    .bind(body["name"].as_str())
    .bind(body["pricingModel"].as_str())
    .bind(body["billingCycle"].as_str())
    .bind(body["basePrice"].as_f64())
    .bind(body["unitPrice"].as_f64())
    .bind(body.get("tiers"))
    .bind(body["usageMetricName"].as_str())
    .bind(body["trialDays"].as_i64().map(|v| v as i32))
    .bind(body["active"].as_bool())
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
    let result = sqlx::query("DELETE FROM pricing_plans WHERE id = $1")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "plan".into(),
            id,
        }
        .into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
