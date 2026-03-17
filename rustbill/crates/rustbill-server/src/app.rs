use axum::Router;
use rustbill_core::config::AppConfig;
use rustbill_core::notifications::email::EmailSender;
use rustbill_core::settings::provider_settings::ProviderSettingsCache;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::routes;

/// Shared application state passed to all handlers.
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Arc<AppConfig>,
    pub http_client: reqwest::Client,
    pub email_sender: Option<EmailSender>,
    pub provider_cache: Arc<ProviderSettingsCache>,
}

pub type SharedState = Arc<AppState>;

/// Build the application state from config.
pub async fn build_state(config: AppConfig) -> anyhow::Result<SharedState> {
    // Database pool
    let db = rustbill_core::db::pool::create_pool(&config.database).await?;

    // HTTP client for outbound requests
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    // Email sender
    let email_sender = EmailSender::from_env();

    // Provider settings cache
    let provider_cache = ProviderSettingsCache::new(db.clone());

    // Seed default admin if no admin user exists
    ensure_default_admin(&db).await?;

    Ok(Arc::new(AppState {
        db,
        config: Arc::new(config),
        http_client,
        email_sender,
        provider_cache,
    }))
}

/// Ensure a default admin user exists. Creates one if the users table is empty.
async fn ensure_default_admin(db: &sqlx::PgPool) -> anyhow::Result<()> {
    let has_admin = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE role = 'admin')",
    )
    .fetch_one(db)
    .await?;

    if !has_admin {
        let password_hash = rustbill_core::auth::password::hash_password("admin123")?;
        sqlx::query(
            r#"INSERT INTO users (id, email, name, password_hash, role, auth_provider, created_at, updated_at)
               VALUES (gen_random_uuid()::text, 'admin@rustbill.local', 'Admin', $1, 'admin', 'default', NOW(), NOW())"#,
        )
        .bind(&password_hash)
        .execute(db)
        .await?;

        tracing::info!("Created default admin user: admin@rustbill.local (password: admin123)");
    }

    Ok(())
}

/// Build the Axum router with all routes and middleware.
pub fn build_router(state: SharedState) -> Router {
    // Public v1 API — API key auth, permissive CORS
    let public_v1 = Router::new()
        .nest("/api/v1", routes::v1::router())
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::api_key_auth::require_api_key,
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    // Auth routes — no session required
    let auth_routes = Router::new().nest("/api/auth", routes::auth::router());

    // Inbound webhooks — no session, signature verification only
    let webhook_routes = Router::new().nest("/api/billing", routes::webhooks_inbound::router());

    // Public license verification — no session required
    let public_license_routes = Router::new()
        .nest("/api/licenses", routes::licenses::public_router());

    // Admin API — session required (middleware applied per-group)
    let admin_api = Router::new()
        .nest("/api/products", routes::products::router())
        .nest("/api/customers", routes::customers::router())
        .nest("/api/deals", routes::deals::router())
        .nest("/api/licenses", routes::licenses::router())
        .nest("/api/api-keys", routes::api_keys::router())
        .nest("/api/billing", routes::billing::router())
        .nest("/api/analytics", routes::analytics::router())
        .nest("/api/search", routes::search::router())
        .nest("/api/settings", routes::settings::router())
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::session_auth::require_session,
        ));

    // Health check
    let health = Router::new().route("/health", axum::routing::get(|| async { "ok" }));

    Router::new()
        .merge(health)
        .merge(public_v1)
        .merge(auth_routes)
        .merge(webhook_routes)
        .merge(public_license_routes)
        .merge(admin_api)
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .with_state(state)
}
