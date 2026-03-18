use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::db::models::{
    CreditReason, Customer, Invoice, InvoiceStatus, PricingPlan, Subscription,
};
use crate::error::{BillingError, Result};

pub struct ChangePlanInput<'a> {
    pub subscription_id: &'a str,
    pub new_plan_id: &'a str,
    pub new_quantity: Option<i32>,
    pub idempotency_key: Option<&'a str>,
    pub now: NaiveDateTime,
}

pub struct ChangePlanOutput {
    pub subscription: Subscription,
    pub invoice: Option<Invoice>,
    pub already_processed: bool,
    pub proration_net: Decimal,
    pub old_plan_name: String,
    pub new_plan_name: String,
    pub customer_id: String,
}

pub async fn change_plan_with_proration(
    pool: &PgPool,
    input: ChangePlanInput<'_>,
) -> Result<ChangePlanOutput> {
    let mut tx = pool.begin().await?;

    let sub: Subscription = sqlx::query_as(
        "SELECT * FROM subscriptions WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(input.subscription_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| BillingError::not_found("subscription", input.subscription_id))?;

    if let Some(key) = input.idempotency_key {
        let existing_invoice: Option<Invoice> = sqlx::query_as(
            "SELECT * FROM invoices WHERE subscription_id = $1 AND idempotency_key = $2 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(input.subscription_id)
        .bind(key)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(invoice) = existing_invoice {
            tx.commit().await?;
            return Ok(ChangePlanOutput {
                subscription: sub,
                invoice: Some(invoice),
                already_processed: true,
                proration_net: Decimal::ZERO,
                old_plan_name: String::new(),
                new_plan_name: String::new(),
                customer_id: String::new(),
            });
        }
    }

    let old_plan: PricingPlan = sqlx::query_as("SELECT * FROM pricing_plans WHERE id = $1")
        .bind(&sub.plan_id)
        .fetch_one(&mut *tx)
        .await?;

    let new_plan: PricingPlan = sqlx::query_as("SELECT * FROM pricing_plans WHERE id = $1")
        .bind(input.new_plan_id)
        .fetch_one(&mut *tx)
        .await?;

    let new_quantity = input.new_quantity.unwrap_or(sub.quantity);

    let proration = crate::billing::proration::calculate_proration(
        &old_plan,
        &new_plan,
        sub.quantity,
        new_quantity,
        sub.current_period_start,
        sub.current_period_end,
        input.now,
    )?;

    let currency: String = sqlx::query_scalar(
        "SELECT currency FROM invoices WHERE subscription_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(input.subscription_id)
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or_else(|| "USD".to_string());

    let mut created_invoice: Option<Invoice> = None;

    if proration.net > Decimal::ZERO {
        let invoice_number: String = sqlx::query_scalar(
            "SELECT 'INV-' || LPAD(nextval('invoice_number_seq')::text, 8, '0')",
        )
        .fetch_one(&mut *tx)
        .await?;

        let due_at = input.now + chrono::Duration::days(30);
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
        .bind(&sub.customer_id)
        .bind(&sub.id)
        .bind(input.now)
        .bind(due_at)
        .bind(&currency)
        .bind(format!(
            "Proration adjustment for subscription {}: {} -> {}",
            sub.id, old_plan.name, new_plan.name
        ))
        .bind(input.idempotency_key)
        .fetch_one(&mut *tx)
        .await?;

        for line_item in &proration.line_items {
            sqlx::query(
                r#"
                INSERT INTO invoice_items
                    (id, invoice_id, description, quantity, unit_price, amount,
                     period_start, period_end)
                VALUES (gen_random_uuid()::text, $1, $2, 1, $3, $4, $5, $6)
                "#,
            )
            .bind(&invoice.id)
            .bind(&line_item.description)
            .bind(line_item.amount)
            .bind(line_item.amount)
            .bind(line_item.period_start)
            .bind(line_item.period_end)
            .execute(&mut *tx)
            .await?;
        }

        let subtotal: Decimal = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0) FROM invoice_items WHERE invoice_id = $1",
        )
        .bind(&invoice.id)
        .fetch_one(&mut *tx)
        .await?;

        let customer: Customer = sqlx::query_as("SELECT * FROM customers WHERE id = $1")
            .bind(&sub.customer_id)
            .fetch_one(&mut *tx)
            .await?;

        let tax_result = crate::billing::tax::resolve_tax(
            pool,
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
            &currency,
            total,
        )
        .await?;

        let amount_due = total - credits_applied;
        if credits_applied > Decimal::ZERO {
            sqlx::query("UPDATE invoices SET credits_applied = $2, amount_due = $3 WHERE id = $1")
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
    } else if proration.net < Decimal::ZERO {
        crate::billing::credits::deposit_in_tx(
            &mut tx,
            &sub.customer_id,
            &currency,
            proration.net.abs(),
            CreditReason::Proration,
            &format!("Proration credit: {} -> {}", old_plan.name, new_plan.name),
            None,
        )
        .await?;
    }

    let rows = sqlx::query(
        r#"UPDATE subscriptions
           SET plan_id = $2, quantity = $3, version = version + 1, updated_at = NOW()
           WHERE id = $1 AND version = $4"#,
    )
    .bind(&sub.id)
    .bind(input.new_plan_id)
    .bind(new_quantity)
    .bind(sub.version)
    .execute(&mut *tx)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(BillingError::conflict(
            "subscription was modified concurrently; retry the request",
        ));
    }

    let updated_sub: Subscription = sqlx::query_as("SELECT * FROM subscriptions WHERE id = $1")
        .bind(&sub.id)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(ChangePlanOutput {
        subscription: updated_sub,
        invoice: created_invoice,
        already_processed: false,
        proration_net: proration.net,
        old_plan_name: old_plan.name,
        new_plan_name: new_plan.name,
        customer_id: sub.customer_id,
    })
}
