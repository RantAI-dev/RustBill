use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct StripeWebhookData {
    pub object: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StripeWebhookEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: StripeWebhookData,
}
