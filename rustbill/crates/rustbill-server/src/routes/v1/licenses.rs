use axum::{extract::{Path, Query, State}, http::StatusCode, routing::{get, post}, Json, Router};
use crate::app::SharedState;
use crate::routes::ApiResult;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/verify", post(verify))
        .route("/{key}", get(get_one).put(update).delete(remove))
        .route("/{key}/activations", get(list_activations))
}

#[derive(serde::Deserialize)]
struct ListParams {
    status: Option<String>,
}

async fn list(
    State(state): State<SharedState>,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(l) FROM licenses l
           WHERE ($1::text IS NULL OR l.status = $1)
           ORDER BY l.created_at DESC"#,
    )
    .bind(&params.status)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    Path(key): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(l) FROM licenses l WHERE l.key = $1",
    )
    .bind(&key)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "license".into(),
        id: key.clone(),
    })?;

    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"INSERT INTO licenses (id, key, product_id, customer_id, deal_id, status, max_activations, metadata, expires_at, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, 'active', $5, $6, $7, now(), now())
           RETURNING to_jsonb(licenses)"#,
    )
    .bind(body["key"].as_str())
    .bind(body["productId"].as_str())
    .bind(body["customerId"].as_str())
    .bind(body["dealId"].as_str())
    .bind(body["maxActivations"].as_i64().map(|v| v as i32))
    .bind(body.get("metadata").unwrap_or(&serde_json::json!({})))
    .bind(body["expiresAt"].as_str())
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    Path(key): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"UPDATE licenses SET
             status = COALESCE($2, status),
             max_activations = COALESCE($3, max_activations),
             metadata = COALESCE($4, metadata),
             expires_at = COALESCE($5::timestamptz, expires_at),
             updated_at = now()
           WHERE key = $1
           RETURNING to_jsonb(licenses)"#,
    )
    .bind(&key)
    .bind(body["status"].as_str())
    .bind(body["maxActivations"].as_i64().map(|v| v as i32))
    .bind(body.get("metadata"))
    .bind(body["expiresAt"].as_str())
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "license".into(),
        id: key.clone(),
    })?;

    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    Path(key): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM licenses WHERE key = $1")
        .bind(&key)
        .execute(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "license".into(),
            id: key,
        }
        .into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn verify(
    State(state): State<SharedState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let key = body["key"].as_str().unwrap_or_default();
    let device_id = body["deviceId"].as_str();
    let product_id = body["productId"].as_str();

    let license = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(l) FROM licenses l
           WHERE l.key = $1
             AND ($2::text IS NULL OR l.product_id = $2)"#,
    )
    .bind(key)
    .bind(product_id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let Some(license) = license else {
        return Ok(Json(serde_json::json!({
            "valid": false,
            "error": "license_not_found",
        })));
    };

    let status = license["status"].as_str().unwrap_or("unknown");
    let valid = status == "active";

    // Check expiry
    let expired = license["expires_at"].as_str().map_or(false, |exp| {
        chrono::DateTime::parse_from_rfc3339(exp)
            .map_or(false, |dt| dt < chrono::Utc::now())
    });

    if expired {
        return Ok(Json(serde_json::json!({
            "valid": false,
            "error": "license_expired",
            "license": license,
        })));
    }

    // If device_id provided, record or verify activation
    if let Some(device_id) = device_id {
        let max_activations = license["max_activations"].as_i64().unwrap_or(i64::MAX);

        let activation_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM license_activations WHERE license_id = $1",
        )
        .bind(license["id"].as_str().unwrap_or_default())
        .fetch_one(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

        // Check if this device is already activated
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM license_activations WHERE license_id = $1 AND device_id = $2",
        )
        .bind(license["id"].as_str().unwrap_or_default())
        .bind(device_id)
        .fetch_optional(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

        if existing.is_none() && activation_count.0 >= max_activations {
            return Ok(Json(serde_json::json!({
                "valid": false,
                "error": "max_activations_reached",
                "currentActivations": activation_count.0,
                "maxActivations": max_activations,
            })));
        }

        if existing.is_none() {
            sqlx::query(
                r#"INSERT INTO license_activations (id, license_id, device_id, activated_at)
                   VALUES (gen_random_uuid()::text, $1, $2, now())"#,
            )
            .bind(license["id"].as_str().unwrap_or_default())
            .bind(device_id)
            .execute(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?;
        }
    }

    Ok(Json(serde_json::json!({
        "valid": valid,
        "license": {
            "key": license["key"],
            "status": license["status"],
            "expiresAt": license["expires_at"],
            "productId": license["product_id"],
        },
    })))
}

async fn list_activations(
    State(state): State<SharedState>,
    Path(key): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(a) FROM license_activations a
           JOIN licenses l ON l.id = a.license_id
           WHERE l.key = $1
           ORDER BY a.activated_at DESC"#,
    )
    .bind(&key)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows))
}
