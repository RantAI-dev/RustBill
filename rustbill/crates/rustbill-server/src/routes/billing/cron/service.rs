use super::repository::CronRepository;
use super::schema::{
    DunningResponse, ExpireLicensesResponse, GenerateInvoicesResponse, LifecycleResponse,
    RunAllDunningResponse, RunAllLicensesResponse, RunAllResponse,
};
use rustbill_core::billing::dunning::DunningConfig;
use rustbill_core::error::BillingError;

pub async fn run_all<R: CronRepository>(repo: &R) -> Result<RunAllResponse, BillingError> {
    let lifecycle = repo.run_full_lifecycle().await?;
    let processed = repo.run_dunning(&DunningConfig::default()).await?;
    let expired = repo.expire_licenses().await?;

    Ok(RunAllResponse {
        success: true,
        jobs: vec![
            "lifecycle".to_string(),
            "dunning".to_string(),
            "expire_licenses".to_string(),
        ],
        lifecycle,
        dunning: RunAllDunningResponse { processed },
        licenses: RunAllLicensesResponse { expired },
    })
}

pub async fn renew_subscriptions<R: CronRepository>(
    repo: &R,
) -> Result<LifecycleResponse, BillingError> {
    let lifecycle = repo.run_full_lifecycle().await?;
    Ok(LifecycleResponse {
        success: true,
        lifecycle,
    })
}

pub async fn generate_invoices<R: CronRepository>(
    repo: &R,
) -> Result<GenerateInvoicesResponse, BillingError> {
    let generated = repo.generate_pending_invoices().await?;
    Ok(GenerateInvoicesResponse {
        success: true,
        generated,
    })
}

pub async fn process_dunning<R: CronRepository>(repo: &R) -> Result<DunningResponse, BillingError> {
    let config = DunningConfig::default();
    let processed = repo.run_dunning(&config).await?;
    Ok(DunningResponse {
        success: true,
        processed,
        config,
    })
}

pub async fn expire_licenses<R: CronRepository>(
    repo: &R,
) -> Result<ExpireLicensesResponse, BillingError> {
    let expired = repo.expire_licenses().await?;
    Ok(ExpireLicensesResponse {
        success: true,
        expired,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rustbill_core::billing::{dunning::DunningConfig, lifecycle::LifecycleResult};
    use rustbill_core::error::BillingError;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    struct MockCronRepository {
        lifecycle_called: AtomicBool,
        invoices_called: AtomicBool,
        dunning_called: AtomicBool,
        expire_called: AtomicBool,
        last_config: Mutex<Option<DunningConfig>>,
    }

    impl MockCronRepository {
        fn new() -> Self {
            Self {
                lifecycle_called: AtomicBool::new(false),
                invoices_called: AtomicBool::new(false),
                dunning_called: AtomicBool::new(false),
                expire_called: AtomicBool::new(false),
                last_config: Mutex::new(None),
            }
        }

        fn lifecycle() -> LifecycleResult {
            LifecycleResult {
                trials_converted: 1,
                canceled: 2,
                pre_generated: 3,
                renewed: 4,
                invoices_generated: 5,
                errors: vec!["warn".to_string()],
            }
        }
    }

    #[async_trait]
    impl CronRepository for MockCronRepository {
        async fn run_full_lifecycle(&self) -> Result<LifecycleResult, BillingError> {
            self.lifecycle_called.store(true, Ordering::SeqCst);
            Ok(Self::lifecycle())
        }

        async fn generate_pending_invoices(&self) -> Result<u64, BillingError> {
            self.invoices_called.store(true, Ordering::SeqCst);
            Ok(7)
        }

        async fn run_dunning(&self, config: &DunningConfig) -> Result<u64, BillingError> {
            self.dunning_called.store(true, Ordering::SeqCst);
            if let Ok(mut guard) = self.last_config.lock() {
                *guard = Some(config.clone());
            }
            Ok(11)
        }

        async fn expire_licenses(&self) -> Result<i64, BillingError> {
            self.expire_called.store(true, Ordering::SeqCst);
            Ok(13)
        }
    }

    #[tokio::test]
    async fn run_all_combines_cron_results() {
        let repo = MockCronRepository::new();

        let response = run_all(&repo).await.unwrap();
        assert!(response.success);
        assert_eq!(
            response.jobs,
            vec!["lifecycle", "dunning", "expire_licenses"]
        );
        assert_eq!(response.lifecycle.trials_converted, 1);
        assert_eq!(response.dunning.processed, 11);
        assert_eq!(response.licenses.expired, 13);
    }

    #[tokio::test]
    async fn process_dunning_uses_default_config() {
        let repo = MockCronRepository::new();

        let response = process_dunning(&repo).await.unwrap();
        assert!(response.success);
        assert_eq!(response.processed, 11);
        assert_eq!(response.config.reminder_days, 3);
        assert_eq!(response.config.warning_days, 7);
        assert_eq!(response.config.final_notice_days, 14);
        assert_eq!(response.config.suspension_days, 30);
        let config = repo.last_config.lock().unwrap().clone().unwrap();
        assert_eq!(config.reminder_days, 3);
    }

    #[tokio::test]
    async fn generate_invoices_forwards_to_repository() {
        let repo = MockCronRepository::new();

        let response = generate_invoices(&repo).await.unwrap();
        assert!(response.success);
        assert_eq!(response.generated, 7);
    }
}
