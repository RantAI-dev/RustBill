use super::repository::SqlxAuthRepository;
use super::schema::{
    KeycloakCallbackQuery, LoginRequest, LoginResponse, LogoutResponse, MeResponse,
};
use super::service;
use crate::app::SharedState;
use crate::routes::ApiResult;
use axum::{
    extract::{Query, State},
    http::{header::HeaderName, HeaderValue, StatusCode},
    routing::{get, post},
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/keycloak/login", get(keycloak_login))
        .route("/keycloak/callback", get(keycloak_callback))
}

fn public_origin(state: &SharedState, env_var: &str) -> String {
    std::env::var(env_var).unwrap_or_else(|_| {
        format!(
            "http://{}:{}",
            state.config.server.host, state.config.server.port
        )
    })
}

fn cookie_value(name: &str, value: &str, max_age: Option<u32>) -> String {
    match max_age {
        Some(age) => format!("{name}={value}; HttpOnly; SameSite=Lax; Path=/; Max-Age={age}"),
        None => format!("{name}={value}; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"),
    }
}

fn extract_cookie(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let cookie = cookie.trim();
                cookie.strip_prefix(&format!("{name}=")).map(String::from)
            })
        })
}

async fn login(
    State(state): State<SharedState>,
    Json(body): Json<LoginRequest>,
) -> ApiResult<(
    StatusCode,
    [(HeaderName, HeaderValue); 1],
    Json<LoginResponse>,
)> {
    let repo = SqlxAuthRepository::new(state.db.clone(), state.http_client.clone());
    let result = service::login(
        &repo,
        &state.config.auth.provider,
        state.config.auth.session_expiry_days,
        &body,
    )
    .await?;

    let cookie = cookie_value(
        "session",
        &result.session_token,
        Some(state.config.auth.session_expiry_days * 86400),
    );
    let response = LoginResponse { user: result.user };
    let cookie_value = HeaderValue::from_str(&cookie)
        .map_err(|e| rustbill_core::error::BillingError::Internal(e.into()))?;

    Ok((
        StatusCode::OK,
        [(axum::http::header::SET_COOKIE, cookie_value)],
        Json(response),
    ))
}

async fn logout(
    State(state): State<SharedState>,
    headers: axum::http::HeaderMap,
) -> ApiResult<(
    StatusCode,
    [(HeaderName, HeaderValue); 1],
    Json<LogoutResponse>,
)> {
    let repo = SqlxAuthRepository::new(state.db.clone(), state.http_client.clone());
    let session_token = extract_cookie(&headers, "session");
    let result = service::logout(
        &repo,
        state.config.auth.keycloak.as_ref(),
        session_token.as_deref(),
    )
    .await?;

    let body = LogoutResponse {
        ok: true,
        redirect_url: result.redirect_url,
    };
    let cookie = HeaderValue::from_static("session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0");

    Ok((
        StatusCode::OK,
        [(axum::http::header::SET_COOKIE, cookie)],
        Json(body),
    ))
}

async fn me(
    State(state): State<SharedState>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<MeResponse>> {
    let repo = SqlxAuthRepository::new(state.db.clone(), state.http_client.clone());
    let token = extract_cookie(&headers, "session")
        .ok_or(rustbill_core::error::BillingError::Unauthorized)?;
    let result = service::me(&repo, &token).await?;
    Ok(Json(MeResponse { user: result.user }))
}

async fn keycloak_login(State(state): State<SharedState>) -> ApiResult<axum::response::Response> {
    let callback_url = format!(
        "{}/api/auth/keycloak/callback",
        public_origin(&state, "APP_URL")
    );
    let result =
        service::keycloak_login(state.config.auth.keycloak.as_ref(), &callback_url).await?;

    let auth_url = HeaderValue::from_str(&result.auth_url)
        .map_err(|e| rustbill_core::error::BillingError::Internal(e.into()))?;
    let state_cookie = HeaderValue::from_str(&result.state_cookie)
        .map_err(|e| rustbill_core::error::BillingError::Internal(e.into()))?;

    let response = axum::http::Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header(axum::http::header::LOCATION, auth_url)
        .header(axum::http::header::SET_COOKIE, state_cookie)
        .body(axum::body::Body::empty())
        .map_err(|e| rustbill_core::error::BillingError::Internal(e.into()))?;

    Ok(response)
}

async fn keycloak_callback(
    State(state): State<SharedState>,
    Query(query): Query<KeycloakCallbackQuery>,
    headers: axum::http::HeaderMap,
) -> ApiResult<axum::response::Response> {
    let repo = SqlxAuthRepository::new(state.db.clone(), state.http_client.clone());
    let stored_state = extract_cookie(&headers, "oauth_state");
    let callback_url = format!(
        "{}/api/auth/keycloak/callback",
        public_origin(&state, "APP_URL")
    );
    let result = service::keycloak_callback(
        &repo,
        state.config.auth.keycloak.as_ref(),
        &query,
        stored_state.as_deref(),
        &callback_url,
        state.config.auth.session_expiry_days,
    )
    .await?;

    let session_cookie = cookie_value(
        "session",
        &result.session_token,
        Some(state.config.auth.session_expiry_days * 86400),
    );
    let clear_state_cookie = cookie_value("oauth_state", "", None);
    let response = axum::http::Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header(axum::http::header::LOCATION, "/")
        .header(
            axum::http::header::SET_COOKIE,
            HeaderValue::from_str(&session_cookie)
                .map_err(|e| rustbill_core::error::BillingError::Internal(e.into()))?,
        )
        .header(
            axum::http::header::SET_COOKIE,
            HeaderValue::from_str(&clear_state_cookie)
                .map_err(|e| rustbill_core::error::BillingError::Internal(e.into()))?,
        )
        .body(axum::body::Body::empty())
        .map_err(|e| rustbill_core::error::BillingError::Internal(e.into()))?;

    Ok(response)
}
