pub mod repository;
pub mod schema;
pub mod service;

use crate::error::Result;
use crate::notifications::email::EmailSender;
use repository::PgLifecycleRepository;
pub use schema::LifecycleResult;
use sqlx::PgPool;

pub async fn run_full_lifecycle(
    pool: &PgPool,
    email_sender: Option<&EmailSender>,
    http_client: &reqwest::Client,
) -> Result<LifecycleResult> {
    let repo = PgLifecycleRepository::new(pool, email_sender, http_client);
    service::run_full_lifecycle(&repo).await
}

pub async fn generate_pending_invoices(
    pool: &PgPool,
    email_sender: Option<&EmailSender>,
    http_client: &reqwest::Client,
) -> Result<u64> {
    let repo = PgLifecycleRepository::new(pool, email_sender, http_client);
    service::generate_pending_invoices(&repo).await
}
