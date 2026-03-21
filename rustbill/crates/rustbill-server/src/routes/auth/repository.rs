use super::schema::{KeycloakTokens, LoginUserRecord};
use async_trait::async_trait;
use rustbill_core::auth::session::AuthUser;
use rustbill_core::config::KeycloakConfig;
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait AuthRepository: Send + Sync {
    async fn find_user_for_login(
        &self,
        email: &str,
    ) -> Result<Option<LoginUserRecord>, BillingError>;

    async fn create_session(&self, user_id: &str, expiry_days: u32)
        -> Result<String, BillingError>;

    async fn delete_session(&self, token: &str) -> Result<(), BillingError>;

    async fn validate_session(&self, token: &str) -> Result<Option<AuthUser>, BillingError>;

    async fn exchange_keycloak_code(
        &self,
        keycloak: &KeycloakConfig,
        code: &str,
        callback_url: &str,
    ) -> Result<KeycloakTokens, BillingError>;

    async fn find_or_create_keycloak_user(
        &self,
        keycloak: &KeycloakConfig,
        access_token: &str,
    ) -> Result<String, BillingError>;

    async fn upsert_keycloak_user(&self, email: &str, name: &str) -> Result<String, BillingError>;
}

#[derive(Clone)]
pub struct SqlxAuthRepository {
    pool: PgPool,
    http: reqwest::Client,
}

impl SqlxAuthRepository {
    pub fn new(pool: PgPool, http: reqwest::Client) -> Self {
        Self { pool, http }
    }
}

#[derive(sqlx::FromRow)]
struct LoginUserRow {
    id: String,
    name: String,
    email: String,
    password_hash: Option<String>,
    role: rustbill_core::db::models::UserRole,
}

#[async_trait]
impl AuthRepository for SqlxAuthRepository {
    async fn find_user_for_login(
        &self,
        email: &str,
    ) -> Result<Option<LoginUserRecord>, BillingError> {
        let row = sqlx::query_as::<_, LoginUserRow>(
            "SELECT id, name, email, password_hash, role FROM users WHERE LOWER(email) = LOWER($1)",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(row.map(|row| LoginUserRecord {
            id: row.id,
            name: row.name,
            email: row.email,
            password_hash: row.password_hash,
            role: row.role,
        }))
    }

    async fn create_session(
        &self,
        user_id: &str,
        expiry_days: u32,
    ) -> Result<String, BillingError> {
        rustbill_core::auth::session::create_session(&self.pool, user_id, expiry_days).await
    }

    async fn delete_session(&self, token: &str) -> Result<(), BillingError> {
        rustbill_core::auth::session::delete_session(&self.pool, token).await
    }

    async fn validate_session(&self, token: &str) -> Result<Option<AuthUser>, BillingError> {
        rustbill_core::auth::session::validate_session(&self.pool, token).await
    }

    async fn exchange_keycloak_code(
        &self,
        keycloak: &KeycloakConfig,
        code: &str,
        callback_url: &str,
    ) -> Result<KeycloakTokens, BillingError> {
        let (access_token, id_token) =
            rustbill_core::auth::keycloak::exchange_code(&self.http, keycloak, code, callback_url)
                .await
                .map_err(|e| BillingError::bad_request(format!("Token exchange failed: {e}")))?;

        Ok(KeycloakTokens {
            access_token,
            id_token,
        })
    }

    async fn find_or_create_keycloak_user(
        &self,
        keycloak: &KeycloakConfig,
        access_token: &str,
    ) -> Result<String, BillingError> {
        rustbill_core::auth::keycloak::find_or_create_user(
            &self.pool,
            keycloak,
            access_token,
            &self.http,
        )
        .await
    }

    async fn upsert_keycloak_user(&self, email: &str, name: &str) -> Result<String, BillingError> {
        sqlx::query_scalar::<_, String>(
            r#"
            INSERT INTO users (id, email, name, role, auth_provider)
            VALUES (gen_random_uuid()::text, $1, $2, 'admin', 'keycloak')
            ON CONFLICT (email) DO UPDATE SET
                name = EXCLUDED.name,
                auth_provider = 'keycloak',
                updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(email)
        .bind(name)
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }
}
