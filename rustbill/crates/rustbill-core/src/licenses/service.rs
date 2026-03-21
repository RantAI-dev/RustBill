use super::repository::LicensesRepository;
use super::schema::{
    ActivationOutcome, CreateLicenseDraft, CreateLicenseRequest, UpdateLicenseDraft,
    UpdateLicenseRequest, VerifyLicenseRequest,
};
use crate::db::models::{License, LicenseActivation, LicenseStatus};
use crate::error::{BillingError, Result};
use crate::licenses::signing::{self, LicensePayload, SignedLicense};
use chrono::Utc;
/// List all licenses, each enriched with an `activation_count` field.

pub async fn list_licenses_with_repo<R: LicensesRepository + ?Sized>(
    repo: &R,
) -> Result<Vec<serde_json::Value>> {
    let rows = repo.list_licenses().await?;

    let mut results = Vec::with_capacity(rows.len());
    for lic in rows {
        let count = repo.activation_count(&lic.key).await?;

        let mut val = serde_json::to_value(&lic).map_err(|e| {
            BillingError::Internal(anyhow::anyhow!(
                "failed to serialize license {}: {e}",
                lic.key
            ))
        })?;
        let obj = val.as_object_mut().ok_or_else(|| {
            BillingError::Internal(anyhow::anyhow!(
                "license {} did not serialize to an object",
                lic.key
            ))
        })?;
        obj.insert("activation_count".to_string(), serde_json::json!(count));
        results.push(val);
    }

    Ok(results)
}

/// Get a single license by key.

pub async fn get_license_with_repo<R: LicensesRepository + ?Sized>(
    repo: &R,
    key: &str,
) -> Result<License> {
    repo.get_license(key)
        .await?
        .ok_or_else(|| BillingError::not_found("license", key))
}

/// Create a new license. Auto-populates `customer_name` / `product_name` from FKs when the
/// corresponding ID is provided.

pub async fn create_license_with_repo<R: LicensesRepository + ?Sized>(
    repo: &R,
    req: CreateLicenseRequest,
) -> Result<License> {
    let customer_name = if let Some(ref cid) = req.customer_id {
        let name = repo.customer_name(cid).await?;
        name.or(req.customer_name.clone()).unwrap_or_default()
    } else {
        req.customer_name.clone().unwrap_or_default()
    };

    let product_name = if let Some(ref pid) = req.product_id {
        let name = repo.product_name(pid).await?;
        name.or(req.product_name.clone()).unwrap_or_default()
    } else {
        req.product_name.clone().unwrap_or_default()
    };

    let license_type = req.license_type.unwrap_or_else(|| "simple".to_string());
    let status = req.status.unwrap_or(LicenseStatus::Active);
    let created_at = Utc::now().format("%Y-%m-%d").to_string();
    let expires_at = req.expires_at.unwrap_or_default();
    let features_json = match req.features.as_ref() {
        Some(features) => serde_json::to_value(features).map_err(|e| {
            BillingError::Internal(anyhow::anyhow!("failed to serialize license features: {e}"))
        })?,
        None => serde_json::json!([]),
    };

    let draft = CreateLicenseDraft {
        customer_id: req.customer_id,
        customer_name,
        product_id: req.product_id,
        product_name,
        status,
        created_at,
        expires_at,
        license_type,
        features: features_json,
        max_activations: req.max_activations,
    };

    repo.create_license(draft).await
}

/// Update an existing license by key.

pub async fn update_license_with_repo<R: LicensesRepository + ?Sized>(
    repo: &R,
    key: &str,
    req: UpdateLicenseRequest,
) -> Result<License> {
    let _existing = get_license_with_repo(repo, key).await?;
    let features_json = match req.features.as_ref() {
        Some(features) => Some(serde_json::to_value(features).map_err(|e| {
            BillingError::Internal(anyhow::anyhow!("failed to serialize license features: {e}"))
        })?),
        None => None,
    };

    let draft = UpdateLicenseDraft {
        customer_id: req.customer_id,
        customer_name: req.customer_name,
        product_id: req.product_id,
        product_name: req.product_name,
        status: req.status,
        expires_at: req.expires_at,
        license_type: req.license_type,
        features: features_json,
        max_activations: req.max_activations,
    };

    repo.update_license(key, draft).await
}

/// Delete a license by key.

pub async fn delete_license_with_repo<R: LicensesRepository + ?Sized>(
    repo: &R,
    key: &str,
) -> Result<()> {
    let result = repo.delete_license(key).await?;
    if result == 0 {
        return Err(BillingError::not_found("license", key));
    }
    Ok(())
}

/// List all activations for a given license key.

pub async fn list_activations_with_repo<R: LicensesRepository + ?Sized>(
    repo: &R,
    license_key: &str,
) -> Result<Vec<LicenseActivation>> {
    let _ = get_license_with_repo(repo, license_key).await?;
    repo.list_activations(license_key).await
}

/// Deactivate (remove) a device activation for a license.

pub async fn deactivate_device_with_repo<R: LicensesRepository + ?Sized>(
    repo: &R,
    license_key: &str,
    device_id: &str,
) -> Result<()> {
    let result = repo.deactivate_device(license_key, device_id).await?;
    if result == 0 {
        return Err(BillingError::not_found(
            "activation",
            format!("{license_key}/{device_id}"),
        ));
    }
    Ok(())
}

/// Retrieve the stored ED25519 keypair from system_settings.
/// Returns `(public_key_pem, private_key_pem)` or `None` if not yet generated.

/// Generate a new ED25519 keypair and store it in system_settings, replacing any existing one.

pub async fn generate_keypair_and_store_with_repo<R: LicensesRepository + ?Sized>(
    repo: &R,
) -> Result<(String, String)> {
    let (public_pem, private_pem) = signing::generate_keypair()?;
    repo.store_public_key(&public_pem).await?;
    repo.store_private_key(&private_pem).await?;

    Ok((public_pem, private_pem))
}

/// Online license verification. Looks up the license by key, checks status and expiry,
/// and optionally upserts a device activation (respecting `max_activations` within a
/// transaction).
///
/// Returns a JSON value describing the verification result.

pub async fn verify_license_online_with_repo<R: LicensesRepository + ?Sized>(
    repo: &R,
    req: VerifyLicenseRequest,
) -> Result<serde_json::Value> {
    let license = get_license_with_repo(repo, &req.license_key).await?;

    if license.status != LicenseStatus::Active {
        return Ok(serde_json::json!({
            "valid": false,
            "license_key": license.key,
            "reason": format!("license status is {:?}", license.status),
        }));
    }

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

    if let Some(ref device_id) = req.device_id {
        match repo
            .record_activation(
                &req.license_key,
                device_id,
                req.device_name.as_deref(),
                req.ip_address.as_deref(),
                license.max_activations,
            )
            .await?
        {
            ActivationOutcome::LimitReached => {
                return Ok(serde_json::json!({
                    "valid": false,
                    "license_key": license.key,
                    "reason": format!("maximum activations ({}) reached", license.max_activations.unwrap_or(0)),
                }));
            }
            ActivationOutcome::Inserted | ActivationOutcome::Updated => {}
        }
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

pub async fn sign_license_by_key_with_repo<R: LicensesRepository + ?Sized>(
    repo: &R,
    key: &str,
) -> Result<License> {
    let license = get_license_with_repo(repo, key).await?;

    let keypair = repo.get_keypair().await?;
    let (_public_pem, private_pem) = keypair.ok_or_else(|| {
        BillingError::bad_request(
            "no signing keypair exists — generate one first via POST /api/licenses/keypair",
        )
    })?;

    let payload = build_payload(&license);
    let signed = signing::sign_license(&payload, &private_pem)?;

    let payload_json = serde_json::to_string(&signed.payload)
        .map_err(|e| BillingError::Internal(anyhow::anyhow!("failed to serialize payload: {e}")))?;

    repo.update_signed_license(key, &payload_json, &signed.signature)
        .await
}

/// Verify an offline license file. Accepts the raw license file content (PEM-style format).
/// Returns a JSON result indicating validity.

pub async fn verify_license_file_with_repo<R: LicensesRepository + ?Sized>(
    repo: &R,
    file_content: &str,
) -> Result<serde_json::Value> {
    let keypair = repo.get_keypair().await?;
    let (public_pem, _private_pem) = keypair
        .ok_or_else(|| BillingError::bad_request("no signing keypair exists — cannot verify"))?;

    let signed = signing::parse_license_file(file_content)?;
    let sig_valid = signing::verify_license(&signed, &public_pem)?;

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

pub async fn export_license_file_with_repo<R: LicensesRepository + ?Sized>(
    repo: &R,
    key: &str,
) -> Result<String> {
    let license = get_license_with_repo(repo, key).await?;

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
