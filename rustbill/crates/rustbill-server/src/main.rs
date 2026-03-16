use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use rustbill_server::app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "billing_server=info,rustbill_core=info,tower_http=info".into()
        }))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    tracing::info!("Starting RustBill Server");

    // Load config
    let config = rustbill_core::config::AppConfig::load()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));

    // Build app state
    let state = app::build_state(config).await?;

    // Build router
    let router = app::build_router(state);

    // Start server with graceful shutdown
    tracing::info!(%addr, "Server listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Received Ctrl+C"),
        _ = terminate => tracing::info!("Received SIGTERM"),
    }
}
