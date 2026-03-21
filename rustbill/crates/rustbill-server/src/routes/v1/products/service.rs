use super::repository::ProductsRepository;
use super::schema::ProductListItem;
use rustbill_core::db::models::Product;
use rustbill_core::error::BillingError;

pub async fn list<R: ProductsRepository>(repo: &R) -> Result<Vec<ProductListItem>, BillingError> {
    crate::routes::products::service::list(repo).await
}

pub async fn get<R: ProductsRepository>(repo: &R, id: &str) -> Result<Product, BillingError> {
    crate::routes::products::service::get(repo, id).await
}
