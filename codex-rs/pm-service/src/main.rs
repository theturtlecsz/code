//! `codex-pm-service` entry point.
//!
//! Lightweight per-user service for bot job management (SPEC-PM-003).
//! Managed by systemd user unit (PM-D6).
//!
//! Supports socket activation (D136), exit-when-idle (D135), and
//! auto-resume of incomplete runs on startup (Phase-1.5).
//!
//! ## Modes
//!
//! - **Service mode** (default): start the IPC server, resume incomplete
//!   runs, accept connections, idle-exit after timeout.
//! - **`--ping`**: connect to the running service socket, send a hello
//!   handshake, verify the response, then exit. Used by
//!   `codex-pm-resume.service` to trigger socket activation after
//!   reboot so that `resume_incomplete()` runs without a real client.

use std::io::{BufRead, Write};
use std::os::unix::io::FromRawFd;
use std::sync::Arc;

use codex_pm_service::manager::BotRunManager;
use codex_pm_service::persistence::PersistenceStore;
use tokio::net::UnixListener;

/// Idle timeout: exit after 60 seconds with no connections and no active runs.
const IDLE_TIMEOUT_SECS: u64 = 60;

/// Poll interval for the idle timer.
const IDLE_POLL_INTERVAL_SECS: u64 = 10;

/// Connect to the service socket, send a hello handshake, verify the
/// response, then exit.  Used by `codex-pm-resume.timer` to trigger
/// socket-activation after reboot so `resume_incomplete()` runs.
fn ping() -> std::io::Result<()> {
    let path = codex_pm_service::default_socket_path();
    let mut stream = std::os::unix::net::UnixStream::connect(&path).map_err(|e| {
        std::io::Error::other(format!("ping: cannot connect to {}: {e}", path.display()))
    })?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(5)))?;

    // Send hello JSON-RPC (newline-delimited)
    let hello = format!(
        r#"{{"id":0,"method":"hello","params":{{"protocol_version":"{}","client_version":"resume-timer"}}}}"#,
        codex_pm_service::PROTOCOL_VERSION,
    );
    stream.write_all(hello.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    // Read one response line
    let mut reader = std::io::BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    // Verify success
    let resp: serde_json::Value = serde_json::from_str(line.trim())
        .map_err(|e| std::io::Error::other(format!("ping: invalid response JSON: {e}")))?;
    if resp.get("result").is_some() {
        eprintln!("ping: service is alive");
        Ok(())
    } else {
        let msg = resp
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        Err(std::io::Error::other(format!("ping: hello failed: {msg}")))
    }
}

/// Create a Unix listener, preferring systemd socket activation (D136).
///
/// If `LISTEN_FDS` is set to >= 1, uses fd 3 (the first passed fd).
/// Otherwise, binds at the default socket path.
fn create_listener() -> std::io::Result<UnixListener> {
    // Check for systemd socket activation (LISTEN_FDS)
    if let Ok(val) = std::env::var("LISTEN_FDS")
        && let Ok(n) = val.parse::<u32>()
        && n >= 1
    {
        tracing::info!("Socket activation: using fd 3 from LISTEN_FDS={n}");
        // fd 3 is the first passed socket
        // SAFETY: fd 3 is guaranteed by systemd socket activation protocol
        let std_listener = unsafe { std::os::unix::net::UnixListener::from_raw_fd(3) };
        std_listener.set_nonblocking(true)?;
        return UnixListener::from_std(std_listener);
    }

    // No socket activation: bind at default path
    let path = codex_pm_service::default_socket_path();

    // Clean up stale socket file
    if path.exists() {
        std::fs::remove_file(&path)?;
    }

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(&path)?;
    tracing::info!("PM service listening on {}", path.display());
    Ok(listener)
}

fn main() -> std::io::Result<()> {
    // --ping: trigger socket activation and exit (used by codex-pm-resume.timer)
    if std::env::args().nth(1).as_deref() == Some("--ping") {
        return ping();
    }

    // Service mode: run the async runtime
    run_service()
}

#[tokio::main]
async fn run_service() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("codex-pm-service v{} starting", env!("CARGO_PKG_VERSION"));

    // Initialize persistence store
    let store = Arc::new(PersistenceStore::new().map_err(|e| {
        std::io::Error::other(format!("Failed to initialize persistence store: {e}"))
    })?);
    tracing::info!("Persistence store at {}", store.base_dir().display());

    let manager = Arc::new(BotRunManager::new(Arc::clone(&store)));

    // Resume incomplete runs before accepting connections
    manager.resume_incomplete().await;

    let listener = create_listener()?;

    // Shutdown coordination via watch channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Signal handler task: Ctrl+C / SIGTERM
    let shutdown_tx_signal = shutdown_tx.clone();
    let mgr_signal = Arc::clone(&manager);
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!(
            "Signal received, shutting down (active runs: {})",
            mgr_signal.active_run_count().await
        );
        let _ = shutdown_tx_signal.send(true);
    });

    // Idle timer task (D135): exit after IDLE_TIMEOUT_SECS with no activity
    let mgr_idle = Arc::clone(&manager);
    tokio::spawn(async move {
        let poll_interval = tokio::time::Duration::from_secs(IDLE_POLL_INTERVAL_SECS);
        let idle_timeout = std::time::Duration::from_secs(IDLE_TIMEOUT_SECS);
        loop {
            tokio::time::sleep(poll_interval).await;

            let active_runs = mgr_idle.active_run_count().await;
            let connections = mgr_idle.connection_count();
            let elapsed = mgr_idle.last_activity_elapsed().await;

            if active_runs == 0 && connections == 0 && elapsed >= idle_timeout {
                tracing::info!(
                    "Idle timeout ({IDLE_TIMEOUT_SECS}s): no connections, no active runs. Exiting."
                );
                let _ = shutdown_tx.send(true);
                break;
            }
        }
    });

    // Start IPC listener (blocks until shutdown)
    codex_pm_service::ipc::serve(manager, listener, shutdown_rx).await?;

    tracing::info!("codex-pm-service exiting cleanly");
    Ok(())
}
