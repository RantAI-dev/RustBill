mod common;

use common::*;
use serde_json::json;
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a user with a given role. Returns the user's email.
async fn create_user_with_role(pool: &PgPool, role: &str) -> String {
    let user_id = uuid::Uuid::new_v4().to_string();
    let password_hash =
        rustbill_core::auth::password::hash_password("testpass123").expect("hash failed");
    let now = chrono::Utc::now().naive_utc();
    let email = format!("{}@test.com", &user_id[..8]);

    sqlx::query(
        r#"INSERT INTO users (id, email, name, password_hash, role, auth_provider, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5::user_role, 'default', $6, $6)"#,
    )
    .bind(&user_id)
    .bind(&email)
    .bind("Test User")
    .bind(&password_hash)
    .bind(role)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert user");

    email
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn login_valid_admin(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let email = create_user_with_role(&pool, "admin").await;

    let resp = server
        .post("/api/auth/login")
        .json(&json!({
            "email": email,
            "password": "testpass123"
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["user"].is_object());
    assert_eq!(body["user"]["email"].as_str().unwrap(), email);
    assert_eq!(body["user"]["role"].as_str().unwrap(), "admin");

    // Should set a session cookie
    let set_cookie = resp
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        set_cookie.contains("session="),
        "expected session cookie, got: {}",
        set_cookie
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_invalid_password(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let email = create_user_with_role(&pool, "admin").await;

    let resp = server
        .post("/api/auth/login")
        .json(&json!({
            "email": email,
            "password": "wrongpassword"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_non_admin_forbidden(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let email = create_user_with_role(&pool, "customer").await;

    let resp = server
        .post("/api/auth/login")
        .json(&json!({
            "email": email,
            "password": "testpass123"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_nonexistent_user(pool: PgPool) {
    let server = test_server(pool).await;

    let resp = server
        .post("/api/auth/login")
        .json(&json!({
            "email": "nobody@example.com",
            "password": "anything"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn me_with_valid_session(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    let resp = server
        .get("/api/auth/me")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["user"].is_object());
    assert!(body["user"]["id"].as_str().is_some());
    assert!(body["user"]["email"].as_str().is_some());
    assert_eq!(body["user"]["role"].as_str().unwrap(), "admin");
}

#[sqlx::test(migrations = "../../migrations")]
async fn me_without_session_unauthorized(pool: PgPool) {
    let server = test_server(pool).await;

    let resp = server.get("/api/auth/me").await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn logout_clears_session(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    let resp = server
        .post("/api/auth/logout")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["ok"].as_bool().unwrap(), true);

    // Session should now be invalid
    let resp = server
        .get("/api/auth/me")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}
