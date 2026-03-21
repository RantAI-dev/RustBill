use super::repository::SearchRepository;
use super::schema::SearchResult;
use crate::error::Result;

pub async fn global_search<R: SearchRepository + ?Sized>(
    repo: &R,
    query: &str,
) -> Result<Vec<SearchResult>> {
    if query.len() < 2 {
        return Ok(Vec::new());
    }

    let pattern = format!("%{query}%");
    let mut results = Vec::new();

    for (id, name) in repo.find_products(&pattern).await? {
        results.push(SearchResult {
            result_type: "product".to_string(),
            id,
            name,
            description: None,
        });
    }

    for (id, name, email) in repo.find_customers(&pattern).await? {
        results.push(SearchResult {
            result_type: "customer".to_string(),
            id,
            name,
            description: Some(email),
        });
    }

    for (key, customer, product) in repo.find_licenses(&pattern).await? {
        results.push(SearchResult {
            result_type: "license".to_string(),
            id: key,
            name: customer,
            description: Some(product),
        });
    }

    for (id, invoice_number) in repo.find_invoices(&pattern).await? {
        results.push(SearchResult {
            result_type: "invoice".to_string(),
            id,
            name: invoice_number,
            description: None,
        });
    }

    for (id, company, product_name) in repo.find_deals(&pattern).await? {
        results.push(SearchResult {
            result_type: "deal".to_string(),
            id,
            name: company,
            description: Some(product_name),
        });
    }

    Ok(results)
}
