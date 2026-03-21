pub mod repository;
pub mod schema;
pub mod service;

use crate::db::models::UsageEvent;
use crate::error::Result;
use repository::PgUsageRepository;
use sqlx::PgPool;

pub use schema::{CreateUsageEventRequest, ListUsageEventsFilter};

pub async fn list_usage_events(pool: &PgPool, subscription_id: &str) -> Result<Vec<UsageEvent>> {
    let repo = PgUsageRepository::new(pool);
    service::list_usage_events(&repo, subscription_id).await
}

pub async fn create_usage_event(pool: &PgPool, req: CreateUsageEventRequest) -> Result<UsageEvent> {
    let repo = PgUsageRepository::new(pool);
    service::create_usage_event(&repo, req).await
}
