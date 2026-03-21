use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDunningLogRequest {
    pub invoice_id: Option<Value>,
    pub subscription_id: Option<Value>,
    pub step: Option<Value>,
    pub scheduled_at: Option<Value>,
    pub executed_at: Option<Value>,
    pub notes: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DunningListParams {
    pub invoice_id: Option<String>,
}
