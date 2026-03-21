use rust_decimal::Decimal;

use crate::billing::proration::calculate_proration;
use crate::error::Result;

use super::repository::PlanChangeRepository;
use super::schema::{ChangePlanInput, ChangePlanOutput, ChangePlanWork};

pub async fn change_plan_with_proration<R: PlanChangeRepository + ?Sized>(
    repo: &R,
    input: ChangePlanInput<'_>,
) -> Result<ChangePlanOutput> {
    let subscription = repo
        .find_subscription_for_update(input.subscription_id)
        .await?;

    if let Some(key) = input.idempotency_key {
        if let Some(invoice) = repo
            .find_proration_invoice(input.subscription_id, key)
            .await?
        {
            return Ok(ChangePlanOutput {
                subscription,
                invoice: Some(invoice),
                already_processed: true,
                proration_net: Decimal::ZERO,
                old_plan_name: String::new(),
                new_plan_name: String::new(),
                customer_id: String::new(),
            });
        }
    }

    let old_plan = repo.get_plan(&subscription.plan_id).await?;
    let new_plan = repo.get_plan(input.new_plan_id).await?;
    let new_quantity = input.new_quantity.unwrap_or(subscription.quantity);
    let proration = calculate_proration(
        &old_plan,
        &new_plan,
        subscription.quantity,
        new_quantity,
        subscription.current_period_start,
        subscription.current_period_end,
        input.now,
    )?;
    let currency = repo
        .get_currency_for_subscription(input.subscription_id)
        .await?;

    let work = ChangePlanWork {
        subscription,
        old_plan,
        new_plan,
        new_quantity,
        currency,
        proration,
        now: input.now,
        idempotency_key: input.idempotency_key.map(str::to_string),
    };

    repo.apply_change_plan(&work).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{
        BillingCycle, Invoice, InvoiceStatus, PricingModel, PricingPlan, Subscription,
        SubscriptionStatus,
    };
    use crate::error::BillingError;
    use async_trait::async_trait;
    use chrono::Utc;
    use rust_decimal::prelude::FromPrimitive;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct StubState {
        subscription: Option<Subscription>,
        existing_invoice: Option<Invoice>,
        old_plan: Option<PricingPlan>,
        new_plan: Option<PricingPlan>,
        currency: String,
        applied: Option<ChangePlanWork>,
        output: Option<ChangePlanOutput>,
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

    #[async_trait]
    impl PlanChangeRepository for StubRepo {
        async fn find_subscription_for_update(
            &self,
            _subscription_id: &str,
        ) -> Result<Subscription> {
            self.state
                .lock()
                .expect("mutex")
                .subscription
                .clone()
                .ok_or_else(|| BillingError::not_found("subscription", "sub_1"))
        }

        async fn find_proration_invoice(
            &self,
            _subscription_id: &str,
            _idempotency_key: &str,
        ) -> Result<Option<Invoice>> {
            Ok(self.state.lock().expect("mutex").existing_invoice.clone())
        }

        async fn get_plan(&self, plan_id: &str) -> Result<PricingPlan> {
            let state = self.state.lock().expect("mutex");
            if state
                .old_plan
                .as_ref()
                .map(|plan| plan.id == plan_id)
                .unwrap_or(false)
            {
                return Ok(state.old_plan.clone().expect("old_plan"));
            }
            if state
                .new_plan
                .as_ref()
                .map(|plan| plan.id == plan_id)
                .unwrap_or(false)
            {
                return Ok(state.new_plan.clone().expect("new_plan"));
            }
            Err(BillingError::not_found("pricing_plan", plan_id))
        }

        async fn get_currency_for_subscription(&self, _subscription_id: &str) -> Result<String> {
            Ok(self.state.lock().expect("mutex").currency.clone())
        }

        async fn apply_change_plan(&self, work: &ChangePlanWork) -> Result<ChangePlanOutput> {
            let mut state = self.state.lock().expect("mutex");
            state.applied = Some(work.clone());
            Ok(state.output.clone().expect("output"))
        }
    }

    fn sample_subscription() -> Subscription {
        let now = Utc::now().naive_utc();
        Subscription {
            id: "sub_1".to_string(),
            customer_id: "cust_1".to_string(),
            plan_id: "plan_old".to_string(),
            status: SubscriptionStatus::Active,
            current_period_start: now - chrono::Duration::days(15),
            current_period_end: now + chrono::Duration::days(15),
            canceled_at: None,
            cancel_at_period_end: false,
            trial_end: None,
            quantity: 1,
            metadata: None,
            stripe_subscription_id: None,
            managed_by: None,
            version: 1,
            deleted_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn sample_plan(id: &str, name: &str, price: f64) -> PricingPlan {
        let now = Utc::now().naive_utc();
        PricingPlan {
            id: id.to_string(),
            product_id: None,
            name: name.to_string(),
            pricing_model: PricingModel::Flat,
            billing_cycle: BillingCycle::Monthly,
            base_price: Decimal::from_f64(price).expect("price"),
            unit_price: None,
            tiers: None,
            usage_metric_name: None,
            trial_days: 0,
            active: true,
            created_at: now,
            updated_at: now,
        }
    }

    fn sample_invoice() -> Invoice {
        let now = Utc::now().naive_utc();
        Invoice {
            id: "inv_1".to_string(),
            invoice_number: "INV-00000001".to_string(),
            customer_id: "cust_1".to_string(),
            subscription_id: Some("sub_1".to_string()),
            status: InvoiceStatus::Issued,
            issued_at: Some(now),
            due_at: None,
            paid_at: None,
            subtotal: Decimal::from(10),
            tax: Decimal::ZERO,
            total: Decimal::from(10),
            currency: "USD".to_string(),
            notes: None,
            stripe_invoice_id: None,
            xendit_invoice_id: None,
            lemonsqueezy_order_id: None,
            version: 1,
            deleted_at: None,
            created_at: now,
            updated_at: now,
            tax_name: None,
            tax_rate: None,
            tax_inclusive: false,
            credits_applied: Decimal::ZERO,
            amount_due: Decimal::from(10),
            auto_charge_attempts: 0,
            idempotency_key: Some("idem-1".to_string()),
        }
    }

    #[tokio::test]
    async fn idempotency_returns_existing_invoice() {
        let repo = StubRepo::with_state(StubState {
            subscription: Some(sample_subscription()),
            existing_invoice: Some(sample_invoice()),
            old_plan: Some(sample_plan("plan_old", "Old", 100.0)),
            new_plan: Some(sample_plan("plan_new", "New", 200.0)),
            currency: "USD".to_string(),
            ..StubState::default()
        });

        let result = change_plan_with_proration(
            &repo,
            ChangePlanInput {
                subscription_id: "sub_1",
                new_plan_id: "plan_new",
                new_quantity: None,
                idempotency_key: Some("idem-1"),
                now: Utc::now().naive_utc(),
            },
        )
        .await
        .expect("plan change");

        assert!(result.already_processed);
        assert!(result.invoice.is_some());
        assert_eq!(result.proration_net, Decimal::ZERO);
    }

    #[tokio::test]
    async fn delegates_to_repository_for_new_change() {
        let repo = StubRepo::with_state(StubState {
            subscription: Some(sample_subscription()),
            old_plan: Some(sample_plan("plan_old", "Old", 100.0)),
            new_plan: Some(sample_plan("plan_new", "New", 200.0)),
            currency: "USD".to_string(),
            output: Some(ChangePlanOutput {
                subscription: sample_subscription(),
                invoice: None,
                already_processed: false,
                proration_net: Decimal::from(10),
                old_plan_name: "Old".to_string(),
                new_plan_name: "New".to_string(),
                customer_id: "cust_1".to_string(),
            }),
            ..StubState::default()
        });

        let result = change_plan_with_proration(
            &repo,
            ChangePlanInput {
                subscription_id: "sub_1",
                new_plan_id: "plan_new",
                new_quantity: Some(2),
                idempotency_key: None,
                now: Utc::now().naive_utc(),
            },
        )
        .await
        .expect("plan change");

        let state = repo.state.lock().expect("mutex");
        assert!(state.applied.is_some());
        assert!(!result.already_processed);
    }

    #[tokio::test]
    async fn missing_subscription_returns_not_found() {
        let repo = StubRepo::default();
        let err = change_plan_with_proration(
            &repo,
            ChangePlanInput {
                subscription_id: "sub_1",
                new_plan_id: "plan_new",
                new_quantity: None,
                idempotency_key: None,
                now: Utc::now().naive_utc(),
            },
        )
        .await
        .expect_err("should fail");

        assert!(err.to_string().contains("not found"));
    }
}
