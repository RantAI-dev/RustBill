mod common;

use axum_test::TestServer;
use common::*;
use serde_json::json;
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn setup(pool: PgPool) -> (TestServer, String) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    (server, token)
}

/// Insert a coupon directly into the database. Returns the coupon ID.
async fn create_test_coupon(pool: &PgPool, code: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO coupons
           (id, code, name, discount_type, discount_value, currency, max_redemptions,
            times_redeemed, active, created_at, updated_at)
           VALUES ($1, $2, $3, 'percentage'::discount_type, 10, 'USD', 5, 0, true, $4, $4)"#,
    )
    .bind(&id)
    .bind(code)
    .bind(format!("Coupon {code}"))
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert coupon");

    id
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn list_coupons_empty(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let resp = server
        .get("/api/billing/coupons")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert!(body.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_coupon(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let resp = server
        .post("/api/billing/coupons")
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "code": "SAVE20",
            "discountType": "percentage",
            "discountValue": 20,
            "maxRedemptions": 100
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["code"].as_str().unwrap(), "SAVE20");
    assert_eq!(body["discount_type"].as_str().unwrap(), "percentage");
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_coupon_by_id(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let coupon_id = create_test_coupon(&pool, "TESTGET").await;

    let resp = server
        .get(&format!("/api/billing/coupons/{coupon_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["code"].as_str().unwrap(), "TESTGET");
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_coupon(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let coupon_id = create_test_coupon(&pool, "UPDATEME").await;

    let resp = server
        .put(&format!("/api/billing/coupons/{coupon_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "code": "UPDATED20",
            "discountType": "fixed_amount",
            "discountValue": 500
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["code"].as_str().unwrap(), "UPDATED20");
    assert_eq!(body["discount_type"].as_str().unwrap(), "fixed_amount");
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_coupon(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let coupon_id = create_test_coupon(&pool, "DELETEME").await;

    let resp = server
        .delete(&format!("/api/billing/coupons/{coupon_id}"))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["success"], json!(true));

    // Verify it is gone
    let resp = server
        .get(&format!("/api/billing/coupons/{coupon_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}
