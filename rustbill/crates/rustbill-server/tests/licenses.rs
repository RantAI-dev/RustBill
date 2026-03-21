mod common;

use common::*;
use serde_json::json;
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn setup(pool: PgPool) -> (axum_test::TestServer, String) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    (server, token)
}

/// Insert a test license directly using the actual table schema.
/// Returns the license key.
async fn insert_test_license(
    pool: &PgPool,
    product_id: Option<&str>,
    customer_id: Option<&str>,
) -> String {
    let key = format!("LIC-TEST-{}", &uuid::Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        r#"INSERT INTO licenses
           (key, customer_id, customer_name, product_id, product_name,
            status, created_at, expires_at, license_type, max_activations)
           VALUES ($1, $2, 'Test Customer', $3, 'Test Product',
                   'active', '2026-01-01', '2027-12-31', 'simple', 5)"#,
    )
    .bind(&key)
    .bind(customer_id)
    .bind(product_id)
    .execute(pool)
    .await
    .expect("failed to insert license");

    key
}

async fn insert_test_activation(
    pool: &PgPool,
    license_key: &str,
    device_id: &str,
    device_name: Option<&str>,
    ip_address: Option<&str>,
) {
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO license_activations
           (id, license_key, device_id, device_name, ip_address, activated_at, last_seen_at)
           VALUES ($1, $2, $3, $4, $5, $6, $6)"#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(license_key)
    .bind(device_id)
    .bind(device_name)
    .bind(ip_address)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert activation");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn list_licenses_empty(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let resp = server
        .get("/api/licenses")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert!(body.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_licenses_returns_seeded(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let key = insert_test_license(&pool, None, None).await;

    let resp = server
        .get("/api/licenses")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["key"].as_str().unwrap(), key);
    assert_eq!(body[0]["status"].as_str().unwrap(), "active");
}

#[sqlx::test(migrations = "../../migrations")]
async fn generate_keypair(pool: PgPool) {
    let (server, token) = setup(pool).await;

    // Initially no keypair exists
    let resp = server
        .get("/api/licenses/keypair")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(!body["exists"].as_bool().unwrap());

    // Generate a keypair
    let resp = server
        .post("/api/licenses/keypair")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = resp.json();
    assert!(body["success"].as_bool().unwrap());
    assert!(body["publicKey"].as_str().is_some());
    let public_key = body["publicKey"].as_str().unwrap().to_string();

    // Verify the keypair now exists
    let resp = server
        .get("/api/licenses/keypair")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["exists"].as_bool().unwrap());
    assert_eq!(body["publicKey"].as_str().unwrap(), public_key);
}

#[sqlx::test(migrations = "../../migrations")]
async fn generate_keypair_overwrites_existing(pool: PgPool) {
    let (server, token) = setup(pool).await;

    // Generate first keypair
    let resp = server
        .post("/api/licenses/keypair")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let first_key = resp.json::<serde_json::Value>()["publicKey"]
        .as_str()
        .unwrap()
        .to_string();

    // Generate second keypair (ON CONFLICT DO UPDATE, so no 409)
    let resp = server
        .post("/api/licenses/keypair")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let second_key = resp.json::<serde_json::Value>()["publicKey"]
        .as_str()
        .unwrap()
        .to_string();

    // Keys should differ (new keypair generated)
    assert_ne!(first_key, second_key);
}

#[sqlx::test(migrations = "../../migrations")]
async fn sign_license(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let key = insert_test_license(&pool, None, None).await;

    // Must generate a keypair first
    server
        .post("/api/licenses/keypair")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    // Sign the license
    let resp = server
        .post(&format!("/api/licenses/{}/sign", key))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["license_key"].as_str().unwrap(), key);
    assert!(body["signed_payload"].as_str().is_some());
    assert!(body["signature"].as_str().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn sign_without_keypair_fails(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let key = insert_test_license(&pool, None, None).await;

    // Try to sign without generating keypair
    let resp = server
        .post(&format!("/api/licenses/{}/sign", key))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    // Should fail with 400 (bad request: "no signing keypair exists")
    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn verify_license_online(pool: PgPool) {
    let (server, _token) = setup(pool.clone()).await;
    let key = insert_test_license(&pool, None, None).await;

    // The /api/licenses/verify endpoint does NOT require AdminUser
    let resp = server
        .post("/api/licenses/verify")
        .json(&json!({ "key": key }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["valid"].as_bool().unwrap());
    // The license object is returned under "license"
    assert!(body["license"].is_object());
    assert_eq!(body["license"]["key"].as_str().unwrap(), key);
}

#[sqlx::test(migrations = "../../migrations")]
async fn verify_nonexistent_license_returns_not_found(pool: PgPool) {
    let (server, _token) = setup(pool).await;

    let resp = server
        .post("/api/licenses/verify")
        .json(&json!({ "key": "DOES-NOT-EXIST" }))
        .await;

    // Returns 404 because the handler uses .ok_or_else NotFound
    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn export_signed_license(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let key = insert_test_license(&pool, None, None).await;

    // Generate keypair and sign
    server
        .post("/api/licenses/keypair")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    server
        .post(&format!("/api/licenses/{}/sign", key))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    // Export the license file
    let resp = server
        .get(&format!("/api/licenses/{}/export", key))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.contains("octet-stream"),
        "expected application/octet-stream, got: {}",
        content_type
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_license(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let key = insert_test_license(&pool, None, None).await;

    let resp = server
        .delete(&format!("/api/licenses/{}", key))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["success"].as_bool().unwrap());

    // Confirm it's gone via verify (should return 404)
    let resp = server
        .post("/api/licenses/verify")
        .json(&json!({ "key": key }))
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_license_activations(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let key = insert_test_license(&pool, None, None).await;
    insert_test_activation(
        &pool,
        &key,
        "device-123",
        Some("Test Device"),
        Some("203.0.113.10"),
    )
    .await;

    let resp = server
        .get(&format!("/api/licenses/{}/activations", key))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["license_key"].as_str().unwrap(), key);
    assert_eq!(body[0]["device_id"].as_str().unwrap(), "device-123");
    assert_eq!(body[0]["device_name"].as_str().unwrap(), "Test Device");
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_license_activation_by_device_id(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let key = insert_test_license(&pool, None, None).await;
    insert_test_activation(&pool, &key, "device-456", Some("Laptop"), None).await;

    let resp = server
        .delete(&format!(
            "/api/licenses/{}/activations?deviceId=device-456",
            key
        ))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["success"].as_bool().unwrap());

    let resp = server
        .get(&format!("/api/licenses/{}/activations", key))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert!(body.is_empty());
}
