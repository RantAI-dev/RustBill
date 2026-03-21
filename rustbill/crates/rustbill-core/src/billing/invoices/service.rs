use super::repository::InvoiceRepository;
use super::schema::{
    AddInvoiceItemRequest, CreateInvoiceDraft, CreateInvoiceRequest, InvoiceItemDraft, InvoiceView,
    ListInvoicesFilter, UpdateInvoiceDraft, UpdateInvoiceRequest,
};
use crate::db::models::{
    BillingCycle, Coupon, DiscountType, Invoice, InvoiceItem, InvoiceStatus, PricingPlan,
    PricingTier, Subscription,
};
use crate::error::{BillingError, Result};
use chrono::Utc;
use rust_decimal::Decimal;
use validator::Validate;

pub async fn list_invoices<R: InvoiceRepository + ?Sized>(
    repo: &R,
    filter: &ListInvoicesFilter,
) -> Result<Vec<InvoiceView>> {
    repo.list_invoices(filter).await
}

pub async fn get_invoice<R: InvoiceRepository + ?Sized>(repo: &R, id: &str) -> Result<Invoice> {
    repo.get_invoice(id)
        .await?
        .ok_or_else(|| BillingError::not_found("invoice", id))
}

pub async fn create_invoice<R: InvoiceRepository + ?Sized>(
    repo: &R,
    req: CreateInvoiceRequest,
) -> Result<Invoice> {
    req.validate().map_err(BillingError::from_validation)?;

    let currency = req.currency.clone().unwrap_or_else(|| "USD".to_string());
    let tax_rate = req.tax_rate.unwrap_or_default();
    let invoice_number = repo.next_invoice_number().await?;

    let mut line_items = Vec::new();

    if let Some(ref sub_id) = req.subscription_id {
        let sub = repo
            .find_subscription(sub_id)
            .await?
            .ok_or_else(|| BillingError::not_found("subscription", sub_id))?;
        let plan = repo
            .find_plan(&sub.plan_id)
            .await?
            .ok_or_else(|| BillingError::not_found("plan", &sub.plan_id))?;

        line_items.push(subscription_line_item(&plan, &sub)?);
    }

    let mut coupon_id_to_increment = None;
    if let Some(ref coupon_code) = req.coupon_code {
        let coupon = repo.find_coupon(coupon_code).await?.ok_or_else(|| {
            BillingError::bad_request(format!("coupon '{coupon_code}' not found or inactive"))
        })?;

        validate_coupon(&coupon)?;

        let subtotal_so_far = line_items.iter().map(|item| item.amount).sum::<Decimal>();
        let discount_amount = discount_amount(&coupon, subtotal_so_far);
        if discount_amount > Decimal::ZERO {
            line_items.push(InvoiceItemDraft {
                description: format!("Discount ({})", coupon.code),
                quantity: Decimal::ONE,
                unit_price: -discount_amount,
                amount: -discount_amount,
                period_start: None,
                period_end: None,
            });
            coupon_id_to_increment = Some(coupon.id.clone());
        }
    }

    let subtotal = line_items.iter().map(|item| item.amount).sum::<Decimal>();
    let tax = (subtotal * tax_rate / Decimal::from(100)).round_dp(2);
    let total = subtotal + tax;

    let draft = CreateInvoiceDraft {
        invoice_number,
        customer_id: req.customer_id,
        subscription_id: req.subscription_id,
        due_at: req.due_at,
        currency,
        notes: req.notes,
        subtotal,
        tax,
        total,
        line_items,
        coupon_id_to_increment,
    };

    repo.create_invoice(&draft).await
}

pub async fn update_invoice<R: InvoiceRepository + ?Sized>(
    repo: &R,
    id: &str,
    req: UpdateInvoiceRequest,
) -> Result<Invoice> {
    req.validate().map_err(BillingError::from_validation)?;
    let is_marking_issued = req.status == Some(InvoiceStatus::Issued);

    let draft = UpdateInvoiceDraft {
        status: req.status,
        due_at: req.due_at,
        notes: req.notes,
        stripe_invoice_id: req.stripe_invoice_id,
        xendit_invoice_id: req.xendit_invoice_id,
        lemonsqueezy_order_id: req.lemonsqueezy_order_id,
        version: req.version,
        issued_at: if is_marking_issued {
            Some(Utc::now().naive_utc())
        } else {
            None
        },
    };

    repo.update_invoice(id, &draft).await?.ok_or_else(|| {
        BillingError::conflict(format!(
            "invoice {id} was modified concurrently (version mismatch)"
        ))
    })
}

pub async fn delete_invoice<R: InvoiceRepository + ?Sized>(repo: &R, id: &str) -> Result<()> {
    let affected = repo.delete_invoice(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("invoice", id));
    }
    Ok(())
}

pub async fn add_invoice_item<R: InvoiceRepository + ?Sized>(
    repo: &R,
    invoice_id: &str,
    req: AddInvoiceItemRequest,
) -> Result<InvoiceItem> {
    req.validate().map_err(BillingError::from_validation)?;
    repo.add_invoice_item(invoice_id, &req).await
}

pub async fn list_invoice_items<R: InvoiceRepository + ?Sized>(
    repo: &R,
    invoice_id: &str,
) -> Result<Vec<InvoiceItem>> {
    repo.list_invoice_items(invoice_id).await
}

fn subscription_line_item(plan: &PricingPlan, sub: &Subscription) -> Result<InvoiceItemDraft> {
    let tiers: Option<Vec<PricingTier>> = plan
        .tiers
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let amount = crate::billing::tiered_pricing::calculate_amount(
        &plan.pricing_model,
        plan.base_price,
        plan.unit_price,
        tiers.as_deref(),
        sub.quantity,
    );

    Ok(InvoiceItemDraft {
        description: format!("{} ({})", plan.name, format_cycle(&plan.billing_cycle)),
        quantity: Decimal::from(sub.quantity),
        unit_price: plan.base_price,
        amount,
        period_start: Some(sub.current_period_start),
        period_end: Some(sub.current_period_end),
    })
}

fn discount_amount(coupon: &Coupon, subtotal: Decimal) -> Decimal {
    match coupon.discount_type {
        DiscountType::Percentage => {
            (subtotal * coupon.discount_value / Decimal::from(100)).round_dp(2)
        }
        DiscountType::FixedAmount => coupon.discount_value.min(subtotal),
    }
}

fn validate_coupon(coupon: &Coupon) -> Result<()> {
    if let Some(max) = coupon.max_redemptions {
        if coupon.times_redeemed >= max {
            return Err(BillingError::bad_request(
                "coupon has reached max redemptions",
            ));
        }
    }

    Ok(())
}

fn format_cycle(cycle: &BillingCycle) -> &'static str {
    match cycle {
        BillingCycle::Monthly => "Monthly",
        BillingCycle::Quarterly => "Quarterly",
        BillingCycle::Yearly => "Yearly",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{
        BillingCycle, Coupon, DiscountType, Invoice, InvoiceItem, InvoiceStatus, PricingModel,
        PricingPlan, Subscription, SubscriptionStatus,
    };
    use async_trait::async_trait;
    use chrono::{NaiveDate, Utc};
    use rust_decimal::Decimal;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        next_invoice_number: String,
        subscription: Option<Subscription>,
        plan: Option<PricingPlan>,
        coupon: Option<Coupon>,
        created_draft: Option<CreateInvoiceDraft>,
        updated_draft: Option<UpdateInvoiceDraft>,
        deleted_id: Option<String>,
        added_item: Option<AddInvoiceItemRequest>,
        list_items: Vec<InvoiceItem>,
        invoice: Option<Invoice>,
        list_rows: Vec<InvoiceView>,
        conflict_on_update: bool,
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
    impl InvoiceRepository for StubRepo {
        async fn list_invoices(&self, _filter: &ListInvoicesFilter) -> Result<Vec<InvoiceView>> {
            Ok(self.state.lock().expect("mutex").list_rows.clone())
        }

        async fn get_invoice(&self, _id: &str) -> Result<Option<Invoice>> {
            Ok(self.state.lock().expect("mutex").invoice.clone())
        }

        async fn next_invoice_number(&self) -> Result<String> {
            Ok(self
                .state
                .lock()
                .expect("mutex")
                .next_invoice_number
                .clone())
        }

        async fn find_subscription(&self, _id: &str) -> Result<Option<Subscription>> {
            Ok(self.state.lock().expect("mutex").subscription.clone())
        }

        async fn find_plan(&self, _id: &str) -> Result<Option<PricingPlan>> {
            Ok(self.state.lock().expect("mutex").plan.clone())
        }

        async fn find_coupon(&self, _code: &str) -> Result<Option<Coupon>> {
            Ok(self.state.lock().expect("mutex").coupon.clone())
        }

        async fn create_invoice(&self, draft: &CreateInvoiceDraft) -> Result<Invoice> {
            let mut state = self.state.lock().expect("mutex");
            state.created_draft = Some(draft.clone());
            Ok(state.invoice.clone().expect("invoice"))
        }

        async fn update_invoice(
            &self,
            _id: &str,
            draft: &UpdateInvoiceDraft,
        ) -> Result<Option<Invoice>> {
            let mut state = self.state.lock().expect("mutex");
            state.updated_draft = Some(draft.clone());
            if state.conflict_on_update {
                Ok(None)
            } else {
                Ok(state.invoice.clone())
            }
        }

        async fn delete_invoice(&self, id: &str) -> Result<u64> {
            self.state.lock().expect("mutex").deleted_id = Some(id.to_string());
            Ok(1)
        }

        async fn add_invoice_item(
            &self,
            _invoice_id: &str,
            req: &AddInvoiceItemRequest,
        ) -> Result<InvoiceItem> {
            let mut state = self.state.lock().expect("mutex");
            state.added_item = Some(req.clone());
            Ok(state.list_items.first().cloned().expect("item"))
        }

        async fn list_invoice_items(&self, _invoice_id: &str) -> Result<Vec<InvoiceItem>> {
            Ok(self.state.lock().expect("mutex").list_items.clone())
        }
    }

    fn sample_invoice() -> Invoice {
        Invoice {
            id: "inv_1".to_string(),
            invoice_number: "INV-00000001".to_string(),
            customer_id: "cus_1".to_string(),
            subscription_id: None,
            status: InvoiceStatus::Draft,
            issued_at: None,
            due_at: None,
            paid_at: None,
            subtotal: Decimal::ZERO,
            tax: Decimal::ZERO,
            total: Decimal::ZERO,
            currency: "USD".to_string(),
            notes: None,
            stripe_invoice_id: None,
            xendit_invoice_id: None,
            lemonsqueezy_order_id: None,
            version: 1,
            deleted_at: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            tax_name: None,
            tax_rate: None,
            tax_inclusive: false,
            credits_applied: Decimal::ZERO,
            amount_due: Decimal::ZERO,
            auto_charge_attempts: 0,
            idempotency_key: None,
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
            quantity: 5,
            metadata: None,
            stripe_subscription_id: None,
            managed_by: None,
            version: 1,
            deleted_at: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    fn sample_plan() -> PricingPlan {
        PricingPlan {
            id: "plan_1".to_string(),
            product_id: None,
            name: "Starter".to_string(),
            pricing_model: PricingModel::PerUnit,
            billing_cycle: BillingCycle::Monthly,
            base_price: Decimal::from(10),
            unit_price: Some(Decimal::from(10)),
            tiers: None,
            usage_metric_name: None,
            trial_days: 0,
            active: true,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    fn sample_coupon() -> Coupon {
        Coupon {
            id: "coupon_1".to_string(),
            code: "SAVE10".to_string(),
            name: "Save 10".to_string(),
            discount_type: DiscountType::Percentage,
            discount_value: Decimal::from(10),
            currency: "USD".to_string(),
            max_redemptions: Some(5),
            times_redeemed: 1,
            valid_from: Utc::now().naive_utc(),
            valid_until: None,
            active: true,
            applies_to: None,
            deleted_at: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    fn sample_item() -> InvoiceItem {
        InvoiceItem {
            id: "item_1".to_string(),
            invoice_id: "inv_1".to_string(),
            description: "Design work".to_string(),
            quantity: Decimal::from(1),
            unit_price: Decimal::from(100),
            amount: Decimal::from(100),
            period_start: None,
            period_end: None,
        }
    }

    #[tokio::test]
    async fn create_invoice_builds_subscription_and_discount_items() {
        let repo = StubRepo::with_state(StubState {
            next_invoice_number: "INV-00000001".to_string(),
            subscription: Some(sample_subscription()),
            plan: Some(sample_plan()),
            coupon: Some(sample_coupon()),
            invoice: Some(sample_invoice()),
            ..StubState::default()
        });

        let result = create_invoice(
            &repo,
            CreateInvoiceRequest {
                customer_id: "cus_1".to_string(),
                subscription_id: Some("sub_1".to_string()),
                due_at: None,
                currency: None,
                notes: Some("hello".to_string()),
                coupon_code: Some("SAVE10".to_string()),
                tax_rate: Some(Decimal::from(5)),
            },
        )
        .await
        .expect("create_invoice");

        let state = repo.state.lock().expect("mutex");
        assert_eq!(result.invoice_number, "INV-00000001");
        assert!(state.created_draft.is_some());
        let draft = state.created_draft.as_ref().expect("draft");
        assert_eq!(draft.currency, "USD");
        assert_eq!(draft.line_items.len(), 2);
        assert_eq!(draft.subtotal, Decimal::from(45));
        assert_eq!(draft.tax, Decimal::new(225, 2));
        assert_eq!(draft.total, Decimal::new(4725, 2));
        assert_eq!(draft.coupon_id_to_increment.as_deref(), Some("coupon_1"));
    }

    #[tokio::test]
    async fn update_invoice_sets_issued_at_for_issued_status() {
        let repo = StubRepo::with_state(StubState {
            invoice: Some(sample_invoice()),
            ..StubState::default()
        });

        let _ = update_invoice(
            &repo,
            "inv_1",
            UpdateInvoiceRequest {
                status: Some(InvoiceStatus::Issued),
                due_at: None,
                notes: None,
                stripe_invoice_id: None,
                xendit_invoice_id: None,
                lemonsqueezy_order_id: None,
                version: 1,
            },
        )
        .await
        .expect("update_invoice");

        let state = repo.state.lock().expect("mutex");
        assert!(state
            .updated_draft
            .as_ref()
            .and_then(|d| d.issued_at)
            .is_some());
    }

    #[tokio::test]
    async fn add_invoice_item_validates_and_forwards() {
        let repo = StubRepo::with_state(StubState {
            list_items: vec![sample_item()],
            invoice: Some(sample_invoice()),
            ..StubState::default()
        });

        let item = add_invoice_item(
            &repo,
            "inv_1",
            AddInvoiceItemRequest {
                description: "Design work".to_string(),
                quantity: Decimal::from(2),
                unit_price: Decimal::from(25),
                period_start: None,
                period_end: None,
            },
        )
        .await
        .expect("add_invoice_item");

        let state = repo.state.lock().expect("mutex");
        assert_eq!(item.id, "item_1");
        assert_eq!(
            state
                .added_item
                .as_ref()
                .map(|req| req.description.as_str()),
            Some("Design work")
        );
    }
}
