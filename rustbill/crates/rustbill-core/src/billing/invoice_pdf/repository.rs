use crate::db::models::{Customer, Invoice, InvoiceItem};
use crate::error::Result;
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
pub trait InvoicePdfRepository {
    async fn get_invoice(&self, invoice_id: &str) -> Result<Invoice>;
    async fn get_customer(&self, customer_id: &str) -> Result<Option<Customer>>;
    async fn list_invoice_items(&self, invoice_id: &str) -> Result<Vec<InvoiceItem>>;
}

pub struct PgInvoicePdfRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PgInvoicePdfRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InvoicePdfRepository for PgInvoicePdfRepository<'_> {
    async fn get_invoice(&self, invoice_id: &str) -> Result<Invoice> {
        let invoice = sqlx::query_as::<_, Invoice>(
            "SELECT * FROM invoices WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(invoice_id)
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| crate::error::BillingError::not_found("invoice", invoice_id))?;

        Ok(invoice)
    }

    async fn get_customer(&self, customer_id: &str) -> Result<Option<Customer>> {
        let customer = sqlx::query_as::<_, Customer>("SELECT * FROM customers WHERE id = $1")
            .bind(customer_id)
            .fetch_optional(self.pool)
            .await?;

        Ok(customer)
    }

    async fn list_invoice_items(&self, invoice_id: &str) -> Result<Vec<InvoiceItem>> {
        let items = sqlx::query_as::<_, InvoiceItem>(
            "SELECT * FROM invoice_items WHERE invoice_id = $1 ORDER BY id",
        )
        .bind(invoice_id)
        .fetch_all(self.pool)
        .await?;

        Ok(items)
    }
}
