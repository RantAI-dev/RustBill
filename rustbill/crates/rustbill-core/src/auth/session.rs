//! Session compatibility layer.

use super::repository::PgAuthRepository;
pub use super::schema::AuthUser;
use sqlx::PgPool;

pub async fn create_session(
    pool: &PgPool,
    user_id: &str,
    expiry_days: u32,
) -> crate::error::Result<String> {
    let repo = PgAuthRepository::new(pool);
    super::service::create_session_with_repo(&repo, user_id, expiry_days).await
}

pub async fn validate_session(
    pool: &PgPool,
    token: &str,
) -> crate::error::Result<Option<AuthUser>> {
    let repo = PgAuthRepository::new(pool);
    super::service::validate_session_with_repo(&repo, token).await
}

pub async fn delete_session(pool: &PgPool, token: &str) -> crate::error::Result<()> {
    let repo = PgAuthRepository::new(pool);
    super::service::delete_session_with_repo(&repo, token).await
}
