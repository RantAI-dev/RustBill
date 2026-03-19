use super::ApiResult;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
    Json, Router,
};

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
        r#"SELECT to_jsonb(k) - 'key_hash' FROM api_keys k
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
    let customer_id = body["customerId"].as_str();

    // Generate a random API key
    let key_plain = rustbill_core::auth::generate_api_key();
    let hashed = rustbill_core::auth::hash_api_key(&key_plain);

    let row = if let Some(customer_id) = customer_id {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO api_keys (id, name, customer_id, key_prefix, key_hash, created_at)
               VALUES (gen_random_uuid()::text, $1, $2, $3, $4, now())
               RETURNING to_jsonb(api_keys.*) - 'key_hash'"#,
        )
        .bind(name)
        .bind(customer_id)
        .bind(&key_plain[..12])
        .bind(&hashed)
        .fetch_one(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?
    } else {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO api_keys (id, name, key_prefix, key_hash, created_at)
               VALUES (gen_random_uuid()::text, $1, $2, $3, now())
               RETURNING to_jsonb(api_keys.*) - 'key_hash'"#,
        )
        .bind(name)
        .bind(&key_plain[..12])
        .bind(&hashed)
        .fetch_one(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?
    };

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
    let result =
        sqlx::query("UPDATE api_keys SET status = 'revoked' WHERE id = $1 AND status = 'active'")
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "api_key".into(),
            id,
        }
        .into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
