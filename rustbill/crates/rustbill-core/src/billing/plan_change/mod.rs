pub mod repository;
pub mod schema;
pub mod service;

use crate::error::Result;
use repository::PgPlanChangeRepository;
use sqlx::PgPool;

pub use schema::{ChangePlanInput, ChangePlanOutput};

pub async fn change_plan_with_proration(
    pool: &PgPool,
    input: ChangePlanInput<'_>,
) -> Result<ChangePlanOutput> {
    let repo = PgPlanChangeRepository::new(pool);
    service::change_plan_with_proration(&repo, input).await
}
