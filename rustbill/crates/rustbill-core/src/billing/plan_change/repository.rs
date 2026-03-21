use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};

use crate::billing::proration::ProrationLineItem;
use crate::db::models::{
    CreditReason, Customer, Invoice, InvoiceStatus, PricingPlan, Subscription,
};
use crate::error::{BillingError, Result};

use super::schema::{ChangePlanOutput, ChangePlanWork};

#[async_trait]
pub trait PlanChangeRepository {
    async fn find_subscription_for_update(&self, subscription_id: &str) -> Result<Subscription>;
    async fn find_proration_invoice(
        &self,
        subscription_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<Invoice>>;
    async fn get_plan(&self, plan_id: &str) -> Result<PricingPlan>;
    async fn get_currency_for_subscription(&self, subscription_id: &str) -> Result<String>;
    async fn apply_change_plan(&self, work: &ChangePlanWork) -> Result<ChangePlanOutput>;
}

pub struct PgPlanChangeRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PgPlanChangeRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PlanChangeRepository for PgPlanChangeRepository<'_> {
    async fn find_subscription_for_update(&self, subscription_id: &str) -> Result<Subscription> {
        sqlx::query_as::<_, Subscription>(
            "SELECT * FROM subscriptions WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(subscription_id)
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| BillingError::not_found("subscription", subscription_id))
    }

    async fn find_proration_invoice(
        &self,
        subscription_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<Invoice>> {
        let invoice = sqlx::query_as::<_, Invoice>(
            "SELECT * FROM invoices WHERE subscription_id = $1 AND idempotency_key = $2 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(subscription_id)
        .bind(idempotency_key)
        .fetch_optional(self.pool)
        .await?;
        Ok(invoice)
    }

    async fn get_plan(&self, plan_id: &str) -> Result<PricingPlan> {
        let plan = sqlx::query_as::<_, PricingPlan>("SELECT * FROM pricing_plans WHERE id = $1")
            .bind(plan_id)
            .fetch_one(self.pool)
            .await?;
        Ok(plan)
    }

    async fn get_currency_for_subscription(&self, subscription_id: &str) -> Result<String> {
        let currency: String = sqlx::query_scalar(
            "SELECT currency FROM invoices WHERE subscription_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(subscription_id)
        .fetch_optional(self.pool)
        .await?
        .unwrap_or_else(|| "USD".to_string());
        Ok(currency)
    }

    async fn apply_change_plan(&self, work: &ChangePlanWork) -> Result<ChangePlanOutput> {
        let mut tx = self.pool.begin().await?;

        let mut created_invoice: Option<Invoice> = None;

        if work.proration.net > Decimal::ZERO {
            let invoice_number: String = sqlx::query_scalar(
                "SELECT 'INV-' || LPAD(nextval('invoice_number_seq')::text, 8, '0')",
            )
            .fetch_one(&mut *tx)
            .await?;

            let due_at = work.now + chrono::Duration::days(30);
            let invoice = sqlx::query_as::<_, Invoice>(
                r#"
                INSERT INTO invoices
                    (id, invoice_number, customer_id, subscription_id, status,
                     issued_at, due_at, subtotal, tax, total, currency, notes, idempotency_key)
                VALUES (gen_random_uuid()::text, $1, $2, $3, 'issued', $4, $5, 0, 0, 0, $6, $7, $8)
                RETURNING *
                "#,
            )
            .bind(&invoice_number)
            .bind(&work.subscription.customer_id)
            .bind(&work.subscription.id)
            .bind(work.now)
            .bind(due_at)
            .bind(&work.currency)
            .bind(format!(
                "Proration adjustment for subscription {}: {} -> {}",
                work.subscription.id, work.old_plan.name, work.new_plan.name
            ))
            .bind(work.idempotency_key.as_deref())
            .fetch_one(&mut *tx)
            .await?;

            for line_item in &work.proration.line_items {
                insert_proration_line_item(&mut tx, &invoice.id, line_item).await?;
            }

            let subtotal: Decimal = sqlx::query_scalar(
                "SELECT COALESCE(SUM(amount), 0) FROM invoice_items WHERE invoice_id = $1",
            )
            .bind(&invoice.id)
            .fetch_one(&mut *tx)
            .await?;

            let customer: Customer = sqlx::query_as("SELECT * FROM customers WHERE id = $1")
                .bind(&work.subscription.customer_id)
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
                &work.subscription.customer_id,
                &invoice.id,
                &work.currency,
                total,
            )
            .await?;

            let amount_due = total - credits_applied;
            if credits_applied > Decimal::ZERO {
                sqlx::query(
                    "UPDATE invoices SET credits_applied = $2, amount_due = $3 WHERE id = $1",
                )
                .bind(&invoice.id)
                .bind(credits_applied)
                .bind(amount_due)
                .execute(&mut *tx)
                .await?;
            }

            if amount_due <= Decimal::ZERO {
                sqlx::query("UPDATE invoices SET status = $2 WHERE id = $1")
                    .bind(&invoice.id)
                    .bind(InvoiceStatus::Paid)
                    .execute(&mut *tx)
                    .await?;
            }

            created_invoice = Some(
                sqlx::query_as::<_, Invoice>("SELECT * FROM invoices WHERE id = $1")
                    .bind(&invoice.id)
                    .fetch_one(&mut *tx)
                    .await?,
            );
        } else if work.proration.net < Decimal::ZERO {
            crate::billing::credits::deposit_in_tx(
                &mut tx,
                &work.subscription.customer_id,
                &work.currency,
                work.proration.net.abs(),
                CreditReason::Proration,
                &format!(
                    "Proration credit: {} -> {}",
                    work.old_plan.name, work.new_plan.name
                ),
                None,
            )
            .await?;
        }

        let rows = sqlx::query(
            r#"UPDATE subscriptions
               SET plan_id = $2, quantity = $3, version = version + 1, updated_at = NOW()
               WHERE id = $1 AND version = $4"#,
        )
        .bind(&work.subscription.id)
        .bind(&work.new_plan.id)
        .bind(work.new_quantity)
        .bind(work.subscription.version)
        .execute(&mut *tx)
        .await?;

        if rows.rows_affected() == 0 {
            return Err(BillingError::conflict(
                "subscription was modified concurrently; retry the request",
            ));
        }

        let updated_sub: Subscription = sqlx::query_as("SELECT * FROM subscriptions WHERE id = $1")
            .bind(&work.subscription.id)
            .fetch_one(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(ChangePlanOutput {
            subscription: updated_sub,
            invoice: created_invoice,
            already_processed: false,
            proration_net: work.proration.net,
            old_plan_name: work.old_plan.name.clone(),
            new_plan_name: work.new_plan.name.clone(),
            customer_id: work.subscription.customer_id.clone(),
        })
    }
}

async fn insert_proration_line_item(
    tx: &mut Transaction<'_, Postgres>,
    invoice_id: &str,
    line_item: &ProrationLineItem,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO invoice_items
            (id, invoice_id, description, quantity, unit_price, amount,
             period_start, period_end)
        VALUES (gen_random_uuid()::text, $1, $2, 1, $3, $4, $5, $6)
        "#,
    )
    .bind(invoice_id)
    .bind(&line_item.description)
    .bind(line_item.amount)
    .bind(line_item.amount)
    .bind(line_item.period_start)
    .bind(line_item.period_end)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
