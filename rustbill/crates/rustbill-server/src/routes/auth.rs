use super::ApiResult;
use crate::app::SharedState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/keycloak/login", get(keycloak_login))
        .route("/keycloak/callback", get(keycloak_callback))
}

// ---------------------------------------------------------------------------
// Keycloak OAuth
// ---------------------------------------------------------------------------

async fn keycloak_login(State(state): State<SharedState>) -> ApiResult<axum::response::Response> {
    use axum::response::IntoResponse;
    use rustbill_core::error::BillingError;

    let kc = state
        .config
        .auth
        .keycloak
        .as_ref()
        .ok_or_else(|| BillingError::bad_request("Keycloak is not configured"))?;

    // Generate random CSRF state (32 bytes -> 64 hex chars)
    let csrf_state = {
        use rand::Rng;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill(&mut bytes);
        hex::encode(bytes)
    };

    // Determine callback URL from config
    let callback_url = format!(
        "{}/api/auth/keycloak/callback",
        std::env::var("APP_URL").unwrap_or_else(|_| format!(
            "http://{}:{}",
            state.config.server.host, state.config.server.port
        ))
    );

    let auth_url = rustbill_core::auth::keycloak::build_auth_url(kc, &callback_url, &csrf_state);

    // Set oauth_state cookie and redirect
    let state_cookie =
        format!("oauth_state={csrf_state}; HttpOnly; SameSite=Lax; Path=/; Max-Age=600");

    Ok((
        StatusCode::TEMPORARY_REDIRECT,
        [
            (axum::http::header::LOCATION, auth_url),
            (axum::http::header::SET_COOKIE, state_cookie),
        ],
    )
        .into_response())
}

#[derive(Deserialize)]
struct KeycloakCallbackParams {
    code: String,
    state: String,
}

async fn keycloak_callback(
    State(state): State<SharedState>,
    Query(params): Query<KeycloakCallbackParams>,
    headers: axum::http::HeaderMap,
) -> ApiResult<axum::response::Response> {
    use rustbill_core::error::BillingError;

    let kc = state
        .config
        .auth
        .keycloak
        .as_ref()
        .ok_or_else(|| BillingError::bad_request("Keycloak is not configured"))?;

    // Extract and validate the oauth_state cookie
    let stored_state = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix("oauth_state=").map(String::from)
            })
        })
        .ok_or(BillingError::bad_request("Missing oauth_state cookie"))?;

    if stored_state != params.state {
        return Err(BillingError::bad_request("CSRF state mismatch").into());
    }

    // Build callback URL (same as login)
    let callback_url = format!(
        "{}/api/auth/keycloak/callback",
        std::env::var("APP_URL").unwrap_or_else(|_| format!(
            "http://{}:{}",
            state.config.server.host, state.config.server.port
        ))
    );

    // Exchange code for tokens
    let (access_token, id_token) = rustbill_core::auth::keycloak::exchange_code(
        &state.http_client,
        kc,
        &params.code,
        &callback_url,
    )
    .await
    .map_err(|e| BillingError::bad_request(format!("Token exchange failed: {e}")))?;

    // Try to extract user info from id_token first, fall back to userinfo endpoint
    let user_id = if let Some(ref id_token_str) = id_token {
        match decode_jwt_payload(id_token_str) {
            Some(claims) => {
                let email = claims["email"]
                    .as_str()
                    .ok_or_else(|| BillingError::bad_request("No email in ID token"))?;
                let name = claims["name"]
                    .as_str()
                    .or_else(|| claims["preferred_username"].as_str())
                    .unwrap_or(email);

                // Check admin role from token if configured
                if let Some(ref admin_role) = kc.admin_role {
                    let has_role = claims["realm_access"]["roles"]
                        .as_array()
                        .map(|roles| roles.iter().any(|r| r.as_str() == Some(admin_role)))
                        .unwrap_or(false);
                    if !has_role {
                        return Err(BillingError::Forbidden.into());
                    }
                }

                // Upsert user
                let user_id = sqlx::query_scalar::<_, String>(
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
                .fetch_one(&state.db)
                .await
                .map_err(BillingError::from)?;

                user_id
            }
            None => {
                // Fall back to userinfo endpoint
                rustbill_core::auth::keycloak::find_or_create_user(
                    &state.db,
                    kc,
                    &access_token,
                    &state.http_client,
                )
                .await?
            }
        }
    } else {
        // No id_token, use userinfo endpoint
        rustbill_core::auth::keycloak::find_or_create_user(
            &state.db,
            kc,
            &access_token,
            &state.http_client,
        )
        .await?
    };

    // Create local session
    let session_token = rustbill_core::auth::create_session(
        &state.db,
        &user_id,
        state.config.auth.session_expiry_days,
    )
    .await?;

    // Set session cookie, clear oauth_state cookie, redirect to /
    let session_cookie = format!(
        "session={session_token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}",
        state.config.auth.session_expiry_days * 86400
    );
    let clear_state_cookie = "oauth_state=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0";

    let response = axum::response::Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header(axum::http::header::LOCATION, "/")
        .header(axum::http::header::SET_COOKIE, session_cookie)
        .header(axum::http::header::SET_COOKIE, clear_state_cookie)
        .body(axum::body::Body::empty())
        .unwrap();

    Ok(response)
}

/// Decode the payload (claims) part of a JWT without verifying the signature.
/// Returns None if the token is malformed.
fn decode_jwt_payload(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    // JWT uses base64url encoding (no padding)
    use base64::Engine;
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    serde_json::from_slice(&payload_bytes).ok()
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

async fn login(
    State(state): State<SharedState>,
    Json(body): Json<LoginRequest>,
) -> ApiResult<axum::response::Response> {
    use axum::response::IntoResponse;
    use rustbill_core::error::BillingError;

    // Block if keycloak is active
    if state.config.auth.provider == "keycloak" {
        return Err(BillingError::bad_request("Use SSO to log in").into());
    }

    // Find user by email (case-insensitive)
    let user = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            Option<String>,
            rustbill_core::db::models::UserRole,
        ),
    >(
        "SELECT id, name, email, password_hash, role FROM users WHERE LOWER(email) = LOWER($1)",
    )
    .bind(&body.email)
    .fetch_optional(&state.db)
    .await
    .map_err(BillingError::from)?
    .ok_or_else(|| BillingError::Unauthorized)?;

    let (id, name, email, password_hash, role) = user;

    // Verify password
    let hash = password_hash.ok_or(BillingError::Unauthorized)?;
    let valid = rustbill_core::auth::verify_password(&body.password, &hash)
        .map_err(|_| BillingError::Unauthorized)?;
    if !valid {
        return Err(BillingError::Unauthorized.into());
    }

    // Block non-admin
    if role != rustbill_core::db::models::UserRole::Admin {
        return Err(BillingError::Forbidden.into());
    }

    // Create session
    let token =
        rustbill_core::auth::create_session(&state.db, &id, state.config.auth.session_expiry_days)
            .await?;

    // Set cookie
    let cookie = format!(
        "session={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}",
        state.config.auth.session_expiry_days * 86400
    );

    let response = (
        StatusCode::OK,
        [(axum::http::header::SET_COOKIE, cookie)],
        Json(serde_json::json!({
            "user": { "id": id, "name": name, "email": email, "role": role }
        })),
    )
        .into_response();

    Ok(response)
}

async fn logout(
    State(state): State<SharedState>,
    headers: axum::http::HeaderMap,
) -> ApiResult<axum::response::Response> {
    use axum::response::IntoResponse;

    let token = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix("session=").map(String::from)
            })
        });

    if let Some(token) = token {
        let _ = rustbill_core::auth::delete_session(&state.db, &token).await;
    }

    let clear_cookie = "session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0";

    let mut response_json = serde_json::json!({ "ok": true });

    // If keycloak, return logout URL
    if let Some(ref kc) = state.config.auth.keycloak {
        let logout_url = rustbill_core::auth::keycloak::build_logout_url(kc, None, "/login");
        response_json["redirectUrl"] = serde_json::json!(logout_url);
    }

    Ok((
        StatusCode::OK,
        [(axum::http::header::SET_COOKIE, clear_cookie.to_string())],
        Json(response_json),
    )
        .into_response())
}

async fn me(
    State(state): State<SharedState>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    use rustbill_core::error::BillingError;

    let token = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix("session=").map(String::from)
            })
        })
        .ok_or(BillingError::Unauthorized)?;

    let user = rustbill_core::auth::validate_session(&state.db, &token)
        .await?
        .ok_or(BillingError::Unauthorized)?;

    Ok(Json(serde_json::json!({
        "user": {
            "id": user.id,
            "name": user.name,
            "email": user.email,
            "role": user.role,
            "customerId": user.customer_id,
        }
    })))
}
