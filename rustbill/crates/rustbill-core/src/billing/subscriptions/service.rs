use super::repository::SubscriptionRepository;
use super::schema::{
    CreateSubscriptionDraft, CreateSubscriptionRequest, ListSubscriptionsFilter, SubscriptionView,
    UpdateSubscriptionRequest,
};
use crate::db::models::{BillingCycle, Subscription, SubscriptionStatus};
use crate::error::{BillingError, Result};
use chrono::NaiveDateTime;
use validator::Validate;

pub async fn list_subscriptions<R: SubscriptionRepository + ?Sized>(
    repo: &R,
    filter: &ListSubscriptionsFilter,
) -> Result<Vec<SubscriptionView>> {
    repo.list_subscriptions(filter).await
}

pub async fn get_subscription<R: SubscriptionRepository + ?Sized>(
    repo: &R,
    id: &str,
) -> Result<Subscription> {
    repo.get_subscription(id)
        .await?
        .ok_or_else(|| BillingError::not_found("subscription", id))
}

pub async fn create_subscription<R: SubscriptionRepository + ?Sized>(
    repo: &R,
    req: CreateSubscriptionRequest,
) -> Result<Subscription> {
    req.validate().map_err(BillingError::from_validation)?;

    let plan = repo
        .find_plan(&req.plan_id)
        .await?
        .ok_or_else(|| BillingError::not_found("plan", &req.plan_id))?;

    let now = chrono::Utc::now().naive_utc();
    let (status, trial_end, current_period_start, current_period_end) = if plan.trial_days > 0 {
        let trial_end = now + chrono::Duration::days(plan.trial_days as i64);
        (
            SubscriptionStatus::Trialing,
            Some(trial_end),
            now,
            trial_end,
        )
    } else {
        let period_end = advance_period(now, &plan.billing_cycle);
        (SubscriptionStatus::Active, None, now, period_end)
    };

    let draft = CreateSubscriptionDraft {
        customer_id: req.customer_id,
        plan_id: req.plan_id,
        status,
        current_period_start,
        current_period_end,
        trial_end,
        quantity: req.quantity,
        metadata: req.metadata,
        stripe_subscription_id: req.stripe_subscription_id,
    };

    repo.create_subscription(&draft).await
}

pub async fn update_subscription<R: SubscriptionRepository + ?Sized>(
    repo: &R,
    id: &str,
    req: UpdateSubscriptionRequest,
) -> Result<Subscription> {
    req.validate().map_err(BillingError::from_validation)?;

    repo.update_subscription(id, &req).await?.ok_or_else(|| {
        BillingError::conflict(format!(
            "subscription {id} was modified concurrently (version mismatch)"
        ))
    })
}

pub async fn delete_subscription<R: SubscriptionRepository + ?Sized>(
    repo: &R,
    id: &str,
) -> Result<()> {
    let result = repo.delete_subscription(id).await?;
    if result == 0 {
        return Err(BillingError::not_found("subscription", id));
    }
    Ok(())
}

pub async fn run_lifecycle<R: SubscriptionRepository + ?Sized>(repo: &R) -> Result<u64> {
    repo.run_lifecycle().await
}

pub fn advance_period(from: NaiveDateTime, cycle: &BillingCycle) -> NaiveDateTime {
    let date = from.date();
    let time = from.time();
    let months = match cycle {
        BillingCycle::Monthly => 1,
        BillingCycle::Quarterly => 3,
        BillingCycle::Yearly => 12,
    };
    let new_date = date
        .checked_add_months(chrono::Months::new(months))
        .unwrap_or_else(|| date + chrono::Duration::days(months as i64 * 30));
    new_date.and_time(time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{
        BillingCycle, PricingModel, PricingPlan, Subscription, SubscriptionStatus,
    };
    use async_trait::async_trait;
    use chrono::{NaiveDate, Utc};
    use rust_decimal::Decimal;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        plan: Option<PricingPlan>,
        subscription: Option<Subscription>,
        list_rows: Vec<SubscriptionView>,
        created: Option<CreateSubscriptionDraft>,
        updated: Option<(String, UpdateSubscriptionRequest)>,
        deleted_id: Option<String>,
        lifecycle_rows: u64,
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

    fn sample_plan(trial_days: i32) -> PricingPlan {
        PricingPlan {
            id: "plan_1".to_string(),
            product_id: None,
            name: "Starter".to_string(),
            pricing_model: PricingModel::Flat,
            billing_cycle: BillingCycle::Monthly,
            base_price: Decimal::from(10),
            unit_price: Some(Decimal::from(2)),
            tiers: None,
            usage_metric_name: None,
            trial_days,
            active: true,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    fn sample_subscription() -> Subscription {
        Subscription {
            id: "sub_1".to_string(),
            customer_id: "cus_1".to_string(),
            plan_id: "plan_1".to_string(),
            status: SubscriptionStatus::Active,
            current_period_start: NaiveDate::from_ymd_opt(2026, 1, 1)
                .expect("date")
                .and_hms_opt(0, 0, 0)
                .expect("time"),
            current_period_end: NaiveDate::from_ymd_opt(2026, 2, 1)
                .expect("date")
                .and_hms_opt(0, 0, 0)
                .expect("time"),
            canceled_at: None,
            cancel_at_period_end: false,
            trial_end: None,
            quantity: 1,
            metadata: None,
            stripe_subscription_id: None,
            managed_by: None,
            version: 1,
            deleted_at: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    fn sample_request() -> CreateSubscriptionRequest {
        CreateSubscriptionRequest {
            customer_id: "cus_1".to_string(),
            plan_id: "plan_1".to_string(),
            quantity: 2,
            metadata: Some(serde_json::json!({"source":"manual"})),
            stripe_subscription_id: Some("sub_ext".to_string()),
        }
    }

    #[async_trait]
    impl SubscriptionRepository for StubRepo {
        async fn list_subscriptions(
            &self,
            _filter: &ListSubscriptionsFilter,
        ) -> Result<Vec<SubscriptionView>> {
            Ok(self.state.lock().expect("mutex").list_rows.clone())
        }

        async fn get_subscription(&self, _id: &str) -> Result<Option<Subscription>> {
            Ok(self.state.lock().expect("mutex").subscription.clone())
        }

        async fn find_plan(&self, _id: &str) -> Result<Option<PricingPlan>> {
            Ok(self.state.lock().expect("mutex").plan.clone())
        }

        async fn create_subscription(
            &self,
            draft: &CreateSubscriptionDraft,
        ) -> Result<Subscription> {
            let mut state = self.state.lock().expect("mutex");
            state.created = Some(draft.clone());
            Ok(state.subscription.clone().expect("subscription"))
        }

        async fn update_subscription(
            &self,
            id: &str,
            req: &UpdateSubscriptionRequest,
        ) -> Result<Option<Subscription>> {
            let mut state = self.state.lock().expect("mutex");
            state.updated = Some((id.to_string(), req.clone()));
            Ok(state.subscription.clone())
        }

        async fn delete_subscription(&self, id: &str) -> Result<u64> {
            self.state.lock().expect("mutex").deleted_id = Some(id.to_string());
            Ok(1)
        }

        async fn run_lifecycle(&self) -> Result<u64> {
            Ok(self.state.lock().expect("mutex").lifecycle_rows)
        }
    }

    #[tokio::test]
    async fn create_subscription_uses_trial_period() {
        let repo = StubRepo::with_state(StubState {
            plan: Some(sample_plan(14)),
            subscription: Some(sample_subscription()),
            ..StubState::default()
        });

        let sub = create_subscription(&repo, sample_request())
            .await
            .expect("create_subscription");

        let state = repo.state.lock().expect("mutex");
        assert_eq!(sub.id, "sub_1");
        assert_eq!(
            state.created.as_ref().map(|d| d.status.clone()),
            Some(SubscriptionStatus::Trialing)
        );
        assert!(state.created.as_ref().and_then(|d| d.trial_end).is_some());
    }

    #[tokio::test]
    async fn update_subscription_returns_conflict_when_missing() {
        let repo = StubRepo::with_state(StubState::default());

        let err = update_subscription(
            &repo,
            "sub_1",
            UpdateSubscriptionRequest {
                status: Some(SubscriptionStatus::Active),
                quantity: Some(3),
                cancel_at_period_end: Some(false),
                canceled_at: None,
                metadata: None,
                stripe_subscription_id: None,
                version: 1,
            },
        )
        .await
        .expect_err("should fail");

        assert!(matches!(err, BillingError::Conflict(_)));
    }

    #[tokio::test]
    async fn delete_subscription_maps_zero_rows_to_not_found() {
        struct ZeroDeleteRepo;

        #[async_trait]
        impl SubscriptionRepository for ZeroDeleteRepo {
            async fn list_subscriptions(
                &self,
                _filter: &ListSubscriptionsFilter,
            ) -> Result<Vec<SubscriptionView>> {
                Ok(vec![])
            }

            async fn get_subscription(&self, _id: &str) -> Result<Option<Subscription>> {
                Ok(Some(sample_subscription()))
            }

            async fn find_plan(&self, _id: &str) -> Result<Option<PricingPlan>> {
                Ok(Some(sample_plan(0)))
            }

            async fn create_subscription(
                &self,
                _draft: &CreateSubscriptionDraft,
            ) -> Result<Subscription> {
                Ok(sample_subscription())
            }

            async fn update_subscription(
                &self,
                _id: &str,
                _req: &UpdateSubscriptionRequest,
            ) -> Result<Option<Subscription>> {
                Ok(Some(sample_subscription()))
            }

            async fn delete_subscription(&self, _id: &str) -> Result<u64> {
                Ok(0)
            }

            async fn run_lifecycle(&self) -> Result<u64> {
                Ok(0)
            }
        }

        let err = delete_subscription(&ZeroDeleteRepo, "sub_1")
            .await
            .expect_err("should fail");
        assert!(matches!(
            err,
            BillingError::NotFound {
                entity: "subscription",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn run_lifecycle_forwards_to_repository() {
        let repo = StubRepo::with_state(StubState {
            lifecycle_rows: 7,
            ..StubState::default()
        });

        let processed = run_lifecycle(&repo).await.expect("run_lifecycle");
        assert_eq!(processed, 7);
    }
}
