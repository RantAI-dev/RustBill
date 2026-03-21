use super::repository::TaxRulesRepository;
use super::schema::{CreateTaxRuleRequest, UpdateTaxRuleRequest};
use rustbill_core::db::models::TaxRule;
use rustbill_core::error::BillingError;

pub async fn list<R: TaxRulesRepository>(repo: &R) -> Result<Vec<TaxRule>, BillingError> {
    repo.list().await
}

pub async fn create<R: TaxRulesRepository>(
    repo: &R,
    body: &CreateTaxRuleRequest,
) -> Result<TaxRule, BillingError> {
    repo.create(body).await
}

pub async fn update<R: TaxRulesRepository>(
    repo: &R,
    id: &str,
    body: &UpdateTaxRuleRequest,
) -> Result<TaxRule, BillingError> {
    repo.update(id, body).await
}

pub async fn remove<R: TaxRulesRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.remove(id).await?;
    Ok(serde_json::json!({ "deleted": true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::billing::tax_rules::repository::TaxRulesRepository;
    use async_trait::async_trait;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use rustbill_core::db::models::TaxRule;
    use std::sync::Mutex;

    fn sample_rule(id: &str) -> TaxRule {
        TaxRule {
            id: id.to_string(),
            country: "US".to_string(),
            region: Some("NY".to_string()),
            tax_name: "Sales Tax".to_string(),
            rate: Decimal::new(800, 4),
            inclusive: false,
            product_category: None,
            active: true,
            effective_from: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
            effective_to: None,
            created_at: NaiveDate::from_ymd_opt(2026, 1, 1)
                .expect("valid date")
                .and_hms_opt(0, 0, 0)
                .expect("valid time"),
        }
    }

    struct MockRepo {
        last_created: Mutex<Option<CreateTaxRuleRequest>>,
        last_updated: Mutex<Option<(String, UpdateTaxRuleRequest)>>,
        removed: Mutex<Option<String>>,
    }

    impl MockRepo {
        fn new() -> Self {
            Self {
                last_created: Mutex::new(None),
                last_updated: Mutex::new(None),
                removed: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl TaxRulesRepository for MockRepo {
        async fn list(&self) -> Result<Vec<TaxRule>, BillingError> {
            Ok(vec![sample_rule("tr-1")])
        }

        async fn create(&self, body: &CreateTaxRuleRequest) -> Result<TaxRule, BillingError> {
            *self.last_created.lock().expect("lock poisoned") = Some(body.clone());
            Ok(sample_rule("tr-new"))
        }

        async fn update(
            &self,
            id: &str,
            body: &UpdateTaxRuleRequest,
        ) -> Result<TaxRule, BillingError> {
            *self.last_updated.lock().expect("lock poisoned") =
                Some((id.to_string(), body.clone()));
            Ok(sample_rule("tr-updated"))
        }

        async fn remove(&self, id: &str) -> Result<(), BillingError> {
            *self.removed.lock().expect("lock poisoned") = Some(id.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn create_forwards_request_fields() {
        let repo = MockRepo::new();
        let body = CreateTaxRuleRequest {
            country: "US".to_string(),
            region: Some("CA".to_string()),
            tax_name: "Sales Tax".to_string(),
            rate: Decimal::new(825, 4),
            inclusive: false,
            product_category: Some("software".to_string()),
        };

        let rule = create(&repo, &body).await.expect("create should succeed");
        assert_eq!(rule.id, "tr-new");
        let captured = repo
            .last_created
            .lock()
            .expect("lock poisoned")
            .clone()
            .expect("request should be captured");
        assert_eq!(captured.country, "US");
        assert_eq!(captured.product_category.as_deref(), Some("software"));
    }

    #[tokio::test]
    async fn remove_wraps_deleted_flag() {
        let repo = MockRepo::new();
        let result = remove(&repo, "tr-1").await.expect("remove should succeed");
        assert_eq!(result["deleted"], serde_json::json!(true));
        assert_eq!(
            repo.removed
                .lock()
                .expect("lock poisoned")
                .clone()
                .expect("id should be captured"),
            "tr-1"
        );
    }
}
