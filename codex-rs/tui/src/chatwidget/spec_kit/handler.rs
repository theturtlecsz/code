//! Spec-Kit command handlers as free functions
//!
//! These functions are extracted from chatwidget.rs to isolate spec-kit code.
//! Using free functions instead of methods to avoid Rust borrow checker issues.

use super::super::ChatWidget; // Parent module (friend access to private fields)
use super::consensus::{expected_agents_for_stage, parse_consensus_stage};
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
use codex_core::protocol::{AgentInfo, InputItem};
use codex_core::slash_commands::format_subagent_command;
use serde_json::json;
use std::sync::Arc;

// Re-export command handlers for backward compatibility
pub use super::command_handlers::{
    halt_spec_auto_with_error, handle_guardrail, handle_spec_consensus, handle_spec_status,
};

// FORK-SPECIFIC (just-every/code): MCP retry configuration
const MCP_RETRY_ATTEMPTS: u32 = 3;
const MCP_RETRY_DELAY_MS: u64 = 100;

// FORK-SPECIFIC (just-every/code): Spec-auto agent retry configuration
const SPEC_AUTO_AGENT_RETRY_ATTEMPTS: u32 = 3;

fn block_on_sync<F, Fut, T>(factory: F) -> T
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

/// Helper to run consensus check with retry logic for MCP initialization
/// FORK-SPECIFIC (just-every/code): Handles MCP connection timing and transient failures
async fn run_consensus_with_retry(
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

        match super::consensus::run_spec_consensus(
            &cwd,
            &spec_id,
            stage,
            telemetry_enabled,
            manager,
        )
        .await
        {
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

/// Auto-submit multi-agent prompt for spec stage
pub fn auto_submit_spec_stage_prompt(widget: &mut ChatWidget, stage: SpecStage, spec_id: &str) {
    let goal = widget
        .spec_auto_state
        .as_ref()
        .map(|s| s.goal.clone())
        .unwrap_or_default();

    let mut arg = spec_id.to_string();
    if !goal.trim().is_empty() {
        arg.push(' ');
        arg.push_str(goal.trim());
    }

    // FORK-SPECIFIC (just-every/code): Pass MCP manager for native context gathering (ARCH-004)
    let mcp_manager_ref = block_on_sync(|| {
        let manager = widget.mcp_manager.clone();
        async move { manager.lock().await.as_ref().cloned() }
    });

    let prompt_result = if let Some(manager) = mcp_manager_ref.clone() {
        crate::spec_prompts::build_stage_prompt_with_mcp(stage, &arg, Some(manager))
    } else {
        crate::spec_prompts::build_stage_prompt(stage, &arg)
    };

    match prompt_result {
        Ok(prompt) => {
            // SPEC-KIT-070: ACE-aligned routing — set aggregator effort per stage
            // Estimate tokens ~ chars/4, escalate on conflict retry
            let prior_conflict_retry = widget
                .spec_auto_state
                .as_ref()
                .map(|s| s.agent_retry_count > 0)
                .unwrap_or(false);

            let routing = super::ace_route_selector::decide_stage_routing(
                stage,
                prompt.len(),
                prior_conflict_retry,
            );

            // Apply aggregator effort by updating gpt_pro args in-session
            apply_aggregator_effort(widget, routing.aggregator_effort);

            // Persist notes in state for cost summary sidecar
            if let Some(state) = widget.spec_auto_state.as_mut() {
                state
                    .aggregator_effort_notes
                    .insert(stage, routing.aggregator_effort.as_str().to_string());
                if let Some(reason) = routing.escalation_reason.as_ref() {
                    state
                        .escalation_reason_notes
                        .insert(stage, reason.clone());
                }
            }
            let mut validate_context: Option<(ValidateRunInfo, String)> = None;

            if stage == SpecStage::Validate {
                let payload_hash = compute_validate_payload_hash(
                    ValidateMode::Auto,
                    stage,
                    spec_id,
                    prompt.as_str(),
                );

                let Some(state_ref) = widget.spec_auto_state.as_ref() else {
                    widget.history_push(crate::history_cell::new_error_event(
                        "No spec-auto state available for validate dispatch.".to_string(),
                    ));
                    return;
                };

                match state_ref.begin_validate_run(&payload_hash) {
                    ValidateBeginOutcome::Started(info) => {
                        record_validate_lifecycle_event(
                            widget,
                            spec_id,
                            &info.run_id,
                            info.attempt,
                            info.dedupe_count,
                            &payload_hash,
                            info.mode,
                            ValidateLifecycleEvent::Queued,
                        );
                        validate_context = Some((info, payload_hash));
                    }
                    ValidateBeginOutcome::Duplicate(info)
                    | ValidateBeginOutcome::Conflict(info) => {
                        record_validate_lifecycle_event(
                            widget,
                            spec_id,
                            &info.run_id,
                            info.attempt,
                            info.dedupe_count,
                            &payload_hash,
                            info.mode,
                            ValidateLifecycleEvent::Deduped,
                        );

                        widget.history_push(crate::history_cell::PlainHistoryCell::new(
                            vec![
                                ratatui::text::Line::from(format!(
                                    "⚠ Validate run already active (run_id: {}, attempt: {})",
                                    info.run_id, info.attempt
                                )),
                                ratatui::text::Line::from(
                                    "Skipping duplicate auto dispatch; awaiting current run.",
                                ),
                            ],
                            HistoryCellType::Notice,
                        ));
                        return;
                    }
                }
            }

            let mut lines: Vec<ratatui::text::Line<'static>> = Vec::new();
            lines.push(ratatui::text::Line::from(format!(
                "Auto-executing multi-agent {} for {}",
                stage.display_name(),
                spec_id
            )));
            lines.push(ratatui::text::Line::from(
                "Launching Gemini, Claude, and GPT Pro...",
            ));

            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                lines,
                HistoryCellType::Notice,
            ));

            let stage_expected: Vec<String> = expected_agents_for_stage(stage)
                .into_iter()
                .filter_map(|agent| {
                    let canonical = agent.canonical_name().to_string();
                    widget
                        .config
                        .agents
                        .iter()
                        .find(|cfg| cfg.enabled && cfg.name.eq_ignore_ascii_case(&canonical))
                        .map(|_| canonical)
                })
                .collect();

            if let Some(state) = widget.spec_auto_state.as_mut() {
                state.phase = SpecAutoPhase::ExecutingAgents {
                    expected_agents: stage_expected,
                    completed_agents: std::collections::HashSet::new(),
                };

                if stage == SpecStage::Validate {
                    if let Some((info, payload_hash)) = validate_context.as_mut() {
                        if let Some(updated) = state.mark_validate_dispatched(&info.run_id) {
                            *info = updated.clone();
                            record_validate_lifecycle_event(
                                widget,
                                spec_id,
                                &updated.run_id,
                                updated.attempt,
                                updated.dedupe_count,
                                payload_hash.as_str(),
                                updated.mode,
                                ValidateLifecycleEvent::Dispatched,
                            );
                        }
                    }
                }
            }

            let user_msg = super::super::message::UserMessage {
                display_text: format!("[spec-auto] {} stage for {}", stage.display_name(), spec_id),
                ordered_items: vec![InputItem::Text { text: prompt }],
            };

            widget.submit_user_message(user_msg);
        }
        Err(err) => {
            halt_spec_auto_with_error(
                widget,
                format!("Failed to build {} prompt: {}", stage.display_name(), err),
            );
        }
    }
}

/// SPEC-KIT-070: Update aggregator (gpt_pro) reasoning effort dynamically
fn apply_aggregator_effort(widget: &mut ChatWidget, effort: super::ace_route_selector::AggregatorEffort) {
    let effort_str = effort.as_str();
    // Find existing config for gpt_pro
    let ro_default = vec![
        "exec".into(),
        "--sandbox".into(),
        "read-only".into(),
        "--skip-git-repo-check".into(),
        "--model".into(),
        "gpt-5-codex".into(),
    ];
    let wr_default = vec![
        "exec".into(),
        "--sandbox".into(),
        "workspace-write".into(),
        "--skip-git-repo-check".into(),
        "--model".into(),
        "gpt-5-codex".into(),
    ];

    // Build new args by taking current args and replacing/adding the effort flag
    let (args_ro, args_wr) = {
        let cfg = widget
            .config
            .agents
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case("gpt_pro"))
            .cloned();

        fn upsert_effort(mut v: Vec<String>, effort: &str) -> Vec<String> {
            // Remove existing effort flag if present
            let mut i = 0;
            while i + 1 < v.len() {
                if v[i] == "-c" && v[i + 1].starts_with("model_reasoning_effort=") {
                    v.remove(i + 1);
                    v.remove(i);
                    continue;
                }
                i += 1;
            }
            v.push("-c".into());
            v.push(format!("model_reasoning_effort=\"{}\"", effort));
            v
        }

        let ro = cfg
            .as_ref()
            .and_then(|c| c.args_read_only.clone())
            .unwrap_or(ro_default.clone());
        let wr = cfg
            .as_ref()
            .and_then(|c| c.args_write.clone())
            .unwrap_or(wr_default.clone());
        (upsert_effort(ro, effort_str), upsert_effort(wr, effort_str))
    };

    widget.apply_agent_update("gpt_pro", true, Some(args_ro), Some(args_wr), None);
}

pub(crate) fn schedule_degraded_follow_up(
    widget: &mut ChatWidget,
    stage: SpecStage,
    spec_id: &str,
) {
    if let Some(state) = widget.spec_auto_state.as_mut() {
        if !state.degraded_followups.insert(stage) {
            return;
        }
    }

    let formatted = format_subagent_command(
        "speckit.checklist",
        spec_id,
        Some(&widget.config.agents),
        Some(&widget.config.subagent_commands),
    );

    let user_msg = super::super::message::UserMessage {
        display_text: format!(
            "[spec-auto] Follow-up checklist for {} ({})",
            spec_id,
            stage.display_name()
        ),
        ordered_items: vec![InputItem::Text {
            text: formatted.prompt,
        }],
    };

    widget.submit_user_message(user_msg);
}

/// Handle all agents completing their tasks
pub fn on_spec_auto_agents_complete(widget: &mut ChatWidget) {
    let Some(state) = widget.spec_auto_state.as_ref() else {
        return;
    };

    // Check which phase we're in
    let expected_agents = match &state.phase {
        SpecAutoPhase::ExecutingAgents {
            expected_agents, ..
        } => expected_agents.clone(),
        SpecAutoPhase::QualityGateExecuting {
            expected_agents, ..
        } => expected_agents.clone(),
        _ => return, // Not in agent execution phase
    };

    // Collect which agents completed successfully
    let mut completed_names = std::collections::HashSet::new();
    for agent_info in &widget.active_agents {
        if matches!(agent_info.status, super::super::AgentStatus::Completed) {
            completed_names.insert(agent_info.name.to_lowercase());
        }
    }

    // Update completed agents in state and determine phase type
    let phase_type = if let Some(state) = widget.spec_auto_state.as_mut() {
        match &mut state.phase {
            SpecAutoPhase::ExecutingAgents {
                completed_agents, ..
            } => {
                *completed_agents = completed_names.clone();
                "regular"
            }
            SpecAutoPhase::QualityGateExecuting {
                completed_agents, ..
            } => {
                *completed_agents = completed_names.clone();
                "quality_gate"
            }
            SpecAutoPhase::QualityGateValidating { .. } => {
                // GPT-5 validation phase - single agent (GPT-5)
                "gpt5_validation"
            }
            _ => "none",
        }
    } else {
        "none"
    };

    // Handle different phase types
    match phase_type {
        "quality_gate" => {
            if !completed_names.is_empty() {
                on_quality_gate_agents_complete(widget);
            }
        }
        "gpt5_validation" => {
            if let Some(state) = widget.spec_auto_state.as_ref() {
                if let SpecAutoPhase::QualityGateValidating { checkpoint, .. } = state.phase {
                    widget
                        .quality_gate_broker
                        .fetch_validation_payload(state.spec_id.clone(), checkpoint);
                }
            }
        }
        "regular" => {
            // Regular stage agents
            let all_complete = expected_agents
                .iter()
                .all(|exp| completed_names.contains(&exp.to_lowercase()));

            if all_complete {
                if let Some(state) = widget.spec_auto_state.as_mut() {
                    state.phase = SpecAutoPhase::CheckingConsensus;
                }
                check_consensus_and_advance_spec_auto(widget);
            }
        }
        _ => {}
    }

    // Check for failures in any phase
    if !matches!(phase_type, "gpt5_validation") {
        // Check for failed agents
        let has_failures = widget
            .active_agents
            .iter()
            .any(|a| matches!(a.status, super::super::AgentStatus::Failed));

        if has_failures {
            // FORK-SPECIFIC: Retry logic for failed agents (just-every/code)
            let (retry_count, current_stage) = {
                let Some(state) = widget.spec_auto_state.as_ref() else {
                    return;
                };
                let Some(stage) = state.current_stage() else {
                    return;
                };
                (state.agent_retry_count, stage)
            };

            if retry_count < SPEC_AUTO_AGENT_RETRY_ATTEMPTS {
                // Retry the stage
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from(format!(
                        "⚠ Agent failures detected. Retrying {}/{} ...",
                        retry_count + 1,
                        SPEC_AUTO_AGENT_RETRY_ATTEMPTS
                    ))],
                    crate::history_cell::HistoryCellType::Notice,
                ));

                // Increment retry count
                if let Some(state) = widget.spec_auto_state.as_mut() {
                    state.agent_retry_count += 1;
                    state.agent_retry_context = Some(format!(
                        "Previous attempt failed (retry {}/{}). Be more thorough and check for edge cases.",
                        retry_count + 1,
                        SPEC_AUTO_AGENT_RETRY_ATTEMPTS
                    ));
                    state.reset_cost_tracking(current_stage);
                }

                // Re-execute the stage with retry context
                let Some(state) = widget.spec_auto_state.as_ref() else {
                    return;
                };
                let spec_id = state.spec_id.clone();
                auto_submit_spec_stage_prompt(widget, current_stage, &spec_id);
                return;
            } else {
                // Max retries exhausted
                let missing: Vec<_> = expected_agents
                    .iter()
                    .filter(|exp| !completed_names.contains(&exp.to_lowercase()))
                    .map(|s| s.as_str())
                    .collect();

                halt_spec_auto_with_error(
                    widget,
                    format!(
                        "Agent execution failed after {} retries. Missing/failed: {:?}",
                        SPEC_AUTO_AGENT_RETRY_ATTEMPTS, missing
                    ),
                );
            }
        }
    }
}

/// Check consensus and advance to next stage
// FORK-SPECIFIC (just-every/code): Made async-aware for native MCP
fn check_consensus_and_advance_spec_auto(widget: &mut ChatWidget) {
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

fn persist_cost_summary(widget: &mut ChatWidget, spec_id: &str) {
    let dir = widget.cost_summary_dir();
    // Attach routing notes for the just-finished stage (if available)
    if let Some(state) = widget.spec_auto_state.as_ref() {
        if let Some(stage) = state.current_stage() {
            let effort = state.aggregator_effort_notes.get(&stage).cloned();
            let reason = state.escalation_reason_notes.get(&stage).cloned();
            widget
                .spec_cost_tracker()
                .set_stage_routing_note(spec_id, stage, effort.as_deref(), reason.as_deref());
        }
    }
    if let Err(err) = widget
        .spec_cost_tracker()
        .write_summary(spec_id, &dir)
    {
        widget.history_push(crate::history_cell::new_warning_event(format!(
            "Failed to write cost summary for {}: {}",
            spec_id, err
        )));
    }
}

pub fn record_agent_costs(widget: &mut ChatWidget, agents: &[AgentInfo]) {
    let tracker = widget.spec_cost_tracker();
    let mut spec_id: Option<String> = None;
    let mut stage_slot: Option<SpecStage> = None;
    let mut to_record: Vec<&AgentInfo> = Vec::new();

    {
        let Some(state) = widget.spec_auto_state.as_mut() else {
            return;
        };
        let Some(stage) = state.current_stage() else {
            return;
        };

        let tracking_active = matches!(
            state.phase,
            SpecAutoPhase::ExecutingAgents { .. } | SpecAutoPhase::QualityGateExecuting { .. }
        );
        if !tracking_active {
            return;
        }

        spec_id = Some(state.spec_id.clone());
        stage_slot = Some(stage);

        for info in agents {
            let status = info.status.to_lowercase();
            if status != "completed" && status != "failed" {
                continue;
            }

            if state.mark_agent_cost_recorded(stage, &info.id) {
                to_record.push(info);
            }
        }
    }

    let Some(spec_id) = spec_id else {
        return;
    };
    let Some(stage) = stage_slot else {
        return;
    };

    for info in to_record {
        let model = info.model.as_deref().unwrap_or("unknown");
        let usage = widget.last_token_usage.clone();
        let (_cost, alert) = tracker.record_agent_call(
            &spec_id,
            stage,
            model,
            usage.input_tokens,
            usage.output_tokens,
        );

        if let Some(alert) = alert {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(alert.to_user_message())],
                HistoryCellType::Notice,
            ));
        }
    }
}


/// Handle /spec-consensus command implementation
///
/// Made pub(crate) so command_handlers module can delegate to it.
pub(crate) fn handle_spec_consensus_impl(widget: &mut ChatWidget, raw_args: String) {
    let trimmed = raw_args.trim();
    if trimmed.is_empty() {
        widget.history_push(crate::history_cell::new_error_event(
            "Usage: /spec-consensus <SPEC-ID> <stage>".to_string(),
        ));
        return;
    }

    let mut parts = trimmed.split_whitespace();
    let Some(spec_id) = parts.next() else {
        widget.history_push(crate::history_cell::new_error_event(
            "Usage: /spec-consensus <SPEC-ID> <stage>".to_string(),
        ));
        return;
    };

    let Some(stage_str) = parts.next() else {
        widget.history_push(crate::history_cell::new_error_event(
            "Usage: /spec-consensus <SPEC-ID> <stage>".to_string(),
        ));
        return;
    };

    let Some(stage) = parse_consensus_stage(stage_str) else {
        widget.history_push(crate::history_cell::new_error_event(format!(
            "Unknown stage '{stage_str}'. Expected plan, tasks, implement, validate, audit, or unlock.",
        )));
        return;
    };

    // FORK-SPECIFIC (just-every/code): Use async MCP consensus with retry
    let consensus_result = block_on_sync(|| {
        let mcp = widget.mcp_manager.clone();
        let cwd = widget.config.cwd.clone();
        let spec = spec_id.to_string();
        let telemetry_enabled = widget.spec_kit_telemetry_enabled();
        async move { run_consensus_with_retry(mcp, cwd, spec, stage, telemetry_enabled).await }
    });

    match consensus_result {
        Ok((lines, ok)) => {
            let cell = crate::history_cell::PlainHistoryCell::new(
                lines,
                if ok {
                    HistoryCellType::Notice
                } else {
                    HistoryCellType::Error
                },
            );
            widget.history_push(cell);
        }
        Err(err) => {
            widget.history_push(crate::history_cell::new_error_event(err.to_string()));
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
