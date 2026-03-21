use super::repository::AuthRepository;
use super::schema::{ApiKeyInfo, AuthUser};
use crate::config::KeycloakConfig;
use crate::error::{BillingError, Result};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{Duration, Utc};
use rand::Rng;
use reqwest::Client;
use sha2::{Digest, Sha256};

pub(crate) fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill(&mut bytes);
    hex::encode(bytes)
}

pub fn generate_api_key() -> String {
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .collect();
    let mut rng = rand::thread_rng();
    let random_part: String = (0..40)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect();
    format!("pk_live_{random_part}")
}

pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn get_key_prefix(key: &str) -> String {
    key.chars().take(12).collect()
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut rand::rngs::OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("password hash failed: {e}"))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    if hash.starts_with("$argon2") {
        let parsed =
            PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("invalid argon2 hash: {e}"))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    } else if hash.starts_with("$2a$") || hash.starts_with("$2b$") || hash.starts_with("$2y$") {
        Ok(bcrypt::verify(password, hash)
            .map_err(|e| anyhow::anyhow!("bcrypt verify failed: {e}"))?)
    } else {
        Ok(false)
    }
}

pub(crate) async fn verify_api_key_with_repo<R: AuthRepository + ?Sized>(
    repo: &R,
    key: &str,
) -> Result<Option<ApiKeyInfo>> {
    let key_hash = hash_api_key(key);
    repo.find_api_key_by_hash(&key_hash).await
}

pub(crate) async fn create_session_with_repo<R: AuthRepository + ?Sized>(
    repo: &R,
    user_id: &str,
    expiry_days: u32,
) -> Result<String> {
    let token = generate_session_token();
    let expires_at = Utc::now().naive_utc() + Duration::days(expiry_days as i64);
    repo.create_session(&token, user_id, expires_at).await?;
    Ok(token)
}

pub(crate) async fn validate_session_with_repo<R: AuthRepository + ?Sized>(
    repo: &R,
    token: &str,
) -> Result<Option<AuthUser>> {
    let row = repo.validate_session(token).await?;

    match row {
        Some(r) if r.expires_at > Utc::now().naive_utc() => Ok(Some(r.into())),
        Some(_) => {
            let _ = repo.delete_session(token).await;
            Ok(None)
        }
        None => Ok(None),
    }
}

pub(crate) async fn delete_session_with_repo<R: AuthRepository + ?Sized>(
    repo: &R,
    token: &str,
) -> Result<()> {
    repo.delete_session(token).await?;
    Ok(())
}

pub fn build_auth_url(config: &KeycloakConfig, redirect_uri: &str, state: &str) -> String {
    format!(
        "{}/protocol/openid-connect/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid email profile&state={}",
        config.realm_url,
        urlencoding::encode(&config.client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(state),
    )
}

#[derive(Debug, serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    id_token: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct UserInfo {
    #[serde(rename = "sub")]
    _sub: String,
    email: Option<String>,
    name: Option<String>,
    preferred_username: Option<String>,
    realm_access: Option<RealmAccess>,
}

#[derive(Debug, serde::Deserialize)]
struct RealmAccess {
    roles: Vec<String>,
}

pub async fn exchange_code(
    http: &Client,
    config: &KeycloakConfig,
    code: &str,
    redirect_uri: &str,
) -> anyhow::Result<(String, Option<String>)> {
    let mut params = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", &config.client_id),
    ];

    let secret_str;
    if let Some(ref secret) = config.client_secret {
        secret_str = secret.clone();
        params.push(("client_secret", &secret_str));
    }

    let resp: TokenResponse = http
        .post(format!(
            "{}/protocol/openid-connect/token",
            config.realm_url
        ))
        .form(&params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok((resp.access_token, resp.id_token))
}

async fn get_user_info(
    http: &Client,
    config: &KeycloakConfig,
    access_token: &str,
) -> anyhow::Result<UserInfo> {
    let info: UserInfo = http
        .get(format!(
            "{}/protocol/openid-connect/userinfo",
            config.realm_url
        ))
        .bearer_auth(access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(info)
}

pub(crate) async fn find_or_create_user_with_repo<R: AuthRepository + ?Sized>(
    repo: &R,
    config: &KeycloakConfig,
    access_token: &str,
    http: &Client,
) -> Result<String> {
    let info = get_user_info(http, config, access_token)
        .await
        .map_err(|e| anyhow::anyhow!("keycloak userinfo failed: {e}"))?;

    let email = info
        .email
        .ok_or_else(|| BillingError::bad_request("Keycloak account has no email"))?;

    if let Some(ref admin_role) = config.admin_role {
        let has_role = info
            .realm_access
            .as_ref()
            .map(|ra| ra.roles.contains(admin_role))
            .unwrap_or(false);
        if !has_role {
            return Err(BillingError::Forbidden);
        }
    }

    let name = info
        .name
        .or(info.preferred_username)
        .unwrap_or_else(|| email.clone());

    repo.upsert_keycloak_user(&email, &name).await
}

pub fn build_logout_url(
    config: &KeycloakConfig,
    id_token: Option<&str>,
    redirect_uri: &str,
) -> String {
    let mut url = format!(
        "{}/protocol/openid-connect/logout?post_logout_redirect_uri={}",
        config.realm_url,
        urlencoding::encode(redirect_uri),
    );
    if let Some(token) = id_token {
        url.push_str(&format!("&id_token_hint={token}"));
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key() {
        let key = generate_api_key();
        assert!(key.starts_with("pk_live_"));
        assert_eq!(key.len(), 8 + 40);
    }

    #[test]
    fn test_hash_api_key_deterministic() {
        let key = "pk_live_test1234";
        let h1 = hash_api_key(key);
        let h2 = hash_api_key(key);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_get_key_prefix() {
        let key = "pk_live_abcdefghijklmnop";
        assert_eq!(get_key_prefix(key), "pk_live_abcd");
    }
}
