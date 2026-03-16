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
async fn list_customers_empty(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let resp = server
        .get("/api/customers")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert!(body.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_customers_with_health_scores(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    create_test_customer(&pool).await;

    let resp = server
        .get("/api/customers")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert_eq!(body.len(), 1);
    // The test customer helper inserts health_score = 80
    assert_eq!(body[0]["health_score"].as_i64().unwrap(), 80);
    assert_eq!(body[0]["trend"].as_str().unwrap(), "stable");
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_customer(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let resp = server
        .post("/api/customers")
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "name": "Acme Corp",
            "email": "billing@acme.com",
            "metadata": { "source": "integration-test" }
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["name"].as_str().unwrap(), "Acme Corp");
    assert_eq!(body["email"].as_str().unwrap(), "billing@acme.com");
    assert!(body["id"].as_str().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_customer_by_id(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let customer_id = create_test_customer(&pool).await;

    let resp = server
        .get(&format!("/api/customers/{}", customer_id))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["id"].as_str().unwrap(), customer_id);
    assert_eq!(body["industry"].as_str().unwrap(), "Technology");
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_customer(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let customer_id = create_test_customer(&pool).await;

    let resp = server
        .put(&format!("/api/customers/{}", customer_id))
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "name": "Updated Customer Name",
            "email": "new-email@test.com"
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["name"].as_str().unwrap(), "Updated Customer Name");
    assert_eq!(body["email"].as_str().unwrap(), "new-email@test.com");
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_customer(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let customer_id = create_test_customer(&pool).await;

    let resp = server
        .delete(&format!("/api/customers/{}", customer_id))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["success"].as_bool().unwrap(), true);

    // Confirm it's gone
    let resp = server
        .get(&format!("/api/customers/{}", customer_id))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_nonexistent_customer_returns_404(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let resp = server
        .get("/api/customers/nonexistent-id-12345")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}
