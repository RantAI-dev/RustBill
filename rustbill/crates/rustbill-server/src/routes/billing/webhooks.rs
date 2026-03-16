use axum::{extract::{Path, State}, http::StatusCode, routing::{delete, get, post, put}, Json, Router};
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
        .route("/{id}/test", post(test_webhook))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(w) FROM webhook_endpoints w ORDER BY w.created_at DESC",
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
        "SELECT to_jsonb(w) FROM webhook_endpoints w WHERE w.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "webhook_endpoint".into(),
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
        r#"INSERT INTO webhook_endpoints (id, url, events, secret, enabled, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, true, now(), now())
           RETURNING to_jsonb(webhook_endpoints)"#,
    )
    .bind(body["url"].as_str())
    .bind(body.get("events").unwrap_or(&serde_json::json!(["*"])))
    .bind(body["secret"].as_str())
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
        r#"UPDATE webhook_endpoints SET
             url = COALESCE($2, url),
             events = COALESCE($3, events),
             enabled = COALESCE($4, enabled),
             updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(webhook_endpoints)"#,
    )
    .bind(&id)
    .bind(body["url"].as_str())
    .bind(body.get("events"))
    .bind(body["enabled"].as_bool())
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "webhook_endpoint".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM webhook_endpoints WHERE id = $1")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "webhook_endpoint".into(),
            id,
        }.into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn test_webhook(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let endpoint = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(w) FROM webhook_endpoints w WHERE w.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "webhook_endpoint".into(),
        id: id.clone(),
    })?;

    let url = endpoint["url"].as_str().unwrap_or_default();

    // Send a test event
    let test_payload = serde_json::json!({
        "type": "test.webhook",
        "data": { "message": "This is a test webhook delivery" },
    });

    let resp = state.http_client
        .post(url)
        .json(&test_payload)
        .send()
        .await;

    match resp {
        Ok(r) => Ok(Json(serde_json::json!({
            "success": true,
            "statusCode": r.status().as_u16(),
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "success": false,
            "error": e.to_string(),
        }))),
    }
}
