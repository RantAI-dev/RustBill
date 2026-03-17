use super::ApiResult;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use axum::http::header;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};

/// Public routes — no session required
pub fn public_router() -> Router<SharedState> {
    Router::new().route("/verify", post(verify))
}

/// Admin routes — session required (applied by caller)
pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/keypair", get(get_keypair).post(create_keypair))
        .route("/{key}", put(update).delete(remove))
        .route("/{key}/sign", post(sign))
        .route("/{key}/export", get(export))
        .route(
            "/{key}/activations",
            get(list_activations).delete(deactivate),
        )
}

#[derive(serde::Deserialize)]
struct ListParams {
    status: Option<String>,
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
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

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
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
    _user: AdminUser,
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
    _user: AdminUser,
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
    // If the body contains a "file" field, do offline file verification
    if let Some(file_content) = body["file"].as_str() {
        let result = rustbill_core::licenses::verify_license_file(&state.db, file_content).await?;
        return Ok(Json(result));
    }

    // Otherwise, do online verification by license key
    let key = body["key"].as_str().unwrap_or_default();
    let device_id = body["deviceId"].as_str();

    let license = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(l) FROM licenses l WHERE l.key = $1",
    )
    .bind(key)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "license".into(),
        id: key.to_string(),
    })?;

    let status = license["status"].as_str().unwrap_or("unknown");
    let valid = status == "active";

    Ok(Json(serde_json::json!({
        "valid": valid,
        "license": license,
        "deviceId": device_id,
    })))
}

async fn sign(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(key): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let license = rustbill_core::licenses::sign_license_by_key(&state.db, &key).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "license_key": license.key,
        "signed_payload": license.signed_payload,
        "signature": license.signature,
    })))
}

async fn export(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(key): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let file_content = rustbill_core::licenses::export_license_file(&state.db, &key).await?;

    let filename = format!("license-{}.lic", key);
    let headers = [
        (header::CONTENT_TYPE, "application/octet-stream".to_string()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        ),
    ];

    Ok((headers, file_content))
}

async fn list_activations(
    State(state): State<SharedState>,
    _user: AdminUser,
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeactivateParams {
    device_id: String,
}

async fn deactivate(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(key): Path<String>,
    Query(params): Query<DeactivateParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query(
        r#"DELETE FROM license_activations
           WHERE license_key = $1
             AND device_id = $2"#,
    )
    .bind(&key)
    .bind(&params.device_id)
    .execute(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "activation".into(),
            id: format!("{}/{}", key, params.device_id),
        }
        .into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn get_keypair(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let keypair = rustbill_core::licenses::get_keypair(&state.db).await?;

    match keypair {
        Some((public_pem, _private_pem)) => Ok(Json(serde_json::json!({
            "exists": true,
            "publicKey": public_pem,
        }))),
        None => Ok(Json(serde_json::json!({
            "exists": false,
            "publicKey": null,
            "message": "No keypair found. POST to create one.",
        }))),
    }
}

async fn create_keypair(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let (public_pem, _private_pem) =
        rustbill_core::licenses::generate_keypair_and_store(&state.db).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "publicKey": public_pem,
            "message": "Ed25519 keypair generated and stored.",
        })),
    ))
}
