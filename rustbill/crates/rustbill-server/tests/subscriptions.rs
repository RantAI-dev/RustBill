mod common;

use axum_test::TestServer;
use common::*;
use serde_json::json;
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn setup(pool: PgPool) -> (TestServer, String, String, String) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let customer_id = create_test_customer(&pool).await;
    let product_id = create_test_product(&pool, "saas").await;
    let plan_id = create_test_plan(&pool, &product_id).await;
    (server, token, customer_id, plan_id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn list_subscriptions_empty(pool: PgPool) {
    let (server, token, _cid, _pid) = setup(pool).await;

    let resp = server
        .get("/api/billing/subscriptions")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert!(body.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_subscription_auto_computes_period(pool: PgPool) {
    let (server, token, customer_id, plan_id) = setup(pool).await;

    let resp = server
        .post("/api/billing/subscriptions")
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "customerId": customer_id,
            "planId": plan_id
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["customer_id"].as_str().unwrap(), customer_id);
    assert_eq!(body["plan_id"].as_str().unwrap(), plan_id);
    assert_eq!(body["status"].as_str().unwrap(), "active");
    // period_start and period_end should be populated
    assert!(body["current_period_start"].as_str().is_some());
    assert!(body["current_period_end"].as_str().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_subscription_with_metadata(pool: PgPool) {
    let (server, token, customer_id, plan_id) = setup(pool).await;

    let resp = server
        .post("/api/billing/subscriptions")
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "customerId": customer_id,
            "planId": plan_id,
            "metadata": { "trial": true, "source": "signup" }
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["metadata"]["trial"], json!(true));
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_subscriptions_returns_created(pool: PgPool) {
    let (server, token, customer_id, plan_id) = setup(pool.clone()).await;

    // Create one via helper
    let _sub_id = create_test_subscription(&pool, &customer_id, &plan_id).await;

    let resp = server
        .get("/api/billing/subscriptions")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["customer_id"].as_str().unwrap(), customer_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_subscription_status(pool: PgPool) {
    let (server, token, customer_id, plan_id) = setup(pool.clone()).await;
    let sub_id = create_test_subscription(&pool, &customer_id, &plan_id).await;

    let resp = server
        .put(&format!("/api/billing/subscriptions/{sub_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({ "status": "paused" }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"].as_str().unwrap(), "paused");
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_subscription_cancels(pool: PgPool) {
    let (server, token, customer_id, plan_id) = setup(pool.clone()).await;
    let sub_id = create_test_subscription(&pool, &customer_id, &plan_id).await;

    let resp = server
        .delete(&format!("/api/billing/subscriptions/{sub_id}"))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["success"], json!(true));

    // Verify the subscription is now cancelled
    let resp = server
        .get(&format!("/api/billing/subscriptions/{sub_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"].as_str().unwrap(), "canceled");
}

#[sqlx::test(migrations = "../../migrations")]
async fn lifecycle_pause_then_resume(pool: PgPool) {
    let (server, token, customer_id, plan_id) = setup(pool.clone()).await;
    let sub_id = create_test_subscription(&pool, &customer_id, &plan_id).await;

    // Pause
    let resp = server
        .post("/api/billing/subscriptions/lifecycle")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({
            "subscriptionId": sub_id,
            "action": "pause"
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"].as_str().unwrap(), "paused");

    // Resume
    let resp = server
        .post("/api/billing/subscriptions/lifecycle")
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "subscriptionId": sub_id,
            "action": "resume"
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"].as_str().unwrap(), "active");
}

#[sqlx::test(migrations = "../../migrations")]
async fn lifecycle_cancel(pool: PgPool) {
    let (server, token, customer_id, plan_id) = setup(pool.clone()).await;
    let sub_id = create_test_subscription(&pool, &customer_id, &plan_id).await;

    let resp = server
        .post("/api/billing/subscriptions/lifecycle")
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "subscriptionId": sub_id,
            "action": "cancel"
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"].as_str().unwrap(), "canceled");
}

#[sqlx::test(migrations = "../../migrations")]
async fn lifecycle_unknown_action_returns_400(pool: PgPool) {
    let (server, token, customer_id, plan_id) = setup(pool.clone()).await;
    let sub_id = create_test_subscription(&pool, &customer_id, &plan_id).await;

    let resp = server
        .post("/api/billing/subscriptions/lifecycle")
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "subscriptionId": sub_id,
            "action": "explode"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
}
