use super::schema::CheckoutContext;
use crate::db::models::{Customer, Invoice};
use crate::error::{BillingError, Result};
use crate::payments::{lemonsqueezy, xendit};
use crate::settings::provider_settings::ProviderSettings;
use async_trait::async_trait;
use reqwest::Client;
use sqlx::PgPool;

#[async_trait]
pub trait CheckoutRepository {
    async fn find_invoice(&self, invoice_id: &str) -> Result<Invoice>;
    async fn find_customer(&self, customer_id: &str) -> Result<Customer>;
    async fn create_xendit_checkout(&self, ctx: &CheckoutContext)
        -> Result<CheckoutProviderResult>;
    async fn save_xendit_invoice_id(&self, invoice_id: &str, provider_id: &str) -> Result<()>;
    async fn create_lemonsqueezy_checkout(
        &self,
        ctx: &CheckoutContext,
    ) -> Result<CheckoutProviderResult>;
    async fn save_lemonsqueezy_checkout_id(
        &self,
        invoice_id: &str,
        provider_id: &str,
    ) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct CheckoutProviderResult {
    pub checkout_url: String,
    pub provider_reference: String,
}

pub struct PgCheckoutRepository<'a> {
    pool: &'a PgPool,
    http: &'a Client,
    settings: &'a ProviderSettings,
}

impl<'a> PgCheckoutRepository<'a> {
    pub fn new(pool: &'a PgPool, http: &'a Client, settings: &'a ProviderSettings) -> Self {
        Self {
            pool,
            http,
            settings,
        }
    }
}

#[async_trait]
impl CheckoutRepository for PgCheckoutRepository<'_> {
    async fn find_invoice(&self, invoice_id: &str) -> Result<Invoice> {
        crate::billing::invoices::get_invoice(self.pool, invoice_id).await
    }

    async fn find_customer(&self, customer_id: &str) -> Result<Customer> {
        let customer = sqlx::query_as::<_, Customer>("SELECT * FROM customers WHERE id = $1")
            .bind(customer_id)
            .fetch_optional(self.pool)
            .await?
            .ok_or_else(|| BillingError::not_found("customer", customer_id))?;

        Ok(customer)
    }

    async fn create_xendit_checkout(
        &self,
        ctx: &CheckoutContext,
    ) -> Result<CheckoutProviderResult> {
        let params = xendit::XenditInvoiceParams {
            invoice_id: ctx.invoice.id.clone(),
            invoice_number: ctx.invoice.invoice_number.clone(),
            total: ctx.invoice.total,
            currency: ctx.invoice.currency.clone(),
            customer_email: Some(ctx.customer.email.clone()),
            customer_name: Some(ctx.customer.name.clone()),
            success_url: ctx.success_url.clone(),
            failure_url: ctx.cancel_url.clone(),
        };

        let created = xendit::create_invoice(self.http, self.settings, params).await?;

        Ok(CheckoutProviderResult {
            checkout_url: created.invoice_url,
            provider_reference: created.xendit_invoice_id,
        })
    }

    async fn save_xendit_invoice_id(&self, invoice_id: &str, provider_id: &str) -> Result<()> {
        sqlx::query("UPDATE invoices SET xendit_invoice_id = $2, updated_at = NOW() WHERE id = $1")
            .bind(invoice_id)
            .bind(provider_id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    async fn create_lemonsqueezy_checkout(
        &self,
        ctx: &CheckoutContext,
    ) -> Result<CheckoutProviderResult> {
        let params = lemonsqueezy::LsCheckoutParams {
            invoice_id: ctx.invoice.id.clone(),
            invoice_number: ctx.invoice.invoice_number.clone(),
            total: ctx.invoice.total,
            currency: ctx.invoice.currency.clone(),
            customer_email: Some(ctx.customer.email.clone()),
            customer_name: Some(ctx.customer.name.clone()),
            success_url: ctx.success_url.clone(),
        };

        let created = lemonsqueezy::create_checkout(self.http, self.settings, params).await?;

        Ok(CheckoutProviderResult {
            checkout_url: created.checkout_url,
            provider_reference: created.checkout_id,
        })
    }

    async fn save_lemonsqueezy_checkout_id(
        &self,
        invoice_id: &str,
        provider_id: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE invoices SET lemonsqueezy_order_id = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(invoice_id)
        .bind(provider_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }
}
