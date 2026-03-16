use crate::db::models::{License, LicenseActivation, LicenseStatus};
use crate::error::{BillingError, Result};
use crate::licenses::signing::{self, LicensePayload, SignedLicense};
use crate::licenses::validation::{
    CreateLicenseRequest, UpdateLicenseRequest, VerifyLicenseRequest,
};
use chrono::Utc;
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// License CRUD
// ---------------------------------------------------------------------------

/// List all licenses, each enriched with an `activation_count` field.
pub async fn list_licenses(pool: &PgPool) -> Result<Vec<serde_json::Value>> {
    let rows = sqlx::query_as::<_, License>("SELECT * FROM licenses ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;

    let mut results = Vec::with_capacity(rows.len());
    for lic in rows {
        let count: Option<i64> =
            sqlx::query_scalar("SELECT COUNT(*) FROM license_activations WHERE license_key = $1")
                .bind(&lic.key)
                .fetch_one(pool)
                .await?;

        let mut val = serde_json::to_value(&lic).unwrap();
        let obj = val.as_object_mut().unwrap();
        obj.insert(
            "activation_count".to_string(),
            serde_json::json!(count.unwrap_or(0)),
        );
        results.push(val);
    }

    Ok(results)
}

/// Get a single license by key.
pub async fn get_license(pool: &PgPool, key: &str) -> Result<License> {
    sqlx::query_as::<_, License>("SELECT * FROM licenses WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| BillingError::not_found("license", key))
}

/// Create a new license. Auto-populates `customer_name` / `product_name` from FKs when the
/// corresponding ID is provided.
pub async fn create_license(pool: &PgPool, req: CreateLicenseRequest) -> Result<License> {
    // Resolve customer name from FK if customer_id is provided.
    let customer_name = if let Some(ref cid) = req.customer_id {
        let name: Option<String> = sqlx::query_scalar("SELECT name FROM customers WHERE id = $1")
            .bind(cid)
            .fetch_optional(pool)
            .await?;
        name.or(req.customer_name.clone()).unwrap_or_default()
    } else {
        req.customer_name.clone().unwrap_or_default()
    };

    // Resolve product name from FK if product_id is provided.
    let product_name = if let Some(ref pid) = req.product_id {
        let name: Option<String> = sqlx::query_scalar("SELECT name FROM products WHERE id = $1")
            .bind(pid)
            .fetch_optional(pool)
            .await?;
        name.or(req.product_name.clone()).unwrap_or_default()
    } else {
        req.product_name.clone().unwrap_or_default()
    };

    let license_type = req.license_type.unwrap_or_else(|| "simple".to_string());
    let status = req.status.unwrap_or(LicenseStatus::Active);
    let now = Utc::now().format("%Y-%m-%d").to_string();
    let expires = req.expires_at.unwrap_or_default();
    let features_json: serde_json::Value = req
        .features
        .as_ref()
        .map(|f| serde_json::to_value(f).unwrap())
        .unwrap_or(serde_json::json!([]));

    let row = sqlx::query_as::<_, License>(
        r#"
        INSERT INTO licenses (
            key, customer_id, customer_name, product_id, product_name,
            status, created_at, expires_at, license_type,
            features, max_activations
        )
        VALUES (
            gen_random_uuid()::text, $1, $2, $3, $4,
            $5, $6, $7, $8,
            $9, $10
        )
        RETURNING *
        "#,
    )
    .bind(&req.customer_id)
    .bind(&customer_name)
    .bind(&req.product_id)
    .bind(&product_name)
    .bind(&status)
    .bind(&now)
    .bind(&expires)
    .bind(&license_type)
    .bind(&features_json)
    .bind(req.max_activations)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Update an existing license by key.
pub async fn update_license(
    pool: &PgPool,
    key: &str,
    req: UpdateLicenseRequest,
) -> Result<License> {
    // Ensure it exists first.
    let _existing = get_license(pool, key).await?;

    let features_json: Option<serde_json::Value> = req
        .features
        .as_ref()
        .map(|f| serde_json::to_value(f).unwrap());

    let row = sqlx::query_as::<_, License>(
        r#"
        UPDATE licenses SET
            customer_id = COALESCE($2, customer_id),
            customer_name = COALESCE($3, customer_name),
            product_id = COALESCE($4, product_id),
            product_name = COALESCE($5, product_name),
            status = COALESCE($6, status),
            expires_at = COALESCE($7, expires_at),
            license_type = COALESCE($8, license_type),
            features = COALESCE($9, features),
            max_activations = COALESCE($10, max_activations)
        WHERE key = $1
        RETURNING *
        "#,
    )
    .bind(key)
    .bind(&req.customer_id)
    .bind(&req.customer_name)
    .bind(&req.product_id)
    .bind(&req.product_name)
    .bind(&req.status)
    .bind(&req.expires_at)
    .bind(&req.license_type)
    .bind(&features_json)
    .bind(req.max_activations)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Delete a license by key.
pub async fn delete_license(pool: &PgPool, key: &str) -> Result<()> {
    let result = sqlx::query("DELETE FROM licenses WHERE key = $1")
        .bind(key)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(BillingError::not_found("license", key));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Activations
// ---------------------------------------------------------------------------

/// List all activations for a given license key.
pub async fn list_activations(pool: &PgPool, license_key: &str) -> Result<Vec<LicenseActivation>> {
    // Ensure the license exists.
    let _lic = get_license(pool, license_key).await?;

    let rows = sqlx::query_as::<_, LicenseActivation>(
        "SELECT * FROM license_activations WHERE license_key = $1 ORDER BY activated_at DESC",
    )
    .bind(license_key)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Deactivate (remove) a device activation for a license.
pub async fn deactivate_device(pool: &PgPool, license_key: &str, device_id: &str) -> Result<()> {
    let result =
        sqlx::query("DELETE FROM license_activations WHERE license_key = $1 AND device_id = $2")
            .bind(license_key)
            .bind(device_id)
            .execute(pool)
            .await?;

    if result.rows_affected() == 0 {
        return Err(BillingError::not_found(
            "activation",
            format!("{license_key}/{device_id}"),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Keypair management (stored in system_settings)
// ---------------------------------------------------------------------------

/// Retrieve the stored ED25519 keypair from system_settings.
/// Returns `(public_key_pem, private_key_pem)` or `None` if not yet generated.
pub async fn get_keypair(pool: &PgPool) -> Result<Option<(String, String)>> {
    let pub_row: Option<String> =
        sqlx::query_scalar("SELECT value FROM system_settings WHERE key = 'license_public_key'")
            .fetch_optional(pool)
            .await?;

    let priv_row: Option<String> =
        sqlx::query_scalar("SELECT value FROM system_settings WHERE key = 'license_private_key'")
            .fetch_optional(pool)
            .await?;

    match (pub_row, priv_row) {
        (Some(pub_pem), Some(priv_pem)) => Ok(Some((pub_pem, priv_pem))),
        _ => Ok(None),
    }
}

/// Generate a new ED25519 keypair and store it in system_settings, replacing any existing one.
pub async fn generate_keypair_and_store(pool: &PgPool) -> Result<(String, String)> {
    let (public_pem, private_pem) = crate::licenses::signing::generate_keypair()?;

    sqlx::query(
        r#"
        INSERT INTO system_settings (key, value, sensitive, updated_at)
        VALUES ('license_public_key', $1, false, NOW())
        ON CONFLICT (key) DO UPDATE SET value = $1, updated_at = NOW()
        "#,
    )
    .bind(&public_pem)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO system_settings (key, value, sensitive, updated_at)
        VALUES ('license_private_key', $1, true, NOW())
        ON CONFLICT (key) DO UPDATE SET value = $1, updated_at = NOW()
        "#,
    )
    .bind(&private_pem)
    .execute(pool)
    .await?;

    Ok((public_pem, private_pem))
}

// ---------------------------------------------------------------------------
// Online verification
// ---------------------------------------------------------------------------

/// Online license verification. Looks up the license by key, checks status and expiry,
/// and optionally upserts a device activation (respecting `max_activations` within a
/// transaction).
///
/// Returns a JSON value describing the verification result.
pub async fn verify_license_online(
    pool: &PgPool,
    req: VerifyLicenseRequest,
) -> Result<serde_json::Value> {
    let license = get_license(pool, &req.license_key).await?;

    // Check status
    if license.status != LicenseStatus::Active {
        return Ok(serde_json::json!({
            "valid": false,
            "license_key": license.key,
            "reason": format!("license status is {:?}", license.status),
        }));
    }

    // Check expiry
    if !license.expires_at.is_empty() {
        if let Ok(exp) = chrono::NaiveDate::parse_from_str(&license.expires_at, "%Y-%m-%d") {
            let today = Utc::now().date_naive();
            if today > exp {
                return Ok(serde_json::json!({
                    "valid": false,
                    "license_key": license.key,
                    "reason": "license has expired",
                }));
            }
        }
    }

    // If device_id is provided, upsert an activation within a transaction.
    if let Some(ref device_id) = req.device_id {
        let mut tx = pool.begin().await?;

        // Check if this device is already activated for this license.
        let existing: Option<LicenseActivation> = sqlx::query_as(
            "SELECT * FROM license_activations WHERE license_key = $1 AND device_id = $2",
        )
        .bind(&req.license_key)
        .bind(device_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(_activation) = existing {
            // Update last_seen_at
            sqlx::query(
                "UPDATE license_activations SET last_seen_at = NOW() WHERE license_key = $1 AND device_id = $2",
            )
            .bind(&req.license_key)
            .bind(device_id)
            .execute(&mut *tx)
            .await?;
        } else {
            // Check max_activations limit
            if let Some(max) = license.max_activations {
                let current_count: Option<i64> = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM license_activations WHERE license_key = $1",
                )
                .bind(&req.license_key)
                .fetch_one(&mut *tx)
                .await?;

                if current_count.unwrap_or(0) >= max as i64 {
                    tx.rollback().await?;
                    return Ok(serde_json::json!({
                        "valid": false,
                        "license_key": license.key,
                        "reason": format!("maximum activations ({max}) reached"),
                    }));
                }
            }

            // Insert new activation
            sqlx::query(
                r#"
                INSERT INTO license_activations (id, license_key, device_id, device_name, ip_address, activated_at, last_seen_at)
                VALUES (gen_random_uuid()::text, $1, $2, $3, $4, NOW(), NOW())
                "#,
            )
            .bind(&req.license_key)
            .bind(device_id)
            .bind(&req.device_name)
            .bind(&req.ip_address)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
    }

    Ok(serde_json::json!({
        "valid": true,
        "license_key": license.key,
        "customer_name": license.customer_name,
        "product_name": license.product_name,
        "status": license.status,
        "expires_at": license.expires_at,
        "features": license.features,
    }))
}

// ---------------------------------------------------------------------------
// License signing (offline)
// ---------------------------------------------------------------------------

/// Build a LicensePayload from a License record.
fn build_payload(license: &License) -> LicensePayload {
    let features: Vec<String> = license
        .features
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    LicensePayload {
        license_id: license.key.clone(),
        customer_id: license.customer_id.clone(),
        customer_name: license.customer_name.clone(),
        product_id: license.product_id.clone(),
        product_name: license.product_name.clone(),
        features,
        max_activations: license.max_activations,
        issued_at: license.created_at.clone(),
        expires_at: license.expires_at.clone(),
        metadata: None,
    }
}

/// Sign a license by key. Fetches the license, builds the payload, signs it with the
/// stored Ed25519 private key, and persists `signed_payload` + `signature` on the license row.
/// Returns the updated license record.
pub async fn sign_license_by_key(pool: &PgPool, key: &str) -> Result<License> {
    let license = get_license(pool, key).await?;

    let keypair = get_keypair(pool).await?;
    let (_public_pem, private_pem) = keypair.ok_or_else(|| {
        BillingError::bad_request(
            "no signing keypair exists — generate one first via POST /api/licenses/keypair",
        )
    })?;

    let payload = build_payload(&license);
    let signed = signing::sign_license(&payload, &private_pem)?;

    let payload_json = serde_json::to_string(&signed.payload)
        .map_err(|e| BillingError::Internal(anyhow::anyhow!("failed to serialize payload: {e}")))?;

    let row = sqlx::query_as::<_, License>(
        r#"
        UPDATE licenses
        SET signed_payload = $2, signature = $3
        WHERE key = $1
        RETURNING *
        "#,
    )
    .bind(key)
    .bind(&payload_json)
    .bind(&signed.signature)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Verify an offline license file. Accepts the raw license file content (PEM-style format).
/// Returns a JSON result indicating validity.
pub async fn verify_license_file(pool: &PgPool, file_content: &str) -> Result<serde_json::Value> {
    let keypair = get_keypair(pool).await?;
    let (public_pem, _private_pem) = keypair
        .ok_or_else(|| BillingError::bad_request("no signing keypair exists — cannot verify"))?;

    let signed = signing::parse_license_file(file_content)?;
    let sig_valid = signing::verify_license(&signed, &public_pem)?;

    // Check expiry
    let expired = if !signed.payload.expires_at.is_empty() {
        if let Ok(exp) = chrono::NaiveDate::parse_from_str(&signed.payload.expires_at, "%Y-%m-%d") {
            Utc::now().date_naive() > exp
        } else {
            false
        }
    } else {
        false
    };

    let valid = sig_valid && !expired;

    let mut reason = Vec::new();
    if !sig_valid {
        reason.push("invalid signature");
    }
    if expired {
        reason.push("license has expired");
    }

    Ok(serde_json::json!({
        "valid": valid,
        "signature_valid": sig_valid,
        "expired": expired,
        "reason": if reason.is_empty() { None } else { Some(reason.join("; ")) },
        "payload": signed.payload,
    }))
}

/// Export a signed license as a downloadable license file string.
/// The license must have been previously signed.
pub async fn export_license_file(pool: &PgPool, key: &str) -> Result<String> {
    let license = get_license(pool, key).await?;

    let signed_payload = license
        .signed_payload
        .as_ref()
        .ok_or_else(|| BillingError::bad_request("license has not been signed yet"))?;

    let signature = license
        .signature
        .as_ref()
        .ok_or_else(|| BillingError::bad_request("license has not been signed yet"))?;

    let payload: LicensePayload = serde_json::from_str(signed_payload)
        .map_err(|e| BillingError::Internal(anyhow::anyhow!("corrupt signed_payload: {e}")))?;

    let signed = SignedLicense {
        payload,
        signature: signature.clone(),
    };

    Ok(signing::to_license_file(&signed))
}
