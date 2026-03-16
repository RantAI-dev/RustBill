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
async fn list_products_empty(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let resp = server
        .get("/api/products")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert!(body.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_products_returns_seeded(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    create_test_product(&pool, "licensed").await;

    let resp = server
        .get("/api/products")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["product_type"].as_str().unwrap(), "licensed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_product_valid(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let resp = server
        .post("/api/products")
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "name": "My SaaS Product",
            "product_type": "saas",
            "target": "50000",
            "mau": 1000,
            "dau": 250
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["name"].as_str().unwrap(), "My SaaS Product");
    assert_eq!(body["product_type"].as_str().unwrap(), "saas");
    assert!(body["id"].as_str().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_product_invalid_name_empty(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let resp = server
        .post("/api/products")
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "name": "",
            "product_type": "licensed"
        }))
        .await;

    // ValidatedJson returns 400 for validation errors
    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = resp.json();
    assert!(body["error"].is_object() || body["error"].is_string());
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_product_by_id(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let product_id = create_test_product(&pool, "api").await;

    let resp = server
        .get(&format!("/api/products/{}", product_id))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["id"].as_str().unwrap(), product_id);
    assert_eq!(body["product_type"].as_str().unwrap(), "api");
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_product(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let product_id = create_test_product(&pool, "licensed").await;

    let resp = server
        .put(&format!("/api/products/{}", product_id))
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "name": "Updated Name",
            "target": "99999"
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["name"].as_str().unwrap(), "Updated Name");
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_product(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let product_id = create_test_product(&pool, "saas").await;

    let resp = server
        .delete(&format!("/api/products/{}", product_id))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["success"].as_bool().unwrap(), true);

    // Confirm it's gone
    let resp = server
        .get(&format!("/api/products/{}", product_id))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}
