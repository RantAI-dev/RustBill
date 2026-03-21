use super::schema::{RenewalInvoiceOutput, SalesEventSpec, SubscriptionTransition};
use crate::analytics::sales_ledger::{emit_sales_event, NewSalesEvent};
use crate::billing::auto_charge::ChargeResult;
use crate::billing::tiered_pricing;
use crate::db::models::*;
use crate::error::Result;
use crate::notifications::{self, email::EmailSender, send};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;

const DEFAULT_CURRENCY: &str = "USD";

#[async_trait]
pub trait LifecycleRepository {
    async fn convert_expired_trials(
        &self,
        now: NaiveDateTime,
    ) -> Result<Vec<SubscriptionTransition>>;
    async fn cancel_at_period_end(&self, now: NaiveDateTime)
        -> Result<Vec<SubscriptionTransition>>;
    async fn list_pre_generatable_subscriptions(
        &self,
        now: NaiveDateTime,
        window_end: NaiveDateTime,
    ) -> Result<Vec<Subscription>>;
    async fn list_renewable_subscriptions(&self, now: NaiveDateTime) -> Result<Vec<Subscription>>;
    async fn find_plan(&self, plan_id: &str) -> Result<PricingPlan>;
    async fn find_existing_renewal_invoice(
        &self,
        sub: &Subscription,
        period_start: NaiveDateTime,
    ) -> Result<Option<Invoice>>;
    async fn ensure_renewal_invoice(
        &self,
        sub: &Subscription,
        now: NaiveDateTime,
        notify_invoice_issued: bool,
    ) -> Result<Option<RenewalInvoiceOutput>>;
    async fn advance_subscription_period(
        &self,
        subscription_id: &str,
        new_period_end: NaiveDateTime,
    ) -> Result<()>;
    async fn get_default_payment_method(
        &self,
        customer_id: &str,
    ) -> Result<Option<SavedPaymentMethod>>;
    async fn try_auto_charge(
        &self,
        invoice: &Invoice,
        payment_method: &SavedPaymentMethod,
    ) -> Result<ChargeResult>;
    async fn settle_auto_charge_success(
        &self,
        invoice: &Invoice,
        payment_method: &SavedPaymentMethod,
        provider_reference: Option<&str>,
    ) -> Result<()>;
    async fn mark_payment_method_failed(&self, method_id: &str) -> Result<()>;
    async fn emit_sales_event(&self, event: SalesEventSpec) -> Result<()>;
    async fn dispatch_subscription_renewed_side_effects(
        &self,
        subscription: &Subscription,
        output: &RenewalInvoiceOutput,
    ) -> Result<()>;
}

pub struct PgLifecycleRepository<'a> {
    pool: &'a PgPool,
    email_sender: Option<&'a EmailSender>,
    http_client: &'a reqwest::Client,
}

impl<'a> PgLifecycleRepository<'a> {
    pub fn new(
        pool: &'a PgPool,
        email_sender: Option<&'a EmailSender>,
        http_client: &'a reqwest::Client,
    ) -> Self {
        Self {
            pool,
            email_sender,
            http_client,
        }
    }
}

#[async_trait]
impl LifecycleRepository for PgLifecycleRepository<'_> {
    async fn convert_expired_trials(
        &self,
        now: NaiveDateTime,
    ) -> Result<Vec<SubscriptionTransition>> {
        let before_rows = sqlx::query_as::<_, Subscription>(
            r#"
            SELECT * FROM subscriptions
            WHERE status = 'trialing'
              AND deleted_at IS NULL
              AND trial_end IS NOT NULL
              AND trial_end <= $1
            "#,
        )
        .bind(now)
        .fetch_all(self.pool)
        .await?;

        sqlx::query(
            r#"
            UPDATE subscriptions
            SET status = 'active',
                current_period_start = $1,
                current_period_end = $1 + (
                    SELECT CASE pp.billing_cycle
                        WHEN 'monthly'   THEN INTERVAL '1 month'
                        WHEN 'quarterly' THEN INTERVAL '3 months'
                        WHEN 'yearly'    THEN INTERVAL '1 year'
                    END
                    FROM pricing_plans pp WHERE pp.id = subscriptions.plan_id
                ),
                version = version + 1,
                updated_at = NOW()
            WHERE status = 'trialing'
              AND deleted_at IS NULL
              AND trial_end IS NOT NULL
              AND trial_end <= $1
            "#,
        )
        .bind(now)
        .execute(self.pool)
        .await?;

        let mut transitions = Vec::new();
        for before in before_rows {
            if let Some(after) = sqlx::query_as::<_, Subscription>(
                "SELECT * FROM subscriptions WHERE id = $1 AND deleted_at IS NULL",
            )
            .bind(&before.id)
            .fetch_optional(self.pool)
            .await?
            {
                transitions.push(SubscriptionTransition { before, after });
            }
        }

        Ok(transitions)
    }

    async fn cancel_at_period_end(
        &self,
        now: NaiveDateTime,
    ) -> Result<Vec<SubscriptionTransition>> {
        let before_rows = sqlx::query_as::<_, Subscription>(
            r#"
            SELECT * FROM subscriptions
            WHERE status = 'active'
              AND deleted_at IS NULL
              AND cancel_at_period_end = true
              AND current_period_end <= $1
            "#,
        )
        .bind(now)
        .fetch_all(self.pool)
        .await?;

        sqlx::query(
            r#"
            UPDATE subscriptions
            SET status = 'canceled',
                canceled_at = $1,
                version = version + 1,
                updated_at = NOW()
            WHERE status = 'active'
              AND deleted_at IS NULL
              AND cancel_at_period_end = true
              AND current_period_end <= $1
            "#,
        )
        .bind(now)
        .execute(self.pool)
        .await?;

        let mut transitions = Vec::new();
        for before in before_rows {
            if let Some(after) = sqlx::query_as::<_, Subscription>(
                "SELECT * FROM subscriptions WHERE id = $1 AND deleted_at IS NULL",
            )
            .bind(&before.id)
            .fetch_optional(self.pool)
            .await?
            {
                transitions.push(SubscriptionTransition { before, after });
            }
        }

        Ok(transitions)
    }

    async fn list_pre_generatable_subscriptions(
        &self,
        now: NaiveDateTime,
        window_end: NaiveDateTime,
    ) -> Result<Vec<Subscription>> {
        let subs = sqlx::query_as::<_, Subscription>(
            r#"
            SELECT * FROM subscriptions
            WHERE status = 'active'
              AND deleted_at IS NULL
              AND cancel_at_period_end = false
              AND current_period_end > $1
              AND current_period_end <= $2
              AND (managed_by IS NULL OR managed_by = '')
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(now)
        .bind(window_end)
        .fetch_all(self.pool)
        .await?;

        Ok(subs)
    }

    async fn list_renewable_subscriptions(&self, now: NaiveDateTime) -> Result<Vec<Subscription>> {
        let subs = sqlx::query_as::<_, Subscription>(
            r#"
            SELECT * FROM subscriptions
            WHERE status = 'active'
              AND deleted_at IS NULL
              AND cancel_at_period_end = false
              AND current_period_end <= $1
              AND (managed_by IS NULL OR managed_by = '')
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(now)
        .fetch_all(self.pool)
        .await?;

        Ok(subs)
    }

    async fn find_plan(&self, plan_id: &str) -> Result<PricingPlan> {
        let plan = sqlx::query_as::<_, PricingPlan>("SELECT * FROM pricing_plans WHERE id = $1")
            .bind(plan_id)
            .fetch_one(self.pool)
            .await?;
        Ok(plan)
    }

    async fn find_existing_renewal_invoice(
        &self,
        sub: &Subscription,
        period_start: NaiveDateTime,
    ) -> Result<Option<Invoice>> {
        let key = renewal_invoice_key(sub, period_start);
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT * FROM invoices
            WHERE subscription_id = $1
              AND idempotency_key = $2
              AND deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(&sub.id)
        .bind(key)
        .fetch_optional(self.pool)
        .await?;

        Ok(invoice)
    }

    async fn ensure_renewal_invoice(
        &self,
        sub: &Subscription,
        now: NaiveDateTime,
        notify_invoice_issued: bool,
    ) -> Result<Option<RenewalInvoiceOutput>> {
        let plan = self.find_plan(&sub.plan_id).await?;
        let quantity = compute_quantity(self.pool, sub, &plan).await?;
        let tiers: Option<Vec<PricingTier>> = plan
            .tiers
            .as_ref()
            .and_then(|value| serde_json::from_value(value.clone()).ok());

        let amount = tiered_pricing::calculate_amount(
            &plan.pricing_model,
            plan.base_price,
            plan.unit_price,
            tiers.as_deref(),
            quantity,
        );

        let new_period_end = crate::billing::subscriptions::advance_period(
            sub.current_period_end,
            &plan.billing_cycle,
        );
        let due_at = now + chrono::Duration::days(30);
        let idempotency_key = renewal_invoice_key(sub, sub.current_period_end);

        let mut tx = self.pool.begin().await?;

        if let Some(existing) = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT * FROM invoices
            WHERE subscription_id = $1
              AND idempotency_key = $2
              AND deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(&sub.id)
        .bind(&idempotency_key)
        .fetch_optional(&mut *tx)
        .await?
        {
            tx.commit().await?;
            return Ok(Some(RenewalInvoiceOutput {
                invoice: existing.clone(),
                final_amount_due: existing.amount_due,
                total: existing.total,
                invoice_number: existing.invoice_number.clone(),
                plan_name: plan.name,
                new_period_end,
            }));
        }

        let invoice_number: String = sqlx::query_scalar(
            "SELECT 'INV-' || LPAD(nextval('invoice_number_seq')::text, 8, '0')",
        )
        .fetch_one(&mut *tx)
        .await?;

        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            INSERT INTO invoices
                (id, invoice_number, customer_id, subscription_id, status,
                 issued_at, due_at, subtotal, tax, total, currency, notes, idempotency_key)
            VALUES (gen_random_uuid()::text, $1, $2, $3, 'issued', $4, $5, 0, 0, 0, 'USD', $6, $7)
            RETURNING *
            "#,
        )
        .bind(&invoice_number)
        .bind(&sub.customer_id)
        .bind(&sub.id)
        .bind(now)
        .bind(due_at)
        .bind(format!("Auto-renewal for subscription {}", sub.id))
        .bind(&idempotency_key)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO invoice_items
                (id, invoice_id, description, quantity, unit_price, amount,
                 period_start, period_end)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&invoice.id)
        .bind(format!(
            "{} ({})",
            plan.name,
            format_cycle(&plan.billing_cycle)
        ))
        .bind(Decimal::from(quantity))
        .bind(plan.base_price)
        .bind(amount)
        .bind(sub.current_period_end)
        .bind(new_period_end)
        .execute(&mut *tx)
        .await?;

        let discounts = sqlx::query_as::<_, SubscriptionDiscountWithCoupon>(
            r#"
            SELECT sd.id, sd.subscription_id, sd.coupon_id, sd.applied_at, sd.expires_at,
                   c.discount_type, c.discount_value, c.code AS coupon_code
            FROM subscription_discounts sd
            JOIN coupons c ON c.id = sd.coupon_id AND c.active = true AND c.deleted_at IS NULL
            WHERE sd.subscription_id = $1
              AND (sd.expires_at IS NULL OR sd.expires_at > $2)
            "#,
        )
        .bind(&sub.id)
        .bind(now)
        .fetch_all(&mut *tx)
        .await?;

        for discount in &discounts {
            let discount_amount = match discount.discount_type {
                DiscountType::Percentage => {
                    (amount * discount.discount_value / Decimal::from(100)).round_dp(2)
                }
                DiscountType::FixedAmount => discount.discount_value.min(amount),
            };

            if discount_amount > Decimal::ZERO {
                sqlx::query(
                    r#"
                    INSERT INTO invoice_items
                        (id, invoice_id, description, quantity, unit_price, amount)
                    VALUES (gen_random_uuid()::text, $1, $2, 1, $3, $4)
                    "#,
                )
                .bind(&invoice.id)
                .bind(format!("Discount ({})", discount.coupon_code))
                .bind(-discount_amount)
                .bind(-discount_amount)
                .execute(&mut *tx)
                .await?;
            }
        }

        let subtotal: Option<Decimal> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0) FROM invoice_items WHERE invoice_id = $1",
        )
        .bind(&invoice.id)
        .fetch_one(&mut *tx)
        .await?;
        let subtotal = subtotal.unwrap_or_default();

        let customer = sqlx::query_as::<_, Customer>("SELECT * FROM customers WHERE id = $1")
            .bind(&sub.customer_id)
            .fetch_one(&mut *tx)
            .await?;

        let tax_result = crate::billing::tax::resolve_tax(
            self.pool,
            customer.billing_country.as_deref().unwrap_or(""),
            customer.billing_state.as_deref(),
            None,
            subtotal,
        )
        .await?;

        let tax_amount = tax_result.amount;
        let total = if tax_result.inclusive {
            subtotal
        } else {
            subtotal + tax_amount
        };

        sqlx::query(
            r#"UPDATE invoices SET
                subtotal = $2, tax = $3, total = $4,
                tax_name = $5, tax_rate = $6, tax_inclusive = $7,
                credits_applied = 0, amount_due = $4,
                updated_at = NOW()
            WHERE id = $1"#,
        )
        .bind(&invoice.id)
        .bind(subtotal)
        .bind(tax_amount)
        .bind(total)
        .bind(&tax_result.name)
        .bind(tax_result.rate)
        .bind(tax_result.inclusive)
        .execute(&mut *tx)
        .await?;

        let credits_applied = crate::billing::credits::apply_to_invoice(
            &mut tx,
            &sub.customer_id,
            &invoice.id,
            &invoice.currency,
            total,
        )
        .await?;

        let final_amount_due = total - credits_applied;

        if credits_applied > Decimal::ZERO {
            sqlx::query("UPDATE invoices SET credits_applied = $2, amount_due = $3 WHERE id = $1")
                .bind(&invoice.id)
                .bind(credits_applied)
                .bind(final_amount_due)
                .execute(&mut *tx)
                .await?;
        }

        if final_amount_due <= Decimal::ZERO {
            sqlx::query("UPDATE invoices SET status = 'paid' WHERE id = $1")
                .bind(&invoice.id)
                .execute(&mut *tx)
                .await?;
        }

        let committed_invoice =
            sqlx::query_as::<_, Invoice>("SELECT * FROM invoices WHERE id = $1")
                .bind(&invoice.id)
                .fetch_one(&mut *tx)
                .await?;

        tx.commit().await?;

        if notify_invoice_issued {
            let due_date = due_at.format("%Y-%m-%d").to_string();
            let _ = send::notify_invoice_issued(
                self.email_sender,
                self.pool,
                &sub.customer_id,
                &invoice_number,
                &total.to_string(),
                &committed_invoice.currency,
                &due_date,
            )
            .await;

            let _ = notifications::emit_billing_event(
                self.pool,
                self.http_client,
                BillingEventType::InvoiceIssued,
                "invoice",
                &committed_invoice.id,
                Some(&sub.customer_id),
                Some(serde_json::json!({
                    "invoice_number": invoice_number,
                    "total": total.to_string(),
                    "due_at": due_date,
                    "pre_generated": true,
                })),
            )
            .await;
        }

        let _ = emit_sales_event(
            self.pool,
            NewSalesEvent {
                occurred_at: Utc::now(),
                event_type: "invoice.issued",
                classification: crate::analytics::sales_ledger::SalesClassification::Billings,
                amount_subtotal: committed_invoice.subtotal,
                amount_tax: committed_invoice.tax,
                amount_total: committed_invoice.total,
                currency: &committed_invoice.currency,
                customer_id: Some(&sub.customer_id),
                subscription_id: Some(&sub.id),
                product_id: None,
                invoice_id: Some(&committed_invoice.id),
                payment_id: None,
                source_table: "invoices",
                source_id: &committed_invoice.id,
                metadata: Some(serde_json::json!({
                    "pre_generated": notify_invoice_issued,
                })),
            },
        )
        .await;

        Ok(Some(RenewalInvoiceOutput {
            invoice: committed_invoice,
            final_amount_due,
            total,
            invoice_number,
            plan_name: plan.name,
            new_period_end,
        }))
    }

    async fn advance_subscription_period(
        &self,
        subscription_id: &str,
        new_period_end: NaiveDateTime,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            UPDATE subscriptions SET
                current_period_start = current_period_end,
                current_period_end = $2,
                version = version + 1,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(subscription_id)
        .bind(new_period_end)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn get_default_payment_method(
        &self,
        customer_id: &str,
    ) -> Result<Option<SavedPaymentMethod>> {
        crate::billing::payment_methods::get_default(self.pool, customer_id).await
    }

    async fn try_auto_charge(
        &self,
        invoice: &Invoice,
        payment_method: &SavedPaymentMethod,
    ) -> Result<ChargeResult> {
        crate::billing::auto_charge::try_auto_charge(
            self.pool,
            invoice,
            payment_method,
            self.http_client,
        )
        .await
    }

    async fn settle_auto_charge_success(
        &self,
        invoice: &Invoice,
        payment_method: &SavedPaymentMethod,
        provider_reference: Option<&str>,
    ) -> Result<()> {
        let method = match payment_method.provider {
            PaymentProvider::Stripe => PaymentMethod::Stripe,
            PaymentProvider::Xendit => PaymentMethod::Xendit,
            PaymentProvider::Lemonsqueezy => PaymentMethod::Lemonsqueezy,
        };

        let mut tx = self.pool.begin().await?;

        let locked_invoice: Invoice =
            sqlx::query_as("SELECT * FROM invoices WHERE id = $1 FOR UPDATE")
                .bind(&invoice.id)
                .fetch_one(&mut *tx)
                .await?;

        if locked_invoice.status == InvoiceStatus::Paid {
            tracing::info!(
                invoice_id = %invoice.id,
                "Auto-charge settlement skipped because invoice is already paid"
            );
            tx.commit().await?;
            return Ok(());
        }

        let paid_at = Utc::now().naive_utc();
        let stripe_ref = if payment_method.provider == PaymentProvider::Stripe {
            provider_reference
        } else {
            None
        };
        let xendit_ref = if payment_method.provider == PaymentProvider::Xendit {
            provider_reference
        } else {
            None
        };

        let payment: Payment = sqlx::query_as(
            r#"
            INSERT INTO payments
                (id, invoice_id, amount, method, reference, paid_at, notes,
                 stripe_payment_intent_id, xendit_payment_id, lemonsqueezy_order_id)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, $8, NULL)
            RETURNING *
            "#,
        )
        .bind(&locked_invoice.id)
        .bind(locked_invoice.amount_due)
        .bind(&method)
        .bind(format!(
            "auto-charge:{}:{}",
            provider_name(&payment_method.provider),
            payment_method.id
        ))
        .bind(paid_at)
        .bind("Auto-charge via saved payment method")
        .bind(stripe_ref)
        .bind(xendit_ref)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE invoices SET status = 'paid', paid_at = $2, version = version + 1, updated_at = NOW() WHERE id = $1",
        )
        .bind(&locked_invoice.id)
        .bind(paid_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let payment_amount = payment.amount.to_string();
        let _ = notifications::emit_billing_event(
            self.pool,
            self.http_client,
            BillingEventType::PaymentReceived,
            "payment",
            &payment.id,
            Some(&locked_invoice.customer_id),
            Some(serde_json::json!({
                "invoice_id": locked_invoice.id,
                "invoice_number": locked_invoice.invoice_number,
                "amount": payment_amount,
                "method": provider_name(&payment_method.provider),
                "auto_charge": true,
            })),
        )
        .await;

        let _ = notifications::emit_billing_event(
            self.pool,
            self.http_client,
            BillingEventType::InvoicePaid,
            "invoice",
            &locked_invoice.id,
            Some(&locked_invoice.customer_id),
            Some(serde_json::json!({
                "invoice_number": locked_invoice.invoice_number,
                "amount_due": locked_invoice.amount_due.to_string(),
                "auto_charge": true,
            })),
        )
        .await;

        Ok(())
    }

    async fn mark_payment_method_failed(&self, method_id: &str) -> Result<()> {
        crate::billing::payment_methods::mark_failed(self.pool, method_id).await
    }

    async fn emit_sales_event(&self, event: SalesEventSpec) -> Result<()> {
        emit_sales_event(
            self.pool,
            NewSalesEvent {
                occurred_at: Utc::now(),
                event_type: event.event_type,
                classification: event.classification,
                amount_subtotal: event.amount_subtotal,
                amount_tax: event.amount_tax,
                amount_total: event.amount_total,
                currency: &event.currency,
                customer_id: event.customer_id.as_deref(),
                subscription_id: event.subscription_id.as_deref(),
                product_id: None,
                invoice_id: event.invoice_id.as_deref(),
                payment_id: event.payment_id.as_deref(),
                source_table: event.source_table,
                source_id: &event.source_id,
                metadata: event.metadata,
            },
        )
        .await
    }

    async fn dispatch_subscription_renewed_side_effects(
        &self,
        subscription: &Subscription,
        output: &RenewalInvoiceOutput,
    ) -> Result<()> {
        let pool_clone = self.pool.clone();
        let http_clone = self.http_client.clone();
        let sub_id = subscription.id.clone();
        let cust_id = subscription.customer_id.clone();
        let invoice_id = output.invoice.id.clone();
        let invoice_number = output.invoice_number.clone();
        let total = output.total.to_string();
        tokio::spawn(async move {
            let _ = notifications::emit_billing_event(
                &pool_clone,
                &http_clone,
                BillingEventType::SubscriptionRenewed,
                "subscription",
                &sub_id,
                Some(&cust_id),
                Some(serde_json::json!({
                    "invoice_id": invoice_id,
                    "invoice_number": invoice_number,
                    "total": total,
                })),
            )
            .await;
        });

        let pool_email = self.pool.clone();
        let email_sender = self.email_sender.cloned();
        let customer_id = subscription.customer_id.clone();
        let plan_name = output.plan_name.clone();
        let invoice_number = output.invoice_number.clone();
        let total_str = output.total.to_string();
        let period_end = output.new_period_end.format("%Y-%m-%d").to_string();
        tokio::spawn(async move {
            send::notify_subscription_renewed(
                email_sender.as_ref(),
                &pool_email,
                &customer_id,
                &plan_name,
                &invoice_number,
                &total_str,
                DEFAULT_CURRENCY,
                &period_end,
            )
            .await;
        });

        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SubscriptionDiscountWithCoupon {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    subscription_id: String,
    #[allow(dead_code)]
    coupon_id: String,
    #[allow(dead_code)]
    applied_at: NaiveDateTime,
    #[allow(dead_code)]
    expires_at: Option<NaiveDateTime>,
    discount_type: DiscountType,
    discount_value: Decimal,
    coupon_code: String,
}

fn renewal_invoice_key(sub: &Subscription, period_start: NaiveDateTime) -> String {
    format!("renewal:{}:{}", sub.id, period_start.format("%Y%m%d%H%M%S"))
}

fn format_cycle(cycle: &BillingCycle) -> &'static str {
    match cycle {
        BillingCycle::Monthly => "Monthly",
        BillingCycle::Quarterly => "Quarterly",
        BillingCycle::Yearly => "Yearly",
    }
}

fn provider_name(provider: &PaymentProvider) -> &'static str {
    match provider {
        PaymentProvider::Stripe => "stripe",
        PaymentProvider::Xendit => "xendit",
        PaymentProvider::Lemonsqueezy => "lemonsqueezy",
    }
}

async fn compute_quantity(pool: &PgPool, sub: &Subscription, plan: &PricingPlan) -> Result<i32> {
    if plan.pricing_model == PricingModel::UsageBased {
        let metric_name = plan.usage_metric_name.as_deref().unwrap_or("api_calls");

        let total_usage: Option<Decimal> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(value), 0) FROM usage_events
            WHERE subscription_id = $1
              AND metric_name = $2
              AND timestamp >= $3
              AND timestamp < $4
            "#,
        )
        .bind(&sub.id)
        .bind(metric_name)
        .bind(sub.current_period_start)
        .bind(sub.current_period_end)
        .fetch_one(pool)
        .await?;
        let total_usage = total_usage.unwrap_or_default();

        let usage = total_usage.to_string().parse::<i64>().unwrap_or(0);
        Ok(usage.max(0) as i32)
    } else {
        Ok(sub.quantity)
    }
}
