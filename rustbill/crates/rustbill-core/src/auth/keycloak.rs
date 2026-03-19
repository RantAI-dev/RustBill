//! Keycloak SSO integration: OIDC token exchange + user creation.

use crate::config::KeycloakConfig;
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    #[serde(rename = "sub")]
    _sub: String,
    email: Option<String>,
    name: Option<String>,
    preferred_username: Option<String>,
    realm_access: Option<RealmAccess>,
}

#[derive(Debug, Deserialize)]
struct RealmAccess {
    roles: Vec<String>,
}

/// Build the Keycloak authorization URL for redirect.
pub fn build_auth_url(config: &KeycloakConfig, redirect_uri: &str, state: &str) -> String {
    format!(
        "{}/protocol/openid-connect/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid email profile&state={}",
        config.realm_url,
        urlencoding::encode(&config.client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(state),
    )
}

/// Exchange authorization code for tokens.
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

/// Fetch user info from Keycloak using access token.
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

/// Find or create a user from Keycloak user info.
/// Returns the user ID.
pub async fn find_or_create_user(
    pool: &PgPool,
    config: &KeycloakConfig,
    access_token: &str,
    http: &Client,
) -> crate::error::Result<String> {
    let info = get_user_info(http, config, access_token)
        .await
        .map_err(|e| anyhow::anyhow!("keycloak userinfo failed: {e}"))?;

    let email = info
        .email
        .ok_or_else(|| crate::error::BillingError::bad_request("Keycloak account has no email"))?;

    // Check if user is admin (has configured admin role)
    if let Some(ref admin_role) = config.admin_role {
        let has_role = info
            .realm_access
            .as_ref()
            .map(|ra| ra.roles.contains(admin_role))
            .unwrap_or(false);
        if !has_role {
            return Err(crate::error::BillingError::Forbidden);
        }
    }

    let name = info
        .name
        .or(info.preferred_username)
        .unwrap_or_else(|| email.clone());

    // Upsert user
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
    .bind(&email)
    .bind(&name)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Build the Keycloak logout URL.
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
