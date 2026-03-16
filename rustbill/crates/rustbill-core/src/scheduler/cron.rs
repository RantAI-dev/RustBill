//! In-process scheduled tasks using tokio-cron-scheduler.

use crate::config::CronConfig;
use crate::notifications::email::EmailSender;
use sqlx::PgPool;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};

/// Start the background scheduler for subscription lifecycle and dunning.
pub async fn start_scheduler(
    config: &CronConfig,
    pool: PgPool,
    email_sender: Option<EmailSender>,
) -> anyhow::Result<JobScheduler> {
    let sched = JobScheduler::new().await?;

    if !config.enabled {
        tracing::info!("Scheduler disabled by config");
        return Ok(sched);
    }

    // Subscription lifecycle — default: every hour
    {
        let pool = pool.clone();
        let email_sender = email_sender.clone();
        let schedule = config.subscription_lifecycle.clone();
        sched
            .add(Job::new_async(schedule.as_str(), move |_uuid, _lock| {
                let pool = pool.clone();
                let email_sender = email_sender.clone();
                Box::pin(async move {
                    tracing::info!("Running subscription lifecycle cron");
                    let http_client = reqwest::Client::new();
                    match crate::billing::lifecycle::run_full_lifecycle(
                        &pool,
                        email_sender.as_ref(),
                        &http_client,
                    )
                    .await
                    {
                        Ok(result) => tracing::info!(
                            trials = result.trials_converted,
                            canceled = result.canceled,
                            renewed = result.renewed,
                            invoices = result.invoices_generated,
                            "Lifecycle cron completed"
                        ),
                        Err(e) => tracing::error!("Lifecycle cron error: {e}"),
                    }
                })
            })?)
            .await?;
    }

    // Dunning — default: every 6 hours
    {
        let pool = pool.clone();
        let schedule = config.dunning.clone();
        sched
            .add(Job::new_async(schedule.as_str(), move |_uuid, _lock| {
                let pool = pool.clone();
                Box::pin(async move {
                    tracing::info!("Running dunning cron");
                    let config = crate::billing::dunning::DunningConfig::default();
                    match crate::billing::dunning::run_dunning(&pool, &config).await {
                        Ok(processed) => {
                            tracing::info!(processed, "Dunning cron completed")
                        }
                        Err(e) => tracing::error!("Dunning cron error: {e}"),
                    }
                })
            })?)
            .await?;
    }

    sched.start().await?;
    tracing::info!("Scheduler started");

    Ok(sched)
}
