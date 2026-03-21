use super::repository::SearchRepository;
use super::schema::SearchParams;
use rustbill_core::error::BillingError;

pub async fn search<R: SearchRepository>(
    repo: &R,
    params: &SearchParams,
) -> Result<serde_json::Value, BillingError> {
    let query = format!("%{}%", params.q);
    let limit = params.limit.unwrap_or(20).min(100);

    let customers = repo.search_customers(&query, limit).await?;
    let products = repo.search_products(&query, limit).await?;
    let licenses = repo.search_licenses(&query, limit).await?;

    let mut results = Vec::new();
    results.extend(customers);
    results.extend(products);
    results.extend(licenses);

    Ok(serde_json::json!({
        "query": params.q,
        "results": results,
        "total": results.len(),
    }))
}
