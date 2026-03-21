use crate::db::models::LicenseStatus;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct CreateLicenseRequest {
    /// If provided, customer_name is auto-populated from the FK.
    pub customer_id: Option<String>,
    /// Fallback if customer_id is not provided.
    pub customer_name: Option<String>,
    /// If provided, product_name is auto-populated from the FK.
    pub product_id: Option<String>,
    /// Fallback if product_id is not provided.
    pub product_name: Option<String>,
    pub status: Option<LicenseStatus>,
    pub expires_at: Option<String>,
    /// "simple" or "signed"
    pub license_type: Option<String>,
    pub features: Option<Vec<String>>,
    pub max_activations: Option<i32>,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct UpdateLicenseRequest {
    pub customer_id: Option<String>,
    pub customer_name: Option<String>,
    pub product_id: Option<String>,
    pub product_name: Option<String>,
    pub status: Option<LicenseStatus>,
    pub expires_at: Option<String>,
    pub license_type: Option<String>,
    pub features: Option<Vec<String>>,
    pub max_activations: Option<i32>,
}

/// Request body for POST /api/v1/licenses/verify (online verification).
#[derive(Debug, Deserialize, Validate, Clone)]
pub struct VerifyLicenseRequest {
    #[validate(length(min = 1))]
    pub license_key: String,
    /// If provided, an activation record is upserted for this device.
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub ip_address: Option<String>,
}

/// Request to generate a cryptographically signed license blob.
#[derive(Debug, Deserialize, Validate, Clone)]
pub struct GenerateSignedLicenseRequest {
    #[validate(length(min = 1))]
    pub license_key: String,
    /// PEM-encoded ED25519 private key.
    #[validate(length(min = 1))]
    pub private_key_pem: String,
}

/// Request to verify an offline signed license blob.
#[derive(Debug, Deserialize, Validate, Clone)]
pub struct OnlineVerifyRequest {
    #[validate(length(min = 1))]
    pub license_file: String,
    /// PEM-encoded ED25519 public key.
    #[validate(length(min = 1))]
    pub public_key_pem: String,
}

#[derive(Debug, Clone)]
pub struct CreateLicenseDraft {
    pub customer_id: Option<String>,
    pub customer_name: String,
    pub product_id: Option<String>,
    pub product_name: String,
    pub status: LicenseStatus,
    pub created_at: String,
    pub expires_at: String,
    pub license_type: String,
    pub features: serde_json::Value,
    pub max_activations: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct UpdateLicenseDraft {
    pub customer_id: Option<String>,
    pub customer_name: Option<String>,
    pub product_id: Option<String>,
    pub product_name: Option<String>,
    pub status: Option<LicenseStatus>,
    pub expires_at: Option<String>,
    pub license_type: Option<String>,
    pub features: Option<serde_json::Value>,
    pub max_activations: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationOutcome {
    Inserted,
    Updated,
    LimitReached,
}
