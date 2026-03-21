use async_trait::async_trait;
use rustbill_core::billing::{dunning::DunningConfig, lifecycle::LifecycleResult};
use rustbill_core::error::BillingError;
use rustbill_core::notifications::email::EmailSender;
use sqlx::PgPool;

#[async_trait]
pub trait CronRepository: Send + Sync {
    async fn run_full_lifecycle(&self) -> Result<LifecycleResult, BillingError>;
    async fn generate_pending_invoices(&self) -> Result<u64, BillingError>;
    async fn run_dunning(&self, config: &DunningConfig) -> Result<u64, BillingError>;
    async fn expire_licenses(&self) -> Result<i64, BillingError>;
}

#[derive(Clone)]
pub struct AppCronRepository {
    pool: PgPool,
    email_sender: Option<EmailSender>,
    http_client: reqwest::Client,
}

impl AppCronRepository {
    pub fn new(
        pool: PgPool,
        email_sender: Option<EmailSender>,
        http_client: reqwest::Client,
    ) -> Self {
        Self {
            pool,
            email_sender,
            http_client,
        }
    }
}

#[async_trait]
impl CronRepository for AppCronRepository {
    async fn run_full_lifecycle(&self) -> Result<LifecycleResult, BillingError> {
        rustbill_core::billing::lifecycle::run_full_lifecycle(
            &self.pool,
            self.email_sender.as_ref(),
            &self.http_client,
        )
        .await
    }

    async fn generate_pending_invoices(&self) -> Result<u64, BillingError> {
        rustbill_core::billing::lifecycle::generate_pending_invoices(
            &self.pool,
            self.email_sender.as_ref(),
            &self.http_client,
        )
        .await
    }

    async fn run_dunning(&self, config: &DunningConfig) -> Result<u64, BillingError> {
        rustbill_core::billing::dunning::run_dunning(&self.pool, config).await
    }

    async fn expire_licenses(&self) -> Result<i64, BillingError> {
        sqlx::query_scalar::<_, i64>(
            r#"WITH expired AS (
                 UPDATE licenses
                 SET status = 'expired', updated_at = now()
                 WHERE status = 'active'
                   AND expires_at IS NOT NULL
                   AND expires_at <= now()
                 RETURNING id
               )
               SELECT COUNT(*) FROM expired"#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }
}
