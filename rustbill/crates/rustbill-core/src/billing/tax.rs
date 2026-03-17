use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::db::models::TaxRule;
use crate::error::Result;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaxResult {
    pub rate: Decimal,
    pub amount: Decimal,
    pub name: String,
    pub inclusive: bool,
}

impl TaxResult {
    pub fn zero() -> Self {
        Self {
            rate: Decimal::ZERO,
            amount: Decimal::ZERO,
            name: String::new(),
            inclusive: false,
        }
    }
}

pub async fn resolve_tax(
    pool: &PgPool,
    country: &str,
    region: Option<&str>,
    _product_category: Option<&str>,
    subtotal: Decimal,
) -> Result<TaxResult> {
    let rule = find_tax_rule(pool, country, region).await?;
    match rule {
        Some(r) => Ok(calculate_tax(subtotal, &r)),
        None => Ok(TaxResult::zero()),
    }
}

async fn find_tax_rule(
    pool: &PgPool,
    country: &str,
    region: Option<&str>,
) -> Result<Option<TaxRule>> {
    let today = chrono::Utc::now().date_naive();

    // Try region-specific first
    if let Some(region) = region {
        let rule: Option<TaxRule> = sqlx::query_as(
            r#"SELECT * FROM tax_rules
               WHERE country = $1 AND region = $2 AND active = TRUE
               AND effective_from <= $3 AND (effective_to IS NULL OR effective_to > $3)
               ORDER BY effective_from DESC LIMIT 1"#,
        )
        .bind(country)
        .bind(region)
        .bind(today)
        .fetch_optional(pool)
        .await?;

        if rule.is_some() {
            return Ok(rule);
        }
    }

    // Fallback to country-only
    let rule: Option<TaxRule> = sqlx::query_as(
        r#"SELECT * FROM tax_rules
           WHERE country = $1 AND region IS NULL AND active = TRUE
           AND effective_from <= $2 AND (effective_to IS NULL OR effective_to > $2)
           ORDER BY effective_from DESC LIMIT 1"#,
    )
    .bind(country)
    .bind(today)
    .fetch_optional(pool)
    .await?;

    Ok(rule)
}

pub fn calculate_tax(subtotal: Decimal, rule: &TaxRule) -> TaxResult {
    let amount = if rule.inclusive {
        let divisor = Decimal::ONE + rule.rate;
        (subtotal * rule.rate / divisor).round_dp(2)
    } else {
        (subtotal * rule.rate).round_dp(2)
    };

    TaxResult {
        rate: rule.rate,
        amount,
        name: rule.tax_name.clone(),
        inclusive: rule.inclusive,
    }
}

// ---- CRUD for admin ----

pub async fn list_tax_rules(pool: &PgPool) -> Result<Vec<TaxRule>> {
    let rules = sqlx::query_as::<_, TaxRule>(
        "SELECT * FROM tax_rules WHERE active = TRUE ORDER BY country, region, effective_from DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rules)
}

pub async fn create_tax_rule(
    pool: &PgPool,
    country: &str,
    region: Option<&str>,
    tax_name: &str,
    rate: Decimal,
    inclusive: bool,
    product_category: Option<&str>,
) -> Result<TaxRule> {
    let rule = sqlx::query_as::<_, TaxRule>(
        r#"INSERT INTO tax_rules (id, country, region, tax_name, rate, inclusive, product_category, active, effective_from)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, TRUE, CURRENT_DATE)
           RETURNING *"#,
    )
    .bind(country)
    .bind(region)
    .bind(tax_name)
    .bind(rate)
    .bind(inclusive)
    .bind(product_category)
    .fetch_one(pool)
    .await?;
    Ok(rule)
}

pub async fn update_tax_rule(
    pool: &PgPool,
    id: &str,
    tax_name: &str,
    rate: Decimal,
    inclusive: bool,
) -> Result<TaxRule> {
    let mut tx = pool.begin().await?;

    let old: TaxRule = sqlx::query_as(
        "UPDATE tax_rules SET effective_to = CURRENT_DATE, active = FALSE WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;

    let new_rule = sqlx::query_as::<_, TaxRule>(
        r#"INSERT INTO tax_rules (id, country, region, tax_name, rate, inclusive, product_category, active, effective_from)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, TRUE, CURRENT_DATE)
           RETURNING *"#,
    )
    .bind(&old.country)
    .bind(&old.region)
    .bind(tax_name)
    .bind(rate)
    .bind(inclusive)
    .bind(&old.product_category)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(new_rule)
}

pub async fn delete_tax_rule(pool: &PgPool, id: &str) -> Result<()> {
    sqlx::query("UPDATE tax_rules SET effective_to = CURRENT_DATE, active = FALSE WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
