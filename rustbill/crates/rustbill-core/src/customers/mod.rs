pub mod repository;
pub mod schema;
pub mod service;
pub mod validation;

use crate::db::models::Customer;
use crate::error::Result;
use repository::PgCustomersRepository;
use schema::{CreateCustomerRequest, UpdateCustomerRequest};
use sqlx::PgPool;

pub async fn list_customers(pool: &PgPool) -> Result<Vec<serde_json::Value>> {
    let repo = PgCustomersRepository::new(pool);
    service::list_customers(&repo).await
}

pub async fn get_customer(pool: &PgPool, id: &str) -> Result<Customer> {
    let repo = PgCustomersRepository::new(pool);
    service::get_customer(&repo, id).await
}

pub async fn create_customer(pool: &PgPool, req: CreateCustomerRequest) -> Result<Customer> {
    let repo = PgCustomersRepository::new(pool);
    service::create_customer(&repo, req).await
}

pub async fn update_customer(
    pool: &PgPool,
    id: &str,
    req: UpdateCustomerRequest,
) -> Result<Customer> {
    let repo = PgCustomersRepository::new(pool);
    service::update_customer(&repo, id, req).await
}

pub async fn delete_customer(pool: &PgPool, id: &str) -> Result<()> {
    let repo = PgCustomersRepository::new(pool);
    service::delete_customer(&repo, id).await
}
