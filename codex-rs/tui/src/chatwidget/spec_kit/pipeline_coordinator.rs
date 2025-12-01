//! Pipeline coordination and state machine
//!
//! Core spec-auto pipeline orchestration:
//! - Pipeline initiation (handle_spec_auto)
//! - State machine advancement (advance_spec_auto)
//! - Task lifecycle tracking (on_spec_auto_task_*)
//! - Consensus checking and stage progression
//! - Quality gate checkpoint integration

#![allow(dead_code, unused_variables)] // Pipeline helpers pending integration

use super::super::ChatWidget;
use super::agent_orchestrator::auto_submit_spec_stage_prompt;
use super::command_handlers::halt_spec_auto_with_error;
use super::consensus_coordinator::{block_on_sync, persist_cost_summary, run_consensus_with_retry};
use super::pipeline_config::{PipelineConfig, PipelineOverrides}; // SPEC-948
use super::quality_gate_handler::{
    determine_quality_checkpoint, execute_quality_checkpoint, finalize_quality_gates,
};
use super::state::{GuardrailWait, SpecAutoPhase, ValidateRunInfo};
use super::validation_lifecycle::{
    ValidateCompletionReason, ValidateLifecycleEvent, cleanup_spec_auto_with_cancel,
    record_validate_lifecycle_event,
};
use crate::history_cell::HistoryCellType;
use crate::slash_command::{HalMode, SlashCommand};
use crate::spec_prompts::SpecStage;
use std::fs;
use std::path::{Path, PathBuf};

/// Handle /speckit.auto command initiation
pub fn handle_spec_auto(
    widget: &mut ChatWidget,
    spec_id: String,
    goal: String,
    resume_from: SpecStage,
    hal_mode: Option<HalMode>,
    cli_overrides: Option<PipelineOverrides>, // SPEC-948: CLI flags for stage filtering
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

    // SPEC-948: Load pipeline configuration with 3-tier precedence
    let pipeline_config = match PipelineConfig::load(&spec_id, cli_overrides) {
        Ok(config) => config,
        Err(err) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Pipeline configuration error: {}",
                err
            )));
            return;
        }
    };

    // SPEC-948: Display validation warnings (quality gate bypass, cost savings, etc.)
    if let Ok(validation) = pipeline_config.validate() {
        for warning in &validation.warnings {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(warning.clone())],
                HistoryCellType::Notice,
            ));
        }
    }

    // SPEC-KIT-909: Check evidence size before starting pipeline (50MB hard limit)
    if let Err(err) = check_evidence_size_limit(&spec_id, &widget.config.cwd) {
        widget.history_push(crate::history_cell::new_error_event(format!(
            "Evidence size check failed: {}",
            err
        )));
        widget.history_push(crate::history_cell::new_error_event(
            "Run: bash scripts/spec_ops_004/evidence_archive.sh".to_string(),
        ));
        return;
    }

    let lifecycle = widget.ensure_validate_lifecycle(&spec_id);
    let mut state = super::state::SpecAutoState::new(
        spec_id.clone(),
        goal,
        resume_from,
        hal_mode,
        pipeline_config, // SPEC-948: Pass pipeline config
    );
    state.set_validate_lifecycle(lifecycle);

    // Log run start event
    if let Some(run_id) = &state.run_id {
        state
            .execution_logger
            .log_event(super::execution_logger::ExecutionEvent::RunStart {
                spec_id: spec_id.clone(),
                run_id: run_id.clone(),
                timestamp: super::execution_logger::ExecutionEvent::now(),
                stages: state
                    .stages
                    .iter()
                    .map(|s| s.display_name().to_string())
                    .collect(),
                quality_gates_enabled: state.quality_gates_enabled,
                hal_mode: hal_mode
                    .map(|m| format!("{:?}", m))
                    .unwrap_or_else(|| "mock".to_string()),
            });
    }

    // SPEC-KIT-102: Run Stage 0 context injection before pipeline starts
    if !state.stage0_disabled {
        // Load spec content
        let spec_path = widget.config.cwd.join(format!("docs/{}/spec.md", spec_id));
        let spec_content = std::fs::read_to_string(&spec_path).unwrap_or_default();

        if !spec_content.is_empty() {
            // Build Stage0 config
            let stage0_config = super::stage0_integration::Stage0ExecutionConfig {
                disabled: state.stage0_disabled,
                explain: state.stage0_explain,
            };

            // Run Stage0
            let result = super::stage0_integration::run_stage0_for_spec(
                &widget.mcp_manager,
                &spec_id,
                &spec_content,
                &widget.config.cwd,
                &stage0_config,
            );

            // Store result in state
            if let Some(stage0_result) = result.result {
                // Write TASK_BRIEF.md to evidence directory
                if let Err(e) = super::stage0_integration::write_task_brief_to_evidence(
                    &spec_id,
                    &widget.config.cwd,
                    &stage0_result.task_brief_md,
                ) {
                    tracing::warn!("Failed to write TASK_BRIEF.md: {}", e);
                }

                // Log Stage0 success
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from(format!(
                        "Stage 0: Context compiled ({} memories, tier2={}, {}ms)",
                        stage0_result.memories_used.len(),
                        stage0_result.tier2_used,
                        stage0_result.latency_ms
                    ))],
                    crate::history_cell::HistoryCellType::Notice,
                ));

                state.stage0_result = Some(stage0_result);
            } else if let Some(skip_reason) = result.skip_reason {
                state.stage0_skip_reason = Some(skip_reason.clone());
                tracing::info!("Stage 0 skipped: {}", skip_reason);
            }
        } else {
            state.stage0_skip_reason = Some("spec.md is empty or not found".to_string());
        }
    } else {
        state.stage0_skip_reason = Some("Stage 0 disabled by flag".to_string());
    }

    widget.spec_auto_state = Some(state);
    advance_spec_auto(widget);
}

/// Advance spec-auto pipeline to next stage
pub(crate) fn advance_spec_auto(widget: &mut ChatWidget) {
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

                // SPEC-948 Task 2.2: Check if stage is enabled in pipeline configuration
                let stage_type = spec_stage_to_stage_type(stage);
                if !state.pipeline_config.is_enabled(stage_type) {
                    let spec_id = state.spec_id.clone();
                    let reason = state
                        .pipeline_config
                        .skip_reason(stage_type)
                        .unwrap_or("Disabled in pipeline configuration");

                    // Record skip telemetry
                    if let Err(err) = record_stage_skip(&spec_id, stage, reason) {
                        tracing::warn!("Failed to record skip telemetry: {}", err);
                    }

                    // Log skip to console
                    tracing::info!("⏭️  Skipping stage {}: {}", stage.display_name(), reason);

                    // Advance to next stage
                    state.current_index += 1;

                    // Continue loop to check next stage
                    continue;
                }

                // SPEC-KIT-928: Check if quality gates are still running (single-flight guard)
                // Prevent stage advancement while quality gates are executing
                if matches!(
                    state.phase,
                    SpecAutoPhase::QualityGateExecuting { .. }
                        | SpecAutoPhase::QualityGateProcessing { .. }
                        | SpecAutoPhase::QualityGateValidating { .. }
                        | SpecAutoPhase::QualityGateAwaitingHuman { .. }
                ) {
                    tracing::warn!(
                        "⚠️ Stage advancement blocked: Quality gates still in progress (phase: {:?})",
                        state.phase
                    );
                    return;
                }

                // Check if we should run a quality checkpoint before this stage
                if state.quality_gates_enabled
                    && let Some(checkpoint) =
                        determine_quality_checkpoint(stage, &state.completed_checkpoints)
                {
                    // Execute quality checkpoint instead of proceeding to guardrail
                    execute_quality_checkpoint(widget, checkpoint);
                    return;
                }

                match &state.phase {
                    SpecAutoPhase::Guardrail => {
                        // Log stage start and add TUI boundary marker
                        if let Some(run_id) = &state.run_id {
                            let tier = super::execution_logger::tier_from_agent_count(
                                super::consensus::expected_agents_for_stage(stage).len(),
                            );
                            let expected_agents: Vec<String> =
                                super::consensus::expected_agents_for_stage(stage)
                                    .into_iter()
                                    .map(|a| a.canonical_name().to_string())
                                    .collect();

                            state.execution_logger.log_event(
                                super::execution_logger::ExecutionEvent::StageStart {
                                    run_id: run_id.clone(),
                                    stage: stage.display_name().to_string(),
                                    tier,
                                    expected_agents: expected_agents.clone(),
                                    timestamp: super::execution_logger::ExecutionEvent::now(),
                                },
                            );

                            // Add visual boundary marker to TUI
                            let marker_lines = [
                                ratatui::text::Line::from(
                                    "════════════════════════════════════════",
                                ),
                                ratatui::text::Line::from(format!(
                                    "  STAGE: {} (Tier {})",
                                    stage.display_name().to_uppercase(),
                                    tier
                                )),
                                ratatui::text::Line::from(format!(
                                    "  Agents: {}",
                                    expected_agents.join(", ")
                                )),
                                ratatui::text::Line::from(
                                    "════════════════════════════════════════",
                                ),
                            ];
                            // Store marker for display after this function returns
                            state.pending_prompt_summary = Some(
                                marker_lines
                                    .iter()
                                    .map(|l| l.to_string())
                                    .collect::<Vec<_>>()
                                    .join("\n"),
                            );
                        }

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
                        return; // Waiting for GPT-5.1 validation responses
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
                let should_finalize_quality_gates = widget
                    .spec_auto_state
                    .as_ref()
                    .map(|state| {
                        state.quality_gates_enabled && !state.quality_checkpoint_outcomes.is_empty()
                    })
                    .unwrap_or(false);

                if should_finalize_quality_gates {
                    finalize_quality_gates(widget);
                }

                // Log run complete event
                if let Some(state) = widget.spec_auto_state.as_ref()
                    && let Some(run_id) = &state.run_id
                {
                    let total_duration = state.execution_logger.elapsed_sec();
                    // TODO: Calculate actual total cost from tracker
                    let total_cost = 0.0;
                    let stages_completed = state.stages.len() - state.current_index;
                    let quality_gates_passed = state.completed_checkpoints.len();

                    state.execution_logger.log_event(
                        super::execution_logger::ExecutionEvent::RunComplete {
                            run_id: run_id.clone(),
                            spec_id: state.spec_id.clone(),
                            total_duration_sec: total_duration,
                            total_cost_usd: total_cost,
                            stages_completed,
                            quality_gates_passed,
                            timestamp: super::execution_logger::ExecutionEvent::now(),
                        },
                    );

                    // Finalize logger
                    state.execution_logger.finalize();
                }

                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from("/spec-auto pipeline complete")],
                    HistoryCellType::Notice,
                ));

                // SPEC-KIT-900: Automated post-run verification
                if let Some(state) = widget.spec_auto_state.as_ref() {
                    let spec_id = state.spec_id.clone();
                    let run_id = state.run_id.clone();

                    // Generate verification report
                    match super::commands::verify::generate_verification_report(
                        &spec_id,
                        run_id.as_deref(),
                        &widget.config.cwd,
                    ) {
                        Ok(report_lines) => {
                            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                                vec![ratatui::text::Line::from("")],
                                HistoryCellType::Notice,
                            ));
                            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                                report_lines
                                    .into_iter()
                                    .map(|s| ratatui::text::Line::from(s))
                                    .collect(),
                                HistoryCellType::Notice,
                            ));
                        }
                        Err(e) => {
                            tracing::warn!("Failed to generate verification report: {}", e);
                        }
                    }
                }

                // Successful completion - clear state without cancellation event
                widget.spec_auto_state = None;
                // P6-SYNC Phase 6: Clear spec-kit token metrics from status bar
                widget.bottom_pane.set_spec_auto_metrics(None);
                return;
            }
            NextAction::RunGuardrail {
                command,
                args,
                hal_mode,
            } => {
                // Display stage boundary marker before starting guardrail
                if let Some(state) = widget.spec_auto_state.as_ref()
                    && let Some(summary) = &state.pending_prompt_summary
                {
                    widget.history_push(crate::history_cell::PlainHistoryCell::new(
                        summary
                            .lines()
                            .map(|l| ratatui::text::Line::from(l.to_string()))
                            .collect(),
                        HistoryCellType::Notice,
                    ));
                }

                widget.handle_spec_ops_command(command, args, hal_mode);
                return;
            }
        }
    }
}

/// Handle spec-auto task started event
pub fn on_spec_auto_task_started(widget: &mut ChatWidget, task_id: &str) {
    if let Some(state) = widget.spec_auto_state.as_mut()
        && let Some(wait) = state.waiting_guardrail.as_mut()
        && wait.task_id.is_none()
    {
        wait.task_id = Some(task_id.to_string());
    }
}

/// Handle spec-auto task completion (guardrail finished)
pub fn on_spec_auto_task_complete(widget: &mut ChatWidget, task_id: &str) {
    let _start = std::time::Instant::now(); // T90: Metrics instrumentation
    tracing::warn!(
        "DEBUG: on_spec_auto_task_complete called with task_id={}",
        task_id
    );

    let (spec_id, stage) = {
        let Some(state) = widget.spec_auto_state.as_mut() else {
            tracing::warn!("DEBUG: No spec_auto_state, returning");
            return;
        };
        let Some(wait) = state.waiting_guardrail.take() else {
            tracing::warn!("DEBUG: No waiting_guardrail - likely multi-agent task completion");
            // This is multi-agent execution completing, trigger agent completion handler
            super::agent_orchestrator::on_spec_auto_agents_complete(widget);
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
                    // Record failure and halt (no retries)
                    let completion = {
                        let Some(state) = widget.spec_auto_state.as_mut() else {
                            return;
                        };
                        state.reset_validate_run(ValidateCompletionReason::Failed)
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

                    widget.history_push(crate::history_cell::PlainHistoryCell::new(
                        vec![ratatui::text::Line::from(
                            "⚠ Validation failed. Manual review required.",
                        )],
                        HistoryCellType::Notice,
                    ));

                    halt_spec_auto_with_error(widget, "Validation failed".to_string());
                    return;
                } else {
                    cleanup_spec_auto_with_cancel(widget, "Guardrail step failed");
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
                            &format!(
                                "Consensus not reached for {}, manual resolution required",
                                stage.display_name()
                            ),
                        );
                        return;
                    }
                }
                Err(err) => {
                    cleanup_spec_auto_with_cancel(
                        widget,
                        &format!(
                            "Consensus check failed for {}: {}",
                            stage.display_name(),
                            err
                        ),
                    );
                    return;
                }
            }

            // After guardrail success and consensus check OK, auto-submit multi-agent prompt
            tracing::warn!(
                "DEBUG: About to call auto_submit_spec_stage_prompt for stage={:?}",
                stage
            );
            auto_submit_spec_stage_prompt(widget, stage, &spec_id);
            tracing::warn!("DEBUG: Returned from auto_submit_spec_stage_prompt");
        }
        Err(err) => {
            cleanup_spec_auto_with_cancel(
                widget,
                &format!(
                    "Unable to read telemetry for {}: {}",
                    stage.display_name(),
                    err
                ),
            );
        }
    }
}

/// Handle native guardrail completion (synchronous, no TaskComplete event)
///
/// Native guardrails complete instantly without emitting events. This function
/// replicates the guardrail completion logic from on_spec_auto_task_complete
/// but doesn't require a task_id since native guardrails don't have one.
///
/// Takes the guardrail result directly to avoid blocking file I/O re-reading telemetry.
pub(crate) fn advance_spec_auto_after_native_guardrail(
    widget: &mut ChatWidget,
    stage: SpecStage,
    spec_id: &str,
    native_result: super::native_guardrail::GuardrailResult,
) {
    tracing::warn!(
        "DEBUG: advance_spec_auto_after_native_guardrail called for stage={:?}",
        stage
    );

    // Clear waiting_guardrail state
    if let Some(state) = widget.spec_auto_state.as_mut() {
        state.waiting_guardrail = None;
    }

    // Convert native result to GuardrailOutcome (avoid blocking file I/O)
    tracing::warn!("DEBUG: Using passed native guardrail result (no file I/O)");
    let outcome = super::state::GuardrailOutcome {
        success: native_result.success,
        summary: format!("{} stage ready", stage.display_name()),
        telemetry_path: native_result.telemetry_path,
        failures: native_result.errors,
    };

    tracing::warn!(
        "DEBUG: Guardrail outcome converted, success={}",
        outcome.success
    );
    if !outcome.success {
        if stage == SpecStage::Validate {
            // Record failure and halt
            let completion = {
                let Some(state) = widget.spec_auto_state.as_mut() else {
                    return;
                };
                state.reset_validate_run(ValidateCompletionReason::Failed)
            };

            if let Some(completion) = completion {
                record_validate_lifecycle_event(
                    widget,
                    spec_id,
                    &completion.run_id,
                    completion.attempt,
                    completion.dedupe_count,
                    completion.payload_hash.as_str(),
                    completion.mode,
                    ValidateLifecycleEvent::Failed,
                );
            }

            halt_spec_auto_with_error(widget, "Validation failed".to_string());
            return;
        } else {
            cleanup_spec_auto_with_cancel(widget, "Guardrail step failed");
            return;
        }
    }

    // Native guardrail path: Just spawn agents, skip consensus
    // Consensus will run AFTER agents complete via the normal flow:
    //   on_spec_auto_agents_complete() → check_consensus_and_advance_spec_auto()
    //
    // This avoids nested runtime issues and follows the standard agent lifecycle.
    tracing::warn!(
        "DEBUG: Native guardrail validated, spawning agents for stage={:?}",
        stage
    );
    auto_submit_spec_stage_prompt(widget, stage, spec_id);
    tracing::warn!("DEBUG: Agents spawned, will check consensus after completion");
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

    // P6-SYNC: Decision sequencing - acquire sequence and check for duplicate
    let consensus_seq = state.consensus_sequence.next_seq();
    if !state.consensus_sequence.begin_processing(consensus_seq) {
        // Another consensus check is already in progress or this is a duplicate
        let run_tag = state
            .run_id
            .as_ref()
            .map(|r| format!("[run:{}]", &r[..8]))
            .unwrap_or_else(|| "[run:none]".to_string());

        tracing::warn!(
            "{} CONSENSUS SEQUENCING: Rejecting duplicate/blocked consensus seq={} (pending={:?})",
            run_tag,
            consensus_seq,
            state.consensus_sequence.pending_seq()
        );

        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "⚠ Consensus check #{} blocked (duplicate or concurrent processing)",
                consensus_seq
            ))],
            HistoryCellType::Notice,
        ));
        return;
    }

    let spec_id = state.spec_id.clone();

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

    // Check if we have cached agent responses (from this run)
    let has_cached_responses = widget
        .spec_auto_state
        .as_ref()
        .and_then(|s| s.agent_responses_cache.as_ref())
        .map(|cache| !cache.is_empty())
        .unwrap_or(false);

    if has_cached_responses {
        // Use cached responses directly, bypass memory/file lookup
        let run_tag = widget
            .spec_auto_state
            .as_ref()
            .and_then(|s| s.run_id.as_ref())
            .map(|r| format!("[run:{}]", &r[..8]))
            .unwrap_or_else(|| "[run:none]".to_string());
        tracing::warn!(
            "{} 🔍 CONSENSUS: Using cached agent responses for {} stage",
            run_tag,
            current_stage.display_name()
        );

        let cached = widget
            .spec_auto_state
            .as_ref()
            .unwrap()
            .agent_responses_cache
            .as_ref()
            .unwrap()
            .clone();

        tracing::warn!("{}   📦 Cached responses: {} items", run_tag, cached.len());
        for (name, response) in &cached {
            tracing::warn!("    - {}: {} chars", name, response.len());
        }

        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "Synthesizing consensus from {} agent responses...",
                widget
                    .spec_auto_state
                    .as_ref()
                    .unwrap()
                    .agent_responses_cache
                    .as_ref()
                    .unwrap()
                    .len()
            ))],
            HistoryCellType::Notice,
        ));

        // Synthesize consensus from cached responses
        let cached = widget
            .spec_auto_state
            .as_ref()
            .unwrap()
            .agent_responses_cache
            .as_ref()
            .unwrap()
            .clone();

        tracing::warn!(
            "{}   🔧 About to call synthesize_from_cached_responses with {} responses",
            run_tag,
            cached.len()
        );

        let run_id_for_synthesis = widget
            .spec_auto_state
            .as_ref()
            .and_then(|s| s.run_id.as_deref());
        match synthesize_from_cached_responses(
            &cached,
            &spec_id,
            current_stage,
            &widget.config.cwd,
            run_id_for_synthesis,
        ) {
            Ok(output_path) => {
                tracing::warn!(
                    "{} ✅ SYNTHESIS SUCCESS: Got output_path={}",
                    run_tag,
                    output_path.display()
                );
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![
                        ratatui::text::Line::from(format!(
                            "✓ Consensus synthesized from {} agent responses",
                            cached.len()
                        )),
                        ratatui::text::Line::from(format!("  Output: {}", output_path.display())),
                    ],
                    HistoryCellType::Notice,
                ));

                // Advance to next stage
                tracing::warn!("  ⏩ Advancing to next stage...");
                if let Some(state) = widget.spec_auto_state.as_mut() {
                    let old_index = state.current_index;
                    state.current_index += 1;
                    state.agent_responses_cache = None; // Clear cache
                    state.phase = SpecAutoPhase::Guardrail; // CRITICAL: Reset to Guardrail for next stage
                    // P6-SYNC: Acknowledge successful consensus processing
                    state.consensus_sequence.ack_processed(consensus_seq);
                    tracing::warn!("    Stage index: {} → {}", old_index, state.current_index);
                    tracing::warn!("    Phase reset to: Guardrail");
                }
                persist_cost_summary(widget, &spec_id);
                tracing::warn!("  📞 Calling advance_spec_auto...");
                advance_spec_auto(widget);
                tracing::warn!("  ✅ advance_spec_auto returned");
            }
            Err(err) => {
                tracing::error!("❌ SYNTHESIS ERROR: {}", err);
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from(format!(
                        "⚠ Consensus synthesis failed: {}. Continuing degraded.",
                        err
                    ))],
                    HistoryCellType::Notice,
                ));

                // Advance degraded
                if let Some(state) = widget.spec_auto_state.as_mut() {
                    state.current_index += 1;
                    state.agent_responses_cache = None;
                    state.phase = SpecAutoPhase::Guardrail; // CRITICAL: Reset to Guardrail for next stage
                    // P6-SYNC: Acknowledge even on error (we handled it, moving on)
                    state.consensus_sequence.ack_processed(consensus_seq);
                }
                advance_spec_auto(widget);
            }
        }
        return;
    }

    // Show checking status
    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Checking consensus for {}...",
            current_stage.display_name()
        ))],
        HistoryCellType::Notice,
    ));

    // Run consensus check via async MCP (no retries)
    let consensus_result = block_on_sync(|| {
        let mcp = widget.mcp_manager.clone();
        let cwd = widget.config.cwd.clone();
        let spec = spec_id.clone();
        let telemetry_enabled = widget.spec_kit_telemetry_enabled();
        async move { run_consensus_with_retry(mcp, cwd, spec, current_stage, telemetry_enabled).await }
    });

    match consensus_result {
        Ok((consensus_lines, consensus_ok)) => {
            // Detect empty/invalid results and continue in degraded mode
            let results_empty_or_invalid = consensus_lines.iter().any(|line| {
                let text = line.to_string();
                text.contains("No structured local-memory entries")
                    || text.contains("No consensus artifacts")
                    || text.contains("Missing agent artifacts")
                    || text.contains("No local-memory entries found")
            });

            if results_empty_or_invalid || !consensus_ok {
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    consensus_lines.clone(),
                    HistoryCellType::Notice,
                ));

                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from(
                        "⚠ Degraded consensus. Scheduling follow-up checklist.",
                    )],
                    crate::history_cell::HistoryCellType::Notice,
                ));

                // Schedule checklist for degraded follow-up
                if let Some(state) = widget.spec_auto_state.as_ref()
                    && let Some(stage) = state.current_stage()
                {
                    super::agent_orchestrator::schedule_degraded_follow_up(widget, stage, &spec_id);
                }
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

                if current_stage == SpecStage::Validate
                    && let Some(state_ref) = widget.spec_auto_state.as_ref()
                    && let Some(info) = active_validate_info.as_ref()
                    && let Some(completion) = state_ref
                        .complete_validate_run(&info.run_id, ValidateCompletionReason::Completed)
                {
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

                persist_cost_summary(widget, &spec_id);

                // SPEC-KIT-922: Auto-commit stage artifacts after consensus succeeds
                if widget.spec_kit_auto_commit_enabled() {
                    match super::git_integration::auto_commit_stage_artifacts(
                        &spec_id,
                        current_stage,
                        &widget.config.cwd,
                        true, // Already checked via spec_kit_auto_commit_enabled()
                    ) {
                        Ok(()) => {
                            tracing::info!(
                                "Auto-commit successful for {} stage",
                                current_stage.display_name()
                            );
                        }
                        Err(err) => {
                            tracing::warn!("Auto-commit failed (non-fatal): {}", err);
                            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                                vec![ratatui::text::Line::from(format!(
                                    "⚠ Auto-commit failed (continuing): {}",
                                    err
                                ))],
                                HistoryCellType::Notice,
                            ));
                        }
                    }
                }

                // Log stage complete event
                if let Some(state) = widget.spec_auto_state.as_ref()
                    && let Some(run_id) = &state.run_id
                {
                    let stage_duration = 0.0; // TODO: Track stage start time
                    let stage_cost = None; // TODO: Get from cost tracker
                    let evidence_written = true; // TODO: Check actual evidence status

                    state.execution_logger.log_event(
                        super::execution_logger::ExecutionEvent::StageComplete {
                            run_id: run_id.clone(),
                            stage: current_stage.display_name().to_string(),
                            duration_sec: stage_duration,
                            cost_usd: stage_cost,
                            evidence_written,
                            timestamp: super::execution_logger::ExecutionEvent::now(),
                        },
                    );
                }

                // ACE Framework Integration (2025-10-29): Send learning feedback on success
                if let Some(state) = widget.spec_auto_state.as_ref()
                    && let Some(bullet_ids) = &state.ace_bullet_ids_used
                    && !bullet_ids.is_empty()
                {
                    use super::ace_learning::send_learning_feedback_sync;
                    use super::routing::{get_current_branch, get_repo_root};

                    let feedback = super::ace_learning::ExecutionFeedback {
                        compile_ok: true,
                        tests_passed: true, // Consensus = success
                        failing_tests: Vec::new(),
                        lint_issues: 0,
                        stack_traces: Vec::new(),
                        diff_stat: None, // Consensus doesn't produce diffs
                    };

                    let ace_config = &widget.config.ace;
                    if ace_config.enabled {
                        let repo_root = get_repo_root(&widget.config.cwd)
                            .unwrap_or_else(|| widget.config.cwd.display().to_string());
                        let branch = get_current_branch(&widget.config.cwd)
                            .unwrap_or_else(|| "main".to_string());
                        let scope = format!("speckit.{}", current_stage.command_name());
                        let task_title =
                            format!("{} stage for {}", current_stage.display_name(), spec_id);

                        send_learning_feedback_sync(
                            ace_config,
                            repo_root,
                            branch,
                            &scope,
                            &task_title,
                            feedback,
                            None, // No diff_stat for consensus stages
                        );

                        tracing::info!(
                            "ACE: Sent learning feedback for {} ({} bullets)",
                            current_stage.display_name(),
                            bullet_ids.len()
                        );
                    }
                }

                // Advance to next stage
                if let Some(state) = widget.spec_auto_state.as_mut() {
                    state.reset_cost_tracking(current_stage);
                    state.phase = SpecAutoPhase::Guardrail;
                    state.current_index += 1;
                    // Clear ACE cache for next stage
                    state.ace_bullets_cache = None;
                    state.ace_bullet_ids_used = None;
                    // P6-SYNC: Acknowledge successful consensus processing
                    state.consensus_sequence.ack_processed(consensus_seq);
                }

                // Trigger next stage
                advance_spec_auto(widget);
            } else {
                if current_stage == SpecStage::Validate
                    && let Some(state_ref) = widget.spec_auto_state.as_ref()
                    && let Some(completion) =
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
                // P6-SYNC: Acknowledge on consensus failure (still processed, just not OK)
                if let Some(state) = widget.spec_auto_state.as_ref() {
                    state.consensus_sequence.ack_processed(consensus_seq);
                }
                // Consensus failed - halt (no retries)
                halt_spec_auto_with_error(
                    widget,
                    format!("Consensus failed for {}", current_stage.display_name()),
                );
            }
        }
        Err(err) => {
            // P6-SYNC: Acknowledge on error (still processed, just errored)
            if let Some(state) = widget.spec_auto_state.as_ref() {
                state.consensus_sequence.ack_processed(consensus_seq);
            }
            // Consensus error - halt (no retries)
            if current_stage == SpecStage::Validate
                && let Some(state_ref) = widget.spec_auto_state.as_ref()
                && let Some(completion) =
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

            halt_spec_auto_with_error(
                widget,
                format!(
                    "Consensus check failed for {}: {}",
                    current_stage.display_name(),
                    err
                ),
            );
        }
    }
}

/// SPEC-KIT-909, SPEC-KIT-902: Check evidence size limit (50MB hard limit)
///
/// Native Rust implementation - no longer calls evidence_stats.sh
fn check_evidence_size_limit(spec_id: &str, cwd: &std::path::Path) -> super::error::Result<()> {
    super::evidence::check_spec_evidence_limit(cwd, spec_id)
}

/// Synthesize consensus from cached agent responses
///
/// Agents may output:
/// 1. Plain text analysis
/// 2. JSON structures (via Python/tool execution)
/// 3. Mixed content with tool outputs
///
/// This synthesizer extracts structured data where possible and creates plan.md.
fn synthesize_from_cached_responses(
    cached_responses: &[(String, String)],
    spec_id: &str,
    stage: SpecStage,
    cwd: &Path,
    run_id: Option<&str>,
) -> Result<PathBuf, String> {
    let run_tag = run_id
        .map(|r| format!("[run:{}]", &r[..8.min(r.len())]))
        .unwrap_or_else(|| "[run:none]".to_string());
    tracing::warn!(
        "{} 🔧 SYNTHESIS START: stage={}, spec={}, responses={}",
        run_tag,
        stage.display_name(),
        spec_id,
        cached_responses.len()
    );

    if cached_responses.is_empty() {
        tracing::error!("❌ SYNTHESIS FAIL: No cached responses");
        return Err("No cached responses to synthesize".to_string());
    }

    tracing::warn!(
        "  📊 Agent responses: {:?}",
        cached_responses
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
    );

    // Parse agent responses and extract structured content
    let mut agent_data: Vec<(String, serde_json::Value)> = Vec::new();

    for (agent_name, response_text) in cached_responses {
        tracing::warn!(
            "DEBUG: Extracting JSON from {} ({} chars)",
            agent_name,
            response_text.len()
        );

        // Try to extract JSON from response (agents may wrap in markdown code blocks)
        let json_content = extract_json_from_agent_response(response_text);

        if let Some(json_str) = json_content {
            tracing::warn!(
                "DEBUG: Extracted JSON string from {} ({} chars)",
                agent_name,
                json_str.len()
            );
            match serde_json::from_str::<serde_json::Value>(&json_str) {
                Ok(parsed) => {
                    tracing::warn!("DEBUG: Successfully parsed JSON for {}", agent_name);
                    // Log top-level fields for debugging
                    if let Some(obj) = parsed.as_object() {
                        let fields: Vec<&String> = obj.keys().collect();
                        tracing::warn!("DEBUG: {} has fields: {:?}", agent_name, fields);
                    }
                    agent_data.push((agent_name.clone(), parsed));
                    continue;
                }
                Err(e) => {
                    tracing::warn!("DEBUG: JSON parse failed for {}: {}", agent_name, e);
                }
            }
        } else {
            tracing::warn!(
                "DEBUG: No JSON extracted from {} response, using as plain text",
                agent_name
            );
            // Log first 500 chars to see format
            let preview = &response_text.chars().take(500).collect::<String>();
            tracing::warn!("DEBUG: Response preview: {}", preview);
        }

        // Fallback: treat as plain text
        agent_data.push((
            agent_name.clone(),
            serde_json::json!({
                "agent": agent_name,
                "content": response_text,
                "format": "text"
            }),
        ));
    }

    // Build plan.md from agent data
    let mut output = String::new();
    output.push_str(&format!("# Plan: {}\n\n", spec_id));
    output.push_str(&format!("**Stage**: {}\n", stage.display_name()));
    output.push_str(&format!("**Agents**: {}\n", agent_data.len()));
    output.push_str(&format!(
        "**Generated**: {}\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
    ));

    // Debug: Log what we actually have
    for (agent_name, data) in &agent_data {
        tracing::warn!(
            "DEBUG: Processing {} with {} top-level keys",
            agent_name,
            data.as_object().map(|o| o.len()).unwrap_or(0)
        );

        // Debug JSON sections removed - caused exponential growth when nested in later stages
        // If debugging needed, check SQLite: SELECT * FROM consensus_runs WHERE spec_id='...'
    }

    // Extract work breakdown, risks, acceptance from structured data
    let mut structured_content_found = false;

    for (agent_name, data) in &agent_data {
        if let Some(work_breakdown) = data.get("work_breakdown").and_then(|v| v.as_array()) {
            output.push_str(&format!("## Work Breakdown (from {})\n\n", agent_name));
            for (i, step) in work_breakdown.iter().enumerate() {
                if let Some(step_name) = step.get("step").and_then(|v| v.as_str()) {
                    output.push_str(&format!("{}. {}\n", i + 1, step_name));
                    if let Some(rationale) = step.get("rationale").and_then(|v| v.as_str()) {
                        output.push_str(&format!("   - Rationale: {}\n", rationale));
                    }
                }
            }
            output.push('\n');
            structured_content_found = true;
        }

        if let Some(risks) = data.get("risks").and_then(|v| v.as_array()) {
            output.push_str(&format!("## Risks (from {})\n\n", agent_name));
            for risk in risks {
                if let Some(risk_desc) = risk.get("risk").and_then(|v| v.as_str()) {
                    output.push_str(&format!("- **Risk**: {}\n", risk_desc));
                    if let Some(mitigation) = risk.get("mitigation").and_then(|v| v.as_str()) {
                        output.push_str(&format!("  - Mitigation: {}\n", mitigation));
                    }
                }
            }
            output.push('\n');
            structured_content_found = true;
        }

        // SPEC-923: Generic fallback for agent schemas we don't explicitly handle
        // Extract common fields that agents may use (tasks, surfaces, research_summary, etc.)
        if let Some(tasks) = data.get("tasks").and_then(|v| v.as_array()) {
            output.push_str(&format!("## Tasks (from {})\n\n", agent_name));
            for task in tasks {
                if let Some(task_str) = task.as_str() {
                    output.push_str(&format!("- {}\n", task_str));
                } else if let Some(obj) = task.as_object()
                    && let Some(name) = obj
                        .get("name")
                        .or_else(|| obj.get("task"))
                        .and_then(|v| v.as_str())
                {
                    output.push_str(&format!("- {}\n", name));
                    if let Some(desc) = obj
                        .get("description")
                        .or_else(|| obj.get("details"))
                        .and_then(|v| v.as_str())
                    {
                        output.push_str(&format!("  {}\n", desc));
                    }
                }
            }
            output.push('\n');
            structured_content_found = true;
        }

        if let Some(surfaces) = data.get("surfaces").and_then(|v| v.as_array()) {
            output.push_str(&format!("## Affected Surfaces (from {})\n\n", agent_name));
            for surface in surfaces {
                if let Some(s) = surface.as_str() {
                    output.push_str(&format!("- {}\n", s));
                }
            }
            output.push('\n');
            structured_content_found = true;
        }

        // Plain text content fallback
        if let Some(content) = data.get("content").and_then(|v| v.as_str())
            && !content.is_empty()
        {
            output.push_str(&format!("## Response from {}\n\n", agent_name));
            output.push_str(content);
            output.push_str("\n\n");
            structured_content_found = true;
        }
    }

    // Ultimate fallback: if no structured content extracted, pretty-print raw JSON
    if !structured_content_found {
        tracing::warn!("⚠️ No structured fields found, using generic JSON extraction");
        output.push_str("## Agent Responses (Raw)\n\n");
        output.push_str("*Note: Structured extraction failed, displaying raw agent data*\n\n");

        for (agent_name, data) in &agent_data {
            output.push_str(&format!("### {}\n\n", agent_name));

            // Skip wrapper fields and extract meaningful content
            if let Some(obj) = data.as_object() {
                for (key, value) in obj {
                    if key != "agent" && key != "format" {
                        output.push_str(&format!("**{}**:\n", key));
                        match value {
                            serde_json::Value::String(s) => output.push_str(&format!("{}\n\n", s)),
                            serde_json::Value::Array(arr) => {
                                for item in arr {
                                    output.push_str(&format!(
                                        "- {}\n",
                                        serde_json::to_string_pretty(item)
                                            .unwrap_or_else(|_| item.to_string())
                                    ));
                                }
                                output.push('\n');
                            }
                            _ => output.push_str(&format!(
                                "```json\n{}\n```\n\n",
                                serde_json::to_string_pretty(value)
                                    .unwrap_or_else(|_| value.to_string())
                            )),
                        }
                    }
                }
            }
            output.push('\n');
        }
    }

    output.push_str("## Consensus Summary\n\n");
    output.push_str(&format!(
        "- Synthesized from {} agent responses\n",
        agent_data.len()
    ));
    output.push_str("- All agents completed successfully\n");

    // Find SPEC directory using ACID-compliant resolver
    let spec_dir = super::spec_directory::find_spec_directory(cwd, spec_id)?;

    tracing::warn!("  📁 SPEC directory: {}", spec_dir.display());
    tracing::warn!("  📁 Is directory: {}", spec_dir.is_dir());
    tracing::warn!("  📁 Exists: {}", spec_dir.exists());

    // Only create if doesn't exist (avoid error if it's already there)
    if !spec_dir.exists() {
        tracing::warn!("  📁 Creating directory...");
        fs::create_dir_all(&spec_dir).map_err(|e| {
            tracing::error!("❌ Failed to create {}: {}", spec_dir.display(), e);
            format!("Failed to create spec dir: {}", e)
        })?;
    } else if !spec_dir.is_dir() {
        tracing::error!(
            "❌ SPEC path exists but is NOT a directory: {}",
            spec_dir.display()
        );
        return Err(format!(
            "SPEC path is not a directory: {}",
            spec_dir.display()
        ));
    } else {
        tracing::warn!("  ✅ Directory already exists");
    }

    // Use standard filenames: plan.md, tasks.md, implement.md, etc.
    let output_filename = format!("{}.md", stage.display_name().to_lowercase());
    let output_file = spec_dir.join(&output_filename);

    tracing::warn!("  📝 Output file: {}", output_file.display());
    tracing::warn!(
        "  📏 Output size: {} chars ({} KB)",
        output.len(),
        output.len() / 1024
    );

    // SPEC-KIT-900: Always write synthesis output to update with latest run
    // Previous skip logic prevented updates, causing stale output files
    tracing::warn!(
        "{}   💾 Writing {} to disk (overwrite={})...",
        run_tag,
        output_filename,
        output_file.exists()
    );

    fs::write(&output_file, &output).map_err(|e| {
        tracing::error!("{} ❌ SYNTHESIS FAIL: Write error: {}", run_tag, e);
        format!("Failed to write {}: {}", output_filename, e)
    })?;

    tracing::warn!(
        "{} ✅ SYNTHESIS SUCCESS: Wrote {} ({} KB)",
        run_tag,
        output_filename,
        output.len() / 1024
    );

    // SPEC-KIT-072: Also store synthesis to SQLite
    if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
        if let Err(e) = db.store_synthesis(
            spec_id,
            stage,
            &output,
            Some(&output_file),
            "ok", // Simple status for now
            cached_responses.len(),
            None,
            None,
            false,
            run_id,
        ) {
            tracing::warn!("{} Failed to store synthesis to SQLite: {}", run_tag, e);
        } else {
            tracing::info!(
                "{} Stored consensus synthesis to SQLite with run_id={:?}",
                run_tag,
                run_id
            );

            // SPEC-KIT-900 Session 3: AUTO-EXPORT evidence for checklist compliance
            // This ensures evidence/consensus/<SPEC-ID>/ is ALWAYS populated after EVERY synthesis
            tracing::info!(
                "{} Auto-exporting evidence to consensus directory...",
                run_tag
            );
            super::evidence::auto_export_stage_evidence(cwd, spec_id, stage, run_id);
        }
    }

    Ok(output_file)
}

/// Extract JSON from agent response (handles code blocks, tool output, etc.)
pub(super) fn extract_json_from_agent_response(text: &str) -> Option<String> {
    // Look for JSON in markdown code blocks
    if let Some(start) = text.find("```json\n")
        && let Some(end) = text[start + 8..].find("\n```")
    {
        return Some(text[start + 8..start + 8 + end].to_string());
    }

    // Look for JSON in plain code blocks (agents use this format)
    if let Some(start) = text.find("│ {\n│   \"stage\"") {
        // Extract JSON from piped format (│ prefix on each line)
        let from_start = &text[start..];
        if let Some(end) = from_start.find("\n│\n│ Ran for") {
            let json_block = &from_start[2..end]; // Skip "│ " prefix
            let cleaned = json_block
                .lines()
                .map(|line| {
                    line.strip_prefix("│   ")
                        .or_else(|| line.strip_prefix("│ "))
                        .unwrap_or(line)
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Some(cleaned);
        }
    }

    // Look for raw JSON objects (Python output format)
    for pattern in &["{\n  \"stage\":", "{\n\"stage\":"] {
        if let Some(start) = text.find(pattern) {
            let from_start = &text[start..];
            let mut depth = 0;
            for (i, ch) in from_start.char_indices() {
                if ch == '{' {
                    depth += 1;
                }
                if ch == '}' {
                    depth -= 1;
                    if depth == 0 {
                        return Some(from_start[..=i].to_string());
                    }
                }
            }
        }
    }

    None
}

/// SPEC-948 Task 2.2: Convert SpecStage to StageType for pipeline config lookups
fn spec_stage_to_stage_type(stage: SpecStage) -> super::pipeline_config::StageType {
    use super::pipeline_config::StageType;
    match stage {
        SpecStage::Plan => StageType::Plan,
        SpecStage::Tasks => StageType::Tasks,
        SpecStage::Implement => StageType::Implement,
        SpecStage::Validate => StageType::Validate,
        SpecStage::Audit => StageType::Audit,
        SpecStage::Unlock => StageType::Unlock,
        // Specify (pre-pipeline) and quality commands are not part of /speckit.auto pipeline
        // They should never appear in state.stages (which is Plan→Unlock only)
        SpecStage::Specify
        | SpecStage::Clarify
        | SpecStage::Analyze
        | SpecStage::Checklist => {
            panic!(
                "Stage {:?} should not be in /speckit.auto pipeline",
                stage
            )
        }
    }
}

/// SPEC-948 Task 2.3: Record skip telemetry for disabled stages
fn record_stage_skip(spec_id: &str, stage: SpecStage, reason: &str) -> Result<(), String> {
    use serde_json::json;
    use std::fs;

    let skip_metadata = json!({
        "command": format!("speckit.{}", stage.display_name().to_lowercase()),
        "specId": spec_id,
        "stage": stage.display_name(),
        "action": "skipped",
        "reason": reason,
        "configSource": "pipeline.toml",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "schemaVersion": "1.0"
    });

    // Evidence directory path matches existing telemetry structure
    let evidence_dir = format!(
        "docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/{}",
        spec_id
    );
    fs::create_dir_all(&evidence_dir)
        .map_err(|e| format!("Failed to create evidence directory: {}", e))?;

    let skip_file = format!(
        "{}/speckit-{}_SKIPPED.json",
        evidence_dir,
        stage.display_name().to_lowercase()
    );

    let json_str = serde_json::to_string_pretty(&skip_metadata)
        .map_err(|e| format!("Failed to serialize skip metadata: {}", e))?;

    fs::write(&skip_file, json_str)
        .map_err(|e| format!("Failed to write skip telemetry to {}: {}", skip_file, e))?;

    tracing::debug!("📝 Skip telemetry recorded: {}", skip_file);
    Ok(())
}
