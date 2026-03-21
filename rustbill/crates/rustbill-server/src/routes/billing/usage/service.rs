use super::repository::UsageRepository;
use super::schema::{CreateUsageEventRequest, UpdateUsageEventRequest, UsageRecordInput};
use rustbill_core::error::BillingError;

pub async fn list_admin<R: UsageRepository>(
    repo: &R,
    subscription_id: Option<&str>,
    metric_name: Option<&str>,
    customer_id: Option<&str>,
) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list_admin(subscription_id, metric_name, customer_id)
        .await
}

pub async fn record_admin<R: UsageRepository>(
    repo: &R,
    req: &CreateUsageEventRequest,
) -> Result<serde_json::Value, BillingError> {
    repo.record_admin(req).await
}

pub async fn summary_admin<R: UsageRepository>(
    repo: &R,
    subscription_id: &str,
) -> Result<serde_json::Value, BillingError> {
    let rows = repo.summary_admin(subscription_id).await?;
    Ok(serde_json::json!({
        "subscriptionId": subscription_id,
        "metrics": rows,
    }))
}

pub async fn update_admin<R: UsageRepository>(
    repo: &R,
    id: &str,
    req: &UpdateUsageEventRequest,
) -> Result<serde_json::Value, BillingError> {
    repo.update_admin(id, req).await
}

pub async fn remove_admin<R: UsageRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let affected = repo.remove_admin(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("usage_event", id));
    }
    Ok(serde_json::json!({ "success": true }))
}

pub async fn list_v1<R: UsageRepository>(
    repo: &R,
    subscription_id: Option<&str>,
    metric: Option<&str>,
) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list_v1(subscription_id, metric).await
}

pub async fn record_v1<R: UsageRepository>(
    repo: &R,
    input: UsageRecordInput,
) -> Result<serde_json::Value, BillingError> {
    let events = match input {
        UsageRecordInput::One(event) => vec![event],
        UsageRecordInput::Many(events) => events,
    };

    let mut results = Vec::with_capacity(events.len());
    for event in &events {
        let row = repo.record_v1(event).await?;
        results.push(row);
    }

    if results.len() == 1 {
        Ok(results
            .into_iter()
            .next()
            .unwrap_or_else(|| serde_json::json!({})))
    } else {
        Ok(serde_json::json!(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::billing::usage::repository::UsageRepository;
    use async_trait::async_trait;

    struct MockUsageRepository;

    #[async_trait]
    impl UsageRepository for MockUsageRepository {
        async fn list_admin(
            &self,
            _subscription_id: Option<&str>,
            _metric_name: Option<&str>,
            _customer_id: Option<&str>,
        ) -> Result<Vec<serde_json::Value>, BillingError> {
            Ok(vec![])
        }

        async fn record_admin(
            &self,
            _req: &CreateUsageEventRequest,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "u1" }))
        }

        async fn summary_admin(
            &self,
            _subscription_id: &str,
        ) -> Result<Vec<serde_json::Value>, BillingError> {
            Ok(vec![
                serde_json::json!({ "metricName": "api_calls", "totalValue": 10 }),
            ])
        }

        async fn update_admin(
            &self,
            _id: &str,
            _req: &UpdateUsageEventRequest,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "u1" }))
        }

        async fn remove_admin(&self, _id: &str) -> Result<u64, BillingError> {
            Ok(1)
        }

        async fn list_v1(
            &self,
            _subscription_id: Option<&str>,
            _metric: Option<&str>,
        ) -> Result<Vec<serde_json::Value>, BillingError> {
            Ok(vec![])
        }

        async fn record_v1(
            &self,
            _req: &CreateUsageEventRequest,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "u1" }))
        }
    }

    #[tokio::test]
    async fn record_v1_single_returns_object() {
        let repo = MockUsageRepository;
        let input = UsageRecordInput::One(CreateUsageEventRequest {
            subscription_id: Some("sub-1".to_string()),
            metric_name: Some("api_calls".to_string()),
            value: Some(1.0),
            timestamp: None,
            idempotency_key: None,
            properties: None,
        });

        let result = record_v1(&repo, input).await.unwrap();
        assert_eq!(result["id"], serde_json::json!("u1"));
    }

    #[tokio::test]
    async fn summary_admin_wraps_metrics() {
        let repo = MockUsageRepository;
        let result = summary_admin(&repo, "sub-1").await.unwrap();
        assert_eq!(result["subscriptionId"], serde_json::json!("sub-1"));
        assert!(result["metrics"].is_array());
    }
}
