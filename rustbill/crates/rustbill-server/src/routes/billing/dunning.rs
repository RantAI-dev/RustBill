use axum::{extract::{Path, State}, http::StatusCode, routing::{get, post, put}, Json, Router};
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(d) FROM dunning_campaigns d ORDER BY d.created_at DESC",
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
        "SELECT to_jsonb(d) FROM dunning_campaigns d WHERE d.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "dunning_campaign".into(),
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
        r#"INSERT INTO dunning_campaigns (id, name, steps, enabled, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, now(), now())
           RETURNING to_jsonb(dunning_campaigns)"#,
    )
    .bind(body["name"].as_str())
    .bind(body.get("steps").unwrap_or(&serde_json::json!([])))
    .bind(body["enabled"].as_bool().unwrap_or(true))
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
        r#"UPDATE dunning_campaigns SET
             name = COALESCE($2, name),
             steps = COALESCE($3, steps),
             enabled = COALESCE($4, enabled),
             updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(dunning_campaigns)"#,
    )
    .bind(&id)
    .bind(body["name"].as_str())
    .bind(body.get("steps"))
    .bind(body["enabled"].as_bool())
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "dunning_campaign".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}
