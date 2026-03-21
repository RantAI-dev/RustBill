use super::repository::DealsRepository;
use super::schema::{CreateDealRequest, UpdateDealRequest};
use crate::db::models::Deal;
use crate::error::Result;

pub async fn list_deals<R: DealsRepository + ?Sized>(
    repo: &R,
    product_type: Option<&str>,
    deal_type: Option<&str>,
) -> Result<Vec<Deal>> {
    repo.list_deals(product_type, deal_type).await
}

pub async fn get_deal<R: DealsRepository + ?Sized>(repo: &R, id: &str) -> Result<Deal> {
    repo.get_deal(id).await
}

pub async fn create_deal<R: DealsRepository + ?Sized>(
    repo: &R,
    req: CreateDealRequest,
) -> Result<Deal> {
    repo.create_deal(req).await
}

pub async fn update_deal<R: DealsRepository + ?Sized>(
    repo: &R,
    id: &str,
    req: UpdateDealRequest,
) -> Result<Deal> {
    repo.update_deal(id, req).await
}

pub async fn delete_deal<R: DealsRepository + ?Sized>(repo: &R, id: &str) -> Result<()> {
    repo.delete_deal(id).await
}
