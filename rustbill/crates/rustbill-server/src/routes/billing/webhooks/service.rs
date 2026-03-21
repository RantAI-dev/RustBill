use super::repository::WebhooksRepository;
use super::schema::{CreateWebhookRequest, UpdateWebhookRequest};
use async_trait::async_trait;
use rustbill_core::error::BillingError;

#[async_trait]
pub trait WebhookDispatcher: Send + Sync {
    async fn post_json(&self, url: &str, payload: &serde_json::Value) -> Result<u16, String>;
}

pub async fn list<R: WebhooksRepository>(repo: &R) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list().await
}

pub async fn get<R: WebhooksRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get(id).await
}

pub async fn create<R: WebhooksRepository>(
    repo: &R,
    body: &CreateWebhookRequest,
) -> Result<serde_json::Value, BillingError> {
    let default_events = body.events_or_default();
    let events = body.events.as_ref().unwrap_or(&default_events);
    repo.create(
        body.url(),
        body.description(),
        events,
        body.secret_or_default(),
    )
    .await
}

pub async fn update<R: WebhooksRepository>(
    repo: &R,
    id: &str,
    body: &UpdateWebhookRequest,
) -> Result<serde_json::Value, BillingError> {
    repo.update(
        id,
        body.url(),
        body.description(),
        body.events(),
        body.status(),
    )
    .await
}

pub async fn remove<R: WebhooksRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let affected = repo.delete(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("webhook_endpoint", id));
    }

    Ok(serde_json::json!({ "success": true }))
}

pub async fn test_webhook<R: WebhooksRepository, D: WebhookDispatcher>(
    repo: &R,
    dispatcher: &D,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let endpoint = repo.get(id).await?;
    let url = endpoint["url"].as_str().unwrap_or_default();

    let test_payload = serde_json::json!({
        "type": "test.webhook",
        "data": { "message": "This is a test webhook delivery" },
    });

    match dispatcher.post_json(url, &test_payload).await {
        Ok(status_code) => Ok(serde_json::json!({
            "success": true,
            "statusCode": status_code,
        })),
        Err(error) => Ok(serde_json::json!({
            "success": false,
            "error": error,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::billing::webhooks::repository::WebhooksRepository;
    use async_trait::async_trait;
    use std::sync::Mutex;

    type CreatedArgs = (Option<String>, Option<String>, serde_json::Value, String);

    struct MockRepo {
        endpoint: serde_json::Value,
        created: Mutex<Option<CreatedArgs>>,
        deleted: Mutex<Option<String>>,
    }

    impl MockRepo {
        fn new(endpoint: serde_json::Value) -> Self {
            Self {
                endpoint,
                created: Mutex::new(None),
                deleted: Mutex::new(None),
            }
        }
    }

    struct MockDispatcher {
        url: Mutex<Option<String>>,
        payload: Mutex<Option<serde_json::Value>>,
        result: Result<u16, String>,
    }

    #[async_trait]
    impl WebhooksRepository for MockRepo {
        async fn list(&self) -> Result<Vec<serde_json::Value>, BillingError> {
            Ok(vec![self.endpoint.clone()])
        }

        async fn get(&self, _id: &str) -> Result<serde_json::Value, BillingError> {
            Ok(self.endpoint.clone())
        }

        async fn create(
            &self,
            url: Option<&str>,
            description: Option<&str>,
            events: &serde_json::Value,
            secret: &str,
        ) -> Result<serde_json::Value, BillingError> {
            *self.created.lock().expect("lock poisoned") = Some((
                url.map(ToString::to_string),
                description.map(ToString::to_string),
                events.clone(),
                secret.to_string(),
            ));
            Ok(self.endpoint.clone())
        }

        async fn update(
            &self,
            _id: &str,
            _url: Option<&str>,
            _description: Option<&str>,
            _events: Option<&serde_json::Value>,
            _status: Option<&str>,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(self.endpoint.clone())
        }

        async fn delete(&self, id: &str) -> Result<u64, BillingError> {
            *self.deleted.lock().expect("lock poisoned") = Some(id.to_string());
            Ok(1)
        }
    }

    #[async_trait]
    impl WebhookDispatcher for MockDispatcher {
        async fn post_json(&self, url: &str, payload: &serde_json::Value) -> Result<u16, String> {
            *self.url.lock().expect("lock poisoned") = Some(url.to_string());
            *self.payload.lock().expect("lock poisoned") = Some(payload.clone());
            self.result.clone()
        }
    }

    #[tokio::test]
    async fn create_applies_default_secret_and_events() {
        let repo = MockRepo::new(serde_json::json!({ "id": "wh-1" }));
        let body = CreateWebhookRequest {
            url: Some(serde_json::json!("https://example.com/hook")),
            description: None,
            events: None,
            secret: None,
        };

        let result = create(&repo, &body).await.expect("create should succeed");
        assert_eq!(result["id"], serde_json::json!("wh-1"));
        let captured = repo
            .created
            .lock()
            .expect("lock poisoned")
            .clone()
            .expect("create should be captured");
        assert_eq!(captured.0.as_deref(), Some("https://example.com/hook"));
        assert_eq!(captured.1, None);
        assert_eq!(captured.2, serde_json::json!(["*"]));
        assert_eq!(captured.3, "default-secret");
    }

    #[tokio::test]
    async fn test_webhook_returns_dispatch_error_payload() {
        let repo = MockRepo::new(serde_json::json!({
            "id": "wh-1",
            "url": "https://example.com/hook"
        }));
        let dispatcher = MockDispatcher {
            url: Mutex::new(None),
            payload: Mutex::new(None),
            result: Err("network down".to_string()),
        };

        let result = test_webhook(&repo, &dispatcher, "wh-1")
            .await
            .expect("test_webhook should succeed");
        assert_eq!(result["success"], serde_json::json!(false));
        assert_eq!(result["error"], serde_json::json!("network down"));
        assert_eq!(
            dispatcher
                .url
                .lock()
                .expect("lock poisoned")
                .clone()
                .expect("url should be captured"),
            "https://example.com/hook"
        );
    }
}
