use axum::{extract::{Path, State}, http::StatusCode, routing::{delete, get, post}, Json, Router};
use crate::app::SharedState;
use crate::extractors::AdminUser;
use super::ApiResult;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", delete(revoke))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(k) - 'hashed_key' FROM api_keys k
           ORDER BY k.created_at DESC"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let name = body["name"].as_str().unwrap_or("default");
    let scopes = body.get("scopes").cloned().unwrap_or(serde_json::json!([]));

    // Generate a random API key
    let key_plain = rustbill_core::auth::generate_api_key();
    let hashed = rustbill_core::auth::hash_api_key(&key_plain);

    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"INSERT INTO api_keys (id, name, prefix, hashed_key, scopes, created_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, now())
           RETURNING to_jsonb(api_keys) - 'hashed_key'"#,
    )
    .bind(name)
    .bind(&key_plain[..12])
    .bind(&hashed)
    .bind(&scopes)
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    // Return the plain key only on creation
    let mut result = row;
    result["key"] = serde_json::Value::String(key_plain);

    Ok((StatusCode::CREATED, Json(result)))
}

async fn revoke(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query(
        "UPDATE api_keys SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL",
    )
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "api_key".into(),
            id,
        }.into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
