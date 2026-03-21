//! Keycloak compatibility layer.

use super::repository::PgAuthRepository;
use crate::config::KeycloakConfig;
use reqwest::Client;
use sqlx::PgPool;

pub use super::service::{build_auth_url, build_logout_url, exchange_code};

pub async fn find_or_create_user(
    pool: &PgPool,
    config: &KeycloakConfig,
    access_token: &str,
    http: &Client,
) -> crate::error::Result<String> {
    let repo = PgAuthRepository::new(pool);
    super::service::find_or_create_user_with_repo(&repo, config, access_token, http).await
}
