//! Convenience functions for sending billing notification emails.
//! Each function accepts an `Option<&EmailSender>` and gracefully
//! logs a warning if email is not configured.

use super::email::EmailSender;
use super::templates;
use sqlx::PgPool;

/// Look up a customer's email address by ID.
async fn customer_email(pool: &PgPool, customer_id: &str) -> Option<(String, String)> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT COALESCE(billing_email, email), name FROM customers WHERE id = $1")
            .bind(customer_id)
            .fetch_optional(pool)
            .await
            .ok()?;

    row
}

/// Send an invoice-created notification.
pub async fn notify_invoice_created(
    email_sender: Option<&EmailSender>,
    pool: &PgPool,
    customer_id: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
) {
    let Some(sender) = email_sender else {
        tracing::warn!("Email not configured, skipping invoice created notification");
        return;
    };

    let Some((email, name)) = customer_email(pool, customer_id).await else {
        tracing::warn!(
            customer_id,
            "Could not find customer email for invoice notification"
        );
        return;
    };

    let (subject, html) = templates::invoice_created(&name, invoice_number, total, currency);
    if !sender.send(&email, &subject, &html).await {
        tracing::warn!(
            customer_id,
            invoice_number,
            "Failed to send invoice created email"
        );
    }
}

/// Send an invoice-issued notification.
pub async fn notify_invoice_issued(
    email_sender: Option<&EmailSender>,
    pool: &PgPool,
    customer_id: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
    due_date: &str,
) {
    let Some(sender) = email_sender else {
        tracing::warn!("Email not configured, skipping invoice issued notification");
        return;
    };

    let Some((email, name)) = customer_email(pool, customer_id).await else {
        tracing::warn!(
            customer_id,
            "Could not find customer email for invoice issued notification"
        );
        return;
    };

    let (subject, html) =
        templates::invoice_issued(&name, invoice_number, total, currency, due_date);
    if !sender.send(&email, &subject, &html).await {
        tracing::warn!(
            customer_id,
            invoice_number,
            "Failed to send invoice issued email"
        );
    }
}

/// Send a payment-received notification.
pub async fn notify_payment_received(
    email_sender: Option<&EmailSender>,
    pool: &PgPool,
    customer_id: &str,
    amount: &str,
    method: &str,
) {
    let Some(sender) = email_sender else {
        tracing::warn!("Email not configured, skipping payment received notification");
        return;
    };

    let Some((email, name)) = customer_email(pool, customer_id).await else {
        tracing::warn!(
            customer_id,
            "Could not find customer email for payment notification"
        );
        return;
    };

    let (subject, html) = templates::payment_received(&name, amount, method);
    if !sender.send(&email, &subject, &html).await {
        tracing::warn!(customer_id, "Failed to send payment received email");
    }
}

/// Send an invoice-paid notification.
pub async fn notify_invoice_paid(
    email_sender: Option<&EmailSender>,
    pool: &PgPool,
    customer_id: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
) {
    let Some(sender) = email_sender else {
        tracing::warn!("Email not configured, skipping invoice paid notification");
        return;
    };

    let Some((email, name)) = customer_email(pool, customer_id).await else {
        tracing::warn!(
            customer_id,
            "Could not find customer email for invoice paid notification"
        );
        return;
    };

    let (subject, html) = templates::invoice_paid(&name, invoice_number, total, currency);
    if !sender.send(&email, &subject, &html).await {
        tracing::warn!(
            customer_id,
            invoice_number,
            "Failed to send invoice paid email"
        );
    }
}

/// Send a subscription-renewed notification.
pub async fn notify_subscription_renewed(
    email_sender: Option<&EmailSender>,
    pool: &PgPool,
    customer_id: &str,
    plan_name: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
    next_period_end: &str,
) {
    let Some(sender) = email_sender else {
        tracing::warn!("Email not configured, skipping subscription renewed notification");
        return;
    };

    let Some((email, name)) = customer_email(pool, customer_id).await else {
        tracing::warn!(
            customer_id,
            "Could not find customer email for renewal notification"
        );
        return;
    };

    let (subject, html) = templates::subscription_renewed(
        &name,
        plan_name,
        invoice_number,
        total,
        currency,
        next_period_end,
    );
    if !sender.send(&email, &subject, &html).await {
        tracing::warn!(customer_id, "Failed to send subscription renewed email");
    }
}
