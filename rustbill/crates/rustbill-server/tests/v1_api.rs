mod common;

use axum::http::{HeaderName, HeaderValue};
use common::{
    create_test_api_key, create_test_api_key_for_customer, create_test_customer,
    create_test_invoice, create_test_plan, create_test_product, create_test_subscription,
    test_server,
};
use rust_decimal::Decimal;
use sqlx::PgPool;

fn bearer_auth(key: &str) -> HeaderValue {
    HeaderValue::from_str(&format!("Bearer {}", key)).unwrap()
}

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
    let _body: Vec<serde_json::Value> = resp.json();
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

#[sqlx::test(migrations = "../../migrations")]
async fn v1_deals_routes_include_legacy_deprecation_headers(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let (_id, key) = create_test_api_key(&pool).await;

    let resp = server
        .get("/api/v1/deals")
        .add_header(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&format!("Bearer {}", key)).unwrap(),
        )
        .await;

    resp.assert_status_ok();
    assert_eq!(resp.header("deprecation").to_str().unwrap_or(""), "true");
    assert_eq!(
        resp.header("x-rustbill-legacy").to_str().unwrap_or(""),
        "deals"
    );
    assert!(
        resp.header("link")
            .to_str()
            .unwrap_or("")
            .contains("/api/v1/billing/subscriptions"),
        "expected v1 successor link header"
    );
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

// -----------------------------------------------------------------------
// Test 8: V1 products get-one returns seeded product
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn v1_products_get_one(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let (_id, key) = create_test_api_key(&pool).await;
    let product_id = create_test_product(&pool, "api").await;

    let resp = server
        .get(&format!("/api/v1/products/{product_id}"))
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["id"].as_str(), Some(product_id.as_str()));
    assert_eq!(body["product_type"].as_str(), Some("api"));
}

// -----------------------------------------------------------------------
// Test 9: V1 billing payment-methods CRUD-ish flow with scoped API key
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn v1_billing_payment_methods_crud_with_scoped_key(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let customer_id = create_test_customer(&pool).await;
    let (_id, key) = create_test_api_key_for_customer(&pool, Some(&customer_id)).await;

    let other_customer_id = create_test_customer(&pool).await;
    let (_other_key_id, other_key) =
        create_test_api_key_for_customer(&pool, Some(&other_customer_id)).await;

    let first_resp = server
        .post("/api/v1/billing/payment-methods")
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .json(&serde_json::json!({
            "customerId": customer_id,
            "provider": "stripe",
            "providerToken": "pm_scoped_1",
            "methodType": "card",
            "label": "Visa ending 4242",
            "lastFour": "4242",
            "expiryMonth": 12,
            "expiryYear": 2028
        }))
        .await;

    first_resp.assert_status_ok();
    let first: serde_json::Value = first_resp.json();
    let first_id = first["id"].as_str().unwrap().to_string();
    assert_eq!(first["customer_id"].as_str(), Some(customer_id.as_str()));
    assert_eq!(first["provider"].as_str(), Some("stripe"));
    assert_eq!(first["is_default"], true);

    let other_resp = server
        .post("/api/v1/billing/payment-methods")
        .add_header(
            HeaderName::from_static("authorization"),
            bearer_auth(&other_key),
        )
        .json(&serde_json::json!({
            "customerId": other_customer_id,
            "provider": "stripe",
            "providerToken": "pm_other_1",
            "methodType": "card",
            "label": "Other customer card",
            "lastFour": "1111",
            "expiryMonth": 1,
            "expiryYear": 2029
        }))
        .await;

    other_resp.assert_status_ok();
    let other_method: serde_json::Value = other_resp.json();
    let other_method_id = other_method["id"].as_str().unwrap().to_string();

    let second_resp = server
        .post("/api/v1/billing/payment-methods")
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .json(&serde_json::json!({
            "customerId": customer_id,
            "provider": "xendit",
            "providerToken": "pm_scoped_2",
            "methodType": "card",
            "label": "Visa ending 5454",
            "lastFour": "5454",
            "expiryMonth": 6,
            "expiryYear": 2029
        }))
        .await;

    second_resp.assert_status_ok();
    let second: serde_json::Value = second_resp.json();
    let second_id = second["id"].as_str().unwrap().to_string();
    assert_eq!(second["customer_id"].as_str(), Some(customer_id.as_str()));
    assert_eq!(second["provider"].as_str(), Some("xendit"));
    assert_eq!(second["is_default"], false);

    let list_resp = server
        .get("/api/v1/billing/payment-methods")
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .await;

    list_resp.assert_status_ok();
    let methods: Vec<serde_json::Value> = list_resp.json();
    assert_eq!(methods.len(), 2);
    let method_ids: Vec<&str> = methods
        .iter()
        .filter_map(|value| value["id"].as_str())
        .collect();
    assert!(method_ids.contains(&first_id.as_str()));
    assert!(method_ids.contains(&second_id.as_str()));
    assert!(methods
        .iter()
        .all(|value| value["customer_id"].as_str() == Some(customer_id.as_str())));

    let default_resp = server
        .post(&format!(
            "/api/v1/billing/payment-methods/{second_id}/default"
        ))
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .await;

    default_resp.assert_status_ok();
    let defaulted: serde_json::Value = default_resp.json();
    assert_eq!(defaulted["id"].as_str(), Some(second_id.as_str()));
    assert_eq!(defaulted["is_default"], true);

    let list_resp = server
        .get("/api/v1/billing/payment-methods")
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .await;

    list_resp.assert_status_ok();
    let methods: Vec<serde_json::Value> = list_resp.json();
    let default_flags: Vec<(&str, bool)> = methods
        .iter()
        .filter_map(|value| {
            Some((
                value["id"].as_str()?,
                value["is_default"].as_bool().unwrap_or(false),
            ))
        })
        .collect();
    assert_eq!(default_flags.len(), 2);
    assert!(default_flags.contains(&(second_id.as_str(), true)));
    assert!(default_flags.contains(&(first_id.as_str(), false)));

    let delete_resp = server
        .delete(&format!("/api/v1/billing/payment-methods/{second_id}"))
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .await;

    delete_resp.assert_status_ok();
    let deleted: serde_json::Value = delete_resp.json();
    assert_eq!(deleted["deleted"], true);

    let list_resp = server
        .get("/api/v1/billing/payment-methods")
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .await;

    list_resp.assert_status_ok();
    let methods: Vec<serde_json::Value> = list_resp.json();
    assert_eq!(methods.len(), 1);
    assert_eq!(methods[0]["id"].as_str(), Some(first_id.as_str()));
    assert_ne!(methods[0]["id"].as_str(), Some(other_method_id.as_str()));
}

// -----------------------------------------------------------------------
// Test 10: V1 billing payment-methods setup surfaces provider-not-configured
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn v1_billing_payment_methods_setup_provider_not_configured(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let customer_id = create_test_customer(&pool).await;
    let (_id, key) = create_test_api_key_for_customer(&pool, Some(&customer_id)).await;

    let resp = server
        .post("/api/v1/billing/payment-methods/setup")
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .json(&serde_json::json!({
            "customerId": customer_id,
            "provider": "stripe"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["error"].as_str(), Some("stripe is not configured"));
}

// -----------------------------------------------------------------------
// Test 11: V1 billing invoices list/get under scoped API key
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn v1_billing_invoices_list_and_get_with_scoped_key(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let customer_id = create_test_customer(&pool).await;
    let (_id, key) = create_test_api_key_for_customer(&pool, Some(&customer_id)).await;

    let other_customer_id = create_test_customer(&pool).await;
    let _other_invoice_id = create_test_invoice(&pool, &other_customer_id).await;

    let invoice_id = create_test_invoice(&pool, &customer_id).await;

    let list_resp = server
        .get(&format!(
            "/api/v1/billing/invoices?customerId={customer_id}"
        ))
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .await;

    list_resp.assert_status_ok();
    let invoices: Vec<serde_json::Value> = list_resp.json();
    assert_eq!(invoices.len(), 1);
    assert_eq!(invoices[0]["id"].as_str(), Some(invoice_id.as_str()));
    assert_eq!(
        invoices[0]["customer_id"].as_str(),
        Some(customer_id.as_str())
    );

    let get_resp = server
        .get(&format!("/api/v1/billing/invoices/{invoice_id}"))
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .await;

    get_resp.assert_status_ok();
    let invoice: serde_json::Value = get_resp.json();
    assert_eq!(invoice["id"].as_str(), Some(invoice_id.as_str()));
    assert_eq!(invoice["customer_id"].as_str(), Some(customer_id.as_str()));
    assert!(invoice["items"].is_array());
    assert!(invoice["payments"].is_array());
}

// -----------------------------------------------------------------------
// Test 12: V1 billing subscriptions list/create/update/change-plan under scoped key
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn v1_billing_subscriptions_lifecycle_with_scoped_key(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let customer_id = create_test_customer(&pool).await;
    let (_id, key) = create_test_api_key_for_customer(&pool, Some(&customer_id)).await;

    let product_id = create_test_product(&pool, "saas").await;
    let first_plan_id = create_test_plan(&pool, &product_id).await;
    let second_plan_id = create_test_plan(&pool, &product_id).await;

    let create_resp = server
        .post("/api/v1/billing/subscriptions")
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .json(&serde_json::json!({
            "customerId": customer_id,
            "planId": first_plan_id,
            "quantity": 2,
            "metadata": {
                "source": "v1",
                "step": "created"
            }
        }))
        .await;

    create_resp.assert_status(axum::http::StatusCode::CREATED);
    let created: serde_json::Value = create_resp.json();
    let subscription_id = created["id"].as_str().unwrap().to_string();
    assert_eq!(created["customer_id"].as_str(), Some(customer_id.as_str()));
    assert_eq!(created["plan_id"].as_str(), Some(first_plan_id.as_str()));
    assert_eq!(created["quantity"].as_i64(), Some(2));
    assert_eq!(created["metadata"]["source"].as_str(), Some("v1"));

    let list_resp = server
        .get(&format!(
            "/api/v1/billing/subscriptions?customerId={customer_id}"
        ))
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .await;

    list_resp.assert_status_ok();
    let subscriptions: Vec<serde_json::Value> = list_resp.json();
    assert_eq!(subscriptions.len(), 1);
    assert_eq!(
        subscriptions[0]["id"].as_str(),
        Some(subscription_id.as_str())
    );
    assert_eq!(
        subscriptions[0]["customer_id"].as_str(),
        Some(customer_id.as_str())
    );

    let update_resp = server
        .put(&format!("/api/v1/billing/subscriptions/{subscription_id}"))
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .json(&serde_json::json!({
            "metadata": {
                "source": "v1",
                "step": "updated"
            }
        }))
        .await;

    update_resp.assert_status_ok();
    let updated: serde_json::Value = update_resp.json();
    assert_eq!(updated["id"].as_str(), Some(subscription_id.as_str()));
    assert_eq!(updated["metadata"]["step"].as_str(), Some("updated"));

    let change_resp = server
        .post(&format!(
            "/api/v1/billing/subscriptions/{subscription_id}/change-plan"
        ))
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .json(&serde_json::json!({
            "planId": second_plan_id,
            "quantity": 2
        }))
        .await;

    change_resp.assert_status_ok();
    let changed: serde_json::Value = change_resp.json();
    assert_eq!(changed["id"].as_str(), Some(subscription_id.as_str()));
    assert_eq!(changed["plan_id"].as_str(), Some(second_plan_id.as_str()));
    assert_eq!(changed["customer_id"].as_str(), Some(customer_id.as_str()));

    let list_resp = server
        .get(&format!(
            "/api/v1/billing/subscriptions?customerId={customer_id}"
        ))
        .add_header(HeaderName::from_static("authorization"), bearer_auth(&key))
        .await;

    list_resp.assert_status_ok();
    let subscriptions: Vec<serde_json::Value> = list_resp.json();
    assert_eq!(subscriptions.len(), 1);
    assert_eq!(
        subscriptions[0]["id"].as_str(),
        Some(subscription_id.as_str())
    );
    assert_eq!(
        subscriptions[0]["plan_id"].as_str(),
        Some(second_plan_id.as_str())
    );
}
