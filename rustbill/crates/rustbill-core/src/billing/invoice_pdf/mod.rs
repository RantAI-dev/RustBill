pub mod repository;
pub mod schema;
pub mod service;

use crate::error::Result;
use repository::PgInvoicePdfRepository;
use sqlx::PgPool;

pub async fn generate_invoice_pdf(pool: &PgPool, invoice_id: &str) -> Result<Vec<u8>> {
    let repo = PgInvoicePdfRepository::new(pool);
    service::generate_invoice_pdf(
        &repo,
        schema::GenerateInvoicePdfRequest {
            invoice_id: invoice_id.to_string(),
        },
    )
    .await
}
