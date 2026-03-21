use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CreateApiKeyRequest {
    pub name: Option<serde_json::Value>,
    #[serde(rename = "customerId")]
    pub customer_id: Option<serde_json::Value>,
}

impl CreateApiKeyRequest {
    pub fn resolved_name(&self) -> &str {
        self.name
            .as_ref()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("default")
    }

    pub fn customer_id(&self) -> Option<&str> {
        self.customer_id
            .as_ref()
            .and_then(serde_json::Value::as_str)
    }
}
