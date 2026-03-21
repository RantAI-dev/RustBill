use super::repository::EventsRepository;
use super::schema::EventsListParams;
use rustbill_core::error::BillingError;

pub async fn list<R: EventsRepository>(
    repo: &R,
    params: &EventsListParams,
) -> Result<serde_json::Value, BillingError> {
    let limit = params.limit();
    let offset = params.offset();
    let rows = repo
        .list(
            params.r#type.as_deref(),
            params.resource_id.as_deref(),
            limit,
            offset,
        )
        .await?;
    let total = repo
        .count(params.r#type.as_deref(), params.resource_id.as_deref())
        .await?;

    Ok(serde_json::json!({
        "data": rows,
        "total": total,
        "limit": limit,
        "offset": offset,
    }))
}

pub async fn get<R: EventsRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get(id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::billing::events::repository::EventsRepository;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct MockRepo {
        limit: Mutex<Option<i64>>,
        offset: Mutex<Option<i64>>,
    }

    impl MockRepo {
        fn new() -> Self {
            Self {
                limit: Mutex::new(None),
                offset: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl EventsRepository for MockRepo {
        async fn list(
            &self,
            _event_type: Option<&str>,
            _resource_id: Option<&str>,
            limit: i64,
            offset: i64,
        ) -> Result<Vec<serde_json::Value>, BillingError> {
            *self.limit.lock().unwrap() = Some(limit);
            *self.offset.lock().unwrap() = Some(offset);
            Ok(vec![serde_json::json!({ "id": "event-1" })])
        }

        async fn count(
            &self,
            _event_type: Option<&str>,
            _resource_id: Option<&str>,
        ) -> Result<i64, BillingError> {
            Ok(1)
        }

        async fn get(&self, _id: &str) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "event-1" }))
        }
    }

    #[tokio::test]
    async fn list_caps_limit_and_wraps_metadata() {
        let repo = MockRepo::new();
        let params = EventsListParams {
            r#type: None,
            resource_id: None,
            limit: Some(500),
            offset: Some(3),
        };

        let result = list(&repo, &params).await;
        assert!(result.is_ok());
        let result = match result {
            Ok(value) => value,
            Err(err) => panic!("unexpected error: {err}"),
        };
        assert_eq!(result["total"], serde_json::json!(1));
        assert_eq!(result["limit"], serde_json::json!(200));
        assert_eq!(result["offset"], serde_json::json!(3));
        assert!(result["data"].is_array());
        assert_eq!(*repo.limit.lock().unwrap(), Some(200));
        assert_eq!(*repo.offset.lock().unwrap(), Some(3));
    }
}
