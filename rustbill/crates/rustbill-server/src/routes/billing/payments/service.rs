use super::repository::PaymentsRepository;
use super::schema::{CreatePaymentRequest, UpdatePaymentRequest};
use rustbill_core::error::BillingError;

pub async fn list<R: PaymentsRepository>(
    repo: &R,
    customer_id: Option<&str>,
) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list(customer_id).await
}

pub async fn get<R: PaymentsRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get(id).await
}

pub async fn create<R: PaymentsRepository>(
    repo: &R,
    body: &CreatePaymentRequest,
) -> Result<serde_json::Value, BillingError> {
    repo.create(body).await
}

pub async fn update<R: PaymentsRepository>(
    repo: &R,
    id: &str,
    body: &UpdatePaymentRequest,
) -> Result<serde_json::Value, BillingError> {
    repo.update(id, body).await
}

pub async fn delete<R: PaymentsRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let affected = repo.delete(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("payment", id));
    }

    Ok(serde_json::json!({ "success": true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::billing::payments::repository::PaymentsRepository;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct MockRepo {
        customer_id: Mutex<Option<String>>,
        delete_rows: u64,
    }

    impl MockRepo {
        fn new(delete_rows: u64) -> Self {
            Self {
                customer_id: Mutex::new(None),
                delete_rows,
            }
        }
    }

    #[async_trait]
    impl PaymentsRepository for MockRepo {
        async fn list(
            &self,
            customer_id: Option<&str>,
        ) -> Result<Vec<serde_json::Value>, BillingError> {
            *self.customer_id.lock().unwrap() = customer_id.map(ToOwned::to_owned);
            Ok(vec![serde_json::json!({ "id": "pay-1" })])
        }

        async fn get(&self, _id: &str) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "pay-1" }))
        }

        async fn create(
            &self,
            _body: &CreatePaymentRequest,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "pay-1" }))
        }

        async fn update(
            &self,
            _id: &str,
            _body: &UpdatePaymentRequest,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "pay-1" }))
        }

        async fn delete(&self, _id: &str) -> Result<u64, BillingError> {
            Ok(self.delete_rows)
        }
    }

    #[tokio::test]
    async fn list_forwards_customer_filter() {
        let repo = MockRepo::new(1);
        let rows = list(&repo, Some("cust-1")).await;

        assert!(rows.is_ok());
        let rows = match rows {
            Ok(rows) => rows,
            Err(err) => panic!("unexpected error: {err}"),
        };
        assert_eq!(rows.len(), 1);
        let captured = repo.customer_id.lock().unwrap().clone();
        assert_eq!(captured.as_deref(), Some("cust-1"));
    }

    #[tokio::test]
    async fn delete_maps_zero_rows_to_not_found() {
        let repo = MockRepo::new(0);
        let result = delete(&repo, "pay-1").await;

        assert!(
            matches!(result, Err(BillingError::NotFound { entity: "payment", id }) if id == "pay-1")
        );
    }
}
