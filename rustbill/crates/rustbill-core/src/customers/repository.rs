use super::schema::{CreateCustomerRequest, CustomerMetrics, UpdateCustomerRequest};
use crate::db::models::{Customer, Trend};
use crate::error::{BillingError, Result};
use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;

#[async_trait]
pub trait CustomersRepository {
    async fn list_customers(&self) -> Result<Vec<Customer>>;
    async fn customer_metrics(&self, customer_id: &str) -> Result<CustomerMetrics>;
    async fn get_customer(&self, id: &str) -> Result<Customer>;
    async fn create_customer(&self, req: CreateCustomerRequest) -> Result<Customer>;
    async fn update_customer(&self, id: &str, req: UpdateCustomerRequest) -> Result<Customer>;
    async fn delete_customer(&self, id: &str) -> Result<u64>;
}

pub struct PgCustomersRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PgCustomersRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CustomersRepository for PgCustomersRepository<'_> {
    async fn list_customers(&self) -> Result<Vec<Customer>> {
        let rows =
            sqlx::query_as::<_, Customer>("SELECT * FROM customers ORDER BY created_at DESC")
                .fetch_all(self.pool)
                .await?;
        Ok(rows)
    }

    async fn customer_metrics(&self, customer_id: &str) -> Result<CustomerMetrics> {
        let total_revenue: Option<Decimal> =
            sqlx::query_scalar("SELECT COALESCE(SUM(value), 0) FROM deals WHERE customer_id = $1")
                .bind(customer_id)
                .fetch_one(self.pool)
                .await?;

        let last_contact: Option<String> =
            sqlx::query_scalar("SELECT MAX(date) FROM deals WHERE customer_id = $1")
                .bind(customer_id)
                .fetch_one(self.pool)
                .await?;

        let active_subs: Option<i64> = sqlx::query_scalar(
            "SELECT COUNT(*) FROM subscriptions WHERE customer_id = $1 AND status = 'active'",
        )
        .bind(customer_id)
        .fetch_one(self.pool)
        .await?;

        let recent_payments: Option<i64> = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM payments p
               JOIN invoices i ON p.invoice_id = i.id
               WHERE i.customer_id = $1
               AND p.paid_at >= NOW() - interval '90 days'"#,
        )
        .bind(customer_id)
        .fetch_one(self.pool)
        .await?;

        let recent_deals: Option<i64> = sqlx::query_scalar(
            "SELECT COUNT(*) FROM deals WHERE customer_id = $1 AND created_at >= NOW() - interval '90 days'",
        )
        .bind(customer_id)
        .fetch_one(self.pool)
        .await?;

        let mut health_score: i32 = 50;
        if active_subs.unwrap_or(0) > 0 {
            health_score += 20;
        }
        if recent_payments.unwrap_or(0) > 0 {
            health_score += 15;
        }
        if recent_deals.unwrap_or(0) > 0 {
            health_score += 15;
        }

        let this_month_rev: Option<Decimal> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(value), 0) FROM deals WHERE customer_id = $1 AND created_at >= date_trunc('month', CURRENT_DATE)",
        )
        .bind(customer_id)
        .fetch_one(self.pool)
        .await?;

        let last_month_rev: Option<Decimal> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(value), 0) FROM deals WHERE customer_id = $1 AND created_at >= date_trunc('month', CURRENT_DATE) - interval '1 month' AND created_at < date_trunc('month', CURRENT_DATE)",
        )
        .bind(customer_id)
        .fetch_one(self.pool)
        .await?;

        let trend = match (this_month_rev, last_month_rev) {
            (Some(tm), Some(lm)) if tm > lm => Trend::Up,
            (Some(tm), Some(lm)) if tm < lm => Trend::Down,
            _ => Trend::Stable,
        };

        Ok(CustomerMetrics {
            total_revenue: total_revenue.unwrap_or_default(),
            last_contact,
            health_score,
            trend,
        })
    }

    async fn get_customer(&self, id: &str) -> Result<Customer> {
        sqlx::query_as::<_, Customer>("SELECT * FROM customers WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?
            .ok_or_else(|| BillingError::not_found("customer", id))
    }

    async fn create_customer(&self, req: CreateCustomerRequest) -> Result<Customer> {
        let row = sqlx::query_as::<_, Customer>(
            r#"
            INSERT INTO customers (id, name, industry, tier, location, contact, email, phone,
                total_revenue, health_score, trend, last_contact,
                billing_email, billing_address, billing_city, billing_state,
                billing_zip, billing_country, tax_id, default_payment_method,
                stripe_customer_id, xendit_customer_id)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7,
                0, 50, 'stable', '',
                $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING *
            "#,
        )
        .bind(&req.name)
        .bind(&req.industry)
        .bind(&req.tier)
        .bind(&req.location)
        .bind(&req.contact)
        .bind(&req.email)
        .bind(&req.phone)
        .bind(&req.billing_email)
        .bind(&req.billing_address)
        .bind(&req.billing_city)
        .bind(&req.billing_state)
        .bind(&req.billing_zip)
        .bind(&req.billing_country)
        .bind(&req.tax_id)
        .bind(&req.default_payment_method)
        .bind(&req.stripe_customer_id)
        .bind(&req.xendit_customer_id)
        .fetch_one(self.pool)
        .await?;

        Ok(row)
    }

    async fn update_customer(&self, id: &str, req: UpdateCustomerRequest) -> Result<Customer> {
        let row = sqlx::query_as::<_, Customer>(
            r#"
            UPDATE customers SET
                name = COALESCE($2, name),
                industry = COALESCE($3, industry),
                tier = COALESCE($4, tier),
                location = COALESCE($5, location),
                contact = COALESCE($6, contact),
                email = COALESCE($7, email),
                phone = COALESCE($8, phone),
                billing_email = COALESCE($9, billing_email),
                billing_address = COALESCE($10, billing_address),
                billing_city = COALESCE($11, billing_city),
                billing_state = COALESCE($12, billing_state),
                billing_zip = COALESCE($13, billing_zip),
                billing_country = COALESCE($14, billing_country),
                tax_id = COALESCE($15, tax_id),
                default_payment_method = COALESCE($16, default_payment_method),
                stripe_customer_id = COALESCE($17, stripe_customer_id),
                xendit_customer_id = COALESCE($18, xendit_customer_id),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&req.name)
        .bind(&req.industry)
        .bind(&req.tier)
        .bind(&req.location)
        .bind(&req.contact)
        .bind(&req.email)
        .bind(&req.phone)
        .bind(&req.billing_email)
        .bind(&req.billing_address)
        .bind(&req.billing_city)
        .bind(&req.billing_state)
        .bind(&req.billing_zip)
        .bind(&req.billing_country)
        .bind(&req.tax_id)
        .bind(&req.default_payment_method)
        .bind(&req.stripe_customer_id)
        .bind(&req.xendit_customer_id)
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| BillingError::not_found("customer", id))?;

        Ok(row)
    }

    async fn delete_customer(&self, id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM customers WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
