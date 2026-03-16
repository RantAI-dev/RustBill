use crate::db::models::*;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// ---- Configuration ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DunningConfig {
    /// Days overdue to send a reminder (e.g. 1).
    pub reminder_days: i64,
    /// Days overdue to send a warning (e.g. 7).
    pub warning_days: i64,
    /// Days overdue to send a final notice (e.g. 14).
    pub final_notice_days: i64,
    /// Days overdue to suspend (e.g. 30).
    pub suspension_days: i64,
}

impl Default for DunningConfig {
    fn default() -> Self {
        Self {
            reminder_days: 3,
            warning_days: 7,
            final_notice_days: 14,
            suspension_days: 30,
        }
    }
}

// ---- Service functions ----

pub async fn list_dunning_log(
    pool: &PgPool,
    invoice_id: Option<&str>,
) -> Result<Vec<DunningLogEntry>> {
    let rows = sqlx::query_as::<_, DunningLogEntry>(
        r#"
        SELECT * FROM dunning_log
        WHERE ($1::text IS NULL OR invoice_id = $1)
        ORDER BY created_at DESC
        "#,
    )
    .bind(invoice_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Run the dunning cascade on all overdue invoices.
///
/// For each overdue invoice, determine which dunning step applies based on
/// how many days past due it is. Only create a log entry if a higher step
/// hasn't already been executed for this invoice.
pub async fn run_dunning(pool: &PgPool, config: &DunningConfig) -> Result<u64> {
    let now = chrono::Utc::now().naive_utc();
    let mut processed: u64 = 0;

    // Find all overdue invoices (status = 'overdue' or status = 'issued' with due_at in the past)
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
    .fetch_all(pool)
    .await?;

    for invoice in &overdue_invoices {
        let due_at = match invoice.due_at {
            Some(d) => d,
            None => continue,
        };

        let days_overdue = (now - due_at).num_days();

        // Determine which step to apply
        let step = if days_overdue >= config.suspension_days {
            DunningStep::Suspension
        } else if days_overdue >= config.final_notice_days {
            DunningStep::FinalNotice
        } else if days_overdue >= config.warning_days {
            DunningStep::Warning
        } else if days_overdue >= config.reminder_days {
            DunningStep::Reminder
        } else {
            continue;
        };

        // Check if this step (or a higher one) was already executed for this invoice
        let already_executed: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT id FROM dunning_log
            WHERE invoice_id = $1
              AND step = $2
              AND executed_at IS NOT NULL
            "#,
        )
        .bind(&invoice.id)
        .bind(&step)
        .fetch_optional(pool)
        .await?;

        if already_executed.is_some() {
            continue;
        }

        let mut tx = pool.begin().await?;

        // Mark invoice as overdue if it was still 'issued'
        if invoice.status == InvoiceStatus::Issued {
            sqlx::query(
                "UPDATE invoices SET status = 'overdue', version = version + 1, updated_at = NOW() WHERE id = $1",
            )
            .bind(&invoice.id)
            .execute(&mut *tx)
            .await?;
        }

        // Insert dunning log entry
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

        // If suspension step, also suspend the related subscription
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
