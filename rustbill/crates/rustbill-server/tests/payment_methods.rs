mod common;

use common::{create_admin_session, create_test_customer, test_server};
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn test_create_payment_method_first_is_default(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let customer_id = create_test_customer(&pool).await;

    // Create first payment method — should auto-set as default
    let resp = server
        .post("/api/billing/payment-methods")
        .json(&serde_json::json!({
            "customerId": customer_id,
            "provider": "stripe",
            "providerToken": "pm_test_123",
            "methodType": "card",
            "label": "Visa ending 4242",
            "lastFour": "4242",
            "expiryMonth": 12,
            "expiryYear": 2028
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let pm: serde_json::Value = resp.json();
    assert_eq!(pm["is_default"], true);
    assert_eq!(pm["provider"], "stripe");
    assert_eq!(pm["last_four"], "4242");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_list_payment_methods(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let customer_id = create_test_customer(&pool).await;

    // Create two methods
    server
        .post("/api/billing/payment-methods")
        .json(&serde_json::json!({
            "customerId": customer_id,
            "provider": "stripe",
            "providerToken": "pm_1",
            "methodType": "card",
            "label": "Visa 4242"
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await
        .assert_status_ok();

    server
        .post("/api/billing/payment-methods")
        .json(&serde_json::json!({
            "customerId": customer_id,
            "provider": "xendit",
            "providerToken": "tok_2",
            "methodType": "ewallet",
            "label": "OVO"
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await
        .assert_status_ok();

    // List
    let resp = server
        .get(&format!(
            "/api/billing/payment-methods?customerId={customer_id}"
        ))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let methods: Vec<serde_json::Value> = resp.json();
    assert_eq!(methods.len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_set_default_payment_method(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let customer_id = create_test_customer(&pool).await;

    // Create first (auto-default)
    let resp = server
        .post("/api/billing/payment-methods")
        .json(&serde_json::json!({
            "customerId": customer_id,
            "provider": "stripe",
            "providerToken": "pm_1",
            "methodType": "card",
            "label": "Card 1"
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let pm1: serde_json::Value = resp.json();
    assert_eq!(pm1["is_default"], true);

    // Create second (not default)
    let resp = server
        .post("/api/billing/payment-methods")
        .json(&serde_json::json!({
            "customerId": customer_id,
            "provider": "xendit",
            "providerToken": "tok_2",
            "methodType": "card",
            "label": "Card 2"
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let pm2: serde_json::Value = resp.json();
    let pm2_id = pm2["id"].as_str().unwrap();
    assert_eq!(pm2["is_default"], false);

    // Set second as default
    let resp = server
        .post(&format!(
            "/api/billing/payment-methods/{pm2_id}/default?customerId={customer_id}"
        ))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let updated: serde_json::Value = resp.json();
    assert_eq!(updated["is_default"], true);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_delete_payment_method(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let customer_id = create_test_customer(&pool).await;

    // Create
    let resp = server
        .post("/api/billing/payment-methods")
        .json(&serde_json::json!({
            "customerId": customer_id,
            "provider": "stripe",
            "providerToken": "pm_del",
            "methodType": "card",
            "label": "Delete me"
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let pm: serde_json::Value = resp.json();
    let pm_id = pm["id"].as_str().unwrap();

    // Delete
    let resp = server
        .delete(&format!(
            "/api/billing/payment-methods/{pm_id}?customerId={customer_id}"
        ))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();

    // List should be empty
    let resp = server
        .get(&format!(
            "/api/billing/payment-methods?customerId={customer_id}"
        ))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let methods: Vec<serde_json::Value> = resp.json();
    assert_eq!(methods.len(), 0);
}
