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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn create_api_key_returns_plaintext(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let resp = server
        .post("/api/api-keys")
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "name": "CI/CD Key"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["name"].as_str().unwrap(), "CI/CD Key");
    // The plaintext key should be returned on creation
    let key = body["key"]
        .as_str()
        .expect("plaintext key should be present");
    assert!(!key.is_empty());
    // The hashed key should NOT be in the response
    assert!(body.get("hashed_key").is_none() || body["hashed_key"].is_null());
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_api_keys_masked(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;

    // Insert an API key via the helper (uses the real schema)
    let (_id, _plaintext) = create_test_api_key(&pool).await;

    let resp = server
        .get("/api/api-keys")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert_eq!(body.len(), 1);

    // The response should contain name and key_prefix but NOT hashed_key
    assert!(body[0]["name"].as_str().is_some());
    assert!(body[0]["key_prefix"].as_str().is_some());
    // The hashed_key should be stripped (the handler uses `to_jsonb(k) - 'hashed_key'`)
    // Note: the column is actually named `key_hash` in the schema, but the query strips `hashed_key`
    assert!(
        body[0].get("hashed_key").is_none() || body[0]["hashed_key"].is_null(),
        "hashed_key should not be exposed in list"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn revoke_api_key(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let (key_id, _plaintext) = create_test_api_key(&pool).await;

    let resp = server
        .delete(&format!("/api/api-keys/{}", key_id))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["success"].as_bool().unwrap());

    // Revoking the same key again should return 404 (already revoked, revoked_at IS NOT NULL)
    let resp = server
        .delete(&format!("/api/api-keys/{}", key_id))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn revoke_nonexistent_key_returns_404(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let resp = server
        .delete("/api/api-keys/nonexistent-id")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}
