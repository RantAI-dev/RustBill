mod common;

use axum::http::{HeaderName, HeaderValue};
use common::{
    create_test_api_key, create_test_api_key_for_customer, create_test_customer, create_test_plan,
    create_test_product, create_test_subscription, test_server,
};
use rust_decimal::Decimal;
use sqlx::PgPool;

// -----------------------------------------------------------------------
// Test 1: V1 request without API key → 401
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn v1_request_without_api_key_returns_401(pool: PgPool) {
    let server = test_server(pool.clone()).await;

    let resp = server.get("/api/v1/products").await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// -----------------------------------------------------------------------
// Test 2: V1 request with valid API key → 200
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn v1_request_with_valid_api_key_returns_200(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let (_id, key) = create_test_api_key(&pool).await;

    let resp = server
        .get("/api/v1/products")
        .add_header(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&format!("Bearer {}", key)).unwrap(),
        )
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    // Empty list is fine — we just want to confirm auth passed
    assert!(body.is_empty() || body.len() > 0);
}

// -----------------------------------------------------------------------
// Test 3: V1 products list returns seeded products
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn v1_products_list(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let (_id, key) = create_test_api_key(&pool).await;

    // Seed two products
    let p1 = create_test_product(&pool, "saas").await;
    let p2 = create_test_product(&pool, "licensed").await;

    let resp = server
        .get("/api/v1/products")
        .add_header(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&format!("Bearer {}", key)).unwrap(),
        )
        .await;

    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert!(
        body.len() >= 2,
        "expected at least 2 products, got {}",
        body.len()
    );

    // Verify our products are in the list
    let ids: Vec<&str> = body.iter().filter_map(|v| v["id"].as_str()).collect();
    assert!(ids.contains(&p1.as_str()), "product p1 not found in list");
    assert!(ids.contains(&p2.as_str()), "product p2 not found in list");
}

// -----------------------------------------------------------------------
// Test 4: V1 licenses CRUD cycle — create, read, update, delete
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn v1_licenses_crud_cycle(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let (_id, key) = create_test_api_key(&pool).await;

    let customer_id = create_test_customer(&pool).await;
    let product_id = create_test_product(&pool, "licensed").await;

    let license_key = format!(
        "TESTLIC-{}",
        uuid::Uuid::new_v4().to_string()[..8].to_uppercase()
    );

    let auth_header = format!("Bearer {}", key);
    let auth_name = HeaderName::from_static("authorization");

    // CREATE
    let create_body = serde_json::json!({
        "key": license_key,
        "productId": product_id,
        "customerId": customer_id,
        "maxActivations": 5
    });

    let resp = server
        .post("/api/v1/licenses")
        .add_header(
            auth_name.clone(),
            HeaderValue::from_str(&auth_header).unwrap(),
        )
        .json(&create_body)
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let created: serde_json::Value = resp.json();
    assert_eq!(created["key"].as_str(), Some(license_key.as_str()));

    // READ
    let resp = server
        .get(&format!("/api/v1/licenses/{}", license_key))
        .add_header(
            auth_name.clone(),
            HeaderValue::from_str(&auth_header).unwrap(),
        )
        .await;

    resp.assert_status_ok();
    let fetched: serde_json::Value = resp.json();
    assert_eq!(fetched["key"].as_str(), Some(license_key.as_str()));

    // UPDATE — change max activations
    let update_body = serde_json::json!({
        "maxActivations": 10
    });

    let resp = server
        .put(&format!("/api/v1/licenses/{}", license_key))
        .add_header(
            auth_name.clone(),
            HeaderValue::from_str(&auth_header).unwrap(),
        )
        .json(&update_body)
        .await;

    resp.assert_status_ok();
    let updated: serde_json::Value = resp.json();
    assert_eq!(updated["max_activations"], 10);

    // DELETE
    let resp = server
        .delete(&format!("/api/v1/licenses/{}", license_key))
        .add_header(
            auth_name.clone(),
            HeaderValue::from_str(&auth_header).unwrap(),
        )
        .await;

    resp.assert_status_ok();
    let deleted: serde_json::Value = resp.json();
    assert_eq!(deleted["success"], true);

    // Verify it's gone
    let resp = server
        .get(&format!("/api/v1/licenses/{}", license_key))
        .add_header(
            auth_name.clone(),
            HeaderValue::from_str(&auth_header).unwrap(),
        )
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}

// -----------------------------------------------------------------------
// Test 5: V1 usage batch POST — record multiple usage events
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn v1_usage_batch_post(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let (_id, key) = create_test_api_key(&pool).await;

    let customer_id = create_test_customer(&pool).await;
    let product_id = create_test_product(&pool, "api").await;
    let plan_id = create_test_plan(&pool, &product_id).await;
    let sub_id = create_test_subscription(&pool, &customer_id, &plan_id).await;

    let auth_header = format!("Bearer {}", key);
    let auth_name = HeaderName::from_static("authorization");

    // Send a batch of 3 usage events
    let batch_body = serde_json::json!([
        { "subscriptionId": sub_id, "metricName": "api_calls", "value": 100 },
        { "subscriptionId": sub_id, "metricName": "api_calls", "value": 200 },
        { "subscriptionId": sub_id, "metricName": "storage_mb", "value": 50 },
    ]);

    let resp = server
        .post("/api/v1/billing/usage")
        .add_header(
            auth_name.clone(),
            HeaderValue::from_str(&auth_header).unwrap(),
        )
        .json(&batch_body)
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = resp.json();

    // Batch response should be an array of 3 items
    assert!(body.is_array(), "expected array response for batch usage");
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 3, "expected 3 usage records in response");

    // Verify each record has a subscription_id matching our subscription
    for record in arr {
        assert_eq!(record["subscription_id"].as_str(), Some(sub_id.as_str()));
    }

    // Verify via a GET that the records are persisted
    let resp = server
        .get(&format!("/api/v1/billing/usage?subscriptionId={}", sub_id))
        .add_header(
            auth_name.clone(),
            HeaderValue::from_str(&auth_header).unwrap(),
        )
        .await;

    resp.assert_status_ok();
    let all_usage: Vec<serde_json::Value> = resp.json();
    assert_eq!(all_usage.len(), 3, "expected 3 persisted usage records");
}

// -----------------------------------------------------------------------
// Test 6: V1 billing credits requires customer-scoped API key
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn v1_billing_credits_requires_scoped_key(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let (_id, key) = create_test_api_key(&pool).await; // unscoped

    let resp = server
        .get("/api/v1/billing/credits")
        .add_header(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&format!("Bearer {}", key)).unwrap(),
        )
        .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// -----------------------------------------------------------------------
// Test 7: V1 billing credits uses customer scope from API key
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn v1_billing_credits_scoped_key_reads_own_customer(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let customer_id = create_test_customer(&pool).await;
    let (_id, key) = create_test_api_key_for_customer(&pool, Some(&customer_id)).await;

    rustbill_core::billing::credits::deposit(
        &pool,
        &customer_id,
        "USD",
        Decimal::new(750, 2),
        rustbill_core::db::models::CreditReason::Manual,
        "seed",
        None,
    )
    .await
    .unwrap();

    let resp = server
        .get("/api/v1/billing/credits")
        .add_header(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&format!("Bearer {}", key)).unwrap(),
        )
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["currency"], "USD");
    assert_eq!(body["balance"], "7.50");
}
