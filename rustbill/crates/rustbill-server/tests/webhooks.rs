mod common;

use axum::http::{HeaderName, HeaderValue};
use common::test_server;
use sqlx::PgPool;

/// Insert a webhook secret into the system_settings table.
async fn set_system_setting(pool: &PgPool, key: &str, value: &str) {
    sqlx::query(
        r#"INSERT INTO system_settings (key, value, sensitive, updated_at)
           VALUES ($1, $2, true, NOW())
           ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()"#,
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await
    .expect("failed to insert system setting");
}

/// Compute Stripe-style HMAC-SHA256 signature: HMAC(timestamp.body, secret), returned as hex.
fn stripe_signature(timestamp: i64, body: &str, secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let payload = format!("{}.{}", timestamp, body);
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Compute LemonSqueezy-style HMAC-SHA256 signature: HMAC(body, secret), returned as hex.
fn lemonsqueezy_signature(body: &str, secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

// -----------------------------------------------------------------------
// Test 1: Stripe webhook with valid signature → 200
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn stripe_webhook_valid_signature_accepted(pool: PgPool) {
    let secret = "whsec_test_stripe_secret";
    set_system_setting(&pool, "stripe_webhook_secret", secret).await;

    let server = test_server(pool.clone()).await;

    let body = serde_json::json!({
        "type": "invoice.paid",
        "data": {
            "object": {
                "id": "in_test_12345",
                "amount_paid": 10000
            }
        }
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let timestamp = chrono::Utc::now().timestamp();
    let sig = stripe_signature(timestamp, &body_str, secret);
    let sig_header = format!("t={},v1={}", timestamp, sig);

    let resp = server
        .post("/api/billing/webhooks/stripe")
        .add_header(
            HeaderName::from_static("stripe-signature"),
            HeaderValue::from_str(&sig_header).unwrap(),
        )
        .add_header(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        )
        .text(body_str)
        .await;

    resp.assert_status_ok();
}

// -----------------------------------------------------------------------
// Test 2: Stripe webhook with invalid signature → 401
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn stripe_webhook_invalid_signature_rejected(pool: PgPool) {
    let secret = "whsec_test_stripe_secret_2";
    set_system_setting(&pool, "stripe_webhook_secret", secret).await;

    let server = test_server(pool.clone()).await;

    let body = serde_json::json!({
        "type": "invoice.paid",
        "data": {
            "object": {
                "id": "in_test_67890"
            }
        }
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let timestamp = chrono::Utc::now().timestamp();
    let sig_header = format!("t={},v1={}", timestamp, "deadbeefdeadbeefdeadbeef");

    let resp = server
        .post("/api/billing/webhooks/stripe")
        .add_header(
            HeaderName::from_static("stripe-signature"),
            HeaderValue::from_str(&sig_header).unwrap(),
        )
        .add_header(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        )
        .text(body_str)
        .await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// -----------------------------------------------------------------------
// Test 3: Xendit webhook with valid callback token → 200
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn xendit_webhook_valid_token_accepted(pool: PgPool) {
    let token = "xnd_test_callback_token";
    set_system_setting(&pool, "xendit_webhook_token", token).await;

    let server = test_server(pool.clone()).await;

    let body = serde_json::json!({
        "event": "invoice.paid",
        "status": "PAID",
        "external_id": "inv-xendit-001",
        "id": "xendit-payment-001",
        "paid_amount": 50000,
        "amount": 50000,
        "data": { "id": "xendit-data-001" }
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let resp = server
        .post("/api/billing/webhooks/xendit")
        .add_header(
            HeaderName::from_static("x-callback-token"),
            HeaderValue::from_static("xnd_test_callback_token"),
        )
        .add_header(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        )
        .text(body_str)
        .await;

    resp.assert_status_ok();
}

// -----------------------------------------------------------------------
// Test 4: Xendit webhook with wrong callback token → 401
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn xendit_webhook_invalid_token_rejected(pool: PgPool) {
    let token = "xnd_test_callback_token_real";
    set_system_setting(&pool, "xendit_webhook_token", token).await;

    let server = test_server(pool.clone()).await;

    let body = serde_json::json!({
        "event": "invoice.paid",
        "status": "PAID",
        "external_id": "inv-bad-001"
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let resp = server
        .post("/api/billing/webhooks/xendit")
        .add_header(
            HeaderName::from_static("x-callback-token"),
            HeaderValue::from_static("wrong-token"),
        )
        .add_header(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        )
        .text(body_str)
        .await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// -----------------------------------------------------------------------
// Test 5: LemonSqueezy webhook with valid HMAC signature → 200
// -----------------------------------------------------------------------
#[sqlx::test(migrations = "../../migrations")]
async fn lemonsqueezy_webhook_valid_signature_accepted(pool: PgPool) {
    let secret = "ls_test_webhook_secret";
    set_system_setting(&pool, "lemonsqueezy_webhook_secret", secret).await;

    let server = test_server(pool.clone()).await;

    let body = serde_json::json!({
        "meta": {
            "event_name": "order_created",
            "custom_data": {
                "invoiceId": "inv-ls-001"
            }
        },
        "data": {
            "id": "ls-order-001",
            "attributes": {
                "total": 2999
            }
        }
    });
    let body_str = serde_json::to_string(&body).unwrap();

    let sig = lemonsqueezy_signature(&body_str, secret);

    let resp = server
        .post("/api/billing/webhooks/lemonsqueezy")
        .add_header(
            HeaderName::from_static("x-signature"),
            HeaderValue::from_str(&sig).unwrap(),
        )
        .add_header(
            HeaderName::from_static("x-event-name"),
            HeaderValue::from_static("order_created"),
        )
        .add_header(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        )
        .text(body_str)
        .await;

    resp.assert_status_ok();
}
