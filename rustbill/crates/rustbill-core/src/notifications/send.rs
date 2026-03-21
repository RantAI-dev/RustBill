//! Convenience functions for sending billing notification emails.
//! Each function accepts an `Option<&EmailSender>` and gracefully
//! logs a warning if email is not configured.

use super::email::EmailSender;
use super::repository::PgNotificationsRepository;
use super::service;
use sqlx::PgPool;

/// Send an invoice-created notification.
pub async fn notify_invoice_created(
    email_sender: Option<&EmailSender>,
    pool: &PgPool,
    customer_id: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
) {
    let repo = PgNotificationsRepository::new(pool);
    service::notify_invoice_created(
        &repo,
        email_sender,
        customer_id,
        invoice_number,
        total,
        currency,
    )
    .await;
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
    let repo = PgNotificationsRepository::new(pool);
    service::notify_invoice_issued(
        &repo,
        email_sender,
        customer_id,
        invoice_number,
        total,
        currency,
        due_date,
    )
    .await;
}

/// Send a payment-received notification.
pub async fn notify_payment_received(
    email_sender: Option<&EmailSender>,
    pool: &PgPool,
    customer_id: &str,
    amount: &str,
    method: &str,
) {
    let repo = PgNotificationsRepository::new(pool);
    service::notify_payment_received(&repo, email_sender, customer_id, amount, method).await;
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
    let repo = PgNotificationsRepository::new(pool);
    service::notify_invoice_paid(
        &repo,
        email_sender,
        customer_id,
        invoice_number,
        total,
        currency,
    )
    .await;
}

/// Send a subscription-renewed notification.
#[allow(clippy::too_many_arguments)]
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
    let repo = PgNotificationsRepository::new(pool);
    service::notify_subscription_renewed(
        &repo,
        email_sender,
        customer_id,
        plan_name,
        invoice_number,
        total,
        currency,
        next_period_end,
    )
    .await;
}
