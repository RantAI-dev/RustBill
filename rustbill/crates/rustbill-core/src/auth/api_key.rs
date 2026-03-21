//! API key compatibility layer.

use super::repository::PgAuthRepository;
pub use super::schema::ApiKeyInfo;
pub use super::service::{generate_api_key, get_key_prefix, hash_api_key};
use sqlx::PgPool;

pub async fn verify_api_key(pool: &PgPool, key: &str) -> crate::error::Result<Option<ApiKeyInfo>> {
    let repo = PgAuthRepository::new(pool);
    super::service::verify_api_key_with_repo(&repo, key).await
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
