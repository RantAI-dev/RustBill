use super::repository::DunningRepository;
use super::schema::{CreateDunningLogRequest, DunningListParams};
use rustbill_core::error::BillingError;

pub async fn list<R: DunningRepository>(
    repo: &R,
    params: &DunningListParams,
) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list(params.invoice_id.as_deref()).await
}

pub async fn get<R: DunningRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get(id).await
}

pub async fn create<R: DunningRepository>(
    repo: &R,
    body: &CreateDunningLogRequest,
) -> Result<serde_json::Value, BillingError> {
    repo.create(body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::billing::dunning::repository::DunningRepository;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct MockRepo {
        invoice_id: Mutex<Option<String>>,
    }

    impl MockRepo {
        fn new() -> Self {
            Self {
                invoice_id: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl DunningRepository for MockRepo {
        async fn list(
            &self,
            invoice_id: Option<&str>,
        ) -> Result<Vec<serde_json::Value>, BillingError> {
            *self.invoice_id.lock().unwrap() = invoice_id.map(ToOwned::to_owned);
            Ok(vec![serde_json::json!({ "id": "log-1" })])
        }

        async fn get(&self, _id: &str) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "log-1" }))
        }

        async fn create(
            &self,
            _body: &CreateDunningLogRequest,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "log-1" }))
        }
    }

    #[tokio::test]
    async fn list_forwards_invoice_filter() {
        let repo = MockRepo::new();
        let params = DunningListParams {
            invoice_id: Some("inv-1".to_string()),
        };

        let rows = list(&repo, &params).await;
        assert!(rows.is_ok());
        let rows = match rows {
            Ok(rows) => rows,
            Err(err) => panic!("unexpected error: {err}"),
        };
        assert_eq!(rows.len(), 1);
        let captured = repo.invoice_id.lock().unwrap().clone();
        assert_eq!(captured.as_deref(), Some("inv-1"));
    }
}
