mod common;

use common::{create_admin_session, test_server};
use serde_json::json;
use sqlx::PgPool;

/// Helper: create customer with billing_country/billing_state for tax lookup (direct SQL)
async fn create_customer_with_billing(
    pool: &PgPool,
    country: &str,
    state: Option<&str>,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO customers
           (id, name, industry, tier, location, contact, email, phone,
            total_revenue, health_score, trend, last_contact,
            billing_country, billing_state, created_at, updated_at)
           VALUES ($1, $2, 'Technology', 'enterprise', $3, 'Jane',
                   $4, '+1-555-0100', 0, 80, 'stable', '', $3, $5, $6, $6)"#,
    )
    .bind(&id)
    .bind(format!("Pipeline Test Co {}", &id[..8]))
    .bind(country)
    .bind(format!("jane-{}@pipeline-test.com", &id[..8]))
    .bind(state)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert customer with billing info");

    id
}

/// Helper: create a flat plan with given base price (direct SQL)
async fn create_flat_plan(pool: &PgPool, name: &str, price: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let product_id = common::create_test_product(pool, "saas").await;
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO pricing_plans
           (id, product_id, name, pricing_model, billing_cycle, base_price, trial_days, active, created_at, updated_at)
           VALUES ($1, $2, $3, 'flat'::pricing_model, 'monthly'::billing_cycle, $4::numeric, 0, true, $5, $5)"#,
    )
    .bind(&id)
    .bind(&product_id)
    .bind(name)
    .bind(price)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert pricing plan");

    id
}

/// Helper: create an active subscription (direct SQL)
async fn create_subscription(pool: &PgPool, customer_id: &str, plan_id: &str) -> String {
    common::create_test_subscription(pool, customer_id, plan_id).await
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_lifecycle_generates_invoice_with_tax(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    // US/CA customer — seed data has 7.25% exclusive Sales Tax
    let customer_id = create_customer_with_billing(&pool, "US", Some("CA")).await;
    let plan_id = create_flat_plan(&pool, "Pro Plan", "100.00").await;
    let _sub_id = create_subscription(&pool, &customer_id, &plan_id).await;

    // Advance subscription period_end to past so lifecycle picks it up
    sqlx::query(
        "UPDATE subscriptions SET current_period_end = NOW() - INTERVAL '1 hour' WHERE customer_id = $1",
    )
    .bind(&customer_id)
    .execute(&pool)
    .await
    .unwrap();

    // Trigger lifecycle via the cron endpoint
    let resp = server
        .post("/api/billing/cron/renew-subscriptions")
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["success"], json!(true));
    assert!(body["renewed"].as_u64().unwrap() >= 1);

    // Check invoice was created with tax fields
    let inv: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(i) FROM invoices i WHERE customer_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&customer_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(inv["tax_name"].as_str().unwrap(), "Sales Tax");
    assert_eq!(inv["tax_inclusive"], false);
    // amount_due should equal total (no credits applied)
    assert!(inv["amount_due"].is_string() || inv["amount_due"].is_number());
    // subtotal should be 100.00
    // tax should be 7.25 (100 * 0.0725)
    // total should be 107.25 (exclusive)
    let subtotal = parse_decimal_value(&inv["subtotal"]);
    let tax = parse_decimal_value(&inv["tax"]);
    let total = parse_decimal_value(&inv["total"]);
    assert_eq!(subtotal, rust_decimal::Decimal::new(10000, 2)); // 100.00
    assert_eq!(tax, rust_decimal::Decimal::new(725, 2)); // 7.25
    assert_eq!(total, rust_decimal::Decimal::new(10725, 2)); // 107.25
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_credits_applied_to_invoice(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    let customer_id = create_customer_with_billing(&pool, "US", Some("CA")).await;
    let plan_id = create_flat_plan(&pool, "Pro Plan", "100.00").await;
    let _sub_id = create_subscription(&pool, &customer_id, &plan_id).await;

    // Add $25 credit
    let resp = server
        .post("/api/billing/credits/adjust")
        .json(&json!({
            "customerId": customer_id,
            "currency": "USD",
            "amount": "25.00",
            "description": "Test credit"
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();

    // Advance subscription period_end to past
    sqlx::query(
        "UPDATE subscriptions SET current_period_end = NOW() - INTERVAL '1 hour' WHERE customer_id = $1",
    )
    .bind(&customer_id)
    .execute(&pool)
    .await
    .unwrap();

    // Trigger lifecycle
    let resp = server
        .post("/api/billing/cron/renew-subscriptions")
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();

    // Check invoice
    let inv: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(i) FROM invoices i WHERE customer_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&customer_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    // credits_applied should be 25.00
    let credits_applied = parse_decimal_value(&inv["credits_applied"]);
    assert_eq!(credits_applied, rust_decimal::Decimal::new(2500, 2));

    // total is 107.25 (100 + 7.25% tax), amount_due = 107.25 - 25.00 = 82.25
    let amount_due = parse_decimal_value(&inv["amount_due"]);
    assert_eq!(amount_due, rust_decimal::Decimal::new(8225, 2));

    // Check credit balance is now 0
    let balance: rust_decimal::Decimal = sqlx::query_scalar(
        "SELECT COALESCE(balance, 0) FROM customer_credit_balances WHERE customer_id = $1 AND currency = 'USD'",
    )
    .bind(&customer_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(balance, rust_decimal::Decimal::ZERO);
}

/// Parse a JSON value that could be a string or number into a Decimal.
fn parse_decimal_value(val: &serde_json::Value) -> rust_decimal::Decimal {
    use std::str::FromStr;
    match val {
        serde_json::Value::String(s) => rust_decimal::Decimal::from_str(s).unwrap(),
        serde_json::Value::Number(n) => {
            rust_decimal::Decimal::from_str(&n.to_string()).unwrap()
        }
        _ => panic!("Expected string or number, got: {:?}", val),
    }
}
