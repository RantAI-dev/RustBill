use crate::notifications::email::EmailSender;
use crate::scheduler::schema::SchedulerRunResult;
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
pub trait SchedulerRepository: Send + Sync {
    async fn run_subscription_lifecycle(&self) -> anyhow::Result<SchedulerRunResult>;
    async fn run_dunning(&self) -> anyhow::Result<SchedulerRunResult>;
}

#[derive(Clone)]
pub struct PgSchedulerRepository {
    pool: PgPool,
    email_sender: Option<EmailSender>,
}

impl PgSchedulerRepository {
    pub fn new(pool: PgPool, email_sender: Option<EmailSender>) -> Self {
        Self { pool, email_sender }
    }
}

#[async_trait]
impl SchedulerRepository for PgSchedulerRepository {
    async fn run_subscription_lifecycle(&self) -> anyhow::Result<SchedulerRunResult> {
        let http_client = reqwest::Client::new();
        let result = crate::billing::lifecycle::run_full_lifecycle(
            &self.pool,
            self.email_sender.as_ref(),
            &http_client,
        )
        .await?;

        tracing::info!(
            trials = result.trials_converted,
            canceled = result.canceled,
            pre_generated = result.pre_generated,
            renewed = result.renewed,
            invoices = result.invoices_generated,
            "Lifecycle cron completed"
        );

        let total = result.trials_converted
            + result.canceled
            + result.pre_generated
            + result.renewed
            + result.invoices_generated;
        let processed = i64::try_from(total).unwrap_or(i64::MAX);
        Ok(SchedulerRunResult { processed })
    }

    async fn run_dunning(&self) -> anyhow::Result<SchedulerRunResult> {
        let config = crate::billing::dunning::DunningConfig::default();
        let processed = crate::billing::dunning::run_dunning(&self.pool, &config).await?;
        tracing::info!(processed, "Dunning cron completed");
        Ok(SchedulerRunResult {
            processed: i64::try_from(processed).unwrap_or(i64::MAX),
        })
    }
}
