pub mod repository;
pub mod schema;
pub mod service;

use crate::db::models::DunningLogEntry;
use crate::error::Result;
use repository::PgDunningRepository;
use sqlx::PgPool;

pub use schema::{DunningConfig, DunningLogFilter};

pub async fn list_dunning_log(
    pool: &PgPool,
    invoice_id: Option<&str>,
) -> Result<Vec<DunningLogEntry>> {
    let repo = PgDunningRepository::new(pool);
    service::list_dunning_log(
        &repo,
        DunningLogFilter {
            invoice_id: invoice_id.map(str::to_string),
        },
    )
    .await
}

pub async fn run_dunning(pool: &PgPool, config: &DunningConfig) -> Result<u64> {
    let repo = PgDunningRepository::new(pool);
    service::run_dunning(&repo, config).await
}
