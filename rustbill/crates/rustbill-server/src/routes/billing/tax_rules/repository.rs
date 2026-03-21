use super::schema::{CreateTaxRuleRequest, UpdateTaxRuleRequest};
use async_trait::async_trait;
use rustbill_core::billing::tax;
use rustbill_core::db::models::TaxRule;
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait TaxRulesRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<TaxRule>, BillingError>;
    async fn create(&self, body: &CreateTaxRuleRequest) -> Result<TaxRule, BillingError>;
    async fn update(&self, id: &str, body: &UpdateTaxRuleRequest) -> Result<TaxRule, BillingError>;
    async fn remove(&self, id: &str) -> Result<(), BillingError>;
}

#[derive(Clone)]
pub struct SqlxTaxRulesRepository {
    pool: PgPool,
}

impl SqlxTaxRulesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TaxRulesRepository for SqlxTaxRulesRepository {
    async fn list(&self) -> Result<Vec<TaxRule>, BillingError> {
        tax::list_tax_rules(&self.pool).await
    }

    async fn create(&self, body: &CreateTaxRuleRequest) -> Result<TaxRule, BillingError> {
        tax::create_tax_rule(
            &self.pool,
            &body.country,
            body.region.as_deref(),
            &body.tax_name,
            body.rate,
            body.inclusive,
            body.product_category.as_deref(),
        )
        .await
    }

    async fn update(&self, id: &str, body: &UpdateTaxRuleRequest) -> Result<TaxRule, BillingError> {
        tax::update_tax_rule(&self.pool, id, &body.tax_name, body.rate, body.inclusive).await
    }

    async fn remove(&self, id: &str) -> Result<(), BillingError> {
        tax::delete_tax_rule(&self.pool, id).await
    }
}
