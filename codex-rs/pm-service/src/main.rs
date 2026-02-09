//! `codex-pm-service` entry point.
//!
//! Lightweight per-user service for bot job management (SPEC-PM-003).
//! Managed by systemd user unit (PM-D6).

use std::sync::Arc;

use codex_pm_service::manager::BotRunManager;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("codex-pm-service v{} starting", env!("CARGO_PKG_VERSION"));

    let manager = Arc::new(BotRunManager::new());

    // Handle graceful shutdown on SIGTERM/SIGINT
    let mgr_clone = Arc::clone(&manager);
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!(
            "Shutting down (active runs: {})",
            mgr_clone.active_run_count().await
        );
        std::process::exit(0);
    });

    // Start IPC listener
    codex_pm_service::ipc::serve(manager, None).await
}
