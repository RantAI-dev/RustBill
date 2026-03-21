use async_trait::async_trait;
use rust_decimal::Decimal;
use rustbill_core::billing::credits as core_credits;
use rustbill_core::db::models::{CreditReason, CustomerCredit};
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait CreditsRepository: Send + Sync {
    async fn adjust(
        &self,
        customer_id: &str,
        currency: &str,
        amount: Decimal,
        reason: CreditReason,
        description: &str,
        invoice_id: Option<&str>,
    ) -> Result<CustomerCredit, BillingError>;

    async fn find_adjustment(&self, id: &str) -> Result<CustomerCredit, BillingError>;

    async fn get_balance(&self, customer_id: &str, currency: &str)
        -> Result<Decimal, BillingError>;

    async fn list_credits(
        &self,
        customer_id: &str,
        currency: Option<&str>,
    ) -> Result<Vec<CustomerCredit>, BillingError>;
}

#[derive(Clone)]
pub struct SqlxCreditsRepository {
    pool: PgPool,
}

impl SqlxCreditsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CreditsRepository for SqlxCreditsRepository {
    async fn adjust(
        &self,
        customer_id: &str,
        currency: &str,
        amount: Decimal,
        reason: CreditReason,
        description: &str,
        invoice_id: Option<&str>,
    ) -> Result<CustomerCredit, BillingError> {
        core_credits::adjust(
            &self.pool,
            customer_id,
            currency,
            amount,
            reason,
            description,
            invoice_id,
        )
        .await
    }

    async fn find_adjustment(&self, id: &str) -> Result<CustomerCredit, BillingError> {
        sqlx::query_as::<_, CustomerCredit>("SELECT * FROM customer_credits WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(BillingError::from)?
            .ok_or_else(|| BillingError::not_found("credit_adjustment", id))
    }

    async fn get_balance(
        &self,
        customer_id: &str,
        currency: &str,
    ) -> Result<Decimal, BillingError> {
        core_credits::get_balance(&self.pool, customer_id, currency).await
    }

    async fn list_credits(
        &self,
        customer_id: &str,
        currency: Option<&str>,
    ) -> Result<Vec<CustomerCredit>, BillingError> {
        core_credits::list_credits(&self.pool, customer_id, currency).await
    }
}
