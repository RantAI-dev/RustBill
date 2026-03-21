pub mod repository;
pub mod schema;
pub mod service;

use crate::db::models::PricingPlan;
use crate::error::Result;
use repository::PgPlansRepository;
use sqlx::PgPool;

pub use schema::{CreatePlanRequest, PlanView, UpdatePlanRequest};

pub async fn list_plans(pool: &PgPool) -> Result<Vec<PlanView>> {
    let repo = PgPlansRepository::new(pool);
    service::list_plans(&repo).await
}

pub async fn get_plan(pool: &PgPool, id: &str) -> Result<PricingPlan> {
    let repo = PgPlansRepository::new(pool);
    service::get_plan(&repo, id).await
}

pub async fn create_plan(pool: &PgPool, req: CreatePlanRequest) -> Result<PricingPlan> {
    let repo = PgPlansRepository::new(pool);
    service::create_plan(&repo, req).await
}

pub async fn update_plan(pool: &PgPool, id: &str, req: UpdatePlanRequest) -> Result<PricingPlan> {
    let repo = PgPlansRepository::new(pool);
    service::update_plan(&repo, id, req).await
}

pub async fn delete_plan(pool: &PgPool, id: &str) -> Result<()> {
    let repo = PgPlansRepository::new(pool);
    service::delete_plan(&repo, id).await
}
