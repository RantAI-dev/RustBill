use super::repository::UsageRepository;
use super::schema::CreateUsageEventRequest;
use crate::db::models::UsageEvent;
use crate::error::{BillingError, Result};
use chrono::Utc;
use validator::Validate;

pub async fn list_usage_events<R: UsageRepository + ?Sized>(
    repo: &R,
    subscription_id: &str,
) -> Result<Vec<UsageEvent>> {
    repo.list_usage_events(subscription_id).await
}

pub async fn create_usage_event<R: UsageRepository + ?Sized>(
    repo: &R,
    req: CreateUsageEventRequest,
) -> Result<UsageEvent> {
    req.validate().map_err(BillingError::from_validation)?;

    if let Some(ref idempotency_key) = req.idempotency_key {
        if let Some(existing) = repo
            .find_usage_event_by_idempotency_key(idempotency_key)
            .await?
        {
            return Ok(existing);
        }
    }

    let timestamp = req.timestamp.unwrap_or_else(|| Utc::now().naive_utc());
    repo.create_usage_event(&req, timestamp).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::NaiveDateTime;
    use rust_decimal::Decimal;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        list_rows: Vec<UsageEvent>,
        existing: Option<UsageEvent>,
        created_req: Option<CreateUsageEventRequest>,
        created_timestamp: Option<NaiveDateTime>,
        created_event: Option<UsageEvent>,
    }

    #[derive(Clone, Default)]
    struct StubRepo {
        state: Arc<Mutex<StubState>>,
    }

    impl StubRepo {
        fn with_state(state: StubState) -> Self {
            Self {
                state: Arc::new(Mutex::new(state)),
            }
        }
    }

    #[async_trait]
    impl UsageRepository for StubRepo {
        async fn list_usage_events(&self, _subscription_id: &str) -> Result<Vec<UsageEvent>> {
            match self.state.lock() {
                Ok(state) => Ok(state.list_rows.clone()),
                Err(_) => Err(BillingError::Internal(anyhow::anyhow!("mutex poisoned"))),
            }
        }

        async fn find_usage_event_by_idempotency_key(
            &self,
            _idempotency_key: &str,
        ) -> Result<Option<UsageEvent>> {
            match self.state.lock() {
                Ok(state) => Ok(state.existing.clone()),
                Err(_) => Err(BillingError::Internal(anyhow::anyhow!("mutex poisoned"))),
            }
        }

        async fn create_usage_event(
            &self,
            req: &CreateUsageEventRequest,
            timestamp: NaiveDateTime,
        ) -> Result<UsageEvent> {
            match self.state.lock() {
                Ok(mut state) => {
                    state.created_req = Some(req.clone());
                    state.created_timestamp = Some(timestamp);
                    if let Some(event) = state.created_event.clone() {
                        Ok(event)
                    } else {
                        Err(BillingError::Internal(anyhow::anyhow!(
                            "missing created event"
                        )))
                    }
                }
                Err(_) => Err(BillingError::Internal(anyhow::anyhow!("mutex poisoned"))),
            }
        }
    }

    fn sample_event() -> UsageEvent {
        UsageEvent {
            id: "usage_1".to_string(),
            subscription_id: "sub_1".to_string(),
            metric_name: "api_calls".to_string(),
            value: Decimal::from(42),
            timestamp: Utc::now().naive_utc(),
            idempotency_key: Some("idem_1".to_string()),
            properties: Some(serde_json::json!({"source": "test"})),
        }
    }

    fn sample_request() -> CreateUsageEventRequest {
        CreateUsageEventRequest {
            subscription_id: "sub_1".to_string(),
            metric_name: "api_calls".to_string(),
            value: Decimal::from(42),
            timestamp: None,
            idempotency_key: Some("idem_1".to_string()),
            properties: Some(serde_json::json!({"source": "test"})),
        }
    }

    #[tokio::test]
    async fn list_usage_events_forwards_repository_rows() {
        let repo = StubRepo::with_state(StubState {
            list_rows: vec![sample_event()],
            ..StubState::default()
        });

        let rows = list_usage_events(&repo, "sub_1").await;
        match rows {
            Ok(rows) => assert_eq!(rows.len(), 1),
            Err(err) => panic!("unexpected error: {err}"),
        }
    }

    #[tokio::test]
    async fn create_usage_event_returns_existing_when_idempotency_hits() {
        let repo = StubRepo::with_state(StubState {
            existing: Some(sample_event()),
            ..StubState::default()
        });

        let result = create_usage_event(&repo, sample_request()).await;
        match result {
            Ok(event) => assert_eq!(event.id, "usage_1"),
            Err(err) => panic!("unexpected error: {err}"),
        }
    }

    #[tokio::test]
    async fn create_usage_event_validates_and_forwards_new_event() {
        let repo = StubRepo::with_state(StubState {
            created_event: Some(sample_event()),
            ..StubState::default()
        });

        let mut req = sample_request();
        req.idempotency_key = None;

        let result = create_usage_event(&repo, req).await;
        match result {
            Ok(event) => {
                assert_eq!(event.id, "usage_1");
                let state = match repo.state.lock() {
                    Ok(state) => state,
                    Err(_) => panic!("mutex poisoned"),
                };
                assert!(state.created_req.is_some());
                assert!(state.created_timestamp.is_some());
            }
            Err(err) => panic!("unexpected error: {err}"),
        }
    }
}
