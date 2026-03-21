use super::repository::PlansRepository;
use super::schema::{CreatePlanRequest, PlanView, UpdatePlanRequest};
use crate::db::models::PricingPlan;
use crate::error::{BillingError, Result};
use validator::Validate;

pub async fn list_plans<R: PlansRepository + ?Sized>(repo: &R) -> Result<Vec<PlanView>> {
    repo.list_plans().await
}

pub async fn get_plan<R: PlansRepository + ?Sized>(repo: &R, id: &str) -> Result<PricingPlan> {
    repo.get_plan(id)
        .await?
        .ok_or_else(|| BillingError::not_found("plan", id))
}

pub async fn create_plan<R: PlansRepository + ?Sized>(
    repo: &R,
    req: CreatePlanRequest,
) -> Result<PricingPlan> {
    req.validate().map_err(BillingError::from_validation)?;
    repo.create_plan(&req).await
}

pub async fn update_plan<R: PlansRepository + ?Sized>(
    repo: &R,
    id: &str,
    req: UpdatePlanRequest,
) -> Result<PricingPlan> {
    req.validate().map_err(BillingError::from_validation)?;

    let _existing = get_plan(repo, id).await?;
    repo.update_plan(id, &req).await
}

pub async fn delete_plan<R: PlansRepository + ?Sized>(repo: &R, id: &str) -> Result<()> {
    let affected = repo.delete_plan(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("plan", id));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{BillingCycle, PricingModel, PricingPlan, PricingTier};
    use async_trait::async_trait;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        plan: Option<PricingPlan>,
        list_rows: Vec<PlanView>,
        create_req: Option<CreatePlanRequest>,
        update_req: Option<(String, UpdatePlanRequest)>,
        delete_rows: u64,
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

    fn sample_plan() -> PricingPlan {
        PricingPlan {
            id: "plan_1".to_string(),
            product_id: Some("prod_1".to_string()),
            name: "Starter".to_string(),
            pricing_model: PricingModel::Flat,
            billing_cycle: BillingCycle::Monthly,
            base_price: Decimal::from(10),
            unit_price: Some(Decimal::from(2)),
            tiers: Some(json!([{ "upTo": 100, "price": 1.5 }])),
            usage_metric_name: Some("api_calls".to_string()),
            trial_days: 14,
            active: true,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    fn sample_plan_view() -> PlanView {
        PlanView {
            id: "plan_1".to_string(),
            product_id: Some("prod_1".to_string()),
            name: "Starter".to_string(),
            pricing_model: PricingModel::Flat,
            billing_cycle: BillingCycle::Monthly,
            base_price: Decimal::from(10),
            unit_price: Some(Decimal::from(2)),
            tiers: Some(json!([{ "upTo": 100, "price": 1.5 }])),
            usage_metric_name: Some("api_calls".to_string()),
            trial_days: 14,
            active: true,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            product_name: Some("Widget".to_string()),
            product_type: None,
        }
    }

    fn sample_create_request() -> CreatePlanRequest {
        CreatePlanRequest {
            product_id: Some("prod_1".to_string()),
            name: "Starter".to_string(),
            pricing_model: PricingModel::Flat,
            billing_cycle: BillingCycle::Monthly,
            base_price: Decimal::from(10),
            unit_price: Some(Decimal::from(2)),
            tiers: Some(vec![PricingTier {
                up_to: Some(100),
                price: 1.5,
            }]),
            usage_metric_name: Some("api_calls".to_string()),
            trial_days: 14,
            active: true,
        }
    }

    fn sample_update_request() -> UpdatePlanRequest {
        UpdatePlanRequest {
            product_id: Some(Some("prod_2".to_string())),
            name: Some("Updated".to_string()),
            pricing_model: Some(PricingModel::Tiered),
            billing_cycle: Some(BillingCycle::Yearly),
            base_price: Some(Decimal::from(20)),
            unit_price: Some(Some(Decimal::from(3))),
            tiers: Some(Some(vec![PricingTier {
                up_to: Some(250),
                price: 2.25,
            }])),
            usage_metric_name: Some(Some("events".to_string())),
            trial_days: Some(30),
            active: Some(false),
        }
    }

    #[async_trait]
    impl PlansRepository for StubRepo {
        async fn list_plans(&self) -> Result<Vec<PlanView>> {
            Ok(self.state.lock().expect("mutex").list_rows.clone())
        }

        async fn get_plan(&self, _id: &str) -> Result<Option<PricingPlan>> {
            Ok(self.state.lock().expect("mutex").plan.clone())
        }

        async fn create_plan(&self, req: &CreatePlanRequest) -> Result<PricingPlan> {
            self.state.lock().expect("mutex").create_req = Some(req.clone());
            Ok(sample_plan())
        }

        async fn update_plan(&self, id: &str, req: &UpdatePlanRequest) -> Result<PricingPlan> {
            self.state.lock().expect("mutex").update_req = Some((id.to_string(), req.clone()));
            Ok(sample_plan())
        }

        async fn delete_plan(&self, _id: &str) -> Result<u64> {
            Ok(self.state.lock().expect("mutex").delete_rows)
        }
    }

    #[tokio::test]
    async fn list_plans_forwards_repository_rows() {
        let repo = StubRepo::with_state(StubState {
            list_rows: vec![sample_plan_view()],
            ..StubState::default()
        });

        let rows = list_plans(&repo).await.expect("list_plans");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Starter");
    }

    #[tokio::test]
    async fn create_plan_validates_and_forwards() {
        let repo = StubRepo::with_state(StubState::default());

        let created = create_plan(&repo, sample_create_request())
            .await
            .expect("create_plan");

        let state = repo.state.lock().expect("mutex");
        assert_eq!(created.id, "plan_1");
        assert!(state.create_req.is_some());
    }

    #[tokio::test]
    async fn update_plan_returns_not_found_when_missing() {
        let repo = StubRepo::with_state(StubState::default());

        let result = update_plan(&repo, "plan_1", sample_update_request()).await;

        assert!(matches!(
            result,
            Err(BillingError::NotFound { entity: "plan", id }) if id == "plan_1"
        ));
    }

    #[tokio::test]
    async fn delete_plan_maps_zero_rows_to_not_found() {
        let repo = StubRepo::with_state(StubState::default());

        let result = delete_plan(&repo, "plan_1").await;

        assert!(matches!(
            result,
            Err(BillingError::NotFound { entity: "plan", id }) if id == "plan_1"
        ));
    }
}
