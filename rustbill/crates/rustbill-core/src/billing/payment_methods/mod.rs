pub mod repository;
pub mod schema;
pub mod service;

use crate::db::models::SavedPaymentMethod;
use crate::error::Result;
use repository::PgPaymentMethodRepository;
use sqlx::PgPool;

pub use schema::{CreatePaymentMethodDraft, CreatePaymentMethodRequest};

pub async fn list_for_customer(
    pool: &PgPool,
    customer_id: &str,
) -> Result<Vec<SavedPaymentMethod>> {
    let repo = PgPaymentMethodRepository::new(pool);
    service::list_for_customer(&repo, customer_id).await
}

pub async fn get_default(pool: &PgPool, customer_id: &str) -> Result<Option<SavedPaymentMethod>> {
    let repo = PgPaymentMethodRepository::new(pool);
    service::get_default(&repo, customer_id).await
}

pub async fn create(pool: &PgPool, req: CreatePaymentMethodRequest) -> Result<SavedPaymentMethod> {
    let repo = PgPaymentMethodRepository::new(pool);
    service::create(&repo, req).await
}

pub async fn set_default(
    pool: &PgPool,
    customer_id: &str,
    method_id: &str,
) -> Result<SavedPaymentMethod> {
    let repo = PgPaymentMethodRepository::new(pool);
    service::set_default(&repo, customer_id, method_id).await
}

pub async fn remove(pool: &PgPool, customer_id: &str, method_id: &str) -> Result<()> {
    let repo = PgPaymentMethodRepository::new(pool);
    service::remove(&repo, customer_id, method_id).await
}

pub async fn mark_failed(pool: &PgPool, method_id: &str) -> Result<()> {
    let repo = PgPaymentMethodRepository::new(pool);
    service::mark_failed(&repo, method_id).await
}
