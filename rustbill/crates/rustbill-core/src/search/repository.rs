use crate::error::Result;
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
pub trait SearchRepository {
    async fn find_products(&self, pattern: &str) -> Result<Vec<(String, String)>>;
    async fn find_customers(&self, pattern: &str) -> Result<Vec<(String, String, String)>>;
    async fn find_licenses(&self, pattern: &str) -> Result<Vec<(String, String, String)>>;
    async fn find_invoices(&self, pattern: &str) -> Result<Vec<(String, String)>>;
    async fn find_deals(&self, pattern: &str) -> Result<Vec<(String, String, String)>>;
}

pub struct PgSearchRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PgSearchRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SearchRepository for PgSearchRepository<'_> {
    async fn find_products(&self, pattern: &str) -> Result<Vec<(String, String)>> {
        let products = sqlx::query_as("SELECT id, name FROM products WHERE name ILIKE $1 LIMIT 5")
            .bind(pattern)
            .fetch_all(self.pool)
            .await?;
        Ok(products)
    }

    async fn find_customers(&self, pattern: &str) -> Result<Vec<(String, String, String)>> {
        let customers = sqlx::query_as(
            "SELECT id, name, email FROM customers WHERE name ILIKE $1 OR email ILIKE $1 LIMIT 5",
        )
        .bind(pattern)
        .fetch_all(self.pool)
        .await?;
        Ok(customers)
    }

    async fn find_licenses(&self, pattern: &str) -> Result<Vec<(String, String, String)>> {
        let licenses = sqlx::query_as(
            "SELECT key, customer_name, product_name FROM licenses WHERE key ILIKE $1 OR customer_name ILIKE $1 LIMIT 5",
        )
        .bind(pattern)
        .fetch_all(self.pool)
        .await?;
        Ok(licenses)
    }

    async fn find_invoices(&self, pattern: &str) -> Result<Vec<(String, String)>> {
        let invoices = sqlx::query_as(
            "SELECT id, invoice_number FROM invoices WHERE invoice_number ILIKE $1 AND deleted_at IS NULL LIMIT 5",
        )
        .bind(pattern)
        .fetch_all(self.pool)
        .await?;
        Ok(invoices)
    }

    async fn find_deals(&self, pattern: &str) -> Result<Vec<(String, String, String)>> {
        let deals = sqlx::query_as(
            "SELECT id, company, product_name FROM deals WHERE company ILIKE $1 OR product_name ILIKE $1 LIMIT 5",
        )
        .bind(pattern)
        .fetch_all(self.pool)
        .await?;
        Ok(deals)
    }
}
