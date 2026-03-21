use super::repository::ApiKeysRepository;
use super::schema::CreateApiKeyRequest;
use rustbill_core::error::BillingError;

pub async fn list<R: ApiKeysRepository>(repo: &R) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list().await
}

pub async fn create<R: ApiKeysRepository>(
    repo: &R,
    body: &CreateApiKeyRequest,
) -> Result<serde_json::Value, BillingError> {
    let key_plain = rustbill_core::auth::generate_api_key();
    let key_hash = rustbill_core::auth::hash_api_key(&key_plain);
    let key_prefix = rustbill_core::auth::get_key_prefix(&key_plain);

    let mut row = repo
        .create(
            body.resolved_name(),
            body.customer_id(),
            &key_prefix,
            &key_hash,
        )
        .await?;

    let obj = row.as_object_mut().ok_or_else(|| {
        BillingError::Internal(anyhow::anyhow!("api key response was not an object"))
    })?;
    obj.insert("key".to_string(), serde_json::Value::String(key_plain));

    Ok(row)
}

pub async fn revoke<R: ApiKeysRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let affected = repo.revoke(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("api_key", id));
    }

    Ok(serde_json::json!({ "success": true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::api_keys::repository::ApiKeysRepository;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct MockRepo {
        list_rows: Vec<serde_json::Value>,
        create_row: serde_json::Value,
        revoke_rows: u64,
        captured_name: Mutex<Option<String>>,
        captured_customer_id: Mutex<Option<Option<String>>>,
        captured_key_prefix: Mutex<Option<String>>,
        captured_key_hash: Mutex<Option<String>>,
    }

    impl MockRepo {
        fn new(revoke_rows: u64) -> Self {
            Self {
                list_rows: vec![serde_json::json!({ "id": "api-key-1" })],
                create_row: serde_json::json!({ "id": "api-key-1", "name": "default" }),
                revoke_rows,
                captured_name: Mutex::new(None),
                captured_customer_id: Mutex::new(None),
                captured_key_prefix: Mutex::new(None),
                captured_key_hash: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl ApiKeysRepository for MockRepo {
        async fn list(&self) -> Result<Vec<serde_json::Value>, BillingError> {
            Ok(self.list_rows.clone())
        }

        async fn create(
            &self,
            name: &str,
            customer_id: Option<&str>,
            key_prefix: &str,
            key_hash: &str,
        ) -> Result<serde_json::Value, BillingError> {
            *self.captured_name.lock().unwrap() = Some(name.to_string());
            *self.captured_customer_id.lock().unwrap() =
                Some(customer_id.map(|value| value.to_string()));
            *self.captured_key_prefix.lock().unwrap() = Some(key_prefix.to_string());
            *self.captured_key_hash.lock().unwrap() = Some(key_hash.to_string());
            Ok(self.create_row.clone())
        }

        async fn revoke(&self, _id: &str) -> Result<u64, BillingError> {
            Ok(self.revoke_rows)
        }
    }

    #[tokio::test]
    async fn create_defaults_name_and_includes_plain_key() {
        let repo = MockRepo::new(1);
        let body = CreateApiKeyRequest {
            name: None,
            customer_id: Some(serde_json::json!("cust-1")),
        };

        let result = create(&repo, &body).await;

        assert!(result.is_ok());
        let row = match result {
            Ok(row) => row,
            Err(err) => panic!("unexpected error: {err}"),
        };

        assert_eq!(row["id"], serde_json::json!("api-key-1"));
        assert!(row["key"].as_str().is_some());
        assert!(row["key"]
            .as_str()
            .map(|key| key.starts_with("pk_live_"))
            .unwrap_or(false));
        assert_eq!(
            repo.captured_name.lock().unwrap().as_deref(),
            Some("default")
        );
        assert_eq!(
            repo.captured_customer_id
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|v| v.as_deref()),
            Some("cust-1")
        );
        assert_eq!(
            repo.captured_key_prefix
                .lock()
                .unwrap()
                .as_deref()
                .map(|prefix| prefix.starts_with("pk_live_") && prefix.len() == 12),
            Some(true)
        );
        assert_eq!(
            repo.captured_key_hash
                .lock()
                .unwrap()
                .as_ref()
                .map(String::len),
            Some(64)
        );
    }

    #[tokio::test]
    async fn revoke_maps_zero_rows_to_not_found() {
        let repo = MockRepo::new(0);
        let result = revoke(&repo, "api-key-1").await;

        assert!(
            matches!(result, Err(BillingError::NotFound { entity: "api_key", id }) if id == "api-key-1")
        );
    }
}
