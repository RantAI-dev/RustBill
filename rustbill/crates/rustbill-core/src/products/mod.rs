pub mod repository;
pub mod schema;
pub mod service;
pub mod validation;

use crate::db::models::Product;
use crate::error::Result;
use repository::PgProductsRepository;
use schema::{CreateProductRequest, UpdateProductRequest};
use sqlx::PgPool;

pub async fn list_products(pool: &PgPool) -> Result<Vec<serde_json::Value>> {
    let repo = PgProductsRepository::new(pool);
    service::list_products(&repo).await
}

pub async fn get_product(pool: &PgPool, id: &str) -> Result<Product> {
    let repo = PgProductsRepository::new(pool);
    service::get_product(&repo, id).await
}

pub async fn create_product(pool: &PgPool, req: CreateProductRequest) -> Result<Product> {
    let repo = PgProductsRepository::new(pool);
    service::create_product(&repo, req).await
}

pub async fn update_product(pool: &PgPool, id: &str, req: UpdateProductRequest) -> Result<Product> {
    let repo = PgProductsRepository::new(pool);
    service::update_product(&repo, id, req).await
}

pub async fn delete_product(pool: &PgPool, id: &str) -> Result<()> {
    let repo = PgProductsRepository::new(pool);
    service::delete_product(&repo, id).await
}
