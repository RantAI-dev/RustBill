//! Shared test helpers for integration tests.

use axum_test::TestServer;
use rustbill_core::auth::api_key::{generate_api_key, hash_api_key, get_key_prefix};
use rustbill_core::auth::password::hash_password;
use rustbill_core::config::{AppConfig, AuthConfig, CronConfig, DatabaseConfig, ServerConfig};
use rustbill_core::settings::provider_settings::ProviderSettingsCache;
use rustbill_server::app::{AppState, build_router};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

/// Build a `TestServer` from the given database pool.
///
/// Creates a minimal `AppConfig` suitable for testing, constructs the full
/// `AppState`, and wraps the router in an `axum_test::TestServer`.
pub async fn test_server(pool: PgPool) -> TestServer {
    let config = AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            cors_origins: vec![],
        },
        database: DatabaseConfig {
            url: String::new(), // not used — we already have a pool
            max_connections: 5,
            min_connections: 1,
        },
        auth: AuthConfig {
            provider: "default".to_string(),
            session_expiry_days: 7,
            keycloak: None,
            cron_secret: Some("test-cron-secret".to_string()),
        },
        cron: CronConfig {
            subscription_lifecycle: "0 0 * * * *".to_string(),
            dunning: "0 0 */6 * * *".to_string(),
            enabled: false,
        },
    };

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build http client");

    let provider_cache = ProviderSettingsCache::new(pool.clone());

    let state = Arc::new(AppState {
        db: pool,
        config: Arc::new(config),
        http_client,
        email_sender: None,
        provider_cache,
    });

    let router = build_router(state);

    TestServer::new(router)
}

/// Create an admin user and session. Returns the session token string.
pub async fn create_admin_session(pool: &PgPool) -> String {
    let user_id = uuid::Uuid::new_v4().to_string();
    let password_hash = hash_password("testpass123").expect("failed to hash password");
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO users (id, email, name, password_hash, role, auth_provider, created_at, updated_at)
           VALUES ($1, $2, $3, $4, 'admin', 'default', $5, $5)"#,
    )
    .bind(&user_id)
    .bind(format!("admin-{}@test.com", &user_id[..8]))
    .bind("Test Admin")
    .bind(&password_hash)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert admin user");

    let token = uuid::Uuid::new_v4().to_string();
    let expires_at = now + chrono::Duration::days(7);

    sqlx::query(
        "INSERT INTO sessions (id, user_id, expires_at, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&token)
    .bind(&user_id)
    .bind(expires_at)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert session");

    token
}

/// Insert a minimal test customer. Returns the customer ID.
pub async fn create_test_customer(pool: &PgPool) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO customers
           (id, name, industry, tier, location, contact, email, phone,
            total_revenue, health_score, trend, last_contact, created_at, updated_at)
           VALUES ($1, $2, 'Technology', 'enterprise', 'US', 'Test Contact',
                   $3, '+1234567890', 0, 80, 'stable', '2025-01-01', $4, $4)"#,
    )
    .bind(&id)
    .bind(format!("Test Customer {}", &id[..8]))
    .bind(format!("customer-{}@test.com", &id[..8]))
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert customer");

    id
}

/// Insert a test product. `product_type` should be `"licensed"`, `"saas"`, or `"api"`.
/// Returns the product ID.
pub async fn create_test_product(pool: &PgPool, product_type: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO products
           (id, name, product_type, revenue, target, change, created_at, updated_at)
           VALUES ($1, $2, $3::product_type, 0, 10000, 0, $4, $4)"#,
    )
    .bind(&id)
    .bind(format!("Test Product {}", &id[..8]))
    .bind(product_type)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert product");

    id
}

/// Insert a test pricing plan for a product. Returns the plan ID.
pub async fn create_test_plan(pool: &PgPool, product_id: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO pricing_plans
           (id, product_id, name, pricing_model, billing_cycle, base_price, trial_days, active, created_at, updated_at)
           VALUES ($1, $2, $3, 'flat'::pricing_model, 'monthly'::billing_cycle, 29.99, 0, true, $4, $4)"#,
    )
    .bind(&id)
    .bind(product_id)
    .bind(format!("Test Plan {}", &id[..8]))
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert pricing plan");

    id
}

/// Insert a test subscription. Returns the subscription ID.
pub async fn create_test_subscription(
    pool: &PgPool,
    customer_id: &str,
    plan_id: &str,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();
    let period_end = now + chrono::Duration::days(30);

    sqlx::query(
        r#"INSERT INTO subscriptions
           (id, customer_id, plan_id, status, current_period_start, current_period_end,
            cancel_at_period_end, quantity, version, created_at, updated_at)
           VALUES ($1, $2, $3, 'active'::subscription_status, $4, $5, false, 1, 1, $4, $4)"#,
    )
    .bind(&id)
    .bind(customer_id)
    .bind(plan_id)
    .bind(now)
    .bind(period_end)
    .execute(pool)
    .await
    .expect("failed to insert subscription");

    id
}

/// Insert a draft invoice for a customer. Returns the invoice ID.
pub async fn create_test_invoice(pool: &PgPool, customer_id: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let invoice_number = format!("INV-TEST-{}", &id[..8]);
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO invoices
           (id, invoice_number, customer_id, status, subtotal, tax, total, currency, version, created_at, updated_at)
           VALUES ($1, $2, $3, 'draft'::invoice_status, 0, 0, 0, 'USD', 1, $4, $4)"#,
    )
    .bind(&id)
    .bind(&invoice_number)
    .bind(customer_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert invoice");

    id
}

/// Insert a test API key. Returns `(id, plaintext_key)`.
pub async fn create_test_api_key(pool: &PgPool) -> (String, String) {
    let id = uuid::Uuid::new_v4().to_string();
    let plaintext_key = generate_api_key();
    let key_hash = hash_api_key(&plaintext_key);
    let key_prefix = get_key_prefix(&plaintext_key);
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO api_keys
           (id, name, key_hash, key_prefix, status, created_at)
           VALUES ($1, $2, $3, $4, 'active'::api_key_status, $5)"#,
    )
    .bind(&id)
    .bind(format!("Test Key {}", &id[..8]))
    .bind(&key_hash)
    .bind(&key_prefix)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert api key");

    (id, plaintext_key)
}
