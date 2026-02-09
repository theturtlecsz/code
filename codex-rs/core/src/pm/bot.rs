//! Bot runner request types (PM holding-state automation).
//!
//! Caller-facing semantics are specified in:
//! - `docs/SPEC-PM-002-bot-runner/spec.md`
//!
//! Runner/service implementation details are specified in:
//! - `docs/SPEC-PM-003-bot-system/spec.md`

use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BotKind {
    Research,
    Review,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BotWriteMode {
    /// Bot is read-only with respect to the repo.
    #[default]
    None,
    /// Bot may stage suggested changes in a bot-owned worktree/branch.
    Worktree,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BotCaptureMode {
    None,
    PromptsOnly,
    FullIo,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct BotRunTrigger {
    /// Logical source of the run request (e.g., "cli", "tui", "headless", "linear").
    pub source: String,

    /// Optional external idempotency key (e.g., Linear delivery id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<String>,

    /// Optional deep-link to the originating UI object (e.g., Linear issue URL).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A request to execute a single bot run for a work item.
///
/// This is intended to be IPC-friendly and safe to persist as a capsule artifact
/// when needed for audit/debug, but the canonical run outputs remain the bot
/// artifacts defined in `SPEC-PM-002` (e.g., `BotRunLog`, `ResearchReport`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct BotRunRequest {
    pub schema_version: String,

    /// Unique identifier for this run (stable across retries).
    pub run_id: String,

    pub work_item_id: String,
    pub kind: BotKind,

    pub capture_mode: BotCaptureMode,

    #[serde(default)]
    pub write_mode: BotWriteMode,

    /// RFC3339 timestamp of when the request was created.
    pub requested_at: String,

    /// Optional trigger metadata (UI bridge, CLI invocation, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<BotRunTrigger>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BotRunRequestValidationError {
    #[error("write_mode=worktree is only allowed for kind=review")]
    WorktreeWriteModeRequiresReview,
}

impl BotRunRequest {
    pub const SCHEMA_VERSION: &'static str = "bot_run_request@1.0";

    pub fn new(
        work_item_id: impl Into<String>,
        kind: BotKind,
        capture_mode: BotCaptureMode,
        write_mode: BotWriteMode,
        trigger: Option<BotRunTrigger>,
    ) -> Result<Self, BotRunRequestValidationError> {
        let req = Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            run_id: Uuid::new_v4().to_string(),
            work_item_id: work_item_id.into(),
            kind,
            capture_mode,
            write_mode,
            requested_at: Utc::now().to_rfc3339(),
            trigger,
        };

        req.validate()?;
        Ok(req)
    }

    pub fn validate(&self) -> Result<(), BotRunRequestValidationError> {
        if self.kind == BotKind::Research && self.write_mode == BotWriteMode::Worktree {
            return Err(BotRunRequestValidationError::WorktreeWriteModeRequiresReview);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_mode_defaults_to_none() {
        let json = serde_json::json!({
            "schema_version": BotRunRequest::SCHEMA_VERSION,
            "run_id": "run_123",
            "work_item_id": "SPEC-PM-001",
            "kind": "research",
            "capture_mode": "prompts_only",
            "requested_at": "2026-02-08T00:00:00Z"
        });

        let req: BotRunRequest = match serde_json::from_value(json) {
            Ok(req) => req,
            Err(err) => panic!("Failed to deserialize BotRunRequest JSON: {err}"),
        };
        assert_eq!(req.write_mode, BotWriteMode::None);
    }

    #[test]
    fn research_worktree_is_invalid() {
        let res = BotRunRequest::new(
            "SPEC-PM-001",
            BotKind::Research,
            BotCaptureMode::PromptsOnly,
            BotWriteMode::Worktree,
            None,
        );

        let err = match res {
            Ok(_) => panic!("Expected BotRunRequest::new() to fail, but it succeeded"),
            Err(err) => err,
        };

        assert_eq!(
            err,
            BotRunRequestValidationError::WorktreeWriteModeRequiresReview
        );
    }
}
