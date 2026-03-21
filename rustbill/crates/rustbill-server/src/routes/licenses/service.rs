use super::repository::{LicensePatch, LicensesRepository, NewLicenseRecord};
use super::schema::{
    CreateLicenseRequest, KeypairCreateResponse, KeypairStatusResponse, LegacyVerifyResponse,
    SignLicenseResponse, UpdateLicenseRequest, VerifyLicenseRequest,
};
use chrono::Utc;
use rustbill_core::db::models::{License, LicenseActivation, LicenseStatus};
use rustbill_core::error::BillingError;
use rustbill_core::licenses::signing::{self, LicensePayload, SignedLicense};

fn not_found(entity: &'static str, id: impl Into<String>) -> BillingError {
    BillingError::not_found(entity, id)
}

fn build_payload(license: &License) -> LicensePayload {
    let features = match license.features.as_ref() {
        Some(value) => serde_json::from_value(value.clone()).unwrap_or_default(),
        None => Vec::new(),
    };

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

async fn create_license<R: LicensesRepository>(
    repo: &R,
    body: &CreateLicenseRequest,
    use_client_created_at: bool,
) -> Result<License, BillingError> {
    let key = body
        .key
        .clone()
        .unwrap_or_else(|| format!("LIC-{}", uuid::Uuid::new_v4()));

    let record = NewLicenseRecord {
        key,
        customer_id: body.customer_id.clone(),
        customer_name: body.customer_name.clone(),
        product_id: body.product_id.clone(),
        product_name: body.product_name.clone(),
        created_at: if use_client_created_at {
            body.starts_at.clone()
        } else {
            None
        },
        expires_at: body.expires_at.clone(),
        license_type: body.license_type.clone(),
        features: body.features.clone(),
        max_activations: body.max_activations,
    };

    repo.insert_license(&record).await
}

async fn update_license<R: LicensesRepository>(
    repo: &R,
    key: &str,
    body: &UpdateLicenseRequest,
    use_client_created_at: bool,
) -> Result<License, BillingError> {
    let patch = LicensePatch {
        status: body.status.clone(),
        customer_name: body.customer_name.clone(),
        product_name: body.product_name.clone(),
        max_activations: body.max_activations,
        created_at: if use_client_created_at {
            body.starts_at.clone()
        } else {
            None
        },
        expires_at: body.expires_at.clone(),
        license_type: body.license_type.clone(),
        features: body.features.clone(),
    };

    repo.update_license(key, &patch)
        .await?
        .ok_or_else(|| not_found("license", key.to_string()))
}

async fn verify_file<R: LicensesRepository>(
    repo: &R,
    file_content: &str,
) -> Result<serde_json::Value, BillingError> {
    let keypair = repo.get_keypair().await?;
    let (public_pem, _private_pem) = keypair
        .ok_or_else(|| BillingError::bad_request("no signing keypair exists — cannot verify"))?;

    let signed = signing::parse_license_file(file_content)?;
    let sig_valid = signing::verify_license(&signed, &public_pem)?;

    let expired = if signed.payload.expires_at.is_empty() {
        false
    } else {
        match chrono::NaiveDate::parse_from_str(&signed.payload.expires_at, "%Y-%m-%d") {
            Ok(exp) => Utc::now().date_naive() > exp,
            Err(_) => false,
        }
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
        "reason": if reason.is_empty() { None::<String> } else { Some(reason.join("; ")) },
        "payload": signed.payload,
    }))
}

pub async fn list<R: LicensesRepository>(
    repo: &R,
    status: Option<&str>,
) -> Result<Vec<License>, BillingError> {
    repo.list_licenses(status).await
}

pub async fn get_one<R: LicensesRepository>(repo: &R, key: &str) -> Result<License, BillingError> {
    repo.get_license(key)
        .await?
        .ok_or_else(|| not_found("license", key.to_string()))
}

pub async fn create_legacy<R: LicensesRepository>(
    repo: &R,
    body: &CreateLicenseRequest,
) -> Result<License, BillingError> {
    create_license(repo, body, true).await
}

pub async fn create_v1<R: LicensesRepository>(
    repo: &R,
    body: &CreateLicenseRequest,
) -> Result<License, BillingError> {
    create_license(repo, body, false).await
}

pub async fn update_legacy<R: LicensesRepository>(
    repo: &R,
    key: &str,
    body: &UpdateLicenseRequest,
) -> Result<License, BillingError> {
    update_license(repo, key, body, true).await
}

pub async fn update_v1<R: LicensesRepository>(
    repo: &R,
    key: &str,
    body: &UpdateLicenseRequest,
) -> Result<License, BillingError> {
    update_license(repo, key, body, false).await
}

pub async fn remove_license<R: LicensesRepository>(
    repo: &R,
    key: &str,
) -> Result<serde_json::Value, BillingError> {
    let rows = repo.delete_license(key).await?;
    if rows == 0 {
        return Err(not_found("license", key.to_string()));
    }

    Ok(serde_json::json!({ "success": true }))
}

pub async fn list_activations<R: LicensesRepository>(
    repo: &R,
    key: &str,
) -> Result<Vec<LicenseActivation>, BillingError> {
    repo.list_activations(key).await
}

pub async fn deactivate<R: LicensesRepository>(
    repo: &R,
    key: &str,
    device_id: &str,
) -> Result<serde_json::Value, BillingError> {
    let rows = repo.delete_activation(key, device_id).await?;
    if rows == 0 {
        return Err(not_found("activation", format!("{}/{}", key, device_id)));
    }

    Ok(serde_json::json!({ "success": true }))
}

pub async fn verify_legacy<R: LicensesRepository>(
    repo: &R,
    body: &VerifyLicenseRequest,
) -> Result<serde_json::Value, BillingError> {
    if let Some(file_content) = body.file.as_deref() {
        return verify_file(repo, file_content).await;
    }

    let key = body.key.as_deref().unwrap_or_default();
    let device_id = body.device_id.clone();
    let license = get_one(repo, key).await?;

    let response = LegacyVerifyResponse {
        valid: license.status == LicenseStatus::Active,
        license,
        device_id,
    };

    serde_json::to_value(response).map_err(|e| {
        BillingError::Internal(anyhow::anyhow!(
            "failed to serialize verification response: {e}"
        ))
    })
}

pub async fn verify_v1<R: LicensesRepository>(
    repo: &R,
    body: &VerifyLicenseRequest,
) -> Result<serde_json::Value, BillingError> {
    let key = body.key.as_deref().unwrap_or_default();
    let device_id = body.device_id.as_deref();
    let product_id = body.product_id.as_deref();

    let Some(license) = repo.get_license(key).await? else {
        return Ok(serde_json::json!({
            "valid": false,
            "error": "license_not_found",
        }));
    };

    if let Some(product_id) = product_id {
        if license.product_id.as_deref() != Some(product_id) {
            return Ok(serde_json::json!({
                "valid": false,
                "error": "license_not_found",
            }));
        }
    }

    let valid = license.status == LicenseStatus::Active;

    let expired = if license.expires_at.is_empty() {
        false
    } else {
        match chrono::NaiveDate::parse_from_str(&license.expires_at, "%Y-%m-%d") {
            Ok(exp) => Utc::now().date_naive() > exp,
            Err(_) => false,
        }
    };

    if expired {
        return Ok(serde_json::json!({
            "valid": false,
            "error": "license_expired",
            "license": license,
        }));
    }

    if let Some(device_id) = device_id {
        let activation_count = repo.count_activations(&license.key).await?;
        let existing = repo.find_activation(&license.key, device_id).await?;

        let max_activations = license.max_activations.unwrap_or(i64::MAX as i32) as i64;
        if existing.is_none() && activation_count >= max_activations {
            return Ok(serde_json::json!({
                "valid": false,
                "error": "max_activations_reached",
                "currentActivations": activation_count,
                "maxActivations": max_activations,
            }));
        }

        if existing.is_none() {
            repo.insert_activation(
                &license.key,
                device_id,
                body.device_name.as_deref(),
                body.ip_address.as_deref(),
            )
            .await?;
        } else {
            repo.update_activation_last_seen(&license.key, device_id)
                .await?;
        }
    }

    Ok(serde_json::json!({
        "valid": valid,
        "license": {
            "key": license.key,
            "status": license.status,
            "expiresAt": license.expires_at,
            "productId": license.product_id,
        },
    }))
}

pub async fn sign_license<R: LicensesRepository>(
    repo: &R,
    key: &str,
) -> Result<SignLicenseResponse, BillingError> {
    let license = get_one(repo, key).await?;

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

    let updated = repo
        .store_signed_license(key, &payload_json, &signed.signature)
        .await?;

    let signed_payload = updated.signed_payload.ok_or_else(|| {
        BillingError::Internal(anyhow::anyhow!(
            "missing signed payload after license signing"
        ))
    })?;
    let signature = updated.signature.ok_or_else(|| {
        BillingError::Internal(anyhow::anyhow!("missing signature after license signing"))
    })?;

    Ok(SignLicenseResponse {
        success: true,
        license_key: updated.key,
        signed_payload,
        signature,
    })
}

pub async fn export_license_file<R: LicensesRepository>(
    repo: &R,
    key: &str,
) -> Result<String, BillingError> {
    let license = get_one(repo, key).await?;

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

pub async fn get_keypair<R: LicensesRepository>(
    repo: &R,
) -> Result<KeypairStatusResponse, BillingError> {
    match repo.get_keypair().await? {
        Some((public_pem, _private_pem)) => Ok(KeypairStatusResponse {
            exists: true,
            public_key: Some(public_pem),
            message: None,
        }),
        None => Ok(KeypairStatusResponse {
            exists: false,
            public_key: None,
            message: Some("No keypair found. POST to create one.".to_string()),
        }),
    }
}

pub async fn create_keypair<R: LicensesRepository>(
    repo: &R,
) -> Result<KeypairCreateResponse, BillingError> {
    let (public_pem, private_pem) = signing::generate_keypair()?;
    repo.store_keypair(&public_pem, &private_pem).await?;

    Ok(KeypairCreateResponse {
        success: true,
        public_key: public_pem,
        message: "Ed25519 keypair generated and stored.".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rustbill_core::db::models::{License, LicenseActivation, LicenseStatus};
    use rustbill_core::error::BillingError;
    use rustbill_core::licenses::signing::{
        generate_keypair, sign_license as sign_payload, to_license_file,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    fn sample_license() -> License {
        License {
            key: "LIC-123".to_string(),
            customer_id: Some("cust-1".to_string()),
            customer_name: "Customer One".to_string(),
            product_id: Some("prod-1".to_string()),
            product_name: "Product One".to_string(),
            status: LicenseStatus::Active,
            created_at: "2026-01-01".to_string(),
            expires_at: "2999-12-31".to_string(),
            license_type: "simple".to_string(),
            signed_payload: None,
            signature: None,
            features: Some(serde_json::json!(["feature-a"])),
            max_activations: Some(1),
        }
    }

    fn sample_activation() -> LicenseActivation {
        LicenseActivation {
            id: "act-1".to_string(),
            license_key: "LIC-123".to_string(),
            device_id: "device-1".to_string(),
            device_name: Some("Device".to_string()),
            ip_address: Some("127.0.0.1".to_string()),
            activated_at: chrono::Utc::now().naive_utc(),
            last_seen_at: chrono::Utc::now().naive_utc(),
        }
    }

    struct MockRepo {
        license: Mutex<Option<License>>,
        keypair: Mutex<Option<(String, String)>>,
        activation_count: i64,
        activation: Mutex<Option<LicenseActivation>>,
        delete_license_rows: u64,
        delete_activation_rows: u64,
        stored_keypair: Mutex<Option<(String, String)>>,
        signed_calls: AtomicUsize,
        activation_updates: AtomicUsize,
        activation_inserts: AtomicUsize,
        signed_store: Mutex<Option<(String, String)>>,
    }

    impl MockRepo {
        fn new(license: Option<License>) -> Self {
            Self {
                license: Mutex::new(license),
                keypair: Mutex::new(None),
                activation_count: 0,
                activation: Mutex::new(None),
                delete_license_rows: 1,
                delete_activation_rows: 1,
                stored_keypair: Mutex::new(None),
                signed_calls: AtomicUsize::new(0),
                activation_updates: AtomicUsize::new(0),
                activation_inserts: AtomicUsize::new(0),
                signed_store: Mutex::new(None),
            }
        }

        fn with_keypair(self, keypair: Option<(String, String)>) -> Self {
            *self.keypair.lock().expect("mutex poisoned") = keypair;
            self
        }

        fn with_activation(self, activation: Option<LicenseActivation>) -> Self {
            *self.activation.lock().expect("mutex poisoned") = activation;
            self
        }

        fn with_activation_count(mut self, count: i64) -> Self {
            self.activation_count = count;
            self
        }

        fn with_delete_activation_rows(mut self, rows: u64) -> Self {
            self.delete_activation_rows = rows;
            self
        }

        fn store_count(&self) -> usize {
            self.signed_calls.load(Ordering::SeqCst)
        }

        fn activation_updates(&self) -> usize {
            self.activation_updates.load(Ordering::SeqCst)
        }

        fn activation_inserts(&self) -> usize {
            self.activation_inserts.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl LicensesRepository for MockRepo {
        async fn list_licenses(&self, _status: Option<&str>) -> Result<Vec<License>, BillingError> {
            Ok(vec![])
        }

        async fn get_license(&self, _key: &str) -> Result<Option<License>, BillingError> {
            Ok(self.license.lock().expect("mutex poisoned").clone())
        }

        async fn insert_license(&self, record: &NewLicenseRecord) -> Result<License, BillingError> {
            let mut license = sample_license();
            license.key = record.key.clone();
            license.customer_id = record.customer_id.clone();
            license.customer_name = record.customer_name.clone().unwrap_or_default();
            license.product_id = record.product_id.clone();
            license.product_name = record.product_name.clone().unwrap_or_default();
            license.created_at = record
                .created_at
                .clone()
                .unwrap_or_else(|| "2026-01-01".to_string());
            license.expires_at = record.expires_at.clone().unwrap_or_default();
            license.license_type = record
                .license_type
                .clone()
                .unwrap_or_else(|| "simple".to_string());
            license.features = record.features.clone();
            license.max_activations = record.max_activations;
            *self.license.lock().expect("mutex poisoned") = Some(license.clone());
            Ok(license)
        }

        async fn update_license(
            &self,
            _key: &str,
            patch: &LicensePatch,
        ) -> Result<Option<License>, BillingError> {
            let mut guard = self.license.lock().expect("mutex poisoned");
            let Some(mut license) = guard.clone() else {
                return Ok(None);
            };

            if let Some(status) = patch.status.as_ref() {
                license.status = match status.as_str() {
                    "expired" => LicenseStatus::Expired,
                    "revoked" => LicenseStatus::Revoked,
                    "suspended" => LicenseStatus::Suspended,
                    _ => LicenseStatus::Active,
                };
            }
            if let Some(customer_name) = patch.customer_name.clone() {
                license.customer_name = customer_name;
            }
            if let Some(product_name) = patch.product_name.clone() {
                license.product_name = product_name;
            }
            if let Some(max_activations) = patch.max_activations {
                license.max_activations = Some(max_activations);
            }
            if let Some(created_at) = patch.created_at.clone() {
                license.created_at = created_at;
            }
            if let Some(expires_at) = patch.expires_at.clone() {
                license.expires_at = expires_at;
            }
            if let Some(license_type) = patch.license_type.clone() {
                license.license_type = license_type;
            }
            if let Some(features) = patch.features.clone() {
                license.features = Some(features);
            }

            *guard = Some(license.clone());
            Ok(Some(license))
        }

        async fn delete_license(&self, _key: &str) -> Result<u64, BillingError> {
            Ok(self.delete_license_rows)
        }

        async fn list_activations(
            &self,
            _key: &str,
        ) -> Result<Vec<LicenseActivation>, BillingError> {
            Ok(vec![])
        }

        async fn delete_activation(
            &self,
            _key: &str,
            _device_id: &str,
        ) -> Result<u64, BillingError> {
            Ok(self.delete_activation_rows)
        }

        async fn get_keypair(&self) -> Result<Option<(String, String)>, BillingError> {
            Ok(self.keypair.lock().expect("mutex poisoned").clone())
        }

        async fn store_keypair(
            &self,
            public_pem: &str,
            private_pem: &str,
        ) -> Result<(), BillingError> {
            *self.stored_keypair.lock().expect("mutex poisoned") =
                Some((public_pem.to_string(), private_pem.to_string()));
            Ok(())
        }

        async fn count_activations(&self, _key: &str) -> Result<i64, BillingError> {
            Ok(self.activation_count)
        }

        async fn find_activation(
            &self,
            _key: &str,
            _device_id: &str,
        ) -> Result<Option<LicenseActivation>, BillingError> {
            Ok(self.activation.lock().expect("mutex poisoned").clone())
        }

        async fn insert_activation(
            &self,
            _key: &str,
            _device_id: &str,
            _device_name: Option<&str>,
            _ip_address: Option<&str>,
        ) -> Result<(), BillingError> {
            self.activation_inserts.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn update_activation_last_seen(
            &self,
            _key: &str,
            _device_id: &str,
        ) -> Result<(), BillingError> {
            self.activation_updates.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn store_signed_license(
            &self,
            _key: &str,
            signed_payload: &str,
            signature: &str,
        ) -> Result<License, BillingError> {
            self.signed_calls.fetch_add(1, Ordering::SeqCst);
            *self.signed_store.lock().expect("mutex poisoned") =
                Some((signed_payload.to_string(), signature.to_string()));

            let mut license = self
                .license
                .lock()
                .expect("mutex poisoned")
                .clone()
                .expect("license should exist in test");
            license.signed_payload = Some(signed_payload.to_string());
            license.signature = Some(signature.to_string());
            Ok(license)
        }
    }

    #[tokio::test]
    async fn verify_legacy_file_branch_uses_offline_verification() {
        let (public_pem, private_pem) = generate_keypair().expect("keypair");
        let payload = LicensePayload {
            license_id: "LIC-123".to_string(),
            customer_id: Some("cust-1".to_string()),
            customer_name: "Customer One".to_string(),
            product_id: Some("prod-1".to_string()),
            product_name: "Product One".to_string(),
            features: vec!["feature-a".to_string()],
            max_activations: Some(2),
            issued_at: "2026-01-01".to_string(),
            expires_at: "2999-12-31".to_string(),
            metadata: None,
        };
        let signed = sign_payload(&payload, &private_pem).expect("sign");
        let license_file = to_license_file(&signed);
        let repo = MockRepo::new(None).with_keypair(Some((public_pem, private_pem)));

        let response = verify_legacy(
            &repo,
            &VerifyLicenseRequest {
                file: Some(license_file),
                key: Some("should-not-matter".to_string()),
                device_id: None,
                device_name: None,
                ip_address: None,
                product_id: None,
            },
        )
        .await
        .expect("verify");

        assert_eq!(response["valid"], serde_json::json!(true));
        assert_eq!(
            response["payload"]["license_id"],
            serde_json::json!("LIC-123")
        );
    }

    #[tokio::test]
    async fn verify_v1_returns_license_not_found_for_product_mismatch() {
        let repo = MockRepo::new(Some(sample_license()));

        let response = verify_v1(
            &repo,
            &VerifyLicenseRequest {
                file: None,
                key: Some("LIC-123".to_string()),
                device_id: None,
                device_name: None,
                ip_address: None,
                product_id: Some("different-product".to_string()),
            },
        )
        .await
        .expect("verify");

        assert_eq!(response["valid"], serde_json::json!(false));
        assert_eq!(response["error"], serde_json::json!("license_not_found"));
    }

    #[tokio::test]
    async fn verify_v1_rejects_new_activation_when_limit_is_reached() {
        let repo = MockRepo::new(Some(sample_license()))
            .with_activation_count(1)
            .with_activation(None);

        let response = verify_v1(
            &repo,
            &VerifyLicenseRequest {
                file: None,
                key: Some("LIC-123".to_string()),
                device_id: Some("new-device".to_string()),
                device_name: Some("New Device".to_string()),
                ip_address: Some("127.0.0.1".to_string()),
                product_id: None,
            },
        )
        .await
        .expect("verify");

        assert_eq!(
            response["error"],
            serde_json::json!("max_activations_reached")
        );
        assert_eq!(response["currentActivations"], serde_json::json!(1));
        assert_eq!(response["maxActivations"], serde_json::json!(1));
    }

    #[tokio::test]
    async fn verify_v1_touches_existing_activation_instead_of_inserting() {
        let repo = MockRepo::new(Some(sample_license()))
            .with_activation_count(1)
            .with_activation(Some(sample_activation()));

        let response = verify_v1(
            &repo,
            &VerifyLicenseRequest {
                file: None,
                key: Some("LIC-123".to_string()),
                device_id: Some("device-1".to_string()),
                device_name: Some("Device".to_string()),
                ip_address: Some("127.0.0.1".to_string()),
                product_id: None,
            },
        )
        .await
        .expect("verify");

        assert_eq!(response["valid"], serde_json::json!(true));
        assert_eq!(repo.activation_updates(), 1);
        assert_eq!(repo.activation_inserts(), 0);
    }

    #[tokio::test]
    async fn sign_license_requires_keypair() {
        let repo = MockRepo::new(Some(sample_license()));

        let response = sign_license(&repo, "LIC-123").await;

        assert!(matches!(
            response,
            Err(BillingError::BadRequest(message))
            if message.contains("no signing keypair exists")
        ));
    }

    #[tokio::test]
    async fn sign_license_persists_signed_payload() {
        let (public_pem, private_pem) = generate_keypair().expect("keypair");
        let repo =
            MockRepo::new(Some(sample_license())).with_keypair(Some((public_pem, private_pem)));

        let response = sign_license(&repo, "LIC-123").await.expect("sign");

        assert!(response.success);
        assert_eq!(response.license_key, "LIC-123");
        assert!(!response.signed_payload.is_empty());
        assert!(!response.signature.is_empty());
        assert_eq!(repo.store_count(), 1);
    }

    #[tokio::test]
    async fn deactivate_maps_zero_rows_to_not_found() {
        let repo = MockRepo::new(Some(sample_license())).with_delete_activation_rows(0);

        let response = deactivate(&repo, "LIC-123", "device-1").await;

        assert!(matches!(
            response,
            Err(BillingError::NotFound { entity: "activation", id })
            if id == "LIC-123/device-1"
        ));
    }
}
