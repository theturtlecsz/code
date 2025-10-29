//! Spec-Kit command handlers as free functions
//!
//! These functions are extracted from chatwidget.rs to isolate spec-kit code.
//! Using free functions instead of methods to avoid Rust borrow checker issues.

use super::super::ChatWidget; // Parent module (friend access to private fields)
use super::agent_orchestrator;
use super::consensus_coordinator::{
    block_on_sync, persist_cost_summary, run_consensus_with_retry,
};
use super::evidence::FilesystemEvidence;
use super::state::{GuardrailWait, SpecAutoPhase, ValidateBeginOutcome, ValidateRunInfo};
use super::validation_lifecycle::{
    cleanup_spec_auto_with_cancel, compute_validate_payload_hash,
    record_validate_lifecycle_event, ValidateCompletionReason, ValidateLifecycleEvent,
    ValidateMode,
};
use crate::history_cell::HistoryCellType;
use crate::slash_command::{HalMode, SlashCommand};
use crate::spec_prompts::SpecStage;

// Re-export command handlers for backward compatibility
pub use super::command_handlers::{
    halt_spec_auto_with_error, handle_guardrail, handle_spec_consensus, handle_spec_status,
};

// Re-export consensus coordination for use by command_handlers
pub(crate) use super::consensus_coordinator::handle_spec_consensus_impl;

// Re-export agent orchestration functions
pub use super::agent_orchestrator::{
    auto_submit_spec_stage_prompt, on_spec_auto_agents_complete, record_agent_costs,
};
pub(crate) use super::agent_orchestrator::{
    schedule_degraded_follow_up, SPEC_AUTO_AGENT_RETRY_ATTEMPTS,
};

// === Spec Auto Pipeline Methods ===

/// Handle /speckit.auto command initiation
pub fn handle_spec_auto(
    widget: &mut ChatWidget,
    spec_id: String,
    goal: String,
    resume_from: SpecStage,
    hal_mode: Option<HalMode>,
) {
    let mut header: Vec<ratatui::text::Line<'static>> = Vec::new();
    header.push(ratatui::text::Line::from(format!("/spec-auto {}", spec_id)));
    if !goal.trim().is_empty() {
        header.push(ratatui::text::Line::from(format!("Goal: {}", goal)));
    }
    header.push(ratatui::text::Line::from(format!(
        "Resume from: {}",
        resume_from.display_name()
    )));
    match hal_mode {
        Some(HalMode::Live) => header.push(ratatui::text::Line::from("HAL mode: live")),
        Some(HalMode::Mock) => header.push(ratatui::text::Line::from("HAL mode: mock")),
        None => header.push(ratatui::text::Line::from("HAL mode: mock (default)")),
    }
    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        header,
        HistoryCellType::Notice,
    ));

    // Validate configuration before starting pipeline (T83)
    if let Err(err) = super::config_validator::SpecKitConfigValidator::validate(&widget.config) {
        widget.history_push(crate::history_cell::new_error_event(format!(
            "Configuration validation failed: {}",
            err
        )));
        return;
    }

    let lifecycle = widget.ensure_validate_lifecycle(&spec_id);
    let mut state = super::state::SpecAutoState::new(spec_id, goal, resume_from, hal_mode);
    state.set_validate_lifecycle(lifecycle);
    widget.spec_auto_state = Some(state);
    advance_spec_auto(widget);
}

/// Advance spec-auto pipeline to next stage
pub fn advance_spec_auto(widget: &mut ChatWidget) {
    if widget.spec_auto_state.is_none() {
        return;
    }
    if widget
        .spec_auto_state
        .as_ref()
        .and_then(|state| state.waiting_guardrail.as_ref())
        .is_some()
    {
        return;
    }

    enum NextAction {
        PipelineComplete,
        RunGuardrail {
            command: SlashCommand,
            args: String,
            hal_mode: Option<HalMode>,
        },
    }

    loop {
        let next_action = {
            let Some(state) = widget.spec_auto_state.as_mut() else {
                return;
            };

            if state.current_index >= state.stages.len() {
                NextAction::PipelineComplete
            } else {
                let stage = state.stages[state.current_index];
                let hal_mode = state.hal_mode;

                // Check if we should run a quality checkpoint before this stage
                if state.quality_gates_enabled {
                    if let Some(checkpoint) =
                        determine_quality_checkpoint(stage, &state.completed_checkpoints)
                    {
                        // Execute quality checkpoint instead of proceeding to guardrail
                        execute_quality_checkpoint(widget, checkpoint);
                        return;
                    }
                }

                match &state.phase {
                    SpecAutoPhase::Guardrail => {
                        let command = super::state::guardrail_for_stage(stage);
                        let args = state.spec_id.clone();
                        state.waiting_guardrail = Some(GuardrailWait {
                            stage,
                            command,
                            task_id: None,
                        });
                        NextAction::RunGuardrail {
                            command,
                            args,
                            hal_mode,
                        }
                    }
                    SpecAutoPhase::ExecutingAgents { .. } => {
                        return;
                    }
                    SpecAutoPhase::CheckingConsensus => {
                        return;
                    }
                    // Quality gate phases
                    SpecAutoPhase::QualityGateExecuting { .. } => {
                        return; // Waiting for quality gate agents
                    }
                    SpecAutoPhase::QualityGateProcessing { .. } => {
                        return; // Processing results
                    }
                    SpecAutoPhase::QualityGateValidating { .. } => {
                        return; // Waiting for GPT-5 validation responses
                    }
                    SpecAutoPhase::QualityGateAwaitingHuman { .. } => {
                        return; // Waiting for human input
                    }
                }
            }
        };

        match next_action {
            NextAction::PipelineComplete => {
                // Finalize quality gates if enabled
                if let Some(state) = widget.spec_auto_state.as_ref() {
                    if state.quality_gates_enabled && !state.quality_checkpoint_outcomes.is_empty()
                    {
                        finalize_quality_gates(widget);
                    }
                }

                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from("/spec-auto pipeline complete")],
                    HistoryCellType::Notice,
                ));
                // Successful completion - clear state without cancellation event
                widget.spec_auto_state = None;
                return;
            }
            NextAction::RunGuardrail {
                command,
                args,
                hal_mode,
            } => {
                widget.handle_spec_ops_command(command, args, hal_mode);
                return;
            }
        }
    }
}

/// Handle spec-auto task started event
pub fn on_spec_auto_task_started(widget: &mut ChatWidget, task_id: &str) {
    if let Some(state) = widget.spec_auto_state.as_mut() {
        if let Some(wait) = state.waiting_guardrail.as_mut() {
            if wait.task_id.is_none() {
                wait.task_id = Some(task_id.to_string());
            }
        }
    }
}

/// Handle spec-auto task completion (guardrail finished)
pub fn on_spec_auto_task_complete(widget: &mut ChatWidget, task_id: &str) {
    let _start = std::time::Instant::now(); // T90: Metrics instrumentation

    let (spec_id, stage) = {
        let Some(state) = widget.spec_auto_state.as_mut() else {
            return;
        };
        let Some(wait) = state.waiting_guardrail.take() else {
            return;
        };
        let Some(expected_id) = wait.task_id.as_deref() else {
            state.waiting_guardrail = Some(wait);
            return;
        };
        if expected_id != task_id {
            state.waiting_guardrail = Some(wait);
            return;
        }
        (state.spec_id.clone(), wait.stage)
    };

    match widget.collect_guardrail_outcome(&spec_id, stage) {
        Ok(outcome) => {
            {
                let Some(state) = widget.spec_auto_state.as_mut() else {
                    return;
                };
                let mut prompt_summary = outcome.summary.clone();
                if !outcome.failures.is_empty() {
                    prompt_summary.push_str(" | Failures: ");
                    prompt_summary.push_str(&outcome.failures.join(", "));
                }
                state.pending_prompt_summary = Some(prompt_summary);
            }

            let mut lines: Vec<ratatui::text::Line<'static>> = Vec::new();
            lines.push(ratatui::text::Line::from(format!(
                "[Spec Ops] {} stage: {}",
                stage.display_name(),
                outcome.summary
            )));
            if let Some(path) = &outcome.telemetry_path {
                lines.push(ratatui::text::Line::from(format!(
                    "  Telemetry: {}",
                    path.display()
                )));
            }
            if !outcome.failures.is_empty() {
                for failure in &outcome.failures {
                    lines.push(ratatui::text::Line::from(format!("  • {failure}")));
                }
            }
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                lines,
                HistoryCellType::Notice,
            ));

            if !outcome.success {
                if stage == SpecStage::Validate {
                    let (exhausted, retry_message, completion) = {
                        let Some(state) = widget.spec_auto_state.as_mut() else {
                            return;
                        };
                        const SPEC_AUTO_MAX_VALIDATE_RETRIES: u32 = 2;

                        let completion = state.reset_validate_run(ValidateCompletionReason::Failed);

                        let exhausted = if state.validate_retries >= SPEC_AUTO_MAX_VALIDATE_RETRIES
                        {
                            true
                        } else {
                            state.validate_retries += 1;
                            let insert_at = state.current_index + 1;
                            state.stages.splice(
                                insert_at..insert_at,
                                vec![SpecStage::Implement, SpecStage::Validate],
                            );
                            false
                        };

                        let retry_message = if exhausted {
                            None
                        } else {
                            Some(format!(
                                "Retrying implementation/validation cycle (attempt {}).",
                                state.validate_retries + 1
                            ))
                        };

                        (exhausted, retry_message, completion)
                    };

                    if let Some(completion) = completion {
                        record_validate_lifecycle_event(
                            widget,
                            &spec_id,
                            &completion.run_id,
                            completion.attempt,
                            completion.dedupe_count,
                            completion.payload_hash.as_str(),
                            completion.mode,
                            ValidateLifecycleEvent::Failed,
                        );
                    }

                    if exhausted {
                        cleanup_spec_auto_with_cancel(
                            widget,
                            "Validation failed repeatedly after maximum retry attempts"
                        );
                        return;
                    }

                    if let Some(message) = retry_message {
                        widget.history_push(crate::history_cell::PlainHistoryCell::new(
                            vec![ratatui::text::Line::from(message)],
                            HistoryCellType::Notice,
                        ));
                    }
                } else {
                    cleanup_spec_auto_with_cancel(
                        widget,
                        "Guardrail step failed"
                    );
                    return;
                }
            }

            // FORK-SPECIFIC (just-every/code): Use async MCP consensus with retry
            let consensus_result = match tokio::runtime::Handle::try_current() {
                Ok(handle) => handle.block_on(run_consensus_with_retry(
                    widget.mcp_manager.clone(),
                    widget.config.cwd.clone(),
                    spec_id.clone(),
                    stage,
                    widget.spec_kit_telemetry_enabled(),
                )),
                Err(_) => Err(super::error::SpecKitError::from_string(
                    "Tokio runtime not available".to_string(),
                )),
            };

            match consensus_result {
                Ok((consensus_lines, ok)) => {
                    let cell = crate::history_cell::PlainHistoryCell::new(
                        consensus_lines,
                        if ok {
                            HistoryCellType::Notice
                        } else {
                            HistoryCellType::Error
                        },
                    );
                    widget.history_push(cell);
                    if !ok {
                        cleanup_spec_auto_with_cancel(
                            widget,
                            &format!("Consensus not reached for {}, manual resolution required", stage.display_name())
                        );
                        return;
                    }
                }
                Err(err) => {
                    cleanup_spec_auto_with_cancel(
                        widget,
                        &format!("Consensus check failed for {}: {}", stage.display_name(), err)
                    );
                    return;
                }
            }

            // After guardrail success and consensus check OK, auto-submit multi-agent prompt
            auto_submit_spec_stage_prompt(widget, stage, &spec_id);
        }
        Err(err) => {
            cleanup_spec_auto_with_cancel(
                widget,
                &format!("Unable to read telemetry for {}: {}", stage.display_name(), err)
            );
        }
    }
}


/// Check consensus and advance to next stage
// FORK-SPECIFIC (just-every/code): Made async-aware for native MCP
pub(crate) fn check_consensus_and_advance_spec_auto(widget: &mut ChatWidget) {
    let Some(state) = widget.spec_auto_state.as_ref() else {
        return;
    };

    let Some(current_stage) = state.current_stage() else {
        halt_spec_auto_with_error(widget, "Invalid stage index".to_string());
        return;
    };

    let spec_id = state.spec_id.clone();
    let retry_count = state.agent_retry_count; // FORK-SPECIFIC: Track retries

    let mut active_validate_info: Option<ValidateRunInfo> = None;
    if current_stage == SpecStage::Validate {
        if let Some(info) = state.active_validate_run() {
            match state.mark_validate_checking(&info.run_id) {
                Some(updated) => {
                    active_validate_info = Some(updated.clone());
                    record_validate_lifecycle_event(
                        widget,
                        &spec_id,
                        &updated.run_id,
                        updated.attempt,
                        updated.dedupe_count,
                        updated.payload_hash.as_str(),
                        updated.mode,
                        ValidateLifecycleEvent::CheckingConsensus,
                    );
                }
                None => {
                    widget.history_push(crate::history_cell::PlainHistoryCell::new(
                        vec![ratatui::text::Line::from(
                            "⚠ Received validate completion without active run; ignoring.",
                        )],
                        HistoryCellType::Notice,
                    ));
                    return;
                }
            }
        } else {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(
                    "⚠ Validate consensus callback arrived after lifecycle reset; skipping.",
                )],
                HistoryCellType::Notice,
            ));
            return;
        }
    }

    // Show checking status
    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Checking consensus for {}...",
            current_stage.display_name()
        ))],
        HistoryCellType::Notice,
    ));

    // FORK-SPECIFIC (just-every/code): Run consensus check via async MCP with retry logic
    let consensus_result = block_on_sync(|| {
        let mcp = widget.mcp_manager.clone();
        let cwd = widget.config.cwd.clone();
        let spec = spec_id.clone();
        let telemetry_enabled = widget.spec_kit_telemetry_enabled();
        async move { run_consensus_with_retry(mcp, cwd, spec, current_stage, telemetry_enabled).await }
    });

    match consensus_result {
        Ok((consensus_lines, consensus_ok)) => {
            // FORK-SPECIFIC: Detect empty/invalid results and retry (just-every/code)
            let results_empty_or_invalid = consensus_lines.iter().any(|line| {
                let text = line.to_string();
                text.contains("No structured local-memory entries")
                    || text.contains("No consensus artifacts")
                    || text.contains("Missing agent artifacts")
                    || text.contains("No local-memory entries found")
            });

            if (results_empty_or_invalid || !consensus_ok)
                && retry_count < SPEC_AUTO_AGENT_RETRY_ATTEMPTS
            {
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    consensus_lines.clone(),
                    HistoryCellType::Notice,
                ));

                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from(format!(
                        "⚠ Empty/invalid agent results. Retrying {}/{} ...",
                        retry_count + 1,
                        SPEC_AUTO_AGENT_RETRY_ATTEMPTS
                    ))],
                    crate::history_cell::HistoryCellType::Notice,
                ));

                if current_stage == SpecStage::Validate {
                    if let Some(state_ref) = widget.spec_auto_state.as_ref() {
                        if let Some(completion) =
                            state_ref.reset_validate_run(ValidateCompletionReason::Reset)
                        {
                            record_validate_lifecycle_event(
                                widget,
                                &spec_id,
                                &completion.run_id,
                                completion.attempt,
                                completion.dedupe_count,
                                completion.payload_hash.as_str(),
                                completion.mode,
                                ValidateLifecycleEvent::Reset,
                            );
                        }
                    }
                }

                // Increment retry and re-execute
                if let Some(state) = widget.spec_auto_state.as_mut() {
                    state.agent_retry_count += 1;
                    state.agent_retry_context = Some(format!(
                        "Previous attempt returned invalid/empty results (retry {}/{}). Store ALL analysis in local-memory with remember command.",
                        retry_count + 1,
                        SPEC_AUTO_AGENT_RETRY_ATTEMPTS
                    ));
                    // Reset to Guardrail phase to re-run the stage
                    state.phase = SpecAutoPhase::Guardrail;
                    state.reset_cost_tracking(current_stage);
                }

                // Re-trigger guardrail → agents for this stage
                advance_spec_auto(widget);
                return;
            }

            // Show consensus result
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                consensus_lines,
                if consensus_ok {
                    HistoryCellType::Notice
                } else {
                    HistoryCellType::Error
                },
            ));

            if consensus_ok {
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from(format!(
                        "✓ {} consensus OK - advancing to next stage",
                        current_stage.display_name()
                    ))],
                    HistoryCellType::Notice,
                ));

                if current_stage == SpecStage::Validate {
                    if let Some(state_ref) = widget.spec_auto_state.as_ref() {
                        if let Some(info) = active_validate_info.as_ref() {
                            if let Some(completion) = state_ref.complete_validate_run(
                                &info.run_id,
                                ValidateCompletionReason::Completed,
                            ) {
                                record_validate_lifecycle_event(
                                    widget,
                                    &spec_id,
                                    &completion.run_id,
                                    completion.attempt,
                                    completion.dedupe_count,
                                    completion.payload_hash.as_str(),
                                    completion.mode,
                                    ValidateLifecycleEvent::Completed,
                                );
                            }
                        }
                    }
                }

                persist_cost_summary(widget, &spec_id);

                // FORK-SPECIFIC: Reset retry counter on success
                if let Some(state) = widget.spec_auto_state.as_mut() {
                    state.agent_retry_count = 0;
                    state.agent_retry_context = None;
                    state.reset_cost_tracking(current_stage);
                    state.phase = SpecAutoPhase::Guardrail;
                    state.current_index += 1;
                }

                // Trigger next stage
                advance_spec_auto(widget);
            } else {
                if current_stage == SpecStage::Validate {
                    if let Some(state_ref) = widget.spec_auto_state.as_ref() {
                        if let Some(completion) =
                            state_ref.reset_validate_run(ValidateCompletionReason::Failed)
                        {
                            record_validate_lifecycle_event(
                                widget,
                                &spec_id,
                                &completion.run_id,
                                completion.attempt,
                                completion.dedupe_count,
                                completion.payload_hash.as_str(),
                                completion.mode,
                                ValidateLifecycleEvent::Failed,
                            );
                        }
                    }
                }
                // Consensus failed after retries
                halt_spec_auto_with_error(
                    widget,
                    format!(
                        "Consensus failed for {} after {} retries",
                        current_stage.display_name(),
                        retry_count
                    ),
                );
            }
        }
        Err(err) => {
            // FORK-SPECIFIC: Retry on consensus errors (just-every/code)
            if retry_count < SPEC_AUTO_AGENT_RETRY_ATTEMPTS {
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from(format!(
                        "⚠ Consensus error: {}. Retrying {}/{} ...",
                        err,
                        retry_count + 1,
                        SPEC_AUTO_AGENT_RETRY_ATTEMPTS
                    ))],
                    crate::history_cell::HistoryCellType::Notice,
                ));

                if current_stage == SpecStage::Validate {
                    if let Some(state_ref) = widget.spec_auto_state.as_ref() {
                        if let Some(completion) =
                            state_ref.reset_validate_run(ValidateCompletionReason::Reset)
                        {
                            record_validate_lifecycle_event(
                                widget,
                                &spec_id,
                                &completion.run_id,
                                completion.attempt,
                                completion.dedupe_count,
                                completion.payload_hash.as_str(),
                                completion.mode,
                                ValidateLifecycleEvent::Reset,
                            );
                        }
                    }
                }

                if let Some(state) = widget.spec_auto_state.as_mut() {
                    state.agent_retry_count += 1;
                    state.agent_retry_context = Some(format!(
                        "Previous attempt had consensus error (retry {}/{}). Ensure proper local-memory storage.",
                        retry_count + 1,
                        SPEC_AUTO_AGENT_RETRY_ATTEMPTS
                    ));
                    state.phase = SpecAutoPhase::Guardrail;
                    state.reset_cost_tracking(current_stage);
                }

                advance_spec_auto(widget);
                return;
            }

            if current_stage == SpecStage::Validate {
                if let Some(state_ref) = widget.spec_auto_state.as_ref() {
                    if let Some(completion) =
                        state_ref.reset_validate_run(ValidateCompletionReason::Failed)
                    {
                        record_validate_lifecycle_event(
                            widget,
                            &spec_id,
                            &completion.run_id,
                            completion.attempt,
                            completion.dedupe_count,
                            completion.payload_hash.as_str(),
                            completion.mode,
                            ValidateLifecycleEvent::Failed,
                        );
                    }
                }
            }

            halt_spec_auto_with_error(
                widget,
                format!(
                    "Consensus check failed for {} after {} retries: {}",
                    current_stage.display_name(),
                    retry_count,
                    err
                ),
            );
        }
    }
}

// === Quality Gate Handlers ===
// MAINT-2: Extracted to quality_gate_handler.rs (925 LOC)
// Re-exported from mod.rs for backward compatibility

pub use super::quality_gate_handler::{
    on_quality_gate_agents_complete, on_quality_gate_answers, on_quality_gate_broker_result,
    on_quality_gate_cancelled, on_quality_gate_validation_result,
};

// Internal quality gate helpers (called from advance_spec_auto)
use super::quality_gate_handler::{
    determine_quality_checkpoint, execute_quality_checkpoint, finalize_quality_gates,
};
