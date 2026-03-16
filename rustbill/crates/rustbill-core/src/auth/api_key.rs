//! API key generation, hashing, and verification.

use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

const API_KEY_PREFIX: &str = "pk_live_";

/// Generate a new API key: pk_live_<40 random alphanumeric chars>
pub fn generate_api_key() -> String {
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .collect();
    let mut rng = rand::thread_rng();
    let random_part: String = (0..40)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect();
    format!("{API_KEY_PREFIX}{random_part}")
}

/// SHA-256 hash of an API key for storage.
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Get the display prefix of an API key (first 12 chars).
pub fn get_key_prefix(key: &str) -> String {
    key.chars().take(12).collect()
}

/// Verify an API key against the database. Returns the key ID and name if valid.
pub async fn verify_api_key(pool: &PgPool, key: &str) -> crate::error::Result<Option<ApiKeyInfo>> {
    let key_hash = hash_api_key(key);

    let row =
        sqlx::query_as::<_, ApiKeyRow>("SELECT id, name, status FROM api_keys WHERE key_hash = $1")
            .bind(&key_hash)
            .fetch_optional(pool)
            .await?;

    match row {
        Some(r) if r.status == "active" => {
            // Update last_used_at (fire-and-forget)
            let pool = pool.clone();
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
            }))
        }
        _ => Ok(None),
    }
}

#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
}

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: String,
    name: String,
    status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key() {
        let key = generate_api_key();
        assert!(key.starts_with("pk_live_"));
        assert_eq!(key.len(), 8 + 40); // prefix + random
    }

    #[test]
    fn test_hash_api_key_deterministic() {
        let key = "pk_live_test1234";
        let h1 = hash_api_key(key);
        let h2 = hash_api_key(key);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_get_key_prefix() {
        let key = "pk_live_abcdefghijklmnop";
        assert_eq!(get_key_prefix(key), "pk_live_abcd");
    }
}
