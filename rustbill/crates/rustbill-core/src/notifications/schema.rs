use crate::db::models::BillingEventType;

#[derive(Debug, Clone)]
pub struct EmitBillingEventRequest {
    pub event_type: BillingEventType,
    pub resource_type: String,
    pub resource_id: String,
    pub customer_id: Option<String>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct WebhookEndpoint {
    pub id: String,
    pub url: String,
    pub secret: String,
    pub events: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct CustomerContact {
    pub email: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct WebhookDispatchPayload {
    pub event: String,
    pub data: Option<serde_json::Value>,
    pub resource_type: String,
    pub resource_id: String,
    pub customer_id: Option<String>,
}

impl WebhookDispatchPayload {
    pub fn as_json(&self) -> serde_json::Value {
        serde_json::json!({
            "event": self.event,
            "data": self.data,
            "resourceType": self.resource_type,
            "resourceId": self.resource_id,
            "customerId": self.customer_id,
        })
    }
}
