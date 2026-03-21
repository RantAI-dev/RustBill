pub mod repository;
pub mod schema;
pub mod service;

use crate::error::Result;
use repository::PgInvoiceRepository;
use sqlx::PgPool;

pub use schema::{
    AddInvoiceItemRequest, CreateInvoiceDraft, CreateInvoiceRequest, InvoiceItemDraft, InvoiceView,
    ListInvoicesFilter, UpdateInvoiceDraft, UpdateInvoiceRequest,
};

pub async fn list_invoices(pool: &PgPool, filter: &ListInvoicesFilter) -> Result<Vec<InvoiceView>> {
    let repo = PgInvoiceRepository::new(pool);
    service::list_invoices(&repo, filter).await
}

pub async fn get_invoice(pool: &PgPool, id: &str) -> Result<crate::db::models::Invoice> {
    let repo = PgInvoiceRepository::new(pool);
    service::get_invoice(&repo, id).await
}

pub async fn create_invoice(
    pool: &PgPool,
    req: CreateInvoiceRequest,
) -> Result<crate::db::models::Invoice> {
    let repo = PgInvoiceRepository::new(pool);
    service::create_invoice(&repo, req).await
}

pub async fn create_invoice_with_notification(
    pool: &PgPool,
    req: CreateInvoiceRequest,
    email_sender: Option<&crate::notifications::email::EmailSender>,
) -> Result<crate::db::models::Invoice> {
    let customer_id = req.customer_id.clone();
    let invoice = create_invoice(pool, req).await?;

    let pool_clone = pool.clone();
    let email_sender_cloned = email_sender.cloned();
    let inv_number = invoice.invoice_number.clone();
    let total_str = invoice.total.to_string();
    let currency = invoice.currency.clone();
    tokio::spawn(async move {
        crate::notifications::send::notify_invoice_created(
            email_sender_cloned.as_ref(),
            &pool_clone,
            &customer_id,
            &inv_number,
            &total_str,
            &currency,
        )
        .await;
    });

    Ok(invoice)
}

pub async fn update_invoice(
    pool: &PgPool,
    id: &str,
    req: UpdateInvoiceRequest,
) -> Result<crate::db::models::Invoice> {
    let repo = PgInvoiceRepository::new(pool);
    service::update_invoice(&repo, id, req).await
}

pub async fn update_invoice_with_notification(
    pool: &PgPool,
    id: &str,
    req: UpdateInvoiceRequest,
    email_sender: Option<&crate::notifications::email::EmailSender>,
) -> Result<crate::db::models::Invoice> {
    let is_marking_paid = req.status == Some(crate::db::models::InvoiceStatus::Paid);
    let is_marking_issued = req.status == Some(crate::db::models::InvoiceStatus::Issued);
    let invoice = update_invoice(pool, id, req).await?;

    if is_marking_paid {
        let pool_clone = pool.clone();
        let email_sender_cloned = email_sender.cloned();
        let customer_id = invoice.customer_id.clone();
        let inv_number = invoice.invoice_number.clone();
        let total_str = invoice.total.to_string();
        let currency = invoice.currency.clone();
        tokio::spawn(async move {
            crate::notifications::send::notify_invoice_paid(
                email_sender_cloned.as_ref(),
                &pool_clone,
                &customer_id,
                &inv_number,
                &total_str,
                &currency,
            )
            .await;
        });
    }

    if is_marking_issued {
        let pool_clone = pool.clone();
        let email_sender_cloned = email_sender.cloned();
        let customer_id = invoice.customer_id.clone();
        let inv_number = invoice.invoice_number.clone();
        let total_str = invoice.total.to_string();
        let currency = invoice.currency.clone();
        let due_date = invoice
            .due_at
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "N/A".to_string());
        tokio::spawn(async move {
            crate::notifications::send::notify_invoice_issued(
                email_sender_cloned.as_ref(),
                &pool_clone,
                &customer_id,
                &inv_number,
                &total_str,
                &currency,
                &due_date,
            )
            .await;
        });
    }

    Ok(invoice)
}

pub async fn delete_invoice(pool: &PgPool, id: &str) -> Result<()> {
    let repo = PgInvoiceRepository::new(pool);
    service::delete_invoice(&repo, id).await
}

pub async fn add_invoice_item(
    pool: &PgPool,
    invoice_id: &str,
    req: AddInvoiceItemRequest,
) -> Result<crate::db::models::InvoiceItem> {
    let repo = PgInvoiceRepository::new(pool);
    service::add_invoice_item(&repo, invoice_id, req).await
}

pub async fn list_invoice_items(
    pool: &PgPool,
    invoice_id: &str,
) -> Result<Vec<crate::db::models::InvoiceItem>> {
    let repo = PgInvoiceRepository::new(pool);
    service::list_invoice_items(&repo, invoice_id).await
}
