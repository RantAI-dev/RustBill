pub mod repository;
pub mod schema;
pub mod service;

use crate::db::models::Refund;
use crate::error::Result;
use repository::PgRefundRepository;
use sqlx::PgPool;

pub use schema::{CreateRefundRequest, ListRefundsFilter};

pub async fn list_refunds(pool: &PgPool, filter: &ListRefundsFilter) -> Result<Vec<Refund>> {
    let repo = PgRefundRepository::new(pool);
    service::list_refunds(&repo, filter).await
}

pub async fn create_refund(pool: &PgPool, req: CreateRefundRequest) -> Result<Refund> {
    let repo = PgRefundRepository::new(pool);
    service::create_refund(&repo, req).await
}
