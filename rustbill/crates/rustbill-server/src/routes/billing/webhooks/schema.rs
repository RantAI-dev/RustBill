use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CreateWebhookRequest {
    pub url: Option<serde_json::Value>,
    pub description: Option<serde_json::Value>,
    pub events: Option<serde_json::Value>,
    pub secret: Option<serde_json::Value>,
}

impl CreateWebhookRequest {
    pub fn url(&self) -> Option<&str> {
        self.url.as_ref().and_then(|value| value.as_str())
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_ref().and_then(|value| value.as_str())
    }

    pub fn events(&self) -> Option<&serde_json::Value> {
        self.events.as_ref()
    }

    pub fn secret_or_default(&self) -> &str {
        self.secret
            .as_ref()
            .and_then(|value| value.as_str())
            .unwrap_or("default-secret")
    }

    pub fn events_or_default(&self) -> serde_json::Value {
        self.events
            .clone()
            .unwrap_or_else(|| serde_json::json!(["*"]))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWebhookRequest {
    pub url: Option<serde_json::Value>,
    pub description: Option<serde_json::Value>,
    pub events: Option<serde_json::Value>,
    pub status: Option<serde_json::Value>,
}

impl UpdateWebhookRequest {
    pub fn url(&self) -> Option<&str> {
        self.url.as_ref().and_then(|value| value.as_str())
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_ref().and_then(|value| value.as_str())
    }

    pub fn events(&self) -> Option<&serde_json::Value> {
        self.events.as_ref()
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_ref().and_then(|value| value.as_str())
    }
}
