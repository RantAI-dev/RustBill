use crate::config::DatabaseConfig;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool(config: &DatabaseConfig) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(600))
        .connect(&config.url)
        .await?;

    tracing::info!(
        max = config.max_connections,
        min = config.min_connections,
        "Database pool created"
    );

    Ok(pool)
}
