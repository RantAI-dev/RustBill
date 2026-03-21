use super::schema::DunningLogFilter;
use crate::db::models::{DunningLogEntry, DunningStep, Invoice, InvoiceStatus};
use crate::error::Result;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use sqlx::PgPool;

#[async_trait]
pub trait DunningRepository: Send + Sync {
    async fn list_dunning_log(&self, filter: &DunningLogFilter) -> Result<Vec<DunningLogEntry>>;
    async fn list_overdue_invoices(&self, now: NaiveDateTime) -> Result<Vec<Invoice>>;
    async fn has_executed_step(&self, invoice_id: &str, step: &DunningStep) -> Result<bool>;
    async fn execute_dunning_step(
        &self,
        invoice: &Invoice,
        step: DunningStep,
        now: NaiveDateTime,
        days_overdue: i64,
    ) -> Result<()>;
}

#[derive(Clone)]
pub struct PgDunningRepository {
    pool: PgPool,
}

impl PgDunningRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl DunningRepository for PgDunningRepository {
    async fn list_dunning_log(&self, filter: &DunningLogFilter) -> Result<Vec<DunningLogEntry>> {
        let rows = sqlx::query_as::<_, DunningLogEntry>(
            r#"
            SELECT * FROM dunning_log
            WHERE ($1::text IS NULL OR invoice_id = $1)
            ORDER BY created_at DESC
            "#,
        )
        .bind(&filter.invoice_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn list_overdue_invoices(&self, now: NaiveDateTime) -> Result<Vec<Invoice>> {
        let overdue_invoices = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT * FROM invoices
            WHERE deleted_at IS NULL
              AND status IN ('issued', 'overdue')
              AND due_at IS NOT NULL
              AND due_at < $1
            "#,
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        Ok(overdue_invoices)
    }

    async fn has_executed_step(&self, invoice_id: &str, step: &DunningStep) -> Result<bool> {
        let already_executed: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT id FROM dunning_log
            WHERE invoice_id = $1
              AND step = $2
              AND executed_at IS NOT NULL
            "#,
        )
        .bind(invoice_id)
        .bind(step)
        .fetch_optional(&self.pool)
        .await?;

        Ok(already_executed.is_some())
    }

    async fn execute_dunning_step(
        &self,
        invoice: &Invoice,
        step: DunningStep,
        now: NaiveDateTime,
        days_overdue: i64,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        if invoice.status == InvoiceStatus::Issued {
            sqlx::query(
                "UPDATE invoices SET status = 'overdue', version = version + 1, updated_at = NOW() WHERE id = $1",
            )
            .bind(&invoice.id)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            r#"
            INSERT INTO dunning_log
                (id, invoice_id, subscription_id, step, scheduled_at, executed_at, notes)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $4, $5)
            "#,
        )
        .bind(&invoice.id)
        .bind(&invoice.subscription_id)
        .bind(&step)
        .bind(now)
        .bind(format!("Auto-dunning: {} days overdue", days_overdue))
        .execute(&mut *tx)
        .await?;

        if step == DunningStep::Suspension {
            if let Some(ref sub_id) = invoice.subscription_id {
                sqlx::query(
                    r#"
                    UPDATE subscriptions
                    SET status = 'paused', version = version + 1, updated_at = NOW()
                    WHERE id = $1 AND deleted_at IS NULL AND status = 'active'
                    "#,
                )
                .bind(sub_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }
}
