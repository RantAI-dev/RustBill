//! Session management: create, validate, delete sessions.

use crate::db::models::UserRole;
use chrono::{NaiveDateTime, Utc};
use rand::Rng;
use serde::Serialize;
use sqlx::PgPool;

/// Authenticated user extracted from a session.
#[derive(Debug, Clone, Serialize)]
pub struct AuthUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub customer_id: Option<String>,
}

/// Create a new session for a user. Returns the session token (64 hex chars).
pub async fn create_session(
    pool: &PgPool,
    user_id: &str,
    expiry_days: u32,
) -> crate::error::Result<String> {
    let token = generate_token();
    let expires_at = Utc::now().naive_utc() + chrono::Duration::days(expiry_days as i64);

    sqlx::query("INSERT INTO sessions (id, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(&token)
        .bind(user_id)
        .bind(expires_at)
        .execute(pool)
        .await?;

    Ok(token)
}

/// Validate a session token. Returns the authenticated user or None.
pub async fn validate_session(
    pool: &PgPool,
    token: &str,
) -> crate::error::Result<Option<AuthUser>> {
    let row = sqlx::query_as::<_, SessionUserRow>(
        r#"
        SELECT u.id, u.name, u.email, u.role, u.customer_id, s.expires_at
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        WHERE s.id = $1
        "#,
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) if r.expires_at > Utc::now().naive_utc() => Ok(Some(AuthUser {
            id: r.id,
            name: r.name,
            email: r.email,
            role: r.role,
            customer_id: r.customer_id,
        })),
        Some(_) => {
            // Session expired — clean it up
            let _ = delete_session(pool, token).await;
            Ok(None)
        }
        None => Ok(None),
    }
}

/// Delete a session by token.
pub async fn delete_session(pool: &PgPool, token: &str) -> crate::error::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(token)
        .execute(pool)
        .await?;
    Ok(())
}

/// Generate a cryptographically random 64-char hex token.
fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill(&mut bytes);
    hex::encode(bytes)
}

#[derive(sqlx::FromRow)]
struct SessionUserRow {
    id: String,
    name: String,
    email: String,
    role: UserRole,
    customer_id: Option<String>,
    expires_at: NaiveDateTime,
}
