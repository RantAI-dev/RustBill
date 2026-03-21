pub mod repository;
pub mod schema;
pub mod service;

use crate::db::models::TaxRule;
use crate::error::Result;
use repository::PgTaxRepository;
use sqlx::PgPool;

pub use schema::{CreateTaxRuleRequest, ResolveTaxRequest, TaxResult, UpdateTaxRuleRequest};

pub async fn resolve_tax(
    pool: &PgPool,
    country: &str,
    region: Option<&str>,
    product_category: Option<&str>,
    subtotal: rust_decimal::Decimal,
) -> Result<TaxResult> {
    let repo = PgTaxRepository::new(pool);
    service::resolve_tax(
        &repo,
        schema::ResolveTaxRequest {
            country: country.to_string(),
            region: region.map(str::to_string),
            product_category: product_category.map(str::to_string),
            subtotal,
        },
    )
    .await
}

pub fn calculate_tax(subtotal: rust_decimal::Decimal, rule: &TaxRule) -> TaxResult {
    service::calculate_tax(subtotal, rule)
}

pub async fn list_tax_rules(pool: &PgPool) -> Result<Vec<TaxRule>> {
    let repo = PgTaxRepository::new(pool);
    service::list_tax_rules(&repo).await
}

pub async fn create_tax_rule(
    pool: &PgPool,
    country: &str,
    region: Option<&str>,
    tax_name: &str,
    rate: rust_decimal::Decimal,
    inclusive: bool,
    product_category: Option<&str>,
) -> Result<TaxRule> {
    let repo = PgTaxRepository::new(pool);
    service::create_tax_rule(
        &repo,
        schema::CreateTaxRuleRequest {
            country: country.to_string(),
            region: region.map(str::to_string),
            tax_name: tax_name.to_string(),
            rate,
            inclusive,
            product_category: product_category.map(str::to_string),
        },
    )
    .await
}

pub async fn update_tax_rule(
    pool: &PgPool,
    id: &str,
    tax_name: &str,
    rate: rust_decimal::Decimal,
    inclusive: bool,
) -> Result<TaxRule> {
    let repo = PgTaxRepository::new(pool);
    service::update_tax_rule(
        &repo,
        id,
        schema::UpdateTaxRuleRequest {
            tax_name: tax_name.to_string(),
            rate,
            inclusive,
        },
    )
    .await
}

pub async fn delete_tax_rule(pool: &PgPool, id: &str) -> Result<()> {
    let repo = PgTaxRepository::new(pool);
    service::delete_tax_rule(&repo, id).await
}
