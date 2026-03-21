use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct LemonSqueezyWebhookEvent {
    pub meta: Option<serde_json::Value>,
    pub data: Option<serde_json::Value>,
}
