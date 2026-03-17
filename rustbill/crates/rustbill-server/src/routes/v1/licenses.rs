use crate::app::SharedState;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

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
           WHERE ($1::text IS NULL OR l.status::text = $1)
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
    let key = body["key"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("LIC-{}", uuid::Uuid::new_v4()));

    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"INSERT INTO licenses (key, customer_id, customer_name, product_id, product_name, status, created_at, expires_at, license_type, features, max_activations)
           VALUES ($1, $2, COALESCE($3, ''), $4, COALESCE($5, ''), 'active', to_char(now(), 'YYYY-MM-DD'), COALESCE($6, ''), COALESCE($7, 'simple'), $8, $9)
           RETURNING to_jsonb(licenses.*)"#,
    )
    .bind(&key)
    .bind(body["customerId"].as_str())
    .bind(body["customerName"].as_str())
    .bind(body["productId"].as_str())
    .bind(body["productName"].as_str())
    .bind(body["expiresAt"].as_str())
    .bind(body["licenseType"].as_str())
    .bind(body.get("features"))
    .bind(body["maxActivations"].as_i64().map(|v| v as i32))
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
             status = COALESCE($2::license_status, status),
             customer_name = COALESCE($3, customer_name),
             product_name = COALESCE($4, product_name),
             max_activations = COALESCE($5, max_activations),
             expires_at = COALESCE($6, expires_at),
             license_type = COALESCE($7, license_type),
             features = COALESCE($8, features)
           WHERE key = $1
           RETURNING to_jsonb(licenses.*)"#,
    )
    .bind(&key)
    .bind(body["status"].as_str())
    .bind(body["customerName"].as_str())
    .bind(body["productName"].as_str())
    .bind(body["maxActivations"].as_i64().map(|v| v as i32))
    .bind(body["expiresAt"].as_str())
    .bind(body["licenseType"].as_str())
    .bind(body.get("features"))
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

    // Check expiry (expires_at is VARCHAR in 'YYYY-MM-DD' format)
    let expired = license["expires_at"].as_str().map_or(false, |exp| {
        chrono::NaiveDate::parse_from_str(exp, "%Y-%m-%d")
            .map_or(false, |d| d < chrono::Utc::now().date_naive())
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
        let license_key = license["key"].as_str().unwrap_or_default();
        let max_activations = license["max_activations"].as_i64().unwrap_or(i64::MAX);

        let activation_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM license_activations WHERE license_key = $1")
                .bind(license_key)
                .fetch_one(&state.db)
                .await
                .map_err(rustbill_core::error::BillingError::from)?;

        // Check if this device is already activated
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM license_activations WHERE license_key = $1 AND device_id = $2",
        )
        .bind(license_key)
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
                r#"INSERT INTO license_activations (id, license_key, device_id, activated_at, last_seen_at)
                   VALUES (gen_random_uuid()::text, $1, $2, now(), now())"#,
            )
            .bind(license_key)
            .bind(device_id)
            .execute(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?;
        } else {
            sqlx::query(
                "UPDATE license_activations SET last_seen_at = now() WHERE license_key = $1 AND device_id = $2",
            )
            .bind(license_key)
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
           WHERE a.license_key = $1
           ORDER BY a.activated_at DESC"#,
    )
    .bind(&key)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows))
}
