use super::repository::{CreateDealParams, DealsRepository, UpdateDealParams};
use super::schema::{CreateDealRequest, DealListQuery, DeleteDealResponse, UpdateDealRequest};
use rust_decimal::Decimal;
use rustbill_core::db::models::{Deal, DealType};
use rustbill_core::error::BillingError;

pub async fn list<R: DealsRepository>(
    repo: &R,
    query: &DealListQuery,
) -> Result<Vec<Deal>, BillingError> {
    repo.list(query.product_type.as_deref(), query.deal_type.as_deref())
        .await
}

pub async fn get<R: DealsRepository>(repo: &R, id: &str) -> Result<Deal, BillingError> {
    repo.get(id).await
}

pub async fn create<R: DealsRepository>(
    repo: &R,
    body: &CreateDealRequest,
) -> Result<Deal, BillingError> {
    let params = CreateDealParams {
        customer_id: body.customer_id.clone(),
        company: body.company.clone(),
        contact: body.contact.clone(),
        email: body.email.clone(),
        value: body.value.unwrap_or(Decimal::ZERO),
        product_id: body.product_id.clone(),
        product_name: body.product_name.clone(),
        product_type: body.product_type.clone(),
        deal_type: body.deal_type.clone().unwrap_or(DealType::Sale),
        date: body.date.clone(),
        license_key: body.license_key.clone(),
        notes: body.notes.clone(),
        usage_metric_label: body.usage_metric_label.clone(),
        usage_metric_value: body.usage_metric_value,
        auto_create_invoice: body.auto_create_invoice.unwrap_or(false),
    };

    repo.create(&params).await
}

pub async fn update<R: DealsRepository>(
    repo: &R,
    id: &str,
    body: &UpdateDealRequest,
) -> Result<Deal, BillingError> {
    let params = UpdateDealParams {
        customer_id: body.customer_id.clone(),
        company: body.company.clone(),
        contact: body.contact.clone(),
        email: body.email.clone(),
        value: body.value,
        product_id: body.product_id.clone(),
        product_name: body.product_name.clone(),
        product_type: body.product_type.clone(),
        deal_type: body.deal_type.clone(),
        date: body.date.clone(),
        license_key: body.license_key.clone(),
        notes: body.notes.clone(),
        usage_metric_label: body.usage_metric_label.clone(),
        usage_metric_value: body.usage_metric_value,
        auto_create_invoice: body.auto_create_invoice,
    };

    repo.update(id, &params).await
}

pub async fn delete<R: DealsRepository>(
    repo: &R,
    id: &str,
) -> Result<DeleteDealResponse, BillingError> {
    let affected = repo.delete(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("deal", id));
    }

    Ok(DeleteDealResponse { success: true })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rust_decimal::Decimal;
    use rustbill_core::db::models::{Deal, DealType, ProductType};
    use rustbill_core::error::BillingError;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    struct MockDealsRepository {
        create_params: Mutex<Option<CreateDealParams>>,
        update_params: Mutex<Option<UpdateDealParams>>,
        delete_rows: Mutex<u64>,
        delete_called: AtomicBool,
    }

    impl MockDealsRepository {
        fn new(delete_rows: u64) -> Self {
            Self {
                create_params: Mutex::new(None),
                update_params: Mutex::new(None),
                delete_rows: Mutex::new(delete_rows),
                delete_called: AtomicBool::new(false),
            }
        }

        fn sample_deal(id: &str) -> Deal {
            Deal {
                id: id.to_string(),
                customer_id: Some("cust-1".to_string()),
                company: "Sample Co".to_string(),
                contact: "Ops".to_string(),
                email: "ops@example.com".to_string(),
                value: Decimal::ZERO,
                product_id: None,
                product_name: "Sample Product".to_string(),
                product_type: ProductType::Licensed,
                deal_type: DealType::Sale,
                date: "2026-03-19".to_string(),
                license_key: None,
                notes: None,
                usage_metric_label: None,
                usage_metric_value: None,
                created_at: chrono::NaiveDateTime::MIN,
                updated_at: chrono::NaiveDateTime::MIN,
            }
        }
    }

    #[async_trait]
    impl DealsRepository for MockDealsRepository {
        async fn list(
            &self,
            _product_type: Option<&str>,
            _deal_type: Option<&str>,
        ) -> Result<Vec<Deal>, BillingError> {
            Ok(vec![Self::sample_deal("deal-1")])
        }

        async fn get(&self, id: &str) -> Result<Deal, BillingError> {
            Ok(Self::sample_deal(id))
        }

        async fn create(&self, body: &CreateDealParams) -> Result<Deal, BillingError> {
            if let Ok(mut guard) = self.create_params.lock() {
                *guard = Some(body.clone());
            }
            Ok(Self::sample_deal("deal-created"))
        }

        async fn update(&self, _id: &str, body: &UpdateDealParams) -> Result<Deal, BillingError> {
            if let Ok(mut guard) = self.update_params.lock() {
                *guard = Some(body.clone());
            }
            Ok(Self::sample_deal("deal-updated"))
        }

        async fn delete(&self, _id: &str) -> Result<u64, BillingError> {
            self.delete_called.store(true, Ordering::SeqCst);
            if let Ok(guard) = self.delete_rows.lock() {
                Ok(*guard)
            } else {
                Ok(0)
            }
        }
    }

    #[tokio::test]
    async fn create_applies_legacy_defaults() {
        let repo = MockDealsRepository::new(1);
        let created = create(&repo, &CreateDealRequest::default())
            .await
            .expect("deal should be created");

        assert_eq!(created.id, "deal-created");
        let captured = repo.create_params.lock().expect("capture");
        let params = captured.as_ref().expect("create params");
        assert_eq!(params.value, Decimal::ZERO);
        assert_eq!(params.deal_type, DealType::Sale);
        assert!(params.company.is_none());
        assert!(params.product_type.is_none());
    }

    #[tokio::test]
    async fn update_passes_partial_fields_through() {
        let repo = MockDealsRepository::new(1);
        let body = UpdateDealRequest {
            company: Some("Updated Co".to_string()),
            deal_type: Some(DealType::Partner),
            ..Default::default()
        };

        let updated = update(&repo, "deal-1", &body).await.expect("deal update");
        assert_eq!(updated.id, "deal-updated");

        let captured = repo.update_params.lock().expect("capture");
        let params = captured.as_ref().expect("update params");
        assert_eq!(params.company.as_deref(), Some("Updated Co"));
        assert_eq!(params.deal_type, Some(DealType::Partner));
    }

    #[tokio::test]
    async fn delete_returns_not_found_when_no_rows_are_affected() {
        let repo = MockDealsRepository::new(0);
        let err = delete(&repo, "deal-missing").await.unwrap_err();

        match err {
            BillingError::NotFound { entity, id } => {
                assert_eq!(entity, "deal");
                assert_eq!(id, "deal-missing");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
