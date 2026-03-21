use rustbill_core::db::models::License;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListLicensesQuery {
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLicenseRequest {
    pub key: Option<String>,
    pub customer_id: Option<String>,
    pub customer_name: Option<String>,
    pub product_id: Option<String>,
    pub product_name: Option<String>,
    pub starts_at: Option<String>,
    pub expires_at: Option<String>,
    pub license_type: Option<String>,
    pub features: Option<serde_json::Value>,
    pub max_activations: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLicenseRequest {
    pub status: Option<String>,
    pub customer_name: Option<String>,
    pub product_name: Option<String>,
    pub max_activations: Option<i32>,
    pub starts_at: Option<String>,
    pub expires_at: Option<String>,
    pub license_type: Option<String>,
    pub features: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyLicenseRequest {
    pub file: Option<String>,
    pub key: Option<String>,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub ip_address: Option<String>,
    pub product_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeactivateLicenseQuery {
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyVerifyResponse {
    pub valid: bool,
    pub license: License,
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeypairStatusResponse {
    pub exists: bool,
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeypairCreateResponse {
    pub success: bool,
    pub public_key: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignLicenseResponse {
    pub success: bool,
    pub license_key: String,
    pub signed_payload: String,
    pub signature: String,
}
