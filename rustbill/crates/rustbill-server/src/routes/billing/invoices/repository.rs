use super::schema::{CreateInvoiceRequest, InvoiceItemInput, UpdateInvoiceRequest};
use async_trait::async_trait;
use rust_decimal::Decimal;
use rustbill_core::analytics::sales_ledger::{
    emit_sales_event, NewSalesEvent, SalesClassification,
};
use rustbill_core::db::models::{Invoice, InvoiceStatus};
use rustbill_core::error::BillingError;
use sqlx::PgPool;
use std::str::FromStr;

#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    async fn list_admin(&self) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn get_admin(&self, id: &str) -> Result<serde_json::Value, BillingError>;
    async fn create_admin(
        &self,
        body: &CreateInvoiceRequest,
        subtotal: f64,
        total: f64,
    ) -> Result<serde_json::Value, BillingError>;
    async fn update_admin(
        &self,
        id: &str,
        body: &UpdateInvoiceRequest,
    ) -> Result<serde_json::Value, BillingError>;
    async fn find_admin_invoice(&self, id: &str) -> Result<Invoice, BillingError>;
    async fn delete_admin(&self, id: &str) -> Result<u64, BillingError>;
    async fn add_item(
        &self,
        invoice_id: &str,
        body: &InvoiceItemInput,
    ) -> Result<serde_json::Value, BillingError>;
    async fn emit_created_event(
        &self,
        invoice_id: &str,
        body: &CreateInvoiceRequest,
        subtotal: f64,
        total: f64,
    ) -> Result<(), BillingError>;
    async fn emit_void_reversal(
        &self,
        invoice: &Invoice,
        trigger: &str,
    ) -> Result<(), BillingError>;

    async fn list_v1(
        &self,
        status: Option<&str>,
        customer_id: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn get_v1(&self, id: &str) -> Result<serde_json::Value, BillingError>;
    async fn list_items(&self, invoice_id: &str) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn generate_pdf(&self, invoice_id: &str) -> Result<Vec<u8>, BillingError>;
    async fn get_invoice_number(&self, invoice_id: &str) -> Result<Option<String>, BillingError>;
}

#[derive(Clone)]
pub struct SqlxInvoiceRepository {
    pool: PgPool,
}

impl SqlxInvoiceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn generate_invoice_number(&self) -> Result<String, BillingError> {
        let from_sequence = sqlx::query_scalar::<_, String>(
            "SELECT 'INV-' || LPAD(nextval('invoice_number_seq')::text, 8, '0')",
        )
        .fetch_one(&self.pool)
        .await;

        match from_sequence {
            Ok(value) => Ok(value),
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("42P01") => {
                let next: i64 = sqlx::query_scalar(
                    r#"
                    SELECT COALESCE(MAX(NULLIF(regexp_replace(invoice_number, '[^0-9]', '', 'g'), '')::bigint), 0) + 1
                    FROM invoices
                    "#,
                )
                .fetch_one(&self.pool)
                .await
                .map_err(BillingError::from)?;

                Ok(format!("INV-{next:08}"))
            }
            Err(err) => Err(BillingError::from(err)),
        }
    }
}

#[async_trait]
impl InvoiceRepository for SqlxInvoiceRepository {
    async fn list_admin(&self) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT to_jsonb(i) FROM invoices i ORDER BY i.created_at DESC"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn get_admin(&self, id: &str) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(i) FROM invoices i WHERE i.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("invoice", id))
    }

    async fn create_admin(
        &self,
        body: &CreateInvoiceRequest,
        subtotal: f64,
        total: f64,
    ) -> Result<serde_json::Value, BillingError> {
        let invoice_number = self.generate_invoice_number().await?;
        let tax = body.normalized_tax();

        let mut tx = self.pool.begin().await.map_err(BillingError::from)?;
        let row = sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO invoices (id, invoice_number, customer_id, subscription_id, status, currency, subtotal, tax, total, due_at, issued_at, notes, created_at, updated_at)
               VALUES (gen_random_uuid()::text, $1, $2, $3, $4::invoice_status, $5, $6, $7, $8, $9::timestamp, $10::timestamp, $11, now(), now())
               RETURNING to_jsonb(invoices.*)"#,
        )
        .bind(invoice_number)
        .bind(&body.customer_id)
        .bind(body.subscription_id.as_deref())
        .bind(body.normalized_status())
        .bind(body.normalized_currency())
        .bind(subtotal)
        .bind(tax)
        .bind(total)
        .bind(body.due_at.as_deref())
        .bind(body.issued_at.as_deref())
        .bind(body.notes.as_deref())
        .fetch_one(&mut *tx)
        .await
        .map_err(BillingError::from)?;

        let invoice_id = row
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                BillingError::Internal(anyhow::anyhow!("invoice insert returned no id"))
            })?;

        for item in body.normalized_items() {
            let Some(description) = item.normalized_description() else {
                continue;
            };

            sqlx::query(
                r#"INSERT INTO invoice_items (id, invoice_id, description, quantity, unit_price, amount, period_start, period_end)
                   VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, NULL, NULL)"#,
            )
            .bind(invoice_id)
            .bind(description)
            .bind(item.normalized_quantity())
            .bind(item.normalized_unit_price())
            .bind(item.normalized_amount())
            .execute(&mut *tx)
            .await
            .map_err(BillingError::from)?;
        }

        tx.commit().await.map_err(BillingError::from)?;
        Ok(row)
    }

    async fn update_admin(
        &self,
        id: &str,
        body: &UpdateInvoiceRequest,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"UPDATE invoices SET
                 status = COALESCE($2::invoice_status, status),
                 notes = COALESCE($3, notes),
                 due_at = COALESCE($4::timestamp, due_at),
                 version = version + 1,
                 updated_at = now()
               WHERE id = $1 AND deleted_at IS NULL
               RETURNING to_jsonb(invoices.*)"#,
        )
        .bind(id)
        .bind(body.status.as_deref())
        .bind(body.notes.as_deref())
        .bind(body.due_at.as_deref())
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("invoice", id))
    }

    async fn find_admin_invoice(&self, id: &str) -> Result<Invoice, BillingError> {
        sqlx::query_as::<_, Invoice>("SELECT * FROM invoices WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(BillingError::from)?
            .ok_or_else(|| BillingError::not_found("invoice", id))
    }

    async fn delete_admin(&self, id: &str) -> Result<u64, BillingError> {
        let result = sqlx::query(
            r#"UPDATE invoices
               SET status = 'void', deleted_at = now(), version = version + 1, updated_at = now()
               WHERE id = $1 AND deleted_at IS NULL"#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(result.rows_affected())
    }

    async fn add_item(
        &self,
        invoice_id: &str,
        body: &InvoiceItemInput,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO invoice_items (id, invoice_id, description, quantity, unit_price, amount, period_start, period_end)
               VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6::timestamp, $7::timestamp)
               RETURNING to_jsonb(invoice_items.*)"#,
        )
        .bind(invoice_id)
        .bind(body.description.as_deref())
        .bind(body.quantity.unwrap_or(1.0))
        .bind(body.unit_price.unwrap_or(0.0))
        .bind(body.amount.unwrap_or(0.0))
        .bind(body.period_start.as_deref())
        .bind(body.period_end.as_deref())
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn emit_created_event(
        &self,
        invoice_id: &str,
        body: &CreateInvoiceRequest,
        subtotal: f64,
        total: f64,
    ) -> Result<(), BillingError> {
        let subtotal_dec = decimal_from_f64(subtotal);
        let tax_dec = decimal_from_f64(body.normalized_tax());
        let total_dec = decimal_from_f64(total);

        if let Err(err) = emit_sales_event(
            &self.pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "invoice.created",
                classification: SalesClassification::Billings,
                amount_subtotal: subtotal_dec,
                amount_tax: tax_dec,
                amount_total: total_dec,
                currency: body.normalized_currency(),
                customer_id: Some(&body.customer_id),
                subscription_id: body.subscription_id.as_deref(),
                product_id: None,
                invoice_id: Some(invoice_id),
                payment_id: None,
                source_table: "invoices",
                source_id: invoice_id,
                metadata: Some(serde_json::json!({
                    "status": body.normalized_status(),
                    "origin": "manual",
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, invoice_id = %invoice_id, "failed to emit sales event invoice.created");
        }

        Ok(())
    }

    async fn emit_void_reversal(
        &self,
        invoice: &Invoice,
        trigger: &str,
    ) -> Result<(), BillingError> {
        let reversal_target: Option<(String, String, Decimal, Decimal, Decimal)> = sqlx::query_as(
            r#"
            SELECT id, event_type, amount_subtotal, amount_tax, amount_total
            FROM sales_events
            WHERE source_table = 'invoices'
              AND source_id = $1
              AND classification = 'billings'
              AND amount_total > 0
              AND event_type IN ('invoice.created', 'invoice.created_from_deal', 'invoice.issued')
            ORDER BY occurred_at DESC, created_at DESC
            LIMIT 1
            "#,
        )
        .bind(&invoice.id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        if let Some((event_id, event_type, subtotal, tax, total)) = reversal_target {
            if let Err(err) = emit_sales_event(
                &self.pool,
                NewSalesEvent {
                    occurred_at: chrono::Utc::now(),
                    event_type: "invoice.reversal",
                    classification: SalesClassification::Billings,
                    amount_subtotal: -subtotal,
                    amount_tax: -tax,
                    amount_total: -total,
                    currency: &invoice.currency,
                    customer_id: Some(&invoice.customer_id),
                    subscription_id: invoice.subscription_id.as_deref(),
                    product_id: None,
                    invoice_id: Some(&invoice.id),
                    payment_id: None,
                    source_table: "invoices",
                    source_id: &invoice.id,
                    metadata: Some(serde_json::json!({
                        "trigger": trigger,
                        "reason": "invoice_voided",
                        "reversal_of_event_id": event_id,
                        "reversal_of_event_type": event_type,
                    })),
                },
            )
            .await
            {
                tracing::warn!(error = %err, invoice_id = %invoice.id, "failed to emit invoice.reversal");
            }
        }

        if let Err(err) = emit_sales_event(
            &self.pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "invoice.voided",
                classification: SalesClassification::Billings,
                amount_subtotal: Decimal::ZERO,
                amount_tax: Decimal::ZERO,
                amount_total: Decimal::ZERO,
                currency: &invoice.currency,
                customer_id: Some(&invoice.customer_id),
                subscription_id: invoice.subscription_id.as_deref(),
                product_id: None,
                invoice_id: Some(&invoice.id),
                payment_id: None,
                source_table: "invoices",
                source_id: &invoice.id,
                metadata: Some(serde_json::json!({
                    "trigger": trigger,
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, invoice_id = %invoice.id, "failed to emit invoice.voided");
        }

        Ok(())
    }

    async fn list_v1(
        &self,
        status: Option<&str>,
        customer_id: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT to_jsonb(i) FROM invoices i
               WHERE ($1::text IS NULL OR i.status::text = $1)
                 AND ($2::text IS NULL OR i.customer_id = $2)
               ORDER BY i.created_at DESC"#,
        )
        .bind(status)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn get_v1(&self, id: &str) -> Result<serde_json::Value, BillingError> {
        let invoice = self.get_admin(id).await?;

        let items = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(li) FROM invoice_items li WHERE li.invoice_id = $1 ORDER BY li.id",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)?;

        let payments = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(p) FROM payments p WHERE p.invoice_id = $1 ORDER BY p.created_at",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)?;

        let mut result = invoice;
        if let Some(obj) = result.as_object_mut() {
            obj.insert("items".to_string(), serde_json::json!(items));
            obj.insert("payments".to_string(), serde_json::json!(payments));
        }

        Ok(result)
    }

    async fn list_items(&self, invoice_id: &str) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(li) FROM invoice_items li WHERE li.invoice_id = $1 ORDER BY li.id",
        )
        .bind(invoice_id)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn generate_pdf(&self, invoice_id: &str) -> Result<Vec<u8>, BillingError> {
        rustbill_core::billing::invoice_pdf::generate_invoice_pdf(&self.pool, invoice_id).await
    }

    async fn get_invoice_number(&self, invoice_id: &str) -> Result<Option<String>, BillingError> {
        sqlx::query_scalar("SELECT invoice_number FROM invoices WHERE id = $1")
            .bind(invoice_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(BillingError::from)
    }
}

pub fn status_was_voided(before: &Invoice, after: &Invoice) -> bool {
    before.status != after.status && matches!(after.status, InvoiceStatus::Void)
}

fn decimal_from_f64(value: f64) -> Decimal {
    Decimal::from_str(&value.to_string()).unwrap_or(Decimal::ZERO)
}
