use std::collections::HashMap;

/// A snapshot of provider settings for use in payment operations.
#[derive(Debug, Clone, Default)]
pub struct ProviderSettings {
    settings: HashMap<String, String>,
}

impl ProviderSettings {
    pub fn new(settings: HashMap<String, String>) -> Self {
        Self { settings }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.settings.get(key).cloned()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ProviderStatus {
    pub stripe: ProviderInfo,
    pub xendit: XenditProviderInfo,
    pub lemonsqueezy: LsProviderInfo,
    pub tax: TaxProviderInfo,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub configured: bool,
    pub secret_key: String,
    pub webhook_secret: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XenditProviderInfo {
    pub configured: bool,
    pub secret_key: String,
    pub webhook_token: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LsProviderInfo {
    pub configured: bool,
    pub api_key: String,
    pub store_id: String,
    pub webhook_secret: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxProviderInfo {
    pub configured: bool,
    pub external_provider: String,
    pub taxjar_api_key: String,
}
