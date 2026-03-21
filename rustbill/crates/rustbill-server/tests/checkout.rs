mod common;

use common::{create_admin_session, create_test_customer, create_test_invoice, test_server};
use cookie::Cookie;
use sqlx::PgPool;

// -----------------------------------------------------------------------
// Test 1: Checkout with unknown provider → error response
//
// The checkout handler delegates to rustbill_core::billing::checkout::create_checkout
// which returns BillingError::ProviderNotConfigured for unknown providers.
// This maps to 503 SERVICE_UNAVAILABLE.
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn checkout_unknown_provider_returns_error(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    let customer_id = create_test_customer(&pool).await;
    let invoice_id = create_test_invoice(&pool, &customer_id).await;

    let resp = server
        .get(&format!(
            "/api/billing/checkout?invoiceId={}&provider=paypal",
            invoice_id
        ))
        .add_cookie(Cookie::new("session", token))
        .await;

    // ProviderNotConfigured maps to 503
    resp.assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);

    let body: serde_json::Value = resp.json();
    assert!(
        body["error"]
            .as_str()
            .unwrap_or("")
            .contains("not configured"),
        "expected error message about provider not configured, got: {:?}",
        body
    );
}

// -----------------------------------------------------------------------
// Test 2: Checkout with non-existent invoice → 404
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn checkout_missing_invoice_returns_404(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    let fake_invoice_id = uuid::Uuid::new_v4().to_string();

    let resp = server
        .get(&format!(
            "/api/billing/checkout?invoiceId={}&provider=stripe",
            fake_invoice_id
        ))
        .add_cookie(Cookie::new("session", token))
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);

    let body: serde_json::Value = resp.json();
    assert!(
        body["error"].as_str().unwrap_or("").contains("not found"),
        "expected not found error, got: {:?}",
        body
    );
}

// -----------------------------------------------------------------------
// Test 3: Checkout requires an authenticated admin session
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn checkout_without_session_returns_unauthorized(pool: PgPool) {
    let server = test_server(pool.clone()).await;

    let customer_id = create_test_customer(&pool).await;
    let invoice_id = create_test_invoice(&pool, &customer_id).await;

    let resp = server
        .get(&format!(
            "/api/billing/checkout?invoiceId={}&provider=stripe",
            invoice_id
        ))
        .await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["error"].as_str().unwrap(), "Unauthorized");
}

// -----------------------------------------------------------------------
// Test 4: Checkout defaults provider to stripe when provider is omitted
//
// The route should still flow through the default stripe path, which surfaces
// the stripe-specific customer validation error without any external calls.
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn checkout_without_provider_defaults_to_stripe(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    let customer_id = create_test_customer(&pool).await;
    let invoice_id = create_test_invoice(&pool, &customer_id).await;

    let resp = server
        .get(&format!("/api/billing/checkout?invoiceId={}", invoice_id))
        .add_cookie(Cookie::new("session", token))
        .await;

    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = resp.json();
    assert!(
        body["error"]
            .as_str()
            .unwrap_or("")
            .contains("Stripe customer ID"),
        "expected stripe-specific validation error, got: {:?}",
        body
    );
}
