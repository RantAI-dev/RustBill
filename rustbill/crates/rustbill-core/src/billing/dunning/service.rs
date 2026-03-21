use super::repository::DunningRepository;
use super::schema::{DunningConfig, DunningLogFilter};
use crate::db::models::{DunningLogEntry, DunningStep};
use crate::error::Result;

pub async fn list_dunning_log<R: DunningRepository + ?Sized>(
    repo: &R,
    filter: DunningLogFilter,
) -> Result<Vec<DunningLogEntry>> {
    repo.list_dunning_log(&filter).await
}

pub async fn run_dunning<R: DunningRepository + ?Sized>(
    repo: &R,
    config: &DunningConfig,
) -> Result<u64> {
    let now = chrono::Utc::now().naive_utc();
    let overdue_invoices = repo.list_overdue_invoices(now).await?;
    let mut processed: u64 = 0;

    for invoice in overdue_invoices {
        let Some(due_at) = invoice.due_at else {
            continue;
        };

        let days_overdue = (now - due_at).num_days();
        let Some(step) = determine_step(days_overdue, config) else {
            continue;
        };

        if repo.has_executed_step(&invoice.id, &step).await? {
            continue;
        }

        repo.execute_dunning_step(&invoice, step.clone(), now, days_overdue)
            .await?;
        processed += 1;

        tracing::info!(
            invoice_id = %invoice.id,
            step = ?step,
            days_overdue,
            "dunning step executed"
        );
    }

    tracing::info!(processed, "dunning run completed");
    Ok(processed)
}

fn determine_step(days_overdue: i64, config: &DunningConfig) -> Option<DunningStep> {
    if days_overdue >= config.suspension_days {
        Some(DunningStep::Suspension)
    } else if days_overdue >= config.final_notice_days {
        Some(DunningStep::FinalNotice)
    } else if days_overdue >= config.warning_days {
        Some(DunningStep::Warning)
    } else if days_overdue >= config.reminder_days {
        Some(DunningStep::Reminder)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{DunningLogEntry, Invoice, InvoiceStatus};
    use async_trait::async_trait;
    use chrono::{NaiveDate, NaiveDateTime, Utc};
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        logs: Vec<DunningLogEntry>,
        invoices: Vec<Invoice>,
        executed: Vec<(String, DunningStep)>,
        applied: Vec<(String, DunningStep, i64)>,
    }

    #[derive(Clone, Default)]
    struct StubRepo {
        state: Arc<Mutex<StubState>>,
    }

    impl StubRepo {
        fn with_state(state: StubState) -> Self {
            Self {
                state: Arc::new(Mutex::new(state)),
            }
        }

        fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, StubState>> {
            self.state.lock().map_err(|_| {
                crate::error::BillingError::Internal(anyhow::anyhow!("mutex poisoned"))
            })
        }
    }

    fn dt(y: i32, m: u32, d: u32) -> NaiveDateTime {
        let date = match NaiveDate::from_ymd_opt(y, m, d) {
            Some(date) => date,
            None => panic!("invalid test date"),
        };
        match date.and_hms_opt(0, 0, 0) {
            Some(dt) => dt,
            None => panic!("invalid test time"),
        }
    }

    #[async_trait]
    impl DunningRepository for StubRepo {
        async fn list_dunning_log(
            &self,
            filter: &DunningLogFilter,
        ) -> Result<Vec<DunningLogEntry>> {
            let state = self.lock_state()?;
            if let Some(invoice_id) = &filter.invoice_id {
                Ok(state
                    .logs
                    .iter()
                    .filter(|entry| &entry.invoice_id == invoice_id)
                    .cloned()
                    .collect())
            } else {
                Ok(state.logs.clone())
            }
        }

        async fn list_overdue_invoices(&self, _now: NaiveDateTime) -> Result<Vec<Invoice>> {
            Ok(self.lock_state()?.invoices.clone())
        }

        async fn has_executed_step(&self, invoice_id: &str, step: &DunningStep) -> Result<bool> {
            Ok(self
                .lock_state()?
                .executed
                .iter()
                .any(|(id, s)| id == invoice_id && s == step))
        }

        async fn execute_dunning_step(
            &self,
            invoice: &Invoice,
            step: DunningStep,
            _now: NaiveDateTime,
            days_overdue: i64,
        ) -> Result<()> {
            let mut state = self.lock_state()?;
            state
                .applied
                .push((invoice.id.clone(), step.clone(), days_overdue));
            state.executed.push((invoice.id.clone(), step));
            Ok(())
        }
    }

    fn sample_invoice(status: InvoiceStatus, days_past_due: i64) -> Invoice {
        let due_date = Utc::now().naive_utc();
        Invoice {
            id: "inv_1".to_string(),
            invoice_number: "INV-00000001".to_string(),
            customer_id: "cus_1".to_string(),
            subscription_id: Some("sub_1".to_string()),
            status,
            issued_at: None,
            due_at: Some(due_date - chrono::Duration::days(days_past_due)),
            paid_at: None,
            subtotal: rust_decimal::Decimal::ZERO,
            tax: rust_decimal::Decimal::ZERO,
            total: rust_decimal::Decimal::ZERO,
            currency: "USD".to_string(),
            notes: None,
            stripe_invoice_id: None,
            xendit_invoice_id: None,
            lemonsqueezy_order_id: None,
            version: 1,
            deleted_at: None,
            created_at: due_date,
            updated_at: due_date,
            tax_name: None,
            tax_rate: None,
            tax_inclusive: false,
            credits_applied: rust_decimal::Decimal::ZERO,
            amount_due: rust_decimal::Decimal::ZERO,
            auto_charge_attempts: 0,
            idempotency_key: None,
        }
    }

    #[tokio::test]
    async fn list_dunning_log_filters_by_invoice() {
        let log = DunningLogEntry {
            id: "log_1".to_string(),
            invoice_id: "inv_1".to_string(),
            subscription_id: Some("sub_1".to_string()),
            step: DunningStep::Reminder,
            scheduled_at: dt(2026, 1, 1),
            executed_at: Some(dt(2026, 1, 1)),
            notes: Some("note".to_string()),
            created_at: dt(2026, 1, 1),
        };
        let repo = StubRepo::with_state(StubState {
            logs: vec![log],
            ..StubState::default()
        });

        let rows = list_dunning_log(
            &repo,
            DunningLogFilter {
                invoice_id: Some("inv_1".to_string()),
            },
        )
        .await;
        let rows = match rows {
            Ok(rows) => rows,
            Err(err) => panic!("list_dunning_log failed: {err}"),
        };

        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn run_dunning_executes_matching_step() {
        let repo = StubRepo::with_state(StubState {
            invoices: vec![sample_invoice(InvoiceStatus::Issued, 8)],
            ..StubState::default()
        });

        let processed = run_dunning(&repo, &DunningConfig::default())
            .await
            .unwrap_or_else(|err| panic!("run_dunning failed: {err}"));

        let state = repo
            .lock_state()
            .unwrap_or_else(|err| panic!("mutex lock failed: {err}"));
        assert_eq!(processed, 1);
        assert_eq!(state.applied.len(), 1);
        assert_eq!(state.applied[0].0, "inv_1");
        assert_eq!(state.applied[0].1, DunningStep::Warning);
    }

    #[tokio::test]
    async fn run_dunning_skips_when_step_already_executed() {
        let repo = StubRepo::with_state(StubState {
            invoices: vec![sample_invoice(InvoiceStatus::Issued, 8)],
            executed: vec![("inv_1".to_string(), DunningStep::Warning)],
            ..StubState::default()
        });

        let processed = run_dunning(&repo, &DunningConfig::default())
            .await
            .unwrap_or_else(|err| panic!("run_dunning failed: {err}"));

        assert_eq!(processed, 0);
        assert!(repo
            .lock_state()
            .unwrap_or_else(|err| panic!("mutex lock failed: {err}"))
            .applied
            .is_empty());
    }
}
