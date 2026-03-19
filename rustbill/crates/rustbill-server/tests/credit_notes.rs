mod common;

use common::*;
use serde_json::json;
use sqlx::PgPool;

async fn setup(pool: PgPool) -> (axum_test::TestServer, String, String, String) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let customer_id = create_test_customer(&pool).await;
    let invoice_id = create_test_invoice(&pool, &customer_id).await;
    (server, token, customer_id, invoice_id)
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_credit_note_emits_sales_event(pool: PgPool) {
    let (server, token, customer_id, invoice_id) = setup(pool.clone()).await;

    let resp = server
        .post("/api/billing/credit-notes")
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "invoiceId": invoice_id,
            "customerId": customer_id,
            "amount": 2500,
            "reason": "billing adjustment"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = resp.json();
    let credit_note_id = body["id"].as_str().unwrap();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sales_events WHERE source_table = 'credit_notes' AND source_id = $1 AND event_type = 'credit_note.created'",
    )
    .bind(credit_note_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn issuing_credit_note_emits_issued_event(pool: PgPool) {
    let (server, token, customer_id, invoice_id) = setup(pool.clone()).await;

    let create = server
        .post("/api/billing/credit-notes")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({
            "invoiceId": invoice_id,
            "customerId": customer_id,
            "amount": 1500,
            "reason": "issue test"
        }))
        .await;
    create.assert_status(axum::http::StatusCode::CREATED);
    let created: serde_json::Value = create.json();
    let credit_note_id = created["id"].as_str().unwrap();

    let update = server
        .put(&format!("/api/billing/credit-notes/{credit_note_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({ "status": "issued" }))
        .await;
    update.assert_status_ok();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sales_events WHERE source_table = 'credit_notes' AND source_id = $1 AND event_type = 'credit_note.issued'",
    )
    .bind(credit_note_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn deleting_credit_note_emits_reversal_with_metadata(pool: PgPool) {
    let (server, token, customer_id, invoice_id) = setup(pool.clone()).await;

    let create = server
        .post("/api/billing/credit-notes")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({
            "invoiceId": invoice_id,
            "customerId": customer_id,
            "amount": 1800,
            "reason": "delete test"
        }))
        .await;
    create.assert_status(axum::http::StatusCode::CREATED);
    let created: serde_json::Value = create.json();
    let credit_note_id = created["id"].as_str().unwrap().to_string();

    let issue = server
        .put(&format!("/api/billing/credit-notes/{credit_note_id}"))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({ "status": "issued" }))
        .await;
    issue.assert_status_ok();

    let delete = server
        .delete(&format!("/api/billing/credit-notes/{credit_note_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;
    delete.assert_status_ok();

    let reversal: serde_json::Value = sqlx::query_scalar(
        r#"SELECT to_jsonb(se)
           FROM sales_events se
           WHERE se.source_table = 'credit_note_revisions'
             AND se.source_id = $1
             AND se.event_type = 'credit_note.reversal'
           ORDER BY se.created_at DESC
           LIMIT 1"#,
    )
    .bind(&credit_note_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        reversal["metadata"]["reversal_of_event_type"],
        json!("credit_note.issued")
    );
    assert!(
        reversal["metadata"]["reversal_of_event_id"]
            .as_str()
            .is_some(),
        "expected reversal_of_event_id metadata"
    );
}
