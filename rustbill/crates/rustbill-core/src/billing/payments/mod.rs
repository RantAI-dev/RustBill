pub mod repository;
pub mod schema;
pub mod service;

use crate::db::models::Payment;
use crate::error::Result;
use repository::PgPaymentsRepository;
use sqlx::PgPool;

pub use schema::{CreatePaymentRequest, ListPaymentsFilter, PaymentView};

pub async fn list_payments(pool: &PgPool, filter: &ListPaymentsFilter) -> Result<Vec<PaymentView>> {
    let repo = PgPaymentsRepository::new(pool);
    service::list_payments(&repo, filter).await
}

pub async fn create_payment(pool: &PgPool, req: CreatePaymentRequest) -> Result<Payment> {
    let repo = PgPaymentsRepository::new(pool);
    service::create_payment(&repo, req).await
}

pub async fn create_payment_with_notification(
    pool: &PgPool,
    req: CreatePaymentRequest,
    email_sender: Option<&crate::notifications::email::EmailSender>,
) -> Result<Payment> {
    let outcome = {
        let repo = PgPaymentsRepository::new(pool);
        service::create_payment_details(&repo, req).await?
    };

    let method_str = format!("{:?}", outcome.payment.method).to_lowercase();

    let pool_clone = pool.clone();
    let email_sender_cloned = email_sender.cloned();
    let customer_id = outcome.invoice.customer_id.clone();
    let amount_str = outcome.payment.amount.to_string();
    let method = method_str;
    tokio::spawn(async move {
        crate::notifications::send::notify_payment_received(
            email_sender_cloned.as_ref(),
            &pool_clone,
            &customer_id,
            &amount_str,
            &method,
        )
        .await;
    });

    if outcome.invoice_became_paid {
        let pool_clone2 = pool.clone();
        let email_sender_cloned2 = email_sender.cloned();
        let customer_id2 = outcome.invoice.customer_id.clone();
        let inv_number = outcome.invoice.invoice_number.clone();
        let total_str = outcome.invoice.total.to_string();
        let currency = outcome.invoice.currency.clone();
        tokio::spawn(async move {
            crate::notifications::send::notify_invoice_paid(
                email_sender_cloned2.as_ref(),
                &pool_clone2,
                &customer_id2,
                &inv_number,
                &total_str,
                &currency,
            )
            .await;
        });
    }

    Ok(outcome.payment)
}
