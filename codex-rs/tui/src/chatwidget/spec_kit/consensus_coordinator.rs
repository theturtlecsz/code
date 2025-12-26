//! Consensus coordination and checking
//!
//! This module handles consensus checking for spec-auto pipeline stages:
//! - Async MCP consensus with retry logic
//! - Blocking async operations in sync context
//! - Cost summary persistence with routing notes
//! - Consensus artifact inspection (/spec-consensus command)
//!
//! **FORK-SPECIFIC (just-every/code)**: Native MCP integration with retry for
//! handling MCP connection timing and transient failures.

use super::super::ChatWidget;
use super::gate_evaluation::run_spec_consensus;
use crate::spec_prompts::SpecStage;
use std::sync::Arc;

// FORK-SPECIFIC (just-every/code): MCP retry configuration
const MCP_RETRY_ATTEMPTS: u32 = 3;
const MCP_RETRY_DELAY_MS: u64 = 100;

/// Block on async operation in sync context
///
/// Handles both cases:
/// 1. Within tokio runtime: use block_in_place
/// 2. Outside runtime: create new current_thread runtime
///
/// Used to run async MCP consensus checks from synchronous pipeline code.
pub(crate) fn block_on_sync<F, Fut, T>(factory: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let handle_clone = handle.clone();
        tokio::task::block_in_place(move || handle_clone.block_on(factory()))
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build runtime")
            .block_on(factory())
    }
}

/// Run consensus check with retry logic for MCP initialization
///
/// FORK-SPECIFIC (just-every/code): Handles MCP connection timing and transient failures
///
/// Retry strategy:
/// - Up to 3 attempts
/// - Exponential backoff: 100ms, 200ms, 400ms
/// - Continues through MCP initialization delays
/// - Returns last error after all retries exhausted
pub(crate) async fn run_consensus_with_retry(
    mcp_manager: Arc<
        tokio::sync::Mutex<Option<Arc<codex_core::mcp_connection_manager::McpConnectionManager>>>,
    >,
    cwd: std::path::PathBuf,
    spec_id: String,
    stage: SpecStage,
    telemetry_enabled: bool,
) -> super::error::Result<(Vec<ratatui::text::Line<'static>>, bool)> {
    let mut last_error = None;

    for attempt in 0..MCP_RETRY_ATTEMPTS {
        let manager_guard = mcp_manager.lock().await;
        let Some(manager) = manager_guard.as_ref() else {
            last_error = Some("MCP manager not initialized yet".to_string());
            drop(manager_guard);

            if attempt < MCP_RETRY_ATTEMPTS - 1 {
                let delay = MCP_RETRY_DELAY_MS * (2_u64.pow(attempt));
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                continue;
            }
            break;
        };

        match run_spec_consensus(&cwd, &spec_id, stage, telemetry_enabled, manager).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e.to_string());
                drop(manager_guard);

                if attempt < MCP_RETRY_ATTEMPTS - 1 {
                    let delay = MCP_RETRY_DELAY_MS * (2_u64.pow(attempt));
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
            }
        }
    }

    Err(super::error::SpecKitError::from_string(
        last_error.unwrap_or_else(|| "MCP consensus check failed after retries".to_string()),
    ))
}

/// Persist cost summary with routing notes
///
/// Attaches aggregator effort and escalation reason notes for the just-finished
/// stage before writing cost summary to filesystem.
///
/// Used after successful consensus to record cost tracking data with context.
pub(crate) fn persist_cost_summary(widget: &mut ChatWidget, spec_id: &str) {
    let dir = widget.cost_summary_dir();
    // Attach routing notes for the just-finished stage (if available)
    if let Some(state) = widget.spec_auto_state.as_ref()
        && let Some(stage) = state.current_stage()
    {
        let effort = state.aggregator_effort_notes.get(&stage).cloned();
        let reason = state.escalation_reason_notes.get(&stage).cloned();
        widget.spec_cost_tracker().set_stage_routing_note(
            spec_id,
            stage,
            effort.as_deref(),
            reason.as_deref(),
        );
    }
    if let Err(err) = widget.spec_cost_tracker().write_summary(spec_id, &dir) {
        widget.history_push(crate::history_cell::new_warning_event(format!(
            "Failed to write cost summary for {}: {}",
            spec_id, err
        )));
    }
}
