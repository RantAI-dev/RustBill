use super::repository::SchedulerRepository;
use crate::config::CronConfig;
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn start_scheduler<R>(config: &CronConfig, repo: R) -> anyhow::Result<JobScheduler>
where
    R: SchedulerRepository + Clone + Send + Sync + 'static,
{
    let sched = JobScheduler::new().await?;

    if !config.enabled {
        tracing::info!("Scheduler disabled by config");
        return Ok(sched);
    }

    {
        let repo = repo.clone();
        let schedule = config.subscription_lifecycle.clone();
        sched
            .add(Job::new_async(schedule.as_str(), move |_uuid, _lock| {
                let repo = repo.clone();
                Box::pin(async move {
                    tracing::info!("Running subscription lifecycle cron");
                    if let Err(error) = repo.run_subscription_lifecycle().await {
                        tracing::error!("Lifecycle cron error: {error}");
                    }
                })
            })?)
            .await?;
    }

    {
        let repo = repo.clone();
        let schedule = config.dunning.clone();
        sched
            .add(Job::new_async(schedule.as_str(), move |_uuid, _lock| {
                let repo = repo.clone();
                Box::pin(async move {
                    tracing::info!("Running dunning cron");
                    if let Err(error) = repo.run_dunning().await {
                        tracing::error!("Dunning cron error: {error}");
                    }
                })
            })?)
            .await?;
    }

    sched.start().await?;
    tracing::info!("Scheduler started");
    Ok(sched)
}
