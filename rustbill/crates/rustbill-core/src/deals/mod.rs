pub mod repository;
pub mod schema;
pub mod service;
pub mod validation;

use crate::db::models::Deal;
use crate::error::Result;
use repository::PgDealsRepository;
use schema::{CreateDealRequest, UpdateDealRequest};
use sqlx::PgPool;

pub use schema::{CreateDealRequest as DealCreateRequest, UpdateDealRequest as DealUpdateRequest};

pub async fn list_deals(
    pool: &PgPool,
    product_type: Option<&str>,
    deal_type: Option<&str>,
) -> Result<Vec<Deal>> {
    let repo = PgDealsRepository::new(pool);
    service::list_deals(&repo, product_type, deal_type).await
}

pub async fn get_deal(pool: &PgPool, id: &str) -> Result<Deal> {
    let repo = PgDealsRepository::new(pool);
    service::get_deal(&repo, id).await
}

pub async fn create_deal(pool: &PgPool, req: CreateDealRequest) -> Result<Deal> {
    let repo = PgDealsRepository::new(pool);
    service::create_deal(&repo, req).await
}

pub async fn update_deal(pool: &PgPool, id: &str, req: UpdateDealRequest) -> Result<Deal> {
    let repo = PgDealsRepository::new(pool);
    service::update_deal(&repo, id, req).await
}

pub async fn delete_deal(pool: &PgPool, id: &str) -> Result<()> {
    let repo = PgDealsRepository::new(pool);
    service::delete_deal(&repo, id).await
}
