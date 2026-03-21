use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUsageParams {
    pub subscription_id: Option<String>,
    pub metric_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUsageV1Params {
    pub subscription_id: Option<String>,
    pub metric: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUsageEventRequest {
    pub subscription_id: Option<String>,
    pub metric_name: Option<String>,
    pub value: Option<f64>,
    pub timestamp: Option<String>,
    pub idempotency_key: Option<String>,
    pub properties: Option<serde_json::Value>,
}

impl CreateUsageEventRequest {
    pub fn normalized_value(&self) -> f64 {
        self.value.unwrap_or(1.0)
    }

    pub fn normalized_properties(&self) -> serde_json::Value {
        self.properties
            .clone()
            .unwrap_or_else(|| serde_json::json!({}))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUsageEventRequest {
    pub metric_name: Option<String>,
    pub value: Option<f64>,
    pub timestamp: Option<String>,
    pub properties: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum UsageRecordInput {
    One(CreateUsageEventRequest),
    Many(Vec<CreateUsageEventRequest>),
}
