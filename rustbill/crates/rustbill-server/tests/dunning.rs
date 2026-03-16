mod common;

use common::{
    create_admin_session, create_test_customer, create_test_plan, create_test_product,
    create_test_subscription, test_server,
};
use cookie::Cookie;
use sqlx::PgPool;

/// Helper: create an overdue invoice linked to a subscription, with due_at set N days in the past.
async fn create_overdue_invoice(
    pool: &PgPool,
    customer_id: &str,
    subscription_id: &str,
    days_overdue: i64,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let invoice_number = format!("INV-DUN-{}", &id[..8]);
    let now = chrono::Utc::now().naive_utc();
    let due_at = now - chrono::Duration::days(days_overdue);

    sqlx::query(
        r#"INSERT INTO invoices
           (id, invoice_number, customer_id, subscription_id, status, subtotal, tax, total,
            currency, due_at, version, created_at, updated_at)
           VALUES ($1, $2, $3, $4, 'issued'::invoice_status, 100, 0, 100, 'USD', $5, 1, $6, $6)"#,
    )
    .bind(&id)
    .bind(&invoice_number)
    .bind(customer_id)
    .bind(subscription_id)
    .bind(due_at)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert overdue invoice");

    id
}

// -----------------------------------------------------------------------
// Test 1: Invoice 5 days overdue → dunning produces a 'reminder' entry
// (default config: reminder_days = 3)
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn dunning_creates_reminder_for_5_day_overdue_invoice(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    let customer_id = create_test_customer(&pool).await;
    let product_id = create_test_product(&pool, "saas").await;
    let plan_id = create_test_plan(&pool, &product_id).await;
    let sub_id = create_test_subscription(&pool, &customer_id, &plan_id).await;
    let invoice_id = create_overdue_invoice(&pool, &customer_id, &sub_id, 5).await;

    // Trigger dunning via the cron endpoint
    let resp = server
        .post("/api/billing/cron/process-dunning")
        .add_cookie(Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["success"], true);

    // Verify dunning_log has a 'reminder' entry for this invoice
    let log: Vec<(String, String)> = sqlx::query_as(
        "SELECT invoice_id, step::text FROM dunning_log WHERE invoice_id = $1",
    )
    .bind(&invoice_id)
    .fetch_all(&pool)
    .await
    .expect("failed to query dunning_log");

    assert_eq!(log.len(), 1, "expected exactly one dunning log entry");
    assert_eq!(log[0].0, invoice_id);
    assert_eq!(log[0].1, "reminder");
}

// -----------------------------------------------------------------------
// Test 2: Invoice 8 days overdue → 'warning' step
// (default config: warning_days = 7)
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn dunning_creates_warning_for_8_day_overdue_invoice(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    let customer_id = create_test_customer(&pool).await;
    let product_id = create_test_product(&pool, "saas").await;
    let plan_id = create_test_plan(&pool, &product_id).await;
    let sub_id = create_test_subscription(&pool, &customer_id, &plan_id).await;
    let invoice_id = create_overdue_invoice(&pool, &customer_id, &sub_id, 8).await;

    let resp = server
        .post("/api/billing/cron/process-dunning")
        .add_cookie(Cookie::new("session", token))
        .await;

    resp.assert_status_ok();

    let log: Vec<(String, String)> = sqlx::query_as(
        "SELECT invoice_id, step::text FROM dunning_log WHERE invoice_id = $1",
    )
    .bind(&invoice_id)
    .fetch_all(&pool)
    .await
    .expect("failed to query dunning_log");

    assert_eq!(log.len(), 1, "expected exactly one dunning log entry");
    assert_eq!(log[0].1, "warning");
}

// -----------------------------------------------------------------------
// Test 3: Invoice 31 days overdue → 'suspension' step and subscription paused
// (default config: suspension_days = 30)
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn dunning_suspends_subscription_for_31_day_overdue_invoice(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    let customer_id = create_test_customer(&pool).await;
    let product_id = create_test_product(&pool, "saas").await;
    let plan_id = create_test_plan(&pool, &product_id).await;
    let sub_id = create_test_subscription(&pool, &customer_id, &plan_id).await;
    let invoice_id = create_overdue_invoice(&pool, &customer_id, &sub_id, 31).await;

    let resp = server
        .post("/api/billing/cron/process-dunning")
        .add_cookie(Cookie::new("session", token))
        .await;

    resp.assert_status_ok();

    // Check dunning_log has 'suspension' step
    let step: (String,) = sqlx::query_as(
        "SELECT step::text FROM dunning_log WHERE invoice_id = $1",
    )
    .bind(&invoice_id)
    .fetch_one(&pool)
    .await
    .expect("failed to query dunning_log");

    assert_eq!(step.0, "suspension");

    // Verify subscription is now 'paused'
    let sub_status: (String,) = sqlx::query_as(
        "SELECT status::text FROM subscriptions WHERE id = $1",
    )
    .bind(&sub_id)
    .fetch_one(&pool)
    .await
    .expect("failed to query subscription");

    assert_eq!(sub_status.0, "paused");
}

// -----------------------------------------------------------------------
// Test 4: Running dunning twice does not create duplicate log entries
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn dunning_run_twice_no_duplicates(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    let customer_id = create_test_customer(&pool).await;
    let product_id = create_test_product(&pool, "saas").await;
    let plan_id = create_test_plan(&pool, &product_id).await;
    let sub_id = create_test_subscription(&pool, &customer_id, &plan_id).await;
    let invoice_id = create_overdue_invoice(&pool, &customer_id, &sub_id, 5).await;

    // First run
    let resp = server
        .post("/api/billing/cron/process-dunning")
        .add_cookie(Cookie::new("session", token.clone()))
        .await;
    resp.assert_status_ok();

    // Second run
    let resp = server
        .post("/api/billing/cron/process-dunning")
        .add_cookie(Cookie::new("session", token))
        .await;
    resp.assert_status_ok();

    // Should still have only one log entry for this invoice + step
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM dunning_log WHERE invoice_id = $1",
    )
    .bind(&invoice_id)
    .fetch_one(&pool)
    .await
    .expect("failed to count dunning_log");

    assert_eq!(count.0, 1, "expected no duplicate dunning entries");
}
