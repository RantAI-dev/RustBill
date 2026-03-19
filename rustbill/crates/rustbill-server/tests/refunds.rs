mod common;

use axum_test::TestServer;
use common::*;
use serde_json::json;
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Insert a payment directly into the database for a given invoice.
/// Returns the payment ID.
async fn create_test_payment(pool: &PgPool, invoice_id: &str, amount: i64) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO payments
           (id, invoice_id, amount, method, reference, paid_at, created_at)
           VALUES ($1, $2, $3, 'manual'::payment_method, 'REF-TEST', $4, $4)"#,
    )
    .bind(&id)
    .bind(invoice_id)
    .bind(amount)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert payment");

    id
}

/// Insert a refund directly. Returns the refund ID.
async fn insert_refund(
    pool: &PgPool,
    payment_id: &str,
    invoice_id: &str,
    amount: i64,
    status: &str,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        r#"INSERT INTO refunds (id, payment_id, invoice_id, amount, reason, status, created_at)
           VALUES ($1, $2, $3, $4, 'test refund', $5::refund_status, now())"#,
    )
    .bind(&id)
    .bind(payment_id)
    .bind(invoice_id)
    .bind(amount)
    .bind(status)
    .execute(pool)
    .await
    .expect("failed to insert refund");

    id
}

async fn setup(pool: PgPool) -> (TestServer, String, String, String, String) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let customer_id = create_test_customer(&pool).await;
    let invoice_id = create_test_invoice(&pool, &customer_id).await;
    let payment_id = create_test_payment(&pool, &invoice_id, 10000).await;
    (server, token, customer_id, invoice_id, payment_id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn list_refunds_empty(pool: PgPool) {
    let (server, token, _cid, _iid, _pid) = setup(pool).await;

    let resp = server
        .get("/api/billing/refunds")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert!(body.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_refund_within_amount(pool: PgPool) {
    let (server, token, _cid, _iid, payment_id) = setup(pool).await;

    let resp = server
        .post("/api/billing/refunds")
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "paymentId": payment_id,
            "amount": 5000,
            "reason": "Customer requested partial refund"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["payment_id"].as_str().unwrap(), payment_id);
    assert_eq!(body["status"].as_str().unwrap(), "pending");
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_refunds_returns_created(pool: PgPool) {
    let (server, token, _cid, invoice_id, payment_id) = setup(pool.clone()).await;

    // Insert a refund directly
    let _refund_id = insert_refund(&pool, &payment_id, &invoice_id, 3000, "pending").await;

    let resp = server
        .get("/api/billing/refunds")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["payment_id"].as_str().unwrap(), payment_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_refund(pool: PgPool) {
    let (server, token, _cid, invoice_id, payment_id) = setup(pool.clone()).await;
    let refund_id = insert_refund(&pool, &payment_id, &invoice_id, 2000, "pending").await;

    let resp = server
        .delete(&format!("/api/billing/refunds/{refund_id}"))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["success"], json!(true));

    // Verify it is gone
    let resp = server
        .get(&format!("/api/billing/refunds/{refund_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn completing_refund_emits_sales_event(pool: PgPool) {
    let (server, token, _cid, invoice_id, payment_id) = setup(pool.clone()).await;
    let refund_id = insert_refund(&pool, &payment_id, &invoice_id, 2000, "pending").await;

    let resp = server
        .put(&format!("/api/billing/refunds/{refund_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({ "status": "completed" }))
        .await;
    resp.assert_status_ok();

    let event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sales_events WHERE source_table = 'refunds' AND source_id = $1 AND event_type = 'refund.completed'",
    )
    .bind(&refund_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(event_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn deleting_completed_refund_emits_reversal_metadata(pool: PgPool) {
    let (server, token, _cid, invoice_id, payment_id) = setup(pool.clone()).await;

    let create_resp = server
        .post("/api/billing/refunds")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({
            "paymentId": payment_id,
            "invoiceId": invoice_id,
            "amount": 1500,
            "reason": "test"
        }))
        .await;
    create_resp.assert_status(axum::http::StatusCode::CREATED);
    let created: serde_json::Value = create_resp.json();
    let refund_id = created["id"].as_str().unwrap().to_string();

    let complete_resp = server
        .put(&format!("/api/billing/refunds/{refund_id}"))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({ "status": "completed" }))
        .await;
    complete_resp.assert_status_ok();

    let delete_resp = server
        .delete(&format!("/api/billing/refunds/{refund_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;
    delete_resp.assert_status_ok();

    let reversal: serde_json::Value = sqlx::query_scalar(
        r#"SELECT to_jsonb(se)
           FROM sales_events se
           WHERE se.source_table = 'refund_revisions'
             AND se.source_id = $1
             AND se.event_type = 'refund.reversal'
           ORDER BY se.created_at DESC
           LIMIT 1"#,
    )
    .bind(&refund_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        reversal["metadata"]["reversal_of_event_type"],
        json!("refund.completed")
    );
    assert!(
        reversal["metadata"]["reversal_of_event_id"]
            .as_str()
            .is_some(),
        "expected reversal_of_event_id metadata"
    );
}
