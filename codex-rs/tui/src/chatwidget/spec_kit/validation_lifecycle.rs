//! Validation lifecycle tracking and telemetry
//!
//! SPEC-KIT-069: Manages validate stage lifecycle events including:
//! - Payload hashing for deduplication
//! - Lifecycle event recording (Queued, Dispatched, Checking, Completed, Failed, Cancelled, Reset)
//! - Cancellation cleanup with proper telemetry
//!
//! This module ensures proper tracking of validate runs across retries and prevents
//! duplicate submissions through payload hash comparison.

use super::super::ChatWidget;
use super::evidence::{EvidenceRepository, FilesystemEvidence};
use super::state;
use crate::history_cell::HistoryCellType;
use crate::spec_prompts::SpecStage;
use chrono::Utc;
use serde_json::json;
use sha2::{Digest, Sha256};

// Re-export types for backward compatibility
pub use super::state::{ValidateCompletionReason, ValidateLifecycleEvent, ValidateMode};

/// Compute deterministic hash for validate payload to enable deduplication
///
/// Hash includes: mode, stage, spec_id, and trimmed payload content.
/// Used by validate lifecycle to detect duplicate submissions.
pub fn compute_validate_payload_hash(
    mode: state::ValidateMode,
    stage: SpecStage,
    spec_id: &str,
    payload: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(mode.as_str().as_bytes());
    hasher.update(b"|");
    hasher.update(stage.command_name().as_bytes());
    hasher.update(b"|");
    hasher.update(spec_id.as_bytes());
    hasher.update(b"|");
    hasher.update(payload.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Record validate lifecycle event to filesystem and local-memory
///
/// Emits dual telemetry:
/// 1. Filesystem evidence (docs/SPEC-OPS-004.../evidence/)
/// 2. Local-memory MCP storage (importance: 8, domain: spec-kit)
///
/// Only records if telemetry is enabled in widget config.
pub fn record_validate_lifecycle_event(
    widget: &mut ChatWidget,
    spec_id: &str,
    run_id: &str,
    attempt: u32,
    dedupe_count: u32,
    payload_hash: &str,
    mode: state::ValidateMode,
    event: state::ValidateLifecycleEvent,
) {
    if !widget.spec_kit_telemetry_enabled() {
        return;
    }

    let telemetry = json!({
        "spec_id": spec_id,
        "stage": "validate",
        "event": event.as_str(),
        "mode": mode.as_str(),
        "stage_run_id": run_id,
        "attempt": attempt,
        "dedupe_count": dedupe_count,
        "payload_hash": payload_hash,
        "timestamp": Utc::now().to_rfc3339(),
    });

    // Write to filesystem evidence
    let repo = FilesystemEvidence::new(widget.config.cwd.clone(), None);
    let _ = repo.write_telemetry_bundle(spec_id, SpecStage::Validate, &telemetry);

    // SPEC-934: Write to SQLite instead of MCP local-memory (async, fire-and-forget)
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let content = match serde_json::to_string(&telemetry) {
            Ok(s) => s,
            Err(_) => return,
        };
        let spec_id_owned = spec_id.to_string();
        handle.spawn(async move {
            if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
                // Store as "validate-lifecycle" stage to distinguish from main validate consensus
                let _ = db.store_artifact_with_stage_name(
                    &spec_id_owned,
                    "validate-lifecycle",
                    "agent-lifecycle-telemetry",
                    &content,
                    None, // run_id not available
                );
            }
        });
    }
}

/// Clean up spec_auto_state and emit Cancelled lifecycle event if validate is active
///
/// FORK-SPECIFIC (just-every/code): FR3 cancellation cleanup for SPEC-KIT-069
///
/// Properly handles:
/// 1. Emitting Cancelled lifecycle event if validate run was active
/// 2. Resetting validate lifecycle state
/// 3. Clearing spec_auto_state
/// 4. Logging cancellation reason to history
pub fn cleanup_spec_auto_with_cancel(widget: &mut ChatWidget, reason: &str) {
    // Extract lifecycle info before borrowing mutably
    let lifecycle_info = widget.spec_auto_state.as_ref().and_then(|state| {
        state.validate_lifecycle.active().map(|info| {
            (
                state.spec_id.clone(),
                info.run_id,
                info.attempt,
                info.dedupe_count,
                info.mode,
            )
        })
    });

    // Emit Cancelled lifecycle event if there was an active run
    if let Some((spec_id, run_id, attempt, dedupe_count, mode)) = lifecycle_info {
        record_validate_lifecycle_event(
            widget,
            &spec_id,
            &run_id,
            attempt,
            dedupe_count,
            "", // Empty payload hash for cancelled runs
            mode,
            state::ValidateLifecycleEvent::Cancelled,
        );

        // Clean up the validate lifecycle state
        if let Some(state_ref) = widget.spec_auto_state.as_ref() {
            let _ = state_ref
                .validate_lifecycle
                .reset_active(state::ValidateCompletionReason::Cancelled);
        }
    }

    // Clear the spec_auto_state
    widget.spec_auto_state = None;
    // P6-SYNC Phase 6: Clear spec-kit token metrics from status bar
    widget.bottom_pane.set_spec_auto_metrics(None);

    // Log cancellation reason for debugging
    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Spec-auto pipeline cancelled: {}",
            reason
        ))],
        HistoryCellType::Error,
    ));
}
