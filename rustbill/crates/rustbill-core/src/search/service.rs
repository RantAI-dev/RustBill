//! Global search across multiple tables.

use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Serialize)]
pub struct SearchResult {
    #[serde(rename = "type")]
    pub result_type: String,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Search across products, customers, deals, licenses, invoices, subscriptions.
pub async fn global_search(pool: &PgPool, query: &str) -> crate::error::Result<Vec<SearchResult>> {
    if query.len() < 2 {
        return Ok(vec![]);
    }

    let pattern = format!("%{query}%");
    let mut results = Vec::new();

    // Products
    let products: Vec<(String, String)> =
        sqlx::query_as("SELECT id, name FROM products WHERE name ILIKE $1 LIMIT 5")
            .bind(&pattern)
            .fetch_all(pool)
            .await?;

    for (id, name) in products {
        results.push(SearchResult {
            result_type: "product".to_string(),
            id,
            name,
            description: None,
        });
    }

    // Customers
    let customers: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, name, email FROM customers WHERE name ILIKE $1 OR email ILIKE $1 LIMIT 5",
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    for (id, name, email) in customers {
        results.push(SearchResult {
            result_type: "customer".to_string(),
            id,
            name,
            description: Some(email),
        });
    }

    // Licenses
    let licenses: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT key, customer_name, product_name FROM licenses WHERE key ILIKE $1 OR customer_name ILIKE $1 LIMIT 5"
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    for (key, customer, product) in licenses {
        results.push(SearchResult {
            result_type: "license".to_string(),
            id: key,
            name: customer,
            description: Some(product),
        });
    }

    // Invoices
    let invoices: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, invoice_number FROM invoices WHERE invoice_number ILIKE $1 AND deleted_at IS NULL LIMIT 5"
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    for (id, number) in invoices {
        results.push(SearchResult {
            result_type: "invoice".to_string(),
            id,
            name: number,
            description: None,
        });
    }

    // Deals
    let deals: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, company, product_name FROM deals WHERE company ILIKE $1 OR product_name ILIKE $1 LIMIT 5"
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    for (id, company, product) in deals {
        results.push(SearchResult {
            result_type: "deal".to_string(),
            id,
            name: company,
            description: Some(product),
        });
    }

    Ok(results)
}
