//! IPC protocol types for the PM service (PM-D8, PM-D9).
//!
//! Defines the JSON-RPC-lite request/response types specific to
//! the PM service protocol. Reuses `JSONRPCMessage` from
//! `app-server-protocol` for wire format.

use codex_core::pm::artifacts::BotRunState;
use codex_core::pm::bot::{BotCaptureMode, BotKind, BotWriteMode};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Error codes (JSON-RPC error.code)
// ─────────────────────────────────────────────────────────────────────────────

/// Standard JSON-RPC errors.
pub const ERR_INVALID_REQUEST: i64 = -32600;
pub const ERR_METHOD_NOT_FOUND: i64 = -32601;
pub const ERR_INVALID_PARAMS: i64 = -32602;

/// PM-specific error codes.
pub const ERR_NEEDS_INPUT: i64 = 10;
pub const ERR_NEEDS_APPROVAL: i64 = 11;
pub const ERR_INVARIANT: i64 = 13;
pub const ERR_DUPLICATE_RUN: i64 = 100;
pub const ERR_CAPSULE: i64 = 200;
pub const ERR_INFRA: i64 = 300;

// ─────────────────────────────────────────────────────────────────────────────
// Handshake (PM-D9)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloParams {
    pub protocol_version: String,
    pub client_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResult {
    pub protocol_version: String,
    pub service_version: String,
    pub capabilities: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// bot.run
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotRunParams {
    pub workspace_path: String,
    pub work_item_id: String,
    pub kind: BotKind,
    pub capture_mode: BotCaptureMode,
    #[serde(default)]
    pub write_mode: BotWriteMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intensity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rebase_target: Option<String>,
    /// If true, client wants push notifications (PM-D24).
    #[serde(default)]
    pub subscribe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotRunResult {
    pub run_id: String,
    pub status: BotRunState,
    pub work_item_id: String,
    pub kind: BotKind,
}

// ─────────────────────────────────────────────────────────────────────────────
// bot.status
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotStatusParams {
    pub workspace_path: String,
    pub work_item_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<BotKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotStatusResult {
    pub runs: Vec<RunSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub run_id: String,
    pub status: BotRunState,
    pub kind: BotKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checkpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// bot.show
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotShowParams {
    pub workspace_path: String,
    pub work_item_id: String,
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// bot.runs (list)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotRunsParams {
    pub workspace_path: String,
    pub work_item_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// bot.cancel
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotCancelParams {
    pub workspace_path: String,
    pub work_item_id: String,
    pub run_id: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// bot.resume
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotResumeParams {
    pub workspace_path: String,
    pub work_item_id: String,
    pub run_id: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// service.status
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatusResult {
    pub uptime_s: u64,
    pub active_runs: usize,
    pub workspaces: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// service.doctor
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDoctorParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    pub name: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDoctorResult {
    pub checks: Vec<DoctorCheck>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Notifications (service -> client, PM-D24)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotProgressNotification {
    pub run_id: String,
    pub phase: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotTerminalNotification {
    pub run_id: String,
    pub status: BotRunState,
    pub exit_code: i32,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_uris: Vec<String>,
}
