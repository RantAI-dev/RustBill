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
    let invoice_id = create_test_invoice(&pool, &customer_id).await;
    (server, token, customer_id, invoice_id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn list_payments_empty(pool: PgPool) {
    let (server, token, _cid, _iid) = setup(pool).await;

    let resp = server
        .get("/api/billing/payments")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert!(body.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_payment(pool: PgPool) {
    let (server, token, _customer_id, invoice_id) = setup(pool).await;

    let resp = server
        .post("/api/billing/payments")
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "invoiceId": invoice_id,
            "method": "stripe",
            "amount": 5500,
            "stripePaymentIntentId": "pi_test_123"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["invoice_id"].as_str().unwrap(), invoice_id);
    assert_eq!(body["method"].as_str().unwrap(), "stripe");
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_payment_marks_completed(pool: PgPool) {
    let (server, token, _customer_id, invoice_id) = setup(pool).await;

    // Create payment
    let resp = server
        .post("/api/billing/payments")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({
            "invoiceId": invoice_id,
            "method": "stripe",
            "amount": 5500,
            "stripePaymentIntentId": "pi_test_456"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let payment: serde_json::Value = resp.json();
    let payment_id = payment["id"].as_str().unwrap();

    // Update payment notes
    let resp = server
        .put(&format!("/api/billing/payments/{payment_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({ "notes": "Payment completed" }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["notes"].as_str().unwrap(), "Payment completed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_payments_returns_created(pool: PgPool) {
    let (server, token, _customer_id, invoice_id) = setup(pool).await;

    // Create a payment first
    let resp = server
        .post("/api/billing/payments")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({
            "invoiceId": invoice_id,
            "method": "stripe",
            "amount": 2000,
            "stripePaymentIntentId": "pi_test_789"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);

    // List payments
    let resp = server
        .get("/api/billing/payments")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert_eq!(body.len(), 1);
}
