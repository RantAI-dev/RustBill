//! In-process scheduled tasks using tokio-cron-scheduler.

use crate::config::CronConfig;
use crate::notifications::email::EmailSender;
use crate::scheduler::repository::PgSchedulerRepository;
use crate::scheduler::service;
use sqlx::PgPool;
use tokio_cron_scheduler::JobScheduler;

/// Start the background scheduler for subscription lifecycle and dunning.
pub async fn start_scheduler(
    config: &CronConfig,
    pool: PgPool,
    email_sender: Option<EmailSender>,
) -> anyhow::Result<JobScheduler> {
    let repo = PgSchedulerRepository::new(pool, email_sender);
    service::start_scheduler(config, repo).await
}
