//! `codex-pm-service` — Bot job management service (SPEC-PM-003).
//!
//! Lightweight persistent service that manages bot runs for work items.
//! Listens on a Unix domain socket at `$XDG_RUNTIME_DIR/codex-pm.sock`
//! and speaks JSON-RPC-lite (PM-D8).
//!
//! ## Decision References
//! - D135: Service-first bot job management
//! - D136: Unix domain socket IPC
//! - PM-D5: Per-user service instance
//! - PM-D7: Socket path convention
//! - PM-D8: JSON-RPC-lite protocol
//! - PM-D9: Protocol handshake
//! - PM-D21: Crate split

pub mod engine;
pub mod ipc;
pub mod manager;
pub mod persistence;
pub mod protocol;

/// Protocol version for the PM service IPC.
pub const PROTOCOL_VERSION: &str = "1.0";

/// Default socket filename.
pub const SOCKET_FILENAME: &str = "codex-pm.sock";

/// Get the default socket path using XDG_RUNTIME_DIR.
///
/// Falls back to `/tmp/codex-pm-<username>.sock` if XDG_RUNTIME_DIR is not set.
pub fn default_socket_path() -> std::path::PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime_dir).join(SOCKET_FILENAME)
    } else {
        // Fallback for systems without XDG_RUNTIME_DIR
        let user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        std::path::PathBuf::from(format!("/tmp/codex-pm-{user}.sock"))
    }
}
