use super::schema::{ApiKeyInfo, AuthUser};
use crate::db::models::UserRole;
use crate::error::Result;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use sqlx::PgPool;

#[async_trait]
pub(crate) trait AuthRepository: Send + Sync {
    async fn find_api_key_by_hash(&self, key_hash: &str) -> Result<Option<ApiKeyInfo>>;
    async fn create_session(
        &self,
        token: &str,
        user_id: &str,
        expires_at: NaiveDateTime,
    ) -> Result<()>;
    async fn validate_session(&self, token: &str) -> Result<Option<SessionUserRow>>;
    async fn delete_session(&self, token: &str) -> Result<u64>;
    async fn upsert_keycloak_user(&self, email: &str, name: &str) -> Result<String>;
}

#[derive(Clone)]
pub struct PgAuthRepository {
    pool: PgPool,
}

impl PgAuthRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct SessionUserRow {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) role: UserRole,
    pub(crate) customer_id: Option<String>,
    pub(crate) expires_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ApiKeyRow {
    id: String,
    name: String,
    customer_id: Option<String>,
    status: String,
}

#[async_trait]
impl AuthRepository for PgAuthRepository {
    async fn find_api_key_by_hash(&self, key_hash: &str) -> Result<Option<ApiKeyInfo>> {
        let row = sqlx::query_as::<_, ApiKeyRow>(
            "SELECT id, name, customer_id, status::text FROM api_keys WHERE key_hash = $1",
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) if r.status == "active" => {
                let pool = self.pool.clone();
                let id = r.id.clone();
                tokio::spawn(async move {
                    let _ = sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
                        .bind(&id)
                        .execute(&pool)
                        .await;
                });

                Ok(Some(ApiKeyInfo {
                    id: r.id,
                    name: r.name,
                    customer_id: r.customer_id,
                }))
            }
            _ => Ok(None),
        }
    }

    async fn create_session(
        &self,
        token: &str,
        user_id: &str,
        expires_at: NaiveDateTime,
    ) -> Result<()> {
        sqlx::query("INSERT INTO sessions (id, user_id, expires_at) VALUES ($1, $2, $3)")
            .bind(&token)
            .bind(user_id)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn validate_session(&self, token: &str) -> Result<Option<SessionUserRow>> {
        sqlx::query_as::<_, SessionUserRow>(
            r#"
            SELECT u.id, u.name, u.email, u.role, u.customer_id, s.expires_at
            FROM sessions s
            JOIN users u ON u.id = s.user_id
            WHERE s.id = $1
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }

    async fn delete_session(&self, token: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(token)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn upsert_keycloak_user(&self, email: &str, name: &str) -> Result<String> {
        let row = sqlx::query_scalar::<_, String>(
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
        .await?;
        Ok(row)
    }
}

impl From<SessionUserRow> for AuthUser {
    fn from(row: SessionUserRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            email: row.email,
            role: row.role,
            customer_id: row.customer_id,
        }
    }
}
