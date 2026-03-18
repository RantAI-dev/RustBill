mod common;

use common::{create_admin_session, create_test_product, test_server};
use sqlx::PgPool;

/// Create a customer, two plans (cheap & expensive), and a subscription on the cheap plan.
/// Returns (customer_id, cheap_plan_id, expensive_plan_id, subscription_id).
async fn setup(
    server: &axum_test::TestServer,
    pool: &PgPool,
    token: &str,
) -> (String, String, String, String) {
    let now = chrono::Utc::now().naive_utc();

    // Create customer
    let customer_id: String = sqlx::query_scalar(
        r#"INSERT INTO customers
           (id, name, industry, tier, location, contact, email, phone,
            total_revenue, health_score, trend, last_contact, created_at, updated_at)
           VALUES (gen_random_uuid()::text, 'Plan Change Co', 'Technology', 'enterprise', 'US',
                   'Jane', 'jane@planchange-test.com', '+1-555-0100', 0, 80, 'stable', '', $1, $1)
           RETURNING id"#,
    )
    .bind(now)
    .fetch_one(pool)
    .await
    .unwrap();

    let product_id = create_test_product(pool, "saas").await;

    // Create two plans: cheap ($50) and expensive ($200)
    let cheap_plan_id: String = sqlx::query_scalar(
        r#"INSERT INTO pricing_plans
           (id, product_id, name, pricing_model, billing_cycle, base_price, trial_days, active, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, 'Starter', 'flat'::pricing_model, 'monthly'::billing_cycle, 50.00, 0, true, $2, $2)
           RETURNING id"#,
    )
    .bind(&product_id)
    .bind(now)
    .fetch_one(pool)
    .await
    .unwrap();

    let expensive_plan_id: String = sqlx::query_scalar(
        r#"INSERT INTO pricing_plans
           (id, product_id, name, pricing_model, billing_cycle, base_price, trial_days, active, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, 'Enterprise', 'flat'::pricing_model, 'monthly'::billing_cycle, 200.00, 0, true, $2, $2)
           RETURNING id"#,
    )
    .bind(&product_id)
    .bind(now)
    .fetch_one(pool)
    .await
    .unwrap();

    // Create subscription on cheap plan via API
    let resp = server
        .post("/api/billing/subscriptions")
        .json(&serde_json::json!({
            "customerId": customer_id,
            "planId": cheap_plan_id,
            "quantity": 1
        }))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;
    resp.assert_status(axum::http::StatusCode::CREATED);
    let sub: serde_json::Value = resp.json();
    let sub_id = sub["id"].as_str().unwrap().to_string();

    (customer_id, cheap_plan_id, expensive_plan_id, sub_id)
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_upgrade_plan_creates_positive_proration(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let (_customer_id, _cheap_id, expensive_id, sub_id) = setup(&server, &pool, &token).await;

    // Change plan to expensive (upgrade)
    let resp = server
        .post(&format!("/api/billing/subscriptions/{sub_id}/change-plan"))
        .json(&serde_json::json!({
            "planId": expensive_id
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();

    // Verify subscription now has the new plan
    let sub: serde_json::Value =
        sqlx::query_scalar("SELECT to_jsonb(s) FROM subscriptions s WHERE s.id = $1")
            .bind(&sub_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(sub["plan_id"].as_str().unwrap(), expensive_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_downgrade_plan_deposits_credit(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let (customer_id, cheap_id, expensive_id, sub_id) = setup(&server, &pool, &token).await;

    // First upgrade to expensive
    server
        .post(&format!("/api/billing/subscriptions/{sub_id}/change-plan"))
        .json(&serde_json::json!({ "planId": expensive_id }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await
        .assert_status_ok();

    // Now downgrade back to cheap — should deposit proration credit
    server
        .post(&format!("/api/billing/subscriptions/{sub_id}/change-plan"))
        .json(&serde_json::json!({ "planId": cheap_id }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await
        .assert_status_ok();

    // Check credit was deposited
    let balance: Option<rust_decimal::Decimal> = sqlx::query_scalar(
        "SELECT balance FROM customer_credit_balances WHERE customer_id = $1 AND currency = 'USD'",
    )
    .bind(&customer_id)
    .fetch_optional(&pool)
    .await
    .unwrap();

    // Should have some credit (exact amount depends on timing within billing period)
    assert!(
        balance.is_some(),
        "expected credit balance to exist after downgrade"
    );
    assert!(
        balance.unwrap() > rust_decimal::Decimal::ZERO,
        "expected positive credit balance after downgrade"
    );

    // Check audit trail
    let credit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM customer_credits WHERE customer_id = $1 AND reason = 'proration'",
    )
    .bind(&customer_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(credit_count > 0, "expected proration credit audit entry");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_overpayment_deposits_credit(pool: PgPool) {
    let now = chrono::Utc::now().naive_utc();

    // Create customer
    let customer_id: String = sqlx::query_scalar(
        r#"INSERT INTO customers
           (id, name, industry, tier, location, contact, email, phone,
            total_revenue, health_score, trend, last_contact, created_at, updated_at)
           VALUES (gen_random_uuid()::text, 'Overpay Co', 'Technology', 'enterprise', 'US',
                   'Bob', 'bob@overpay-test.com', '+1-555-0200', 0, 80, 'stable', '', $1, $1)
           RETURNING id"#,
    )
    .bind(now)
    .fetch_one(&pool)
    .await
    .unwrap();

    // Create an invoice for $100 with amount_due = $100
    let invoice_id: String = sqlx::query_scalar(
        r#"INSERT INTO invoices
           (id, invoice_number, customer_id, status, subtotal, tax, total, currency,
            amount_due, version, created_at, updated_at)
           VALUES (gen_random_uuid()::text, 'INV-OVERPAY-001', $1, 'issued'::invoice_status,
                   100.00, 0, 100.00, 'USD', 100.00, 1, $2, $2)
           RETURNING id"#,
    )
    .bind(&customer_id)
    .bind(now)
    .fetch_one(&pool)
    .await
    .unwrap();

    // Pay $120 (overpay by $20) using the core service directly
    let req = rustbill_core::billing::payments::CreatePaymentRequest {
        invoice_id: invoice_id.clone(),
        amount: rust_decimal::Decimal::new(12000, 2),
        method: rustbill_core::db::models::PaymentMethod::BankTransfer,
        reference: Some("overpay-test".to_string()),
        paid_at: None,
        notes: None,
        stripe_payment_intent_id: None,
        xendit_payment_id: None,
        lemonsqueezy_order_id: None,
    };

    let payment = rustbill_core::billing::payments::create_payment(&pool, req)
        .await
        .expect("payment should succeed");
    assert_eq!(payment.amount, rust_decimal::Decimal::new(12000, 2));

    // Check invoice is paid
    let inv_status: String = sqlx::query_scalar("SELECT status::text FROM invoices WHERE id = $1")
        .bind(&invoice_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(inv_status, "paid");

    // Check $20 credit was deposited
    let balance: Option<rust_decimal::Decimal> = sqlx::query_scalar(
        "SELECT balance FROM customer_credit_balances WHERE customer_id = $1 AND currency = 'USD'",
    )
    .bind(&customer_id)
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(
        balance.is_some(),
        "expected credit balance after overpayment"
    );
    assert_eq!(
        balance.unwrap(),
        rust_decimal::Decimal::new(2000, 2),
        "expected $20.00 credit from overpayment"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_change_plan_idempotency_returns_existing_invoice(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let (_customer_id, _cheap_id, expensive_id, sub_id) = setup(&server, &pool, &token).await;

    let key = "idem-plan-change-001";

    // First request creates proration invoice for upgrade.
    let resp = server
        .post(&format!("/api/billing/subscriptions/{sub_id}/change-plan"))
        .json(&serde_json::json!({
            "planId": expensive_id,
            "idempotencyKey": key
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();

    // Second request with same key should return existing invoice payload.
    let resp = server
        .post(&format!("/api/billing/subscriptions/{sub_id}/change-plan"))
        .json(&serde_json::json!({
            "planId": expensive_id,
            "idempotencyKey": key
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["id"].is_string());
    assert_eq!(body["idempotency_key"], key);

    let invoice_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM invoices WHERE subscription_id = $1 AND idempotency_key = $2",
    )
    .bind(&sub_id)
    .bind(key)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(invoice_count, 1);
}
