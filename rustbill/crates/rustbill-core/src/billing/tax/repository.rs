use super::schema::{CreateTaxRuleRequest, TaxResult, UpdateTaxRuleRequest};
use crate::db::models::TaxRule;
use crate::error::Result;
use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;

#[async_trait]
pub trait TaxRepository: Send + Sync {
    async fn find_tax_rule(&self, country: &str, region: Option<&str>) -> Result<Option<TaxRule>>;

    async fn list_tax_rules(&self) -> Result<Vec<TaxRule>>;

    async fn create_tax_rule(&self, req: &CreateTaxRuleRequest) -> Result<TaxRule>;

    async fn update_tax_rule(&self, id: &str, req: &UpdateTaxRuleRequest) -> Result<TaxRule>;

    async fn delete_tax_rule(&self, id: &str) -> Result<()>;

    async fn resolve_external_tax(
        &self,
        country: &str,
        region: Option<&str>,
        product_category: Option<&str>,
        subtotal: Decimal,
    ) -> Result<Option<TaxResult>>;
}

#[derive(Clone)]
pub struct PgTaxRepository {
    pool: PgPool,
}

impl PgTaxRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl TaxRepository for PgTaxRepository {
    async fn find_tax_rule(&self, country: &str, region: Option<&str>) -> Result<Option<TaxRule>> {
        let today = chrono::Utc::now().date_naive();

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
            .fetch_optional(&self.pool)
            .await?;

            if rule.is_some() {
                return Ok(rule);
            }
        }

        let rule: Option<TaxRule> = sqlx::query_as(
            r#"SELECT * FROM tax_rules
               WHERE country = $1 AND region IS NULL AND active = TRUE
               AND effective_from <= $2 AND (effective_to IS NULL OR effective_to > $2)
               ORDER BY effective_from DESC LIMIT 1"#,
        )
        .bind(country)
        .bind(today)
        .fetch_optional(&self.pool)
        .await?;

        Ok(rule)
    }

    async fn list_tax_rules(&self) -> Result<Vec<TaxRule>> {
        let rules = sqlx::query_as::<_, TaxRule>(
            "SELECT * FROM tax_rules WHERE active = TRUE ORDER BY country, region, effective_from DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rules)
    }

    async fn create_tax_rule(&self, req: &CreateTaxRuleRequest) -> Result<TaxRule> {
        let rule = sqlx::query_as::<_, TaxRule>(
            r#"INSERT INTO tax_rules (id, country, region, tax_name, rate, inclusive, product_category, active, effective_from)
               VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, TRUE, CURRENT_DATE)
               RETURNING *"#,
        )
        .bind(&req.country)
        .bind(&req.region)
        .bind(&req.tax_name)
        .bind(req.rate)
        .bind(req.inclusive)
        .bind(&req.product_category)
        .fetch_one(&self.pool)
        .await?;
        Ok(rule)
    }

    async fn update_tax_rule(&self, id: &str, req: &UpdateTaxRuleRequest) -> Result<TaxRule> {
        let mut tx = self.pool.begin().await?;

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
        .bind(&req.tax_name)
        .bind(req.rate)
        .bind(req.inclusive)
        .bind(&old.product_category)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(new_rule)
    }

    async fn delete_tax_rule(&self, id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE tax_rules SET effective_to = CURRENT_DATE, active = FALSE WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn resolve_external_tax(
        &self,
        country: &str,
        region: Option<&str>,
        product_category: Option<&str>,
        subtotal: Decimal,
    ) -> Result<Option<TaxResult>> {
        if country.trim().is_empty() || subtotal <= Decimal::ZERO {
            return Ok(None);
        }

        let provider = get_setting(&self.pool, "external_tax_provider").await?;
        if provider.is_empty() {
            return Ok(None);
        }

        let external = match provider.to_lowercase().as_str() {
            "taxjar" => fetch_taxjar_tax(&self.pool, country, region, subtotal).await,
            "stripe" => fetch_stripe_tax(&self.pool, country, region, subtotal).await,
            _ => Ok(None),
        };

        let Some(external) = external? else {
            return Ok(None);
        };

        cache_external_rule(&self.pool, country, region, product_category, &external).await?;

        Ok(Some(TaxResult {
            rate: external.rate,
            amount: external.amount,
            name: external.name,
            inclusive: external.inclusive,
        }))
    }
}

#[derive(Debug)]
struct ExternalTaxResult {
    rate: Decimal,
    amount: Decimal,
    name: String,
    inclusive: bool,
}

async fn cache_external_rule(
    pool: &PgPool,
    country: &str,
    region: Option<&str>,
    product_category: Option<&str>,
    external: &ExternalTaxResult,
) -> Result<()> {
    let today = chrono::Utc::now().date_naive();
    let effective_to = today + chrono::Duration::days(90);

    sqlx::query(
        r#"INSERT INTO tax_rules
           (id, country, region, tax_name, rate, inclusive, product_category, active, effective_from, effective_to, created_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, TRUE, $7, $8, NOW())"#,
    )
    .bind(country)
    .bind(region)
    .bind(&external.name)
    .bind(external.rate)
    .bind(external.inclusive)
    .bind(product_category)
    .bind(today)
    .bind(effective_to)
    .execute(pool)
    .await?;

    Ok(())
}

async fn fetch_taxjar_tax(
    pool: &PgPool,
    country: &str,
    region: Option<&str>,
    subtotal: Decimal,
) -> Result<Option<ExternalTaxResult>> {
    let api_key = get_setting(pool, "taxjar_api_key").await?;
    if api_key.is_empty() {
        return Ok(None);
    }

    let amount = subtotal.to_string().parse::<f64>().unwrap_or(0.0);
    let body = serde_json::json!({
        "to_country": country,
        "to_state": region.unwrap_or(""),
        "amount": amount,
        "shipping": 0.0,
    });

    let client = reqwest::Client::new();
    let resp = match client
        .post("https://api.taxjar.com/v2/taxes")
        .header("Authorization", format!("Token {}", api_key))
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!("taxjar fallback request failed: {err}");
            return Ok(None);
        }
    };

    if !resp.status().is_success() {
        tracing::warn!("taxjar fallback returned status {}", resp.status());
        return Ok(None);
    }

    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    let amount_collect = body["tax"]["amount_to_collect"].as_f64().unwrap_or(0.0);
    let rate = body["tax"]["rate"].as_f64().unwrap_or(0.0) / 100.0;

    Ok(Some(ExternalTaxResult {
        rate: Decimal::from_str_exact(&format!("{rate:.6}")).unwrap_or(Decimal::ZERO),
        amount: Decimal::from_str_exact(&format!("{amount_collect:.2}")).unwrap_or(Decimal::ZERO),
        name: "TaxJar".to_string(),
        inclusive: false,
    }))
}

async fn fetch_stripe_tax(
    pool: &PgPool,
    country: &str,
    region: Option<&str>,
    subtotal: Decimal,
) -> Result<Option<ExternalTaxResult>> {
    let secret_key = get_setting(pool, "stripe_secret_key").await?;
    if secret_key.is_empty() {
        return Ok(None);
    }

    let amount_minor = ((subtotal * Decimal::new(100, 0)).round_dp(0))
        .to_string()
        .parse::<i64>()
        .unwrap_or(0);

    let mut form = vec![
        ("currency", "usd".to_string()),
        ("customer_details[address][country]", country.to_string()),
        (
            "customer_details[address][state]",
            region.unwrap_or("").to_string(),
        ),
        ("line_items[0][amount]", amount_minor.to_string()),
        ("line_items[0][reference]", "invoice_subtotal".to_string()),
    ];
    form.push(("customer_details[address_source]", "shipping".to_string()));

    let client = reqwest::Client::new();
    let resp = match client
        .post("https://api.stripe.com/v1/tax/calculations")
        .bearer_auth(secret_key)
        .form(&form)
        .send()
        .await
    {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!("stripe tax fallback request failed: {err}");
            return Ok(None);
        }
    };

    if !resp.status().is_success() {
        tracing::warn!("stripe tax fallback returned status {}", resp.status());
        return Ok(None);
    }

    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    let amount_tax = body["tax_amount_exclusive"].as_i64().unwrap_or(0);
    if amount_minor <= 0 {
        return Ok(None);
    }
    let rate = (amount_tax as f64) / (amount_minor as f64);

    Ok(Some(ExternalTaxResult {
        rate: Decimal::from_str_exact(&format!("{rate:.6}")).unwrap_or(Decimal::ZERO),
        amount: Decimal::new(amount_tax, 2),
        name: "Stripe Tax".to_string(),
        inclusive: false,
    }))
}

async fn get_setting(pool: &PgPool, key: &str) -> Result<String> {
    Ok(
        sqlx::query_scalar::<_, String>("SELECT value FROM system_settings WHERE key = $1")
            .bind(key)
            .fetch_optional(pool)
            .await?
            .unwrap_or_default(),
    )
}
