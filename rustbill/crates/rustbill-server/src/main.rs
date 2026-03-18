use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

use rustbill_server::app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

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

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "rustbill_server=info,rustbill_core=info,tower_http=info".into());

    let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".to_string());
    let log_format = std::env::var("LOG_FORMAT").ok();

    let use_json = match log_format
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
    {
        Some(format) if format == "json" => true,
        Some(format) if format == "pretty" => false,
        Some(format) => {
            eprintln!(
                "Unsupported LOG_FORMAT='{format}'. Falling back to RUN_MODE-based format selection."
            );
            run_mode == "production"
        }
        None => run_mode == "production",
    };

    if use_json {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .pretty()
            .with_target(false)
            .init();
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
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
