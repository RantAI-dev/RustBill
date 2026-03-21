use super::repository::PlansRepository;
use super::schema::{CreatePlanRequest, UpdatePlanRequest};
use rustbill_core::error::BillingError;

pub async fn list<R: PlansRepository>(repo: &R) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list().await
}

pub async fn get<R: PlansRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get(id).await
}

pub async fn create<R: PlansRepository>(
    repo: &R,
    body: &CreatePlanRequest,
) -> Result<serde_json::Value, BillingError> {
    repo.create(body).await
}

pub async fn update<R: PlansRepository>(
    repo: &R,
    id: &str,
    body: &UpdatePlanRequest,
) -> Result<serde_json::Value, BillingError> {
    repo.update(id, body).await
}

pub async fn delete<R: PlansRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let affected = repo.delete(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("plan", id));
    }

    Ok(serde_json::json!({ "success": true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::billing::plans::repository::PlansRepository;
    use async_trait::async_trait;

    struct MockRepo {
        delete_rows: u64,
    }

    impl MockRepo {
        fn new(delete_rows: u64) -> Self {
            Self { delete_rows }
        }
    }

    #[async_trait]
    impl PlansRepository for MockRepo {
        async fn list(&self) -> Result<Vec<serde_json::Value>, BillingError> {
            Ok(vec![serde_json::json!({ "id": "plan-1" })])
        }

        async fn get(&self, _id: &str) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "plan-1" }))
        }

        async fn create(
            &self,
            _body: &CreatePlanRequest,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "plan-1" }))
        }

        async fn update(
            &self,
            _id: &str,
            _body: &UpdatePlanRequest,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "plan-1" }))
        }

        async fn delete(&self, _id: &str) -> Result<u64, BillingError> {
            Ok(self.delete_rows)
        }
    }

    #[tokio::test]
    async fn delete_maps_zero_rows_to_not_found() {
        let repo = MockRepo::new(0);
        let result = delete(&repo, "plan-1").await;

        assert!(
            matches!(result, Err(BillingError::NotFound { entity: "plan", id }) if id == "plan-1")
        );
    }
}
