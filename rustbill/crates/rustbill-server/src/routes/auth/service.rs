use super::repository::AuthRepository;
use super::schema::{
    KeycloakCallbackQuery, KeycloakCallbackResult, KeycloakLoginResult, LoginRequest, LoginResult,
    LogoutResult, MeResult, PublicUserResponse, SessionUserResponse,
};
use base64::Engine;
use rand::Rng;
use rustbill_core::auth::keycloak::{build_auth_url, build_logout_url};
use rustbill_core::config::KeycloakConfig;
use rustbill_core::db::models::UserRole;
use rustbill_core::error::BillingError;

fn random_state() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill(&mut bytes);
    hex::encode(bytes)
}

fn session_user_to_response(user: rustbill_core::auth::session::AuthUser) -> SessionUserResponse {
    SessionUserResponse {
        id: user.id,
        name: user.name,
        email: user.email,
        role: user.role,
        customer_id: user.customer_id,
    }
}

fn login_user_to_response(user: &super::schema::LoginUserRecord) -> PublicUserResponse {
    PublicUserResponse {
        id: user.id.clone(),
        name: user.name.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
    }
}

fn claims_has_role(claims: &serde_json::Value, admin_role: &str) -> bool {
    claims["realm_access"]["roles"]
        .as_array()
        .map(|roles| roles.iter().any(|role| role.as_str() == Some(admin_role)))
        .unwrap_or(false)
}

fn decode_jwt_payload(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    serde_json::from_slice(&payload_bytes).ok()
}

pub async fn login<R: AuthRepository>(
    repo: &R,
    auth_provider: &str,
    expiry_days: u32,
    body: &LoginRequest,
) -> Result<LoginResult, BillingError> {
    if auth_provider == "keycloak" {
        return Err(BillingError::bad_request("Use SSO to log in"));
    }

    let user = repo
        .find_user_for_login(&body.email)
        .await?
        .ok_or(BillingError::Unauthorized)?;

    let hash = user
        .password_hash
        .clone()
        .ok_or(BillingError::Unauthorized)?;
    let valid = rustbill_core::auth::password::verify_password(&body.password, &hash)
        .map_err(|_| BillingError::Unauthorized)?;
    if !valid {
        return Err(BillingError::Unauthorized);
    }

    if user.role != UserRole::Admin {
        return Err(BillingError::Forbidden);
    }

    let session_token = repo.create_session(&user.id, expiry_days).await?;

    Ok(LoginResult {
        user: login_user_to_response(&user),
        session_token,
    })
}

pub async fn logout<R: AuthRepository>(
    repo: &R,
    keycloak: Option<&KeycloakConfig>,
    session_token: Option<&str>,
) -> Result<LogoutResult, BillingError> {
    if let Some(token) = session_token {
        let _ = repo.delete_session(token).await;
    }

    let redirect_url = keycloak.map(|kc| build_logout_url(kc, None, "/login"));
    Ok(LogoutResult { redirect_url })
}

pub async fn me<R: AuthRepository>(repo: &R, token: &str) -> Result<MeResult, BillingError> {
    let user = repo
        .validate_session(token)
        .await?
        .ok_or(BillingError::Unauthorized)?;

    Ok(MeResult {
        user: session_user_to_response(user),
    })
}

pub async fn keycloak_login(
    keycloak: Option<&KeycloakConfig>,
    callback_url: &str,
) -> Result<KeycloakLoginResult, BillingError> {
    let kc = keycloak.ok_or_else(|| BillingError::bad_request("Keycloak is not configured"))?;
    let csrf_state = random_state();
    let auth_url = build_auth_url(kc, callback_url, &csrf_state);
    let state_cookie =
        format!("oauth_state={csrf_state}; HttpOnly; SameSite=Lax; Path=/; Max-Age=600");

    Ok(KeycloakLoginResult {
        auth_url,
        state_cookie,
    })
}

pub async fn keycloak_callback<R: AuthRepository>(
    repo: &R,
    keycloak: Option<&KeycloakConfig>,
    query: &KeycloakCallbackQuery,
    stored_state: Option<&str>,
    callback_url: &str,
    expiry_days: u32,
) -> Result<KeycloakCallbackResult, BillingError> {
    let kc = keycloak.ok_or_else(|| BillingError::bad_request("Keycloak is not configured"))?;

    let stored_state =
        stored_state.ok_or_else(|| BillingError::bad_request("Missing oauth_state cookie"))?;
    if stored_state != query.state {
        return Err(BillingError::bad_request("CSRF state mismatch"));
    }

    let tokens = repo
        .exchange_keycloak_code(kc, &query.code, callback_url)
        .await?;

    let user_id = if let Some(ref id_token_str) = tokens.id_token {
        match decode_jwt_payload(id_token_str) {
            Some(claims) => {
                let email = claims["email"]
                    .as_str()
                    .ok_or_else(|| BillingError::bad_request("No email in ID token"))?;
                let name = claims["name"]
                    .as_str()
                    .or_else(|| claims["preferred_username"].as_str())
                    .unwrap_or(email);

                if let Some(ref admin_role) = kc.admin_role {
                    if !claims_has_role(&claims, admin_role) {
                        return Err(BillingError::Forbidden);
                    }
                }

                repo.upsert_keycloak_user(email, name).await?
            }
            None => {
                repo.find_or_create_keycloak_user(kc, &tokens.access_token)
                    .await?
            }
        }
    } else {
        repo.find_or_create_keycloak_user(kc, &tokens.access_token)
            .await?
    };

    let session_token = repo.create_session(&user_id, expiry_days).await?;

    Ok(KeycloakCallbackResult { session_token })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::auth::schema::{KeycloakTokens, LoginUserRecord};
    use async_trait::async_trait;
    use rustbill_core::auth::session::AuthUser;
    use rustbill_core::config::KeycloakConfig;
    use rustbill_core::db::models::UserRole;
    use rustbill_core::error::BillingError;
    use std::sync::Mutex;

    struct MockRepo {
        user: Option<LoginUserRecord>,
        session_token: String,
        deleted: Mutex<Vec<String>>,
        validated: Option<AuthUser>,
    }

    impl MockRepo {
        fn new() -> Self {
            Self {
                user: Some(LoginUserRecord {
                    id: "user-1".into(),
                    name: "Admin".into(),
                    email: "admin@example.com".into(),
                    password_hash: Some(
                        rustbill_core::auth::password::hash_password("secret")
                            .expect("hash should work"),
                    ),
                    role: UserRole::Admin,
                }),
                session_token: "sess-1".into(),
                deleted: Mutex::new(Vec::new()),
                validated: Some(AuthUser {
                    id: "user-1".into(),
                    name: "Admin".into(),
                    email: "admin@example.com".into(),
                    role: UserRole::Admin,
                    customer_id: Some("cust-1".into()),
                }),
            }
        }
    }

    #[async_trait]
    impl AuthRepository for MockRepo {
        async fn find_user_for_login(
            &self,
            _email: &str,
        ) -> Result<Option<LoginUserRecord>, BillingError> {
            Ok(self.user.clone())
        }

        async fn create_session(
            &self,
            _user_id: &str,
            _expiry_days: u32,
        ) -> Result<String, BillingError> {
            Ok(self.session_token.clone())
        }

        async fn delete_session(&self, token: &str) -> Result<(), BillingError> {
            if let Ok(mut guard) = self.deleted.lock() {
                guard.push(token.to_string());
            }
            Ok(())
        }

        async fn validate_session(&self, _token: &str) -> Result<Option<AuthUser>, BillingError> {
            Ok(self.validated.clone())
        }

        async fn exchange_keycloak_code(
            &self,
            _keycloak: &KeycloakConfig,
            _code: &str,
            _callback_url: &str,
        ) -> Result<KeycloakTokens, BillingError> {
            Ok(KeycloakTokens {
                access_token: "access".into(),
                id_token: None,
            })
        }

        async fn find_or_create_keycloak_user(
            &self,
            _keycloak: &KeycloakConfig,
            _access_token: &str,
        ) -> Result<String, BillingError> {
            Ok("kc-user".into())
        }

        async fn upsert_keycloak_user(
            &self,
            _email: &str,
            _name: &str,
        ) -> Result<String, BillingError> {
            Ok("kc-user".into())
        }
    }

    #[tokio::test]
    async fn login_blocks_keycloak_provider() {
        let repo = MockRepo::new();
        let err = login(
            &repo,
            "keycloak",
            7,
            &LoginRequest {
                email: "admin@example.com".into(),
                password: "secret".into(),
            },
        )
        .await
        .unwrap_err();

        assert!(matches!(err, BillingError::BadRequest(_)));
    }

    #[tokio::test]
    async fn me_uses_session_validation() {
        let repo = MockRepo::new();
        let result = me(&repo, "sess-1").await.expect("me should succeed");
        assert_eq!(result.user.customer_id.as_deref(), Some("cust-1"));
    }

    #[tokio::test]
    async fn keycloak_login_returns_redirect_cookie() {
        let keycloak = KeycloakConfig {
            realm_url: "https://kc.example/realms/demo".into(),
            client_id: "client-1".into(),
            client_secret: None,
            admin_role: None,
        };
        let result = keycloak_login(
            Some(&keycloak),
            "https://app.example/api/auth/keycloak/callback",
        )
        .await
        .expect("keycloak login should succeed");
        assert!(result.auth_url.contains("client_id="));
        assert!(result.state_cookie.contains("oauth_state="));
    }
}
