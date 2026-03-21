use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct XenditWebhookEvent {
    pub event: Option<String>,
    pub status: Option<String>,
    pub external_id: Option<String>,
    pub id: Option<String>,
    pub paid_amount: Option<serde_json::Value>,
    pub amount: Option<serde_json::Value>,
    pub data: Option<serde_json::Value>,
}
