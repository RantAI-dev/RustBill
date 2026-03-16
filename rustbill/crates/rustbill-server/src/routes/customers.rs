use super::ApiResult;
use crate::app::SharedState;
use crate::extractors::{AdminUser, ValidatedJson};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
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
    let rows = sqlx::query_as::<_, (serde_json::Value,)>(
        "SELECT to_jsonb(c) FROM customers c ORDER BY c.created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows.into_iter().map(|r| r.0).collect()))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(c) FROM customers c WHERE c.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "customer".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let name = body["name"].as_str().unwrap_or_default();
    let email = body["email"].as_str().unwrap_or_default();
    let metadata = body
        .get("metadata")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"INSERT INTO customers (id, name, email, metadata, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, now(), now())
           RETURNING to_jsonb(customers)"#,
    )
    .bind(name)
    .bind(email)
    .bind(&metadata)
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
    let name = body["name"].as_str();
    let email = body["email"].as_str();
    let metadata = body.get("metadata");

    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"UPDATE customers SET
             name = COALESCE($2, name),
             email = COALESCE($3, email),
             metadata = COALESCE($4, metadata),
             updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(customers)"#,
    )
    .bind(&id)
    .bind(name)
    .bind(email)
    .bind(metadata)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "customer".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM customers WHERE id = $1")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "customer".into(),
            id,
        }
        .into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
