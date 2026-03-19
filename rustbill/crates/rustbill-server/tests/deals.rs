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

/// Insert a test deal directly into the database using the actual table schema.
/// Returns the deal ID.
async fn insert_test_deal(
    pool: &PgPool,
    product_id: Option<&str>,
    product_type: &str,
    deal_type: &str,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO deals
           (id, customer_id, company, contact, email, value, product_id,
            product_name, product_type, deal_type, date, created_at, updated_at)
           VALUES ($1, NULL, $2, $3, $4, 5000.00, $5,
                   $6, $7::product_type, $8::deal_type, '2026-01-15', $9, $9)"#,
    )
    .bind(&id)
    .bind(format!("Test Company {}", &id[..8]))
    .bind("Jane Doe")
    .bind(format!("deal-{}@test.com", &id[..8]))
    .bind(product_id)
    .bind("Test Product")
    .bind(product_type)
    .bind(deal_type)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert deal");

    id
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn list_deals_empty(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let resp = server
        .get("/api/deals")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert!(body.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_deals_returns_seeded(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    insert_test_deal(&pool, None, "licensed", "sale").await;
    insert_test_deal(&pool, None, "saas", "trial").await;

    let resp = server
        .get("/api/deals")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert_eq!(body.len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_deals_filter_by_deal_type(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    insert_test_deal(&pool, None, "licensed", "sale").await;
    insert_test_deal(&pool, None, "saas", "trial").await;

    let resp = server
        .get("/api/deals?dealType=trial")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["deal_type"].as_str().unwrap(), "trial");
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_deal_by_id(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let deal_id = insert_test_deal(&pool, None, "api", "sale").await;

    let resp = server
        .get(&format!("/api/deals/{}", deal_id))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["id"].as_str().unwrap(), deal_id);
    assert_eq!(body["product_type"].as_str().unwrap(), "api");
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_deal(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let deal_id = insert_test_deal(&pool, None, "licensed", "sale").await;

    let resp = server
        .put(&format!("/api/deals/{}", deal_id))
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "name": "Updated Deal",
            "type": "saas",
            "dealType": "partner"
        }))
        .await;

    // The update handler uses COALESCE with name/type/deal_type columns that may
    // not exist in the actual schema. If the schema doesn't have those columns,
    // this may return 500. We test the handler as written.
    // If the schema matches the handler, the update should succeed:
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["id"].as_str().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_deal(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let deal_id = insert_test_deal(&pool, None, "licensed", "sale").await;

    let resp = server
        .delete(&format!("/api/deals/{}", deal_id))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["success"].as_bool().unwrap(), true);

    // Confirm deletion
    let resp = server
        .get(&format!("/api/deals/{}", deal_id))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_nonexistent_deal_returns_404(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let resp = server
        .get("/api/deals/nonexistent-id")
        .add_cookie(cookie::Cookie::new("session", token))
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn updating_deal_emits_reversal_and_replacement_events(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;

    let create_resp = server
        .post("/api/deals")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({
            "company": "Ledger Co",
            "contact": "Ops",
            "email": "ops@ledger.test",
            "value": "1000.00",
            "product_name": "Ledger Product",
            "product_type": "saas",
            "deal_type": "sale",
            "date": "2026-03-19"
        }))
        .await;
    create_resp.assert_status(axum::http::StatusCode::CREATED);
    let created: serde_json::Value = create_resp.json();
    let deal_id = created["id"].as_str().unwrap();

    let update_resp = server
        .put(&format!("/api/deals/{deal_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .json(&json!({
            "value": "1200.00",
            "deal_type": "partner"
        }))
        .await;
    update_resp.assert_status_ok();

    let reversal_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sales_events WHERE event_type = 'deal.reversal' AND metadata ->> 'deal_id' = $1",
    )
    .bind(deal_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(reversal_count, 1);

    let replacement_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sales_events WHERE event_type = 'deal.updated' AND metadata ->> 'deal_id' = $1",
    )
    .bind(deal_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(replacement_count, 1);

    let reversal: serde_json::Value = sqlx::query_scalar(
        r#"SELECT to_jsonb(se)
           FROM sales_events se
           WHERE se.event_type = 'deal.reversal'
             AND se.metadata ->> 'deal_id' = $1
           ORDER BY se.created_at DESC
           LIMIT 1"#,
    )
    .bind(deal_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let replacement: serde_json::Value = sqlx::query_scalar(
        r#"SELECT to_jsonb(se)
           FROM sales_events se
           WHERE se.event_type = 'deal.updated'
             AND se.metadata ->> 'deal_id' = $1
           ORDER BY se.created_at DESC
           LIMIT 1"#,
    )
    .bind(deal_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        reversal["metadata"]["superseded_by_event_id"]
            .as_str()
            .is_some(),
        "expected superseded_by_event_id on reversal"
    );
    assert_eq!(
        reversal["metadata"]["superseded_by_event_id"],
        replacement["id"]
    );
    assert_eq!(
        replacement["metadata"]["replaces_event_type"],
        json!("deal.created")
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn deleting_deal_emits_reversal_metadata_link(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;

    let create_resp = server
        .post("/api/deals")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({
            "company": "Delete Co",
            "contact": "Ops",
            "email": "ops@delete.test",
            "value": "800.00",
            "product_name": "Delete Product",
            "product_type": "saas",
            "deal_type": "sale",
            "date": "2026-03-19"
        }))
        .await;
    create_resp.assert_status(axum::http::StatusCode::CREATED);
    let created: serde_json::Value = create_resp.json();
    let deal_id = created["id"].as_str().unwrap();

    let delete_resp = server
        .delete(&format!("/api/deals/{deal_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;
    delete_resp.assert_status_ok();

    let reversal: serde_json::Value = sqlx::query_scalar(
        r#"SELECT to_jsonb(se)
           FROM sales_events se
           WHERE se.event_type = 'deal.reversal'
             AND se.metadata ->> 'deal_id' = $1
           ORDER BY se.created_at DESC
           LIMIT 1"#,
    )
    .bind(deal_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        reversal["metadata"]["reversal_of_event_type"],
        json!("deal.created")
    );
    assert!(
        reversal["metadata"]["reversal_of_event_id"]
            .as_str()
            .is_some(),
        "expected reversal_of_event_id in metadata"
    );
}
