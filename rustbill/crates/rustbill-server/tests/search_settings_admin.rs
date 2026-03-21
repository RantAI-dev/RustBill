mod common;

use common::{create_admin_session, create_test_customer, create_test_product, test_server};
use cookie::Cookie;
use serde_json::{json, Value};
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn setup(pool: PgPool) -> (axum_test::TestServer, String) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    (server, token)
}

async fn insert_test_license(pool: &PgPool) -> String {
    let key_suffix = uuid::Uuid::new_v4().to_string();
    let license_key = format!("LIC-TEST-{}", &key_suffix[..8]);
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO licenses
           (key, customer_id, customer_name, product_id, product_name,
            status, created_at, expires_at, license_type, max_activations)
           VALUES ($1, $2, $3, $4, $5, 'active', $6, $7, 'simple', 5)"#,
    )
    .bind(&license_key)
    .bind(uuid::Uuid::new_v4().to_string())
    .bind("Test Customer")
    .bind(uuid::Uuid::new_v4().to_string())
    .bind("Test Product")
    .bind(now.format("%Y-%m-%d").to_string())
    .bind(
        (now + chrono::Duration::days(365))
            .format("%Y-%m-%d")
            .to_string(),
    )
    .execute(pool)
    .await
    .expect("failed to insert license");

    license_key
}

fn assert_masked(value: &Value, expected_original: &str) {
    let masked = value.as_str().expect("expected string value");
    assert_ne!(masked, expected_original);
    assert!(
        masked.starts_with("••••••••"),
        "expected masked value, got {masked}"
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn search_admin_returns_customers_products_and_licenses(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;

    let customer_id = create_test_customer(&pool).await;
    let product_id = create_test_product(&pool, "saas").await;
    let license_key = insert_test_license(&pool).await;

    let resp = server
        .get("/api/search?q=Test&limit=10")
        .add_cookie(Cookie::new("session", token))
        .await;

    resp.assert_status_ok();
    let body: Value = resp.json();

    assert_eq!(body["query"], json!("Test"));
    assert_eq!(body["total"], json!(3));

    let results = body["results"]
        .as_array()
        .expect("results should be an array");
    assert_eq!(results.len(), 3);

    let mut types = results
        .iter()
        .map(|item| item["type"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    types.sort();
    assert_eq!(types, vec!["customer", "license", "product"]);

    let customer = results
        .iter()
        .find(|item| item["type"] == "customer")
        .expect("customer result missing");
    assert_eq!(customer["data"]["id"], json!(customer_id));

    let product = results
        .iter()
        .find(|item| item["type"] == "product")
        .expect("product result missing");
    assert_eq!(product["data"]["id"], json!(product_id));

    let license = results
        .iter()
        .find(|item| item["type"] == "license")
        .expect("license result missing");
    assert_eq!(license["data"]["key"], json!(license_key));
}

#[sqlx::test(migrations = "../../migrations")]
async fn payment_provider_settings_get_and_put_round_trip(pool: PgPool) {
    let (server, token) = setup(pool).await;

    let get_resp = server
        .get("/api/settings/payment-providers")
        .add_cookie(Cookie::new("session", token.clone()))
        .await;

    get_resp.assert_status_ok();
    let initial: Value = get_resp.json();

    assert_eq!(initial["stripe"]["configured"], json!(false));
    assert_eq!(initial["xendit"]["configured"], json!(false));
    assert_eq!(initial["lemonsqueezy"]["configured"], json!(false));
    assert_eq!(initial["tax"]["configured"], json!(false));

    let stripe_secret = "sk_test_1234567890";
    let stripe_webhook_secret = "whsec_abcdef123456";

    let put_resp = server
        .put("/api/settings/payment-providers")
        .add_cookie(Cookie::new("session", token.clone()))
        .json(&json!({
            "provider": "stripe",
            "settings": {
                "secretKey": stripe_secret,
                "webhookSecret": stripe_webhook_secret
            }
        }))
        .await;

    put_resp.assert_status_ok();
    let updated: Value = put_resp.json();

    assert_eq!(updated["stripe"]["configured"], json!(true));
    assert_masked(&updated["stripe"]["secretKey"], stripe_secret);
    assert_masked(&updated["stripe"]["webhookSecret"], stripe_webhook_secret);

    let confirm_resp = server
        .get("/api/settings/payment-providers")
        .add_cookie(Cookie::new("session", token))
        .await;

    confirm_resp.assert_status_ok();
    let confirmed: Value = confirm_resp.json();

    assert_eq!(confirmed["stripe"]["configured"], json!(true));
    assert_masked(&confirmed["stripe"]["secretKey"], stripe_secret);
    assert_masked(&confirmed["stripe"]["webhookSecret"], stripe_webhook_secret);
}
