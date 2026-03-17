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
        "SELECT to_jsonb(c) FROM coupons c WHERE c.deleted_at IS NULL ORDER BY c.created_at DESC",
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
        "SELECT to_jsonb(c) FROM coupons c WHERE c.id = $1 AND c.deleted_at IS NULL",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "coupon".into(),
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
        r#"INSERT INTO coupons (id, code, name, discount_type, discount_value, currency, max_redemptions, times_redeemed, valid_from, valid_until, active, applies_to, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, COALESCE($5, 'USD'), $6, 0, COALESCE($7::timestamp, now()), $8::timestamp, COALESCE($9, true), $10, now(), now())
           RETURNING to_jsonb(coupons)"#,
    )
    .bind(body["code"].as_str())
    .bind(body["name"].as_str().unwrap_or_else(|| body["code"].as_str().unwrap_or("Untitled")))
    .bind(body["discountType"].as_str())
    .bind(body["discountValue"].as_f64().unwrap_or(0.0))
    .bind(body["currency"].as_str())
    .bind(body["maxRedemptions"].as_i64().map(|v| v as i32))
    .bind(body["validFrom"].as_str())
    .bind(body["validUntil"].as_str())
    .bind(body["active"].as_bool())
    .bind(body.get("appliesTo"))
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
        r#"UPDATE coupons SET
             code = COALESCE($2, code),
             name = COALESCE($3, name),
             discount_type = COALESCE($4, discount_type),
             discount_value = COALESCE($5, discount_value),
             currency = COALESCE($6, currency),
             max_redemptions = COALESCE($7, max_redemptions),
             valid_until = COALESCE($8::timestamp, valid_until),
             active = COALESCE($9, active),
             applies_to = COALESCE($10, applies_to),
             updated_at = now()
           WHERE id = $1 AND deleted_at IS NULL
           RETURNING to_jsonb(coupons)"#,
    )
    .bind(&id)
    .bind(body["code"].as_str())
    .bind(body["name"].as_str())
    .bind(body["discountType"].as_str())
    .bind(body["discountValue"].as_f64())
    .bind(body["currency"].as_str())
    .bind(body["maxRedemptions"].as_i64().map(|v| v as i32))
    .bind(body["validUntil"].as_str())
    .bind(body["active"].as_bool())
    .bind(body.get("appliesTo"))
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "coupon".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query("UPDATE coupons SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "coupon".into(),
            id,
        }
        .into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
