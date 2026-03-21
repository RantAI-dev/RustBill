use async_trait::async_trait;
use rustbill_core::billing::payments::CreatePaymentRequest;
use rustbill_core::billing::refunds::CreateRefundRequest;
use rustbill_core::db::models::{Invoice, Payment};
use rustbill_core::error::BillingError;
use rustbill_core::notifications::email::EmailSender;
use sqlx::PgPool;

#[async_trait]
pub trait StripeWebhookRepository: Send + Sync {
    async fn record_event(
        &self,
        event_type: &str,
        resource_id: Option<&str>,
        data: &serde_json::Value,
    ) -> Result<(), BillingError>;
    async fn find_invoice_by_stripe_invoice_id(
        &self,
        stripe_invoice_id: &str,
    ) -> Result<Option<Invoice>, BillingError>;
    async fn mark_invoice_paid(
        &self,
        invoice_id: &str,
        paid_at: chrono::NaiveDateTime,
    ) -> Result<(), BillingError>;
    async fn mark_invoice_overdue_by_stripe_invoice_id(
        &self,
        stripe_invoice_id: &str,
    ) -> Result<(), BillingError>;
    async fn mark_subscription_canceled_by_stripe_subscription_id(
        &self,
        stripe_subscription_id: &str,
        canceled_at: chrono::NaiveDateTime,
    ) -> Result<(), BillingError>;
    async fn find_payment_by_stripe_payment_intent_id(
        &self,
        stripe_payment_intent_id: &str,
    ) -> Result<Option<Payment>, BillingError>;
    async fn create_payment_with_notification(
        &self,
        req: CreatePaymentRequest,
        email_sender: Option<&EmailSender>,
    ) -> Result<(), BillingError>;
    async fn create_refund(&self, req: CreateRefundRequest) -> Result<(), BillingError>;
}

#[derive(Clone)]
pub struct SqlxStripeWebhookRepository {
    pool: PgPool,
}

impl SqlxStripeWebhookRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StripeWebhookRepository for SqlxStripeWebhookRepository {
    async fn record_event(
        &self,
        event_type: &str,
        resource_id: Option<&str>,
        data: &serde_json::Value,
    ) -> Result<(), BillingError> {
        sqlx::query(
            r#"INSERT INTO billing_events (id, event_type, resource_type, resource_id, data, created_at)
               VALUES (gen_random_uuid()::text, $1::billing_event_type, 'stripe', COALESCE($2, ''), $3, now())"#,
        )
        .bind(event_type)
        .bind(resource_id)
        .bind(data)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(())
    }

    async fn find_invoice_by_stripe_invoice_id(
        &self,
        stripe_invoice_id: &str,
    ) -> Result<Option<Invoice>, BillingError> {
        sqlx::query_as("SELECT * FROM invoices WHERE stripe_invoice_id = $1 AND deleted_at IS NULL")
            .bind(stripe_invoice_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(BillingError::from)
    }

    async fn mark_invoice_paid(
        &self,
        invoice_id: &str,
        paid_at: chrono::NaiveDateTime,
    ) -> Result<(), BillingError> {
        sqlx::query(
            "UPDATE invoices SET status = 'paid', paid_at = $2, version = version + 1, updated_at = NOW() WHERE id = $1",
        )
        .bind(invoice_id)
        .bind(paid_at)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(())
    }

    async fn mark_invoice_overdue_by_stripe_invoice_id(
        &self,
        stripe_invoice_id: &str,
    ) -> Result<(), BillingError> {
        sqlx::query(
            "UPDATE invoices SET status = 'overdue', version = version + 1, updated_at = NOW() WHERE stripe_invoice_id = $1 AND deleted_at IS NULL AND status != 'paid'",
        )
        .bind(stripe_invoice_id)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(())
    }

    async fn mark_subscription_canceled_by_stripe_subscription_id(
        &self,
        stripe_subscription_id: &str,
        canceled_at: chrono::NaiveDateTime,
    ) -> Result<(), BillingError> {
        sqlx::query(
            "UPDATE subscriptions SET status = 'canceled', canceled_at = $2, version = version + 1, updated_at = NOW() WHERE stripe_subscription_id = $1 AND deleted_at IS NULL",
        )
        .bind(stripe_subscription_id)
        .bind(canceled_at)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(())
    }

    async fn find_payment_by_stripe_payment_intent_id(
        &self,
        stripe_payment_intent_id: &str,
    ) -> Result<Option<Payment>, BillingError> {
        sqlx::query_as("SELECT * FROM payments WHERE stripe_payment_intent_id = $1")
            .bind(stripe_payment_intent_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(BillingError::from)
    }

    async fn create_payment_with_notification(
        &self,
        req: CreatePaymentRequest,
        email_sender: Option<&EmailSender>,
    ) -> Result<(), BillingError> {
        let _ = rustbill_core::billing::payments::create_payment_with_notification(
            &self.pool,
            req,
            email_sender,
        )
        .await?;

        Ok(())
    }

    async fn create_refund(&self, req: CreateRefundRequest) -> Result<(), BillingError> {
        let _ = rustbill_core::billing::refunds::create_refund(&self.pool, req).await?;
        Ok(())
    }
}
