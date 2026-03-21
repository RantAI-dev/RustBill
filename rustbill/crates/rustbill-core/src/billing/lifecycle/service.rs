use super::repository::LifecycleRepository;
use super::schema::{LifecycleResult, RenewalInvoiceOutput, SalesEventSpec};
use crate::analytics::sales_ledger::SalesClassification;
use crate::billing::auto_charge::ChargeResult;
use crate::billing::tiered_pricing;
use crate::db::models::{
    BillingCycle, InvoiceStatus, PricingTier, Subscription, SubscriptionStatus,
};
use crate::error::Result;
use chrono::{NaiveDateTime, Utc};
use rust_decimal::Decimal;

const DEFAULT_PRE_RENEWAL_INVOICE_DAYS: i64 = 7;
const MAX_PRE_RENEWAL_INVOICE_DAYS: i64 = 90;
const DEFAULT_CURRENCY: &str = "USD";

pub async fn run_full_lifecycle<R: LifecycleRepository + ?Sized>(
    repo: &R,
) -> Result<LifecycleResult> {
    let mut result = LifecycleResult::default();

    result.trials_converted = convert_expired_trials(repo).await?;
    result.canceled = cancel_at_period_end(repo).await?;

    let (pre_generated, pregen_errors) = pre_generate_upcoming_invoices(repo).await?;
    result.pre_generated = pre_generated;
    result.errors.extend(pregen_errors);

    let (renewed, invoiced, errors) = renew_active_subscriptions(repo).await?;
    result.renewed = renewed;
    result.invoices_generated = invoiced;
    result.errors.extend(errors);

    tracing::info!(
        trials_converted = result.trials_converted,
        canceled = result.canceled,
        pre_generated = result.pre_generated,
        renewed = result.renewed,
        invoices_generated = result.invoices_generated,
        "Full lifecycle completed"
    );

    Ok(result)
}

pub async fn generate_pending_invoices<R: LifecycleRepository + ?Sized>(repo: &R) -> Result<u64> {
    let result = run_full_lifecycle(repo).await?;
    Ok(result.invoices_generated)
}

async fn convert_expired_trials<R: LifecycleRepository + ?Sized>(repo: &R) -> Result<u64> {
    let now = Utc::now().naive_utc();
    let transitions = repo.convert_expired_trials(now).await?;
    let count = transitions.len() as u64;

    if count > 0 {
        tracing::info!(count, "Converted expired trials to active");
        for transition in transitions {
            emit_mrr_change_event(
                repo,
                &transition.before,
                &transition.after,
                "lifecycle_trial_convert",
            )
            .await;
        }
    }

    Ok(count)
}

async fn cancel_at_period_end<R: LifecycleRepository + ?Sized>(repo: &R) -> Result<u64> {
    let now = Utc::now().naive_utc();
    let transitions = repo.cancel_at_period_end(now).await?;
    let count = transitions.len() as u64;

    if count > 0 {
        tracing::info!(count, "Canceled subscriptions at period end");
        for transition in transitions {
            emit_mrr_change_event(
                repo,
                &transition.before,
                &transition.after,
                "lifecycle_cancel_at_period_end",
            )
            .await;
        }
    }

    Ok(count)
}

async fn pre_generate_upcoming_invoices<R: LifecycleRepository + ?Sized>(
    repo: &R,
) -> Result<(u64, Vec<String>)> {
    let now = Utc::now().naive_utc();
    let window_end = now + chrono::Duration::days(MAX_PRE_RENEWAL_INVOICE_DAYS);
    let mut generated = 0;
    let mut errors = Vec::new();

    let subs = repo
        .list_pre_generatable_subscriptions(now, window_end)
        .await?;
    for sub in &subs {
        let lead_days = pre_renewal_invoice_days(sub);
        if lead_days <= 0 {
            continue;
        }

        let days_until_end = days_until(now, sub.current_period_end);
        if days_until_end <= 0 || days_until_end > lead_days {
            continue;
        }

        match repo.ensure_renewal_invoice(sub, now, true).await {
            Ok(Some(output)) => {
                generated += 1;
                tracing::info!(
                    subscription_id = %sub.id,
                    invoice_id = %output.invoice.id,
                    invoice_number = %output.invoice.invoice_number,
                    "Pre-generated renewal invoice"
                );
            }
            Ok(None) => {}
            Err(err) => {
                tracing::error!(
                    subscription_id = %sub.id,
                    error = %err,
                    "Failed to pre-generate renewal invoice"
                );
                errors.push(format!("sub {}: {}", sub.id, err));
            }
        }
    }

    Ok((generated, errors))
}

async fn renew_active_subscriptions<R: LifecycleRepository + ?Sized>(
    repo: &R,
) -> Result<(u64, u64, Vec<String>)> {
    let now = Utc::now().naive_utc();
    let mut renewed = 0;
    let mut invoiced = 0;
    let mut errors = Vec::new();

    let subs = repo.list_renewable_subscriptions(now).await?;
    for sub in &subs {
        match renew_single_subscription(repo, sub, now).await {
            Ok(()) => {
                renewed += 1;
                invoiced += 1;
            }
            Err(err) => {
                tracing::error!(subscription_id = %sub.id, error = %err, "Failed to renew subscription");
                errors.push(format!("sub {}: {}", sub.id, err));
            }
        }
    }

    Ok((renewed, invoiced, errors))
}

async fn renew_single_subscription<R: LifecycleRepository + ?Sized>(
    repo: &R,
    sub: &Subscription,
    now: NaiveDateTime,
) -> Result<()> {
    let plan = repo.find_plan(&sub.plan_id).await?;
    let new_period_end = advance_period(sub.current_period_end, &plan.billing_cycle);
    let invoice_output = if let Some(existing) = repo
        .find_existing_renewal_invoice(sub, sub.current_period_end)
        .await?
    {
        RenewalInvoiceOutput {
            invoice: existing.clone(),
            final_amount_due: existing.amount_due,
            total: existing.total,
            invoice_number: existing.invoice_number.clone(),
            plan_name: plan.name.clone(),
            new_period_end,
        }
    } else {
        repo.ensure_renewal_invoice(sub, now, false)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "renewal invoice creation returned no invoice for subscription {}",
                    sub.id
                )
            })?
    };

    repo.advance_subscription_period(&sub.id, invoice_output.new_period_end)
        .await?;

    let invoice = invoice_output.invoice.clone();
    let invoice_id = invoice.id.clone();
    let final_amount_due = invoice_output.final_amount_due;

    if final_amount_due > Decimal::ZERO && invoice.status != InvoiceStatus::Paid {
        if let Ok(Some(payment_method)) = repo.get_default_payment_method(&sub.customer_id).await {
            match repo.try_auto_charge(&invoice, &payment_method).await {
                Ok(ChargeResult::Success { provider_reference }) => {
                    match repo
                        .settle_auto_charge_success(
                            &invoice,
                            &payment_method,
                            provider_reference.as_deref(),
                        )
                        .await
                    {
                        Ok(()) => tracing::info!("Auto-charge settled invoice {}", invoice_id),
                        Err(err) => tracing::error!(
                            "Auto-charge payment settlement failed for invoice {}: {}",
                            invoice_id,
                            err
                        ),
                    }
                }
                Ok(ChargeResult::PermanentFailure(reason)) => {
                    tracing::warn!(
                        "Auto-charge permanently failed for invoice {}: {}",
                        invoice_id,
                        reason
                    );
                    repo.mark_payment_method_failed(&payment_method.id)
                        .await
                        .ok();
                }
                Ok(result) => {
                    tracing::info!(
                        "Auto-charge result for invoice {}: {:?}",
                        invoice_id,
                        result
                    );
                }
                Err(err) => {
                    tracing::error!("Auto-charge error for invoice {}: {}", invoice_id, err);
                }
            }
        }
    }

    repo.dispatch_subscription_renewed_side_effects(sub, &invoice_output)
        .await?;

    tracing::info!(
        subscription_id = %sub.id,
        invoice_number = %invoice_output.invoice_number,
        total = %invoice_output.total,
        "Subscription renewed with invoice"
    );

    if let Err(err) = repo
        .emit_sales_event(SalesEventSpec {
            event_type: "subscription.renewed",
            classification: SalesClassification::Recurring,
            amount_subtotal: invoice_output.total,
            amount_tax: Decimal::ZERO,
            amount_total: invoice_output.total,
            currency: DEFAULT_CURRENCY.to_string(),
            customer_id: Some(sub.customer_id.clone()),
            subscription_id: Some(sub.id.clone()),
            invoice_id: Some(invoice_id.clone()),
            payment_id: None,
            source_table: "invoices",
            source_id: invoice_id,
            metadata: Some(serde_json::json!({
                "invoice_number": invoice_output.invoice_number,
            })),
        })
        .await
    {
        tracing::warn!(error = %err, subscription_id = %sub.id, "failed to emit sales event subscription.renewed");
    }

    Ok(())
}

fn advance_period(from: NaiveDateTime, cycle: &BillingCycle) -> NaiveDateTime {
    crate::billing::subscriptions::advance_period(from, cycle)
}

fn contributes_to_mrr(status: &SubscriptionStatus) -> bool {
    matches!(
        status,
        SubscriptionStatus::Active | SubscriptionStatus::PastDue
    )
}

async fn subscription_mrr<R: LifecycleRepository + ?Sized>(
    repo: &R,
    sub: &Subscription,
) -> Result<Decimal> {
    let plan = repo.find_plan(&sub.plan_id).await?;
    let tiers: Option<Vec<PricingTier>> = plan
        .tiers
        .as_ref()
        .and_then(|value| serde_json::from_value(value.clone()).ok());

    Ok(tiered_pricing::calculate_amount(
        &plan.pricing_model,
        plan.base_price,
        plan.unit_price,
        tiers.as_deref(),
        sub.quantity,
    ))
}

async fn emit_mrr_change_event<R: LifecycleRepository + ?Sized>(
    repo: &R,
    before: &Subscription,
    after: &Subscription,
    trigger: &str,
) {
    let old_mrr = match subscription_mrr(repo, before).await {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!(error = %err, subscription_id = %before.id, "failed to compute previous MRR");
            return;
        }
    };
    let new_mrr = match subscription_mrr(repo, after).await {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!(error = %err, subscription_id = %after.id, "failed to compute current MRR");
            return;
        }
    };

    let old_effective = if contributes_to_mrr(&before.status) {
        old_mrr
    } else {
        Decimal::ZERO
    };
    let new_effective = if contributes_to_mrr(&after.status) {
        new_mrr
    } else {
        Decimal::ZERO
    };

    let delta = new_effective - old_effective;
    if delta == Decimal::ZERO {
        return;
    }

    let event_type = if delta > Decimal::ZERO {
        "mrr_expanded"
    } else if new_effective == Decimal::ZERO
        && old_effective > Decimal::ZERO
        && matches!(after.status, SubscriptionStatus::Canceled)
    {
        "mrr_churned"
    } else {
        "mrr_contracted"
    };

    let amount = delta.abs();
    let source_id = format!("{}:v{}", after.id, after.version);
    if let Err(err) = repo
        .emit_sales_event(SalesEventSpec {
            event_type,
            classification: SalesClassification::Recurring,
            amount_subtotal: amount,
            amount_tax: Decimal::ZERO,
            amount_total: amount,
            currency: DEFAULT_CURRENCY.to_string(),
            customer_id: Some(after.customer_id.clone()),
            subscription_id: Some(after.id.clone()),
            invoice_id: None,
            payment_id: None,
            source_table: "subscription_revisions",
            source_id,
            metadata: Some(serde_json::json!({
                "trigger": trigger,
                "from_status": before.status,
                "to_status": after.status,
                "from_plan_id": before.plan_id,
                "to_plan_id": after.plan_id,
                "from_quantity": before.quantity,
                "to_quantity": after.quantity,
            })),
        })
        .await
    {
        tracing::warn!(error = %err, subscription_id = %after.id, "failed to emit recurring MRR change event");
    }
}

fn pre_renewal_invoice_days(sub: &Subscription) -> i64 {
    let from_metadata = sub
        .metadata
        .as_ref()
        .and_then(|meta| meta.as_object())
        .and_then(|obj| {
            obj.get("preRenewalInvoiceDays")
                .or_else(|| obj.get("pre_renewal_invoice_days"))
        })
        .and_then(|value| value.as_i64());

    from_metadata
        .unwrap_or(DEFAULT_PRE_RENEWAL_INVOICE_DAYS)
        .clamp(0, MAX_PRE_RENEWAL_INVOICE_DAYS)
}

fn days_until(from: NaiveDateTime, to: NaiveDateTime) -> i64 {
    let seconds = (to - from).num_seconds();
    if seconds <= 0 {
        0
    } else {
        (seconds + 86_399) / 86_400
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{
        BillingCycle, PaymentProvider, PricingModel, PricingPlan, SavedPaymentMethod,
        SavedPaymentMethodStatus, SavedPaymentMethodType,
    };
    use async_trait::async_trait;
    use chrono::{NaiveDate, Utc};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct StubState {
        plan: Option<PricingPlan>,
        emitted_events: Vec<SalesEventSpec>,
    }

    #[derive(Clone, Default)]
    struct StubRepo {
        state: Arc<Mutex<StubState>>,
    }

    #[async_trait]
    impl LifecycleRepository for StubRepo {
        async fn convert_expired_trials(
            &self,
            _now: NaiveDateTime,
        ) -> Result<Vec<super::super::schema::SubscriptionTransition>> {
            Ok(Vec::new())
        }

        async fn cancel_at_period_end(
            &self,
            _now: NaiveDateTime,
        ) -> Result<Vec<super::super::schema::SubscriptionTransition>> {
            Ok(Vec::new())
        }

        async fn list_pre_generatable_subscriptions(
            &self,
            _now: NaiveDateTime,
            _window_end: NaiveDateTime,
        ) -> Result<Vec<Subscription>> {
            Ok(Vec::new())
        }

        async fn list_renewable_subscriptions(
            &self,
            _now: NaiveDateTime,
        ) -> Result<Vec<Subscription>> {
            Ok(Vec::new())
        }

        async fn find_plan(&self, _plan_id: &str) -> Result<PricingPlan> {
            Ok(self
                .state
                .lock()
                .expect("mutex")
                .plan
                .clone()
                .expect("plan"))
        }

        async fn find_existing_renewal_invoice(
            &self,
            _sub: &Subscription,
            _period_start: NaiveDateTime,
        ) -> Result<Option<crate::db::models::Invoice>> {
            Ok(None)
        }

        async fn ensure_renewal_invoice(
            &self,
            _sub: &Subscription,
            _now: NaiveDateTime,
            _notify_invoice_issued: bool,
        ) -> Result<Option<RenewalInvoiceOutput>> {
            Ok(None)
        }

        async fn advance_subscription_period(
            &self,
            _subscription_id: &str,
            _new_period_end: NaiveDateTime,
        ) -> Result<()> {
            Ok(())
        }

        async fn get_default_payment_method(
            &self,
            _customer_id: &str,
        ) -> Result<Option<SavedPaymentMethod>> {
            Ok(None)
        }

        async fn try_auto_charge(
            &self,
            _invoice: &crate::db::models::Invoice,
            _payment_method: &SavedPaymentMethod,
        ) -> Result<ChargeResult> {
            Ok(ChargeResult::ManagedExternally)
        }

        async fn settle_auto_charge_success(
            &self,
            _invoice: &crate::db::models::Invoice,
            _payment_method: &SavedPaymentMethod,
            _provider_reference: Option<&str>,
        ) -> Result<()> {
            Ok(())
        }

        async fn mark_payment_method_failed(&self, _method_id: &str) -> Result<()> {
            Ok(())
        }

        async fn emit_sales_event(&self, event: SalesEventSpec) -> Result<()> {
            self.state.lock().expect("mutex").emitted_events.push(event);
            Ok(())
        }

        async fn dispatch_subscription_renewed_side_effects(
            &self,
            _subscription: &Subscription,
            _output: &RenewalInvoiceOutput,
        ) -> Result<()> {
            Ok(())
        }
    }

    fn sample_plan(price: i64) -> PricingPlan {
        PricingPlan {
            id: "plan_1".to_string(),
            product_id: None,
            name: "Starter".to_string(),
            pricing_model: PricingModel::Flat,
            billing_cycle: BillingCycle::Monthly,
            base_price: Decimal::from(price),
            unit_price: None,
            tiers: None,
            usage_metric_name: None,
            trial_days: 0,
            active: true,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    fn sample_subscription(status: SubscriptionStatus) -> Subscription {
        Subscription {
            id: "sub_1".to_string(),
            customer_id: "cus_1".to_string(),
            plan_id: "plan_1".to_string(),
            status,
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
            version: 2,
            deleted_at: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    #[test]
    fn pre_renewal_invoice_days_defaults_and_clamps() {
        let mut sub = sample_subscription(SubscriptionStatus::Active);
        assert_eq!(pre_renewal_invoice_days(&sub), 7);

        sub.metadata = Some(serde_json::json!({"preRenewalInvoiceDays": 120}));
        assert_eq!(pre_renewal_invoice_days(&sub), 90);

        sub.metadata = Some(serde_json::json!({"pre_renewal_invoice_days": -4}));
        assert_eq!(pre_renewal_invoice_days(&sub), 0);
    }

    #[test]
    fn days_until_rounds_up_partial_days() {
        let from = NaiveDate::from_ymd_opt(2026, 1, 1)
            .expect("date")
            .and_hms_opt(0, 0, 0)
            .expect("time");
        let same_day = from + chrono::Duration::hours(12);
        let past = from - chrono::Duration::hours(1);

        assert_eq!(days_until(from, same_day), 1);
        assert_eq!(days_until(from, past), 0);
    }

    #[tokio::test]
    async fn emit_mrr_change_event_marks_churn_correctly() {
        let repo = StubRepo {
            state: Arc::new(Mutex::new(StubState {
                plan: Some(sample_plan(25)),
                emitted_events: Vec::new(),
            })),
        };
        let before = sample_subscription(SubscriptionStatus::Active);
        let mut after = sample_subscription(SubscriptionStatus::Canceled);
        after.version = 3;

        emit_mrr_change_event(&repo, &before, &after, "test_trigger").await;

        let state = repo.state.lock().expect("mutex");
        assert_eq!(state.emitted_events.len(), 1);
        assert_eq!(state.emitted_events[0].event_type, "mrr_churned");
        assert_eq!(
            state.emitted_events[0].source_table,
            "subscription_revisions"
        );
    }

    #[tokio::test]
    async fn emit_mrr_change_event_skips_zero_delta() {
        let repo = StubRepo {
            state: Arc::new(Mutex::new(StubState {
                plan: Some(sample_plan(25)),
                emitted_events: Vec::new(),
            })),
        };
        let before = sample_subscription(SubscriptionStatus::Active);
        let mut after = sample_subscription(SubscriptionStatus::PastDue);
        after.version = 4;

        emit_mrr_change_event(&repo, &before, &after, "test_trigger").await;

        let state = repo.state.lock().expect("mutex");
        assert!(state.emitted_events.is_empty());
    }

    #[allow(dead_code)]
    fn _sample_method() -> SavedPaymentMethod {
        SavedPaymentMethod {
            id: "pm_1".to_string(),
            customer_id: "cus_1".to_string(),
            provider: PaymentProvider::Stripe,
            provider_token: "pm_token".to_string(),
            method_type: SavedPaymentMethodType::Card,
            label: "card".to_string(),
            last_four: Some("4242".to_string()),
            expiry_month: Some(12),
            expiry_year: Some(2030),
            is_default: true,
            status: SavedPaymentMethodStatus::Active,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }
}
