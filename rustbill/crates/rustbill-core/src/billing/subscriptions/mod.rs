pub mod repository;
pub mod schema;
pub mod service;

use crate::error::Result;
use repository::PgSubscriptionRepository;
use sqlx::PgPool;

pub use schema::{
    CreateSubscriptionDraft, CreateSubscriptionRequest, ListSubscriptionsFilter, SubscriptionView,
    UpdateSubscriptionRequest,
};
pub use service::advance_period;

pub async fn list_subscriptions(
    pool: &PgPool,
    filter: &ListSubscriptionsFilter,
) -> Result<Vec<SubscriptionView>> {
    let repo = PgSubscriptionRepository::new(pool);
    service::list_subscriptions(&repo, filter).await
}

pub async fn get_subscription(pool: &PgPool, id: &str) -> Result<crate::db::models::Subscription> {
    let repo = PgSubscriptionRepository::new(pool);
    service::get_subscription(&repo, id).await
}

pub async fn create_subscription(
    pool: &PgPool,
    req: CreateSubscriptionRequest,
) -> Result<crate::db::models::Subscription> {
    let repo = PgSubscriptionRepository::new(pool);
    service::create_subscription(&repo, req).await
}

pub async fn update_subscription(
    pool: &PgPool,
    id: &str,
    req: UpdateSubscriptionRequest,
) -> Result<crate::db::models::Subscription> {
    let repo = PgSubscriptionRepository::new(pool);
    service::update_subscription(&repo, id, req).await
}

pub async fn delete_subscription(pool: &PgPool, id: &str) -> Result<()> {
    let repo = PgSubscriptionRepository::new(pool);
    service::delete_subscription(&repo, id).await
}

pub async fn run_lifecycle(pool: &PgPool) -> Result<u64> {
    let repo = PgSubscriptionRepository::new(pool);
    service::run_lifecycle(&repo).await
}
