use super::repository::TaxRepository;
use super::schema::{
    calculate_tax as calculate_tax_result, CreateTaxRuleRequest, ResolveTaxRequest, TaxResult,
    UpdateTaxRuleRequest,
};
use crate::db::models::TaxRule;
use crate::error::{BillingError, Result};
use validator::Validate;

pub async fn resolve_tax<R: TaxRepository + ?Sized>(
    repo: &R,
    req: ResolveTaxRequest,
) -> Result<TaxResult> {
    if req.country.trim().is_empty() || req.subtotal <= rust_decimal::Decimal::ZERO {
        return Ok(TaxResult::zero());
    }

    let rule = repo
        .find_tax_rule(&req.country, req.region.as_deref())
        .await?;
    match rule {
        Some(r) => Ok(calculate_tax_result(req.subtotal, &r)),
        None => {
            if let Some(external) = repo
                .resolve_external_tax(
                    &req.country,
                    req.region.as_deref(),
                    req.product_category.as_deref(),
                    req.subtotal,
                )
                .await?
            {
                return Ok(external);
            }
            Ok(TaxResult::zero())
        }
    }
}

pub fn calculate_tax(subtotal: rust_decimal::Decimal, rule: &TaxRule) -> TaxResult {
    calculate_tax_result(subtotal, rule)
}

pub async fn list_tax_rules<R: TaxRepository + ?Sized>(repo: &R) -> Result<Vec<TaxRule>> {
    repo.list_tax_rules().await
}

pub async fn create_tax_rule<R: TaxRepository + ?Sized>(
    repo: &R,
    req: CreateTaxRuleRequest,
) -> Result<TaxRule> {
    req.validate().map_err(BillingError::from_validation)?;
    repo.create_tax_rule(&req).await
}

pub async fn update_tax_rule<R: TaxRepository + ?Sized>(
    repo: &R,
    id: &str,
    req: UpdateTaxRuleRequest,
) -> Result<TaxRule> {
    req.validate().map_err(BillingError::from_validation)?;
    repo.update_tax_rule(id, &req).await
}

pub async fn delete_tax_rule<R: TaxRepository + ?Sized>(repo: &R, id: &str) -> Result<()> {
    repo.delete_tax_rule(id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        rule: Option<TaxRule>,
        external: Option<TaxResult>,
        created: Option<CreateTaxRuleRequest>,
        updated: Option<(String, UpdateTaxRuleRequest)>,
        deleted: Option<String>,
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

    fn sample_rule() -> TaxRule {
        TaxRule {
            id: "tax-1".to_string(),
            country: "US".to_string(),
            region: Some("CA".to_string()),
            tax_name: "Sales Tax".to_string(),
            rate: Decimal::new(825, 4),
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

    #[async_trait]
    impl TaxRepository for StubRepo {
        async fn find_tax_rule(
            &self,
            _country: &str,
            _region: Option<&str>,
        ) -> Result<Option<TaxRule>> {
            Ok(self.state.lock().expect("mutex").rule.clone())
        }

        async fn list_tax_rules(&self) -> Result<Vec<TaxRule>> {
            Ok(vec![sample_rule()])
        }

        async fn create_tax_rule(&self, req: &CreateTaxRuleRequest) -> Result<TaxRule> {
            self.state.lock().expect("mutex").created = Some(req.clone());
            Ok(sample_rule())
        }

        async fn update_tax_rule(&self, id: &str, req: &UpdateTaxRuleRequest) -> Result<TaxRule> {
            self.state.lock().expect("mutex").updated = Some((id.to_string(), req.clone()));
            Ok(sample_rule())
        }

        async fn delete_tax_rule(&self, id: &str) -> Result<()> {
            self.state.lock().expect("mutex").deleted = Some(id.to_string());
            Ok(())
        }

        async fn resolve_external_tax(
            &self,
            _country: &str,
            _region: Option<&str>,
            _product_category: Option<&str>,
            _subtotal: Decimal,
        ) -> Result<Option<TaxResult>> {
            Ok(self.state.lock().expect("mutex").external.clone())
        }
    }

    #[tokio::test]
    async fn resolve_tax_prefers_local_rule() {
        let repo = StubRepo::with_state(StubState {
            rule: Some(sample_rule()),
            external: Some(TaxResult {
                rate: Decimal::new(100, 4),
                amount: Decimal::new(100, 2),
                name: "External".to_string(),
                inclusive: false,
            }),
            ..StubState::default()
        });

        let result = resolve_tax(
            &repo,
            ResolveTaxRequest {
                country: "US".to_string(),
                region: Some("CA".to_string()),
                product_category: None,
                subtotal: Decimal::from(100),
            },
        )
        .await
        .expect("resolve_tax should succeed");

        assert_eq!(result.name, "Sales Tax");
        assert_eq!(result.amount, Decimal::new(825, 2));
    }

    #[tokio::test]
    async fn resolve_tax_uses_external_fallback() {
        let repo = StubRepo::with_state(StubState {
            external: Some(TaxResult {
                rate: Decimal::new(700, 4),
                amount: Decimal::new(700, 2),
                name: "Stripe Tax".to_string(),
                inclusive: false,
            }),
            ..StubState::default()
        });

        let result = resolve_tax(
            &repo,
            ResolveTaxRequest {
                country: "US".to_string(),
                region: None,
                product_category: Some("software".to_string()),
                subtotal: Decimal::from(100),
            },
        )
        .await
        .expect("resolve_tax should succeed");

        assert_eq!(result.name, "Stripe Tax");
        assert_eq!(result.amount, Decimal::new(700, 2));
    }

    #[tokio::test]
    async fn resolve_tax_returns_zero_for_empty_country() {
        let repo = StubRepo::default();
        let result = resolve_tax(
            &repo,
            ResolveTaxRequest {
                country: String::new(),
                region: None,
                product_category: None,
                subtotal: Decimal::from(100),
            },
        )
        .await
        .expect("resolve_tax should succeed");

        assert_eq!(result.amount, Decimal::ZERO);
        assert_eq!(result.name, "");
    }

    #[tokio::test]
    async fn create_update_delete_forward_to_repository() {
        let repo = StubRepo::default();

        let created = create_tax_rule(
            &repo,
            CreateTaxRuleRequest {
                country: "US".to_string(),
                region: Some("CA".to_string()),
                tax_name: "Sales Tax".to_string(),
                rate: Decimal::new(825, 4),
                inclusive: false,
                product_category: Some("software".to_string()),
            },
        )
        .await
        .expect("create_tax_rule should succeed");
        assert_eq!(created.id, "tax-1");

        let updated = update_tax_rule(
            &repo,
            "tax-1",
            UpdateTaxRuleRequest {
                tax_name: "Updated Tax".to_string(),
                rate: Decimal::new(900, 4),
                inclusive: true,
            },
        )
        .await
        .expect("update_tax_rule should succeed");
        assert_eq!(updated.id, "tax-1");

        delete_tax_rule(&repo, "tax-1")
            .await
            .expect("delete_tax_rule should succeed");

        let state = repo.state.lock().expect("mutex");
        assert_eq!(
            state.created.as_ref().map(|r| r.country.as_str()),
            Some("US")
        );
        assert_eq!(
            state.updated.as_ref().map(|(id, _)| id.as_str()),
            Some("tax-1")
        );
        assert_eq!(state.deleted.as_deref(), Some("tax-1"));
    }
}
