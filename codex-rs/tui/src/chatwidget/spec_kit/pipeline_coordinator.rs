//! Pipeline coordination and state machine
//!
//! Core spec-auto pipeline orchestration:
//! - Pipeline initiation (handle_spec_auto)
//! - State machine advancement (advance_spec_auto)
//! - Task lifecycle tracking (on_spec_auto_task_*)
//! - Consensus checking and stage progression
//! - Quality gate checkpoint integration
//! - SPEC-KIT-977: Policy capture at run start

#![allow(dead_code, unused_variables)] // Pipeline helpers pending integration

use super::super::ChatWidget;
use super::agent_orchestrator::auto_submit_spec_stage_prompt;
use super::command_handlers::{halt_spec_auto_no_resume, halt_spec_auto_with_error};
use super::consensus_coordinator::{block_on_sync, persist_cost_summary, run_consensus_with_retry};
use super::pipeline_config::{CapsuleExportConfig, PipelineConfig, PipelineOverrides}; // SPEC-948
use super::quality_gate_handler::{
    determine_quality_checkpoint, execute_quality_checkpoint, finalize_quality_gates,
};
use super::state::{GuardrailWait, SpecAutoPhase, ValidateRunInfo};
use super::validation_lifecycle::{
    ValidateCompletionReason, ValidateLifecycleEvent, cleanup_spec_auto_with_cancel,
    record_validate_lifecycle_event,
};
use crate::history_cell::HistoryCellType;
use crate::memvid_adapter::{
    BranchId, CapsuleHandle, LLMCaptureMode, default_capsule_config, policy_capture,
};
use crate::slash_command::{HalMode, SlashCommand};
use crate::spec_prompts::SpecStage;
use std::fs;
use std::path::{Path, PathBuf};

use super::stage0_integration::Tier2Trace;

/// Emit Tier2 degraded-mode retrieval events when health check failed.
///
/// ADR-003: Best-effort, non-blocking - emit events to audit trail for replay verification.
/// Emits RetrievalRequest/Response events with source="tier2:notebooklm" when:
/// - Tier2 was enabled AND
/// - Health check was performed and failed (error.is_some())
///
/// Does NOT emit for: pre-check hit, no notebook configured, Tier2 disabled, or healthy Tier2.
pub(crate) fn emit_tier2_degraded_events_if_needed(
    cwd: &Path,
    spec_id: &str,
    run_id: &str,
    trace: &Tier2Trace,
) {
    // Gate: only emit if health check actually failed (not merely skipped)
    if !trace.should_emit_degraded_event() {
        return;
    }

    use crate::memvid_adapter::{RetrievalRequestPayload, RetrievalResponsePayload};
    use uuid::Uuid;

    let Ok(handle) = CapsuleHandle::open(default_capsule_config(cwd)) else {
        tracing::warn!(target: "stage0", "Failed to open capsule for Tier2 degraded-mode events");
        return;
    };

    if handle.switch_branch(BranchId::for_run(run_id)).is_err() {
        tracing::warn!(target: "stage0", "Failed to switch branch for Tier2 degraded-mode events");
        return;
    }

    let request_id = Uuid::new_v4().to_string();

    let req_payload = RetrievalRequestPayload {
        request_id: request_id.clone(),
        query: String::new(), // No query in health check (privacy-safe)
        config: serde_json::json!({
            "base_url": &trace.base_url,
            "notebook_id": &trace.notebook_id,
            "tier2_enabled": trace.enabled,
            "health_ok": trace.health_ok,
            "skip_reason": &trace.skip_reason,
            "latency_ms": trace.latency_ms,
        }),
        source: "tier2:notebooklm".to_string(),
        stage: Some("Stage0".to_string()),
        role: None,
    };

    let resp_payload = RetrievalResponsePayload {
        request_id,
        hit_uris: vec![], // No results in degraded mode
        fused_scores: None,
        explainability: None,
        latency_ms: trace.latency_ms,
        error: trace.error.clone(),
    };

    if let Err(e) = handle.emit_retrieval_request(spec_id, run_id, &req_payload) {
        tracing::warn!(
            target: "stage0",
            error = %e,
            "Failed to emit Tier2 degraded-mode RetrievalRequest (best-effort)"
        );
    }
    if let Err(e) = handle.emit_retrieval_response(spec_id, run_id, Some("Stage0"), &resp_payload) {
        tracing::warn!(
            target: "stage0",
            error = %e,
            "Failed to emit Tier2 degraded-mode RetrievalResponse (best-effort)"
        );
    }
}

/// Handle /speckit.auto command initiation
pub fn handle_spec_auto(
    widget: &mut ChatWidget,
    spec_id: String,
    goal: String,
    resume_from: SpecStage,
    hal_mode: Option<HalMode>,
    cli_overrides: Option<PipelineOverrides>, // SPEC-948: CLI flags for stage filtering
    stage0_config: super::stage0_integration::Stage0ExecutionConfig, // SPEC-KIT-102: Stage 0 config
) {
    // FILE-BASED TRACE: handle_spec_auto entry (SPEC-DOGFOOD-001 S29)
    // SPEC-DOGFOOD-001: Re-entry guard - prevent duplicate pipeline execution
    if let Some(existing_state) = widget.spec_auto_state.as_ref() {
        tracing::warn!(
            "🚨 DUPLICATE PIPELINE: Pipeline already running for {} (phase: {:?}). Ignoring duplicate /speckit.auto {}",
            existing_state.spec_id,
            existing_state.phase,
            spec_id
        );
        widget.history_push(crate::history_cell::new_error_event(format!(
            "Pipeline already running for {}. Wait for completion or cancel with Esc.",
            existing_state.spec_id
        )));
        return;
    }

    let mut header: Vec<ratatui::text::Line<'static>> = Vec::new();
    header.push(ratatui::text::Line::from(format!(
        "/speckit.auto {}",
        spec_id
    )));
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
    // Clone cli_overrides before consuming it, as we need it for maieutic resume
    let cli_overrides_for_load = cli_overrides.clone();
    let pipeline_config = match PipelineConfig::load(&spec_id, cli_overrides_for_load) {
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

    // P91/SPEC-KIT-105: Phase -1 Constitution Readiness Gate
    // P92: Now returns bool - Block mode can abort pipeline
    if !run_constitution_readiness_gate(widget) {
        // Gate blocked execution - abort pipeline
        return;
    }

    // Phase 2: Intake Presence Gate
    // Check for IntakeCompleted event in capsule; if missing, show backfill modal
    // Run AFTER constitution gate, BEFORE maieutic gate
    if !run_intake_presence_gate(
        widget,
        &spec_id,
        &goal,
        resume_from,
        hal_mode,
        cli_overrides.clone(),
        stage0_config.clone(),
    ) {
        // Backfill modal opened, pipeline paused - will resume via SpecIntakeSubmitted event
        return;
    }

    // P93/D130: Mandatory Maieutic Elicitation Gate
    // Run AFTER constitution gate, BEFORE Stage0 execution
    // If maieutic needs elicitation, shows modal and returns false (pipeline pauses)
    // When modal completes, MaieuticSubmitted event resumes via resume_pipeline_after_maieutic()
    if !run_maieutic_gate(
        widget,
        &spec_id,
        &goal,
        resume_from,
        hal_mode,
        cli_overrides.clone(),
        stage0_config.clone(),
    ) {
        // Modal opened, pipeline paused - will resume via MaieuticSubmitted event
        return;
    }

    // D131: Load capture mode from governance policy for ship gate validation
    let capture_mode = codex_stage0::GovernancePolicy::load(None)
        .ok()
        .and_then(|gov| LLMCaptureMode::from_str(&gov.capture.mode))
        .unwrap_or(LLMCaptureMode::PromptsOnly);

    let lifecycle = widget.ensure_validate_lifecycle(&spec_id);
    let mut state = super::state::SpecAutoState::new(
        spec_id.clone(),
        goal,
        resume_from,
        hal_mode,
        pipeline_config, // SPEC-948: Pass pipeline config
        capture_mode,    // D131: Capture mode for ship gate
    );
    state.set_validate_lifecycle(lifecycle);

    // SPEC-KIT-102: Set Stage 0 configuration from CLI flags
    state.stage0_disabled = stage0_config.disabled;
    state.stage0_explain = stage0_config.explain;

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

    // SPEC-KIT-977: Capture policy snapshot at run start for phase 4→5 gate
    if let Some(run_id) = &state.run_id {
        // Use canonical capsule config (SPEC-KIT-971/977 alignment)
        let capsule_config = default_capsule_config(&widget.config.cwd);

        // Open capsule and capture policy (non-blocking - continue on failure)
        match CapsuleHandle::open(capsule_config) {
            Ok(handle) => {
                // SPEC-KIT-971: Switch to run branch before any writes
                // Invariant: every run writes to run/<RUN_ID> branch
                if let Err(e) = handle.switch_branch(BranchId::for_run(run_id)) {
                    tracing::warn!(
                        run_id = %run_id,
                        error = %e,
                        "Failed to switch capsule branch for run"
                    );
                }

                let stage0_cfg = codex_stage0::Stage0Config::load().unwrap_or_default();
                match policy_capture::capture_and_store_policy(
                    &handle,
                    &stage0_cfg,
                    &spec_id,
                    run_id,
                ) {
                    Ok(snapshot) => {
                        // Store policy info in state for phase 4→5 verification
                        state.policy_id = Some(snapshot.policy_id.clone());
                        state.policy_hash = Some(snapshot.hash.clone());
                        if let Some(info) = handle.current_policy() {
                            state.policy_uri = Some(info.uri.as_str().to_string());
                        }
                        tracing::info!(
                            policy_id = %snapshot.policy_id,
                            hash = %snapshot.hash,
                            "Policy snapshot captured at run start"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to capture policy snapshot - continuing without policy binding");
                    }
                }
                // Handle dropped here - lock released
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to open capsule for policy capture - continuing without policy binding");
            }
        }
    }

    // SPEC-KIT-102: Run Stage 0 context injection before pipeline starts
    // FILE-BASED TRACE: Stage0 decision (SPEC-DOGFOOD-001 S29)
    if !stage0_config.disabled {
        // Load spec content
        let spec_path = widget.config.cwd.join(format!("docs/{}/spec.md", spec_id));
        let spec_content = std::fs::read_to_string(&spec_path).unwrap_or_default();

        if !spec_content.is_empty() {
            // Log Stage0Start event
            if let Some(run_id) = &state.run_id {
                state.execution_logger.log_event(
                    super::execution_logger::ExecutionEvent::Stage0Start {
                        run_id: run_id.clone(),
                        spec_id: spec_id.clone(),
                        tier2_enabled: codex_stage0::Stage0Config::load().ok().is_some_and(|cfg| {
                            let notebook_override = widget
                                .config
                                .stage0
                                .notebook
                                .clone()
                                .or(widget.config.stage0.notebook_url.clone())
                                .or(widget.config.stage0.notebook_id.clone())
                                .unwrap_or_default();
                            if !notebook_override.trim().is_empty() {
                                return true;
                            }
                            cfg.tier2.enabled && !cfg.tier2.notebook.trim().is_empty()
                        }),
                        explain_enabled: stage0_config.explain,
                        timestamp: super::execution_logger::ExecutionEvent::now(),
                    },
                );
            }

            // SPEC-DOGFOOD-001 S31: Spawn Stage0 async - TUI remains responsive
            // Uses spawn_stage0_async to run in background, poll in on_commit_tick
            let pending = super::stage0_integration::spawn_stage0_async(
                widget.config.clone(),
                spec_id.clone(),
                spec_content.clone(),
                widget.config.cwd.clone(),
                stage0_config.clone(),
            );

            // Store pending operation for polling
            widget.stage0_pending = Some(pending);

            // S33 FIX: Start commit animation so on_commit_tick polls for Stage0 result
            // Without this, poll_stage0_pending is never called because CommitTick
            // events are only generated during streaming responses.
            widget
                .app_event_tx
                .send(crate::app_event::AppEvent::StartCommitAnimation);

            // Set phase to Stage0Pending and store state
            state.phase = super::state::SpecAutoPhase::Stage0Pending {
                status: "Starting Stage0...".to_string(),
                started_at: std::time::Instant::now(),
            };
            widget.spec_auto_state = Some(state);

            // Show status message
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(
                    "⚙️  Stage 0: Starting (background)...",
                )],
                crate::history_cell::HistoryCellType::Notice,
            ));

            // Return early - on_commit_tick will poll for completion and call process_stage0_result
            return;
        } else {
            // Log Stage0Complete event (skipped - no spec content)
            let skip_reason = "spec.md is empty or not found";
            if let Some(run_id) = &state.run_id {
                let event = super::execution_logger::ExecutionEvent::Stage0Complete {
                    run_id: run_id.clone(),
                    spec_id: spec_id.clone(),
                    duration_ms: 0,
                    tier2_used: false,
                    cache_hit: false,
                    hybrid_used: false,
                    memories_used: 0,
                    task_brief_written: false,
                    skip_reason: Some(skip_reason.to_string()),
                    timestamp: super::execution_logger::ExecutionEvent::now(),
                };

                tracing::info!(
                    target: "stage0",
                    event_type = "Stage0Complete",
                    spec_id = %spec_id,
                    result = "skipped",
                    skip_reason = skip_reason,
                    "Stage 0 skipped - no spec content"
                );

                state.execution_logger.log_event(event);
            }
            state.stage0_skip_reason = Some(skip_reason.to_string());

            // SPEC-DOGFOOD-001 FIX: Show skip reason in TUI
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "Stage 0: Skipped ({})",
                    skip_reason
                ))],
                crate::history_cell::HistoryCellType::Notice,
            ));
        }
    } else {
        // Log Stage0Complete event (disabled by flag)
        let skip_reason = "Stage 0 disabled by flag";
        if let Some(run_id) = &state.run_id {
            let event = super::execution_logger::ExecutionEvent::Stage0Complete {
                run_id: run_id.clone(),
                spec_id: spec_id.clone(),
                duration_ms: 0,
                tier2_used: false,
                cache_hit: false,
                hybrid_used: false,
                memories_used: 0,
                task_brief_written: false,
                skip_reason: Some(skip_reason.to_string()),
                timestamp: super::execution_logger::ExecutionEvent::now(),
            };

            tracing::info!(
                target: "stage0",
                event_type = "Stage0Complete",
                spec_id = %spec_id,
                result = "skipped",
                skip_reason = skip_reason,
                "Stage 0 disabled by flag"
            );

            state.execution_logger.log_event(event);
        }
        state.stage0_skip_reason = Some(skip_reason.to_string());

        // SPEC-DOGFOOD-001 FIX: Show skip reason in TUI
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "Stage 0: Skipped ({})",
                skip_reason
            ))],
            crate::history_cell::HistoryCellType::Notice,
        ));
    }

    widget.spec_auto_state = Some(state);
    advance_spec_auto(widget);
}

// ─────────────────────────────────────────────────────────────────────────────
// SPEC-DOGFOOD-001 S31: Async Stage0 Result Processing
// ─────────────────────────────────────────────────────────────────────────────

/// Process Stage0 result after async completion
///
/// Called from on_commit_tick when Stage0PendingOperation.result_rx receives a result.
/// Handles the same logic as the original blocking code: writing TASK_BRIEF.md,
/// DIVINE_TRUTH.md, logging, and transitioning to Guardrail phase.
pub fn process_stage0_result(
    widget: &mut ChatWidget,
    result: super::stage0_integration::Stage0ExecutionResult,
    spec_id: String,
) {
    // S33 FIX: Stop the commit animation that was started for Stage0 polling
    widget
        .app_event_tx
        .send(crate::app_event::AppEvent::StopCommitAnimation);

    // Check state exists
    if widget.spec_auto_state.is_none() {
        widget.history_push(crate::history_cell::new_error_event(
            "Stage0 result received but no spec_auto_state - internal error".to_string(),
        ));
        return;
    }

    // Show Stage0 completion status to user (before state borrow)
    {
        let status_msg = if result.result.is_some() {
            let tier2_info = if result.tier2_used {
                "Tier2 ✓".to_string()
            } else if result.cache_hit {
                "Tier2 (cached)".to_string()
            } else if let Some(ref reason) = result.tier2_skip_reason {
                format!("Tier2 skipped: {}", reason)
            } else {
                "Tier2 ✗".to_string()
            };
            format!("Stage0 complete: {}ms | {}", result.duration_ms, tier2_info)
        } else if let Some(ref reason) = result.skip_reason {
            format!("Stage0 skipped: {}", reason)
        } else {
            "Stage0 complete".to_string()
        };
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!("⚙️  {}", status_msg))],
            crate::history_cell::HistoryCellType::Notice,
        ));
    }

    // Process result - collect UI messages to push after state updates
    let mut ui_messages: Vec<Box<dyn crate::history_cell::HistoryCell>> = Vec::new();

    // Extract run_id early for persist_or_strip (before stage0_result borrow)
    let run_id_for_persist = widget
        .spec_auto_state
        .as_ref()
        .and_then(|s| s.run_id.clone());

    // Process result and update state
    if let Some(ref original_result) = result.result {
        // Clone to mutable for persist-or-strip
        let mut stage0_result = original_result.clone();

        // Persist Product Knowledge pack or strip lane if persistence fails
        if let Some(ref run_id) = run_id_for_persist {
            super::stage0_integration::persist_or_strip_product_knowledge(
                &spec_id,
                run_id,
                &widget.config.cwd,
                &mut stage0_result,
            );
        }

        // ─────────────────────────────────────────────────────────────────────────────
        // Phase 2 ADR-003: Emit pre-check retrieval events (best-effort, non-blocking)
        // ─────────────────────────────────────────────────────────────────────────────
        if let (Some(run_id), Some(trace)) = (&run_id_for_persist, &result.precheck_trace) {
            use crate::memvid_adapter::{
                BranchId, CapsuleHandle, RetrievalRequestPayload, RetrievalResponsePayload,
                default_capsule_config,
            };
            use uuid::Uuid;

            if let Ok(handle) = CapsuleHandle::open(default_capsule_config(&widget.config.cwd)) {
                if handle.switch_branch(BranchId::for_run(run_id)).is_ok() {
                    let request_id = Uuid::new_v4().to_string();

                    let req_payload = RetrievalRequestPayload {
                        request_id: request_id.clone(),
                        query: trace.query.clone(),
                        config: serde_json::json!({
                            "domain": &trace.domain,
                            "limit": trace.limit,
                            "min_importance": trace.min_importance,
                            "threshold": trace.threshold,
                            "decision": if trace.hit { "hit" } else { "miss" },
                        }),
                        source: format!("precheck:{}", trace.domain),
                        stage: Some("Stage0".to_string()),
                        role: None,
                    };

                    let resp_payload = RetrievalResponsePayload {
                        request_id,
                        hit_uris: trace.hit_uris.clone(),
                        fused_scores: Some(trace.fused_scores.clone()),
                        explainability: Some(
                            serde_json::json!({"max_relevance": trace.max_relevance}),
                        ),
                        latency_ms: trace.latency_ms,
                        error: trace.error.clone(),
                    };

                    if let Err(e) = handle.emit_retrieval_request(&spec_id, run_id, &req_payload) {
                        tracing::warn!(
                            target: "stage0",
                            error = %e,
                            "Failed to emit precheck RetrievalRequest (best-effort)"
                        );
                    }
                    if let Err(e) = handle.emit_retrieval_response(
                        &spec_id,
                        run_id,
                        Some("Stage0"),
                        &resp_payload,
                    ) {
                        tracing::warn!(
                            target: "stage0",
                            error = %e,
                            "Failed to emit precheck RetrievalResponse (best-effort)"
                        );
                    }
                }
            }
        }

        // ─────────────────────────────────────────────────────────────────────────────
        // ADR-003: Emit PK degraded-mode retrieval events when routing was disabled
        // Best-effort, non-blocking - emit events to audit trail for replay verification
        // ─────────────────────────────────────────────────────────────────────────────
        if let (Some(run_id), Some(trace)) = (&run_id_for_persist, &result.pk_routing_trace) {
            // Only emit if PK was enabled but routing was disabled (degraded mode)
            if trace.enabled && !trace.routing_enabled {
                use crate::memvid_adapter::{
                    BranchId, CapsuleHandle, RetrievalRequestPayload, RetrievalResponsePayload,
                    default_capsule_config,
                };
                use uuid::Uuid;

                if let Ok(handle) = CapsuleHandle::open(default_capsule_config(&widget.config.cwd))
                {
                    if handle.switch_branch(BranchId::for_run(run_id)).is_ok() {
                        let request_id = Uuid::new_v4().to_string();

                        let req_payload = RetrievalRequestPayload {
                            request_id: request_id.clone(),
                            query: String::new(), // No query executed in degraded mode
                            config: serde_json::json!({
                                "domain": &trace.domain,
                                "pk_enabled": trace.enabled,
                                "routing_enabled": trace.routing_enabled,
                                "gate_reason": &trace.reason,
                                "latency_ms": trace.latency_ms,
                            }),
                            source: format!("product-knowledge:{}", trace.domain),
                            stage: Some("Stage0".to_string()),
                            role: None,
                        };

                        let resp_payload = RetrievalResponsePayload {
                            request_id,
                            hit_uris: vec![], // No results in degraded mode
                            fused_scores: None,
                            explainability: None,
                            latency_ms: trace.latency_ms,
                            error: Some(format!("skipped: {}", trace.reason)),
                        };

                        if let Err(e) =
                            handle.emit_retrieval_request(&spec_id, run_id, &req_payload)
                        {
                            tracing::warn!(
                                target: "stage0",
                                error = %e,
                                "Failed to emit PK degraded-mode RetrievalRequest (best-effort)"
                            );
                        }
                        if let Err(e) = handle.emit_retrieval_response(
                            &spec_id,
                            run_id,
                            Some("Stage0"),
                            &resp_payload,
                        ) {
                            tracing::warn!(
                                target: "stage0",
                                error = %e,
                                "Failed to emit PK degraded-mode RetrievalResponse (best-effort)"
                            );
                        }
                    }
                }
            }
        }

        // ADR-003: Emit Tier2 degraded-mode retrieval events when health check failed
        if let (Some(run_id), Some(trace)) = (&run_id_for_persist, &result.tier2_trace) {
            emit_tier2_degraded_events_if_needed(&widget.config.cwd, &spec_id, run_id, trace);
        }

        // Write TASK_BRIEF.md to evidence directory (uses potentially stripped task_brief_md)
        let task_brief_path = super::stage0_integration::write_task_brief_to_evidence(
            &spec_id,
            &widget.config.cwd,
            &stage0_result.task_brief_md,
        );
        let task_brief_written = task_brief_path.is_ok();

        if !task_brief_written {
            tracing::warn!("Failed to write TASK_BRIEF.md");
        }

        // Write DIVINE_TRUTH.md to evidence directory
        let divine_truth_path = super::stage0_integration::write_divine_truth_to_evidence(
            &spec_id,
            &widget.config.cwd,
            &stage0_result.divine_truth.raw_markdown,
        );
        if let Err(ref e) = divine_truth_path {
            tracing::warn!("Failed to write DIVINE_TRUTH.md: {}", e);
        }

        // Store system pointer memory (best-effort, non-blocking)
        super::stage0_integration::store_stage0_system_pointer(
            &spec_id,
            &result,
            task_brief_path.as_ref().ok().map(|p| p.as_path()),
            divine_truth_path.as_ref().ok().map(|p| p.as_path()),
            None,
        );

        // Log Stage0Complete event (success) - access state briefly
        if let Some(ref mut state) = widget.spec_auto_state {
            if let Some(run_id) = &state.run_id {
                let event = super::execution_logger::ExecutionEvent::Stage0Complete {
                    run_id: run_id.clone(),
                    spec_id: spec_id.clone(),
                    duration_ms: result.duration_ms,
                    tier2_used: result.tier2_used,
                    cache_hit: result.cache_hit,
                    hybrid_used: result.hybrid_retrieval_used,
                    memories_used: stage0_result.memories_used.len(),
                    task_brief_written,
                    skip_reason: None,
                    timestamp: super::execution_logger::ExecutionEvent::now(),
                };

                tracing::info!(
                    target: "stage0",
                    event_type = "Stage0Complete",
                    spec_id = %spec_id,
                    result = "success",
                    duration_ms = result.duration_ms,
                    tier2_used = result.tier2_used,
                    cache_hit = result.cache_hit,
                    hybrid_used = result.hybrid_retrieval_used,
                    memories_used = stage0_result.memories_used.len(),
                    task_brief_written = task_brief_written,
                    "Stage 0 completed successfully"
                );

                state.execution_logger.log_event(event);
            }
            state.stage0_result = Some(stage0_result.clone());
        }

        // Prepare UI message
        let tier2_status = if stage0_result.tier2_used {
            "tier2=yes".to_string()
        } else if let Some(ref reason) = result.tier2_skip_reason {
            format!("tier2=skipped ({})", reason)
        } else {
            "tier2=no".to_string()
        };
        ui_messages.push(Box::new(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "Stage 0: Context compiled ({} memories, {}, {}ms)",
                stage0_result.memories_used.len(),
                tier2_status,
                stage0_result.latency_ms
            ))],
            crate::history_cell::HistoryCellType::Notice,
        )));
    } else if let Some(ref skip_reason) = result.skip_reason {
        // Log Stage0Complete event (skipped)
        if let Some(ref mut state) = widget.spec_auto_state {
            if let Some(run_id) = &state.run_id {
                let event = super::execution_logger::ExecutionEvent::Stage0Complete {
                    run_id: run_id.clone(),
                    spec_id: spec_id.clone(),
                    duration_ms: result.duration_ms,
                    tier2_used: false,
                    cache_hit: false,
                    hybrid_used: false,
                    memories_used: 0,
                    task_brief_written: false,
                    skip_reason: Some(skip_reason.clone()),
                    timestamp: super::execution_logger::ExecutionEvent::now(),
                };

                tracing::info!(
                    target: "stage0",
                    event_type = "Stage0Complete",
                    spec_id = %spec_id,
                    result = "skipped",
                    duration_ms = result.duration_ms,
                    skip_reason = %skip_reason,
                    "Stage 0 skipped"
                );

                state.execution_logger.log_event(event);
            }
            state.stage0_skip_reason = Some(skip_reason.clone());
        }

        ui_messages.push(Box::new(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "Stage 0: Skipped ({})",
                skip_reason
            ))],
            crate::history_cell::HistoryCellType::Notice,
        )));
    } else {
        ui_messages.push(Box::new(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(
                "Stage 0: Completed (no result or skip_reason returned - possible bug)",
            )],
            crate::history_cell::HistoryCellType::Notice,
        )));
    }

    // Push UI messages (state borrow is released)
    for msg in ui_messages {
        widget.history_push(msg);
    }

    // Transition to Guardrail phase and continue pipeline
    if let Some(ref mut state) = widget.spec_auto_state {
        state.phase = super::state::SpecAutoPhase::Guardrail;
    }
    advance_spec_auto(widget);
}

// ─────────────────────────────────────────────────────────────────────────────
// P92/SPEC-KIT-105: Planning Pipeline Handler
// ─────────────────────────────────────────────────────────────────────────────

/// Handle /speckit.plan-pipeline command - planning-only pipeline
///
/// Runs Stage 0 → Specify → Plan → Tasks stages only, stopping before Implement.
/// Respects constitution gate - will abort in Block mode if constitution incomplete.
///
/// # Arguments
/// * `widget` - Chat widget reference
/// * `spec_id` - SPEC ID to process (e.g., "SPEC-KIT-105")
pub fn handle_spec_plan(widget: &mut ChatWidget, spec_id: String) {
    let mut header: Vec<ratatui::text::Line<'static>> = Vec::new();
    header.push(ratatui::text::Line::from(format!(
        "/speckit.plan-pipeline {}",
        spec_id
    )));
    header.push(ratatui::text::Line::from(
        "Pipeline: Stage 0 → Specify → Plan → Tasks (P92)",
    ));
    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        header,
        HistoryCellType::Notice,
    ));

    // Validate configuration before starting pipeline
    if let Err(err) = super::config_validator::SpecKitConfigValidator::validate(&widget.config) {
        widget.history_push(crate::history_cell::new_error_event(format!(
            "Configuration validation failed: {}",
            err
        )));
        return;
    }

    // P92/SPEC-KIT-105: Run constitution readiness gate (Block mode can abort)
    if !run_constitution_readiness_gate(widget) {
        // Gate blocked execution - abort pipeline
        return;
    }

    // Check evidence size limit
    if let Err(err) = check_evidence_size_limit(&spec_id, &widget.config.cwd) {
        widget.history_push(crate::history_cell::new_error_event(format!(
            "Evidence size check failed: {}",
            err
        )));
        return;
    }

    // Load pipeline configuration (planning stages only)
    let pipeline_config = match super::pipeline_config::PipelineConfig::load(&spec_id, None) {
        Ok(config) => config,
        Err(err) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Pipeline configuration error: {}",
                err
            )));
            return;
        }
    };

    let lifecycle = widget.ensure_validate_lifecycle(&spec_id);

    // Create state with planning stages only: Specify → Plan → Tasks
    let mut state = super::state::SpecAutoState::new_planning_only(
        spec_id.clone(),
        String::new(), // No goal for plan pipeline
        pipeline_config,
    );
    state.set_validate_lifecycle(lifecycle);

    // Run Stage 0 context injection
    let spec_path = widget.config.cwd.join(format!("docs/{}/spec.md", spec_id));
    let spec_content = std::fs::read_to_string(&spec_path).unwrap_or_default();

    if !spec_content.is_empty() {
        // Log Stage0Start event
        if let Some(run_id) = &state.run_id {
            state.execution_logger.log_event(
                super::execution_logger::ExecutionEvent::Stage0Start {
                    run_id: run_id.clone(),
                    spec_id: spec_id.clone(),
                    tier2_enabled: true,
                    explain_enabled: false,
                    timestamp: super::execution_logger::ExecutionEvent::now(),
                },
            );
        }

        // Run Stage0 with default config (no progress reporting for resume path)
        let stage0_config = super::stage0_integration::Stage0ExecutionConfig::default();
        let result = super::stage0_integration::run_stage0_for_spec(
            &widget.config,
            &spec_id,
            &spec_content,
            &widget.config.cwd,
            &stage0_config,
            None, // No progress for resume
        );

        if let Some(ref original_result) = result.result {
            // Clone to mutable for persist-or-strip
            let mut stage0_result = original_result.clone();

            // Persist Product Knowledge pack or strip lane if persistence fails
            if let Some(ref run_id) = state.run_id {
                super::stage0_integration::persist_or_strip_product_knowledge(
                    &spec_id,
                    run_id,
                    &widget.config.cwd,
                    &mut stage0_result,
                );
            }

            // ─────────────────────────────────────────────────────────────────────────────
            // Phase 2 ADR-003: Emit pre-check retrieval events (best-effort, non-blocking)
            // ─────────────────────────────────────────────────────────────────────────────
            if let (Some(run_id), Some(trace)) = (&state.run_id, &result.precheck_trace) {
                use crate::memvid_adapter::{
                    BranchId, CapsuleHandle, RetrievalRequestPayload, RetrievalResponsePayload,
                    default_capsule_config,
                };
                use uuid::Uuid;

                if let Ok(handle) = CapsuleHandle::open(default_capsule_config(&widget.config.cwd))
                {
                    if handle.switch_branch(BranchId::for_run(run_id)).is_ok() {
                        let request_id = Uuid::new_v4().to_string();

                        let req_payload = RetrievalRequestPayload {
                            request_id: request_id.clone(),
                            query: trace.query.clone(),
                            config: serde_json::json!({
                                "domain": &trace.domain,
                                "limit": trace.limit,
                                "min_importance": trace.min_importance,
                                "threshold": trace.threshold,
                                "decision": if trace.hit { "hit" } else { "miss" },
                            }),
                            source: format!("precheck:{}", trace.domain),
                            stage: Some("Stage0".to_string()),
                            role: None,
                        };

                        let resp_payload = RetrievalResponsePayload {
                            request_id,
                            hit_uris: trace.hit_uris.clone(),
                            fused_scores: Some(trace.fused_scores.clone()),
                            explainability: Some(
                                serde_json::json!({"max_relevance": trace.max_relevance}),
                            ),
                            latency_ms: trace.latency_ms,
                            error: trace.error.clone(),
                        };

                        if let Err(e) =
                            handle.emit_retrieval_request(&spec_id, run_id, &req_payload)
                        {
                            tracing::warn!(
                                target: "stage0",
                                error = %e,
                                "Failed to emit precheck RetrievalRequest (best-effort)"
                            );
                        }
                        if let Err(e) = handle.emit_retrieval_response(
                            &spec_id,
                            run_id,
                            Some("Stage0"),
                            &resp_payload,
                        ) {
                            tracing::warn!(
                                target: "stage0",
                                error = %e,
                                "Failed to emit precheck RetrievalResponse (best-effort)"
                            );
                        }
                    }
                }
            }

            // Write TASK_BRIEF.md to evidence directory (uses potentially stripped task_brief_md)
            let task_brief_path = super::stage0_integration::write_task_brief_to_evidence(
                &spec_id,
                &widget.config.cwd,
                &stage0_result.task_brief_md,
            );

            // Write DIVINE_TRUTH.md to evidence directory
            let divine_truth_path = super::stage0_integration::write_divine_truth_to_evidence(
                &spec_id,
                &widget.config.cwd,
                &stage0_result.divine_truth.raw_markdown,
            );
            if let Err(ref e) = divine_truth_path {
                tracing::warn!("Failed to write DIVINE_TRUTH.md: {}", e);
            }

            // CONVERGENCE: Store system pointer memory (best-effort, non-blocking)
            super::stage0_integration::store_stage0_system_pointer(
                &spec_id,
                &result,
                task_brief_path.as_ref().ok().map(|p| p.as_path()),
                divine_truth_path.as_ref().ok().map(|p| p.as_path()),
                None, // TODO: Pass notebook_id when available from config
            );

            // Log Stage0Complete event
            if let Some(run_id) = &state.run_id {
                state.execution_logger.log_event(
                    super::execution_logger::ExecutionEvent::Stage0Complete {
                        run_id: run_id.clone(),
                        spec_id: spec_id.clone(),
                        duration_ms: result.duration_ms,
                        tier2_used: result.tier2_used,
                        cache_hit: result.cache_hit,
                        hybrid_used: result.hybrid_retrieval_used,
                        memories_used: stage0_result.memories_used.len(),
                        task_brief_written: true,
                        skip_reason: None,
                        timestamp: super::execution_logger::ExecutionEvent::now(),
                    },
                );
            }

            // CONVERGENCE: Surface Tier2 diagnostics in output
            let tier2_status = if stage0_result.tier2_used {
                "tier2=yes".to_string()
            } else if let Some(ref reason) = result.tier2_skip_reason {
                format!("tier2=skipped ({})", reason)
            } else {
                "tier2=no".to_string()
            };
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "Stage 0: Context compiled ({} memories, {}, {}ms)",
                    stage0_result.memories_used.len(),
                    tier2_status,
                    stage0_result.latency_ms
                ))],
                crate::history_cell::HistoryCellType::Notice,
            ));

            state.stage0_result = Some(stage0_result.clone());
        } else if let Some(skip_reason) = result.skip_reason {
            state.stage0_skip_reason = Some(skip_reason);
        }
    } else {
        state.stage0_skip_reason = Some("spec.md is empty or not found".to_string());
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

                // SPEC-KIT-928: Check if quality gates or Stage0 are still running (single-flight guard)
                // Prevent stage advancement while blocking operations are in progress
                if matches!(
                    state.phase,
                    SpecAutoPhase::Stage0Pending { .. }
                        | SpecAutoPhase::QualityGateExecuting { .. }
                        | SpecAutoPhase::QualityGateProcessing { .. }
                        | SpecAutoPhase::QualityGateValidating { .. }
                        | SpecAutoPhase::QualityGateAwaitingHuman { .. }
                ) {
                    tracing::warn!(
                        "⚠️ Stage advancement blocked: Async operation in progress (phase: {:?})",
                        state.phase
                    );
                    return;
                }

                // D131/D132: Ensure ACE frame exists before ship gate validation
                // If capture mode allows persistence and no ACE frame exists, create one
                if stage == SpecStage::Unlock {
                    let spec_id_for_ace = state.spec_id.clone();
                    let capture_mode_for_ace = state.capture_mode;
                    let cwd_for_ace = widget.config.cwd.clone();

                    // Only create ACE frame if capture mode allows persistence
                    if capture_mode_for_ace != LLMCaptureMode::None {
                        let run_id_for_check = state.run_id.clone().unwrap_or_default();
                        if !super::ship_gate::has_ace_milestone_frame(
                            &spec_id_for_ace,
                            &run_id_for_check,
                            &cwd_for_ace,
                        ) {
                            // Create pre-ship ACE reflection with routine success
                            tracing::info!(
                                spec_id = %spec_id_for_ace,
                                "Creating pre-ship ACE frame (always required before ship)"
                            );

                            let pre_ship_reflection = super::ace_reflector::ReflectionResult {
                                schema_version: super::ace_reflector::ACE_FRAME_SCHEMA_VERSION
                                    .to_string(),
                                patterns: vec![],
                                successes: vec!["Pipeline completed successfully".to_string()],
                                failures: vec![],
                                recommendations: vec![],
                                summary: "Pre-ship ACE reflection: routine success".to_string(),
                            };

                            if let Err(e) = super::ace_reflector::persist_ace_frame(
                                &spec_id_for_ace,
                                &pre_ship_reflection,
                                capture_mode_for_ace,
                                &cwd_for_ace,
                            ) {
                                tracing::warn!("Failed to create pre-ship ACE frame: {}", e);
                            }
                        }
                    }
                }

                // D131/D132: Ship gate validation before Unlock stage
                // Validates explainability artifacts (Maieutic Spec, ACE frames) are present
                // capture=none blocks ship with actionable messaging
                if stage == SpecStage::Unlock {
                    let spec_id_for_gate = state.spec_id.clone();
                    let run_id_for_gate = state.run_id.clone().unwrap_or_default();
                    let capture_mode_for_gate = state.capture_mode;
                    let cwd = widget.config.cwd.clone();

                    match super::ship_gate::validate_ship_gate(
                        &spec_id_for_gate,
                        &run_id_for_gate,
                        capture_mode_for_gate,
                        &cwd,
                    ) {
                        super::ship_gate::ShipGateResult::Allowed => {
                            // Ship gate passed, continue to Unlock stage
                            tracing::info!(
                                spec_id = %spec_id_for_gate,
                                "Ship gate: Passed, proceeding to Unlock"
                            );
                        }
                        super::ship_gate::ShipGateResult::BlockedPrivateScratch => {
                            // Private scratch mode - fail with actionable message
                            halt_spec_auto_no_resume(
                                widget,
                                super::ship_gate::PRIVATE_SCRATCH_MESSAGE.to_string(),
                            );
                            return;
                        }
                        super::ship_gate::ShipGateResult::BlockedMissingArtifact { artifact } => {
                            // Missing artifact - fail with resume hint
                            halt_spec_auto_with_error(
                                widget,
                                format!("Ship blocked: missing {}", artifact),
                            );
                            return;
                        }
                    }
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
                            // SPEC-KIT-981: Use config-aware agent selection
                            let stage_agents_config = Some(&widget.config.speckit_stage_agents);
                            let tier = super::execution_logger::tier_from_agent_count(
                                super::gate_evaluation::expected_agents_for_stage(
                                    stage,
                                    stage_agents_config,
                                )
                                .len(),
                            );
                            let expected_agents: Vec<String> =
                                super::gate_evaluation::expected_agents_for_stage(
                                    stage,
                                    stage_agents_config,
                                )
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
                    SpecAutoPhase::Stage0Pending { .. } => {
                        return; // Stage0 running in background - poll in on_commit_tick
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
                    vec![ratatui::text::Line::from("/speckit.auto pipeline complete")],
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

                // SPEC-KIT-920: Signal automation success for exit code
                widget
                    .app_event_tx
                    .send(crate::app_event::AppEvent::AutomationSuccess);

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
                // SPEC-KIT-971: Create capsule checkpoint after git commit
                let commit_result = if widget.spec_kit_auto_commit_enabled() {
                    match super::git_integration::auto_commit_stage_artifacts(
                        &spec_id,
                        current_stage,
                        &widget.config.cwd,
                        true, // Already checked via spec_kit_auto_commit_enabled()
                    ) {
                        Ok(Some(result)) => {
                            tracing::info!(
                                "Auto-commit successful for {} stage ({})",
                                current_stage.display_name(),
                                result.commit_hash
                            );
                            Some(result)
                        }
                        Ok(None) => {
                            tracing::debug!(
                                "No changes to commit for {} stage",
                                current_stage.display_name()
                            );
                            None
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
                            None
                        }
                    }
                } else {
                    None
                };

                // SPEC-KIT-971: Create capsule checkpoint for stage boundary
                if let Some(state) = widget.spec_auto_state.as_ref()
                    && let Some(run_id) = &state.run_id
                {
                    let commit_hash = commit_result.as_ref().map(|r| r.commit_hash.as_str());
                    match super::git_integration::create_capsule_checkpoint(
                        &spec_id,
                        run_id,
                        current_stage,
                        commit_hash,
                        &widget.config.cwd,
                    ) {
                        Ok(checkpoint_id) => {
                            tracing::info!(
                                "Created capsule checkpoint {} for {} stage",
                                checkpoint_id,
                                current_stage.display_name()
                            );

                            // SPEC-KIT-971: Merge run branch to main at Unlock completion
                            // Invariant: Merge modes are curated|full only (never squash/ff)
                            if current_stage == SpecStage::Unlock {
                                match super::git_integration::merge_run_branch_to_main(
                                    &spec_id,
                                    run_id,
                                    &widget.config.cwd,
                                ) {
                                    Ok(merge_checkpoint_id) => {
                                        tracing::info!(
                                            "Merged run branch {} to main (checkpoint: {})",
                                            run_id,
                                            merge_checkpoint_id
                                        );
                                    }
                                    Err(err) => {
                                        tracing::warn!(
                                            "Branch merge at Unlock failed (non-fatal): {}",
                                            err
                                        );
                                    }
                                }

                                // SPEC-KIT-974 AC#4: Risk-based auto-export at Unlock
                                if let Some(state) = widget.spec_auto_state.as_ref() {
                                    let export_config = &state.pipeline_config.capsule.export;
                                    if should_auto_export_capsule(export_config) {
                                        let output_path = widget
                                            .config
                                            .cwd
                                            .join("docs/specs")
                                            .join(&spec_id)
                                            .join("runs")
                                            .join(run_id)
                                            .join("capsule.mv2e");
                                        if let Err(e) = auto_export_capsule_at_unlock(
                                            &widget.config.cwd,
                                            &spec_id,
                                            run_id,
                                            &output_path,
                                        ) {
                                            tracing::warn!(
                                                "Auto-export at Unlock failed (non-fatal): {}",
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            tracing::warn!("Capsule checkpoint failed (non-fatal): {}", err);
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
                // Stage review failed - halt (no retries)
                halt_spec_auto_with_error(
                    widget,
                    format!("Stage review failed for {}", current_stage.display_name()),
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
        SpecStage::Specify | SpecStage::Clarify | SpecStage::Analyze | SpecStage::Checklist => {
            panic!("Stage {:?} should not be in /speckit.auto pipeline", stage)
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

    // Evidence directory path (dynamic based on spec_id)
    let evidence_dir = super::evidence::evidence_base_for_spec(std::path::Path::new("."), spec_id);
    fs::create_dir_all(&evidence_dir)
        .map_err(|e| format!("Failed to create evidence directory: {}", e))?;

    let skip_file = evidence_dir.join(format!(
        "speckit-{}_SKIPPED.json",
        stage.display_name().to_lowercase()
    ));

    let json_str = serde_json::to_string_pretty(&skip_metadata)
        .map_err(|e| format!("Failed to serialize skip metadata: {}", e))?;

    fs::write(&skip_file, json_str)
        .map_err(|e| format!("Failed to write skip telemetry to {:?}: {}", skip_file, e))?;

    tracing::debug!("📝 Skip telemetry recorded: {:?}", skip_file);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// SPEC-KIT-974 AC#4: Risk-based auto-export at Unlock
// ─────────────────────────────────────────────────────────────────────────────

/// Determine if risk-based auto-export should trigger at Unlock.
///
/// SPEC-KIT-974 AC#4: Controls when capsule is auto-exported.
///
/// - `always`: Always auto-export at Unlock completion
/// - `risk`: Auto-export only when risk conditions are met (high_risk OR audit_handoff_required)
/// - `manual`: Never auto-export; user must run `speckit capsule export` explicitly
fn should_auto_export_capsule(config: &CapsuleExportConfig) -> bool {
    match config.mode.as_str() {
        "always" => true,
        "manual" => false,
        "risk" => {
            // Risk mode: export iff high_risk OR audit_handoff_required
            config.high_risk || config.audit_handoff_required
        }
        _ => {
            tracing::warn!(
                "Unknown capsule.export.mode '{}', defaulting to manual",
                config.mode
            );
            false
        }
    }
}

/// Best-effort auto-export capsule at Unlock completion.
///
/// SPEC-KIT-974 AC#4: Performs encrypted, safe-mode, non-interactive export.
fn auto_export_capsule_at_unlock(
    cwd: &std::path::Path,
    spec_id: &str,
    run_id: &str,
    output_path: &std::path::Path,
) -> Result<(), String> {
    use crate::memvid_adapter::{CapsuleHandle, ExportOptions};

    // Ensure output directory exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create export directory: {}", e))?;
    }

    // Open capsule
    let capsule_config = default_capsule_config(cwd);
    let handle = CapsuleHandle::open(capsule_config)
        .map_err(|e| format!("Failed to open capsule for export: {}", e))?;

    // Export with LOCKED settings: encrypt=true, safe_mode=true, interactive=false
    // Passphrase is sourced from SPECKIT_MEMVID_PASSPHRASE env var (required for non-interactive)
    // Branch isolation: export only from the run-specific branch
    let options = ExportOptions {
        output_path: output_path.to_path_buf(),
        spec_id: Some(spec_id.to_string()),
        run_id: Some(run_id.to_string()),
        branch: Some(BranchId::for_run(run_id)),
        encrypt: true,
        safe_mode: true,
        interactive: false, // Non-interactive; requires SPECKIT_MEMVID_PASSPHRASE env var
    };

    let result = handle
        .export(&options)
        .map_err(|e| format!("Export failed: {}", e))?;

    tracing::info!(
        "SPEC-KIT-974: Auto-exported capsule at Unlock ({} bytes, safe_mode=true, encrypt=true)",
        result.bytes_written
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// P91/SPEC-KIT-105: Constitution Readiness Gate
// P92: Added Block mode support
// ─────────────────────────────────────────────────────────────────────────────

/// Run Phase -1 constitution readiness gate check
///
/// This function checks if the project has a valid constitution and displays
/// warnings/errors if it's missing or incomplete.
///
/// # Returns
/// - `true`: Pipeline should proceed
/// - `false`: Pipeline should abort (Block mode + constitution issues)
///
/// # Behavior
/// - If gate_mode is Skip: Returns true (no check)
/// - If gate_mode is Warn (default): Shows warnings but returns true
/// - If gate_mode is Block (P92): Shows error and returns false when constitution is incomplete
pub fn run_constitution_readiness_gate(widget: &mut ChatWidget) -> bool {
    // Try to load Stage0 config and check gate mode
    let config = match codex_stage0::Stage0Config::load() {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!("Could not load Stage0 config for gate check: {}", e);
            return true; // Can't check, allow to proceed
        }
    };

    // Check if gate is skipped
    if config.phase1_gate_mode == codex_stage0::GateMode::Skip {
        tracing::debug!("Constitution gate check skipped (phase1_gate_mode = skip)");
        return true;
    }

    // Try to connect to overlay DB and run readiness check
    let db = match codex_stage0::OverlayDb::connect_and_init(&config) {
        Ok(d) => d,
        Err(e) => {
            // Can't check - log but don't block
            tracing::debug!("Could not connect to overlay DB for gate check: {}", e);
            return true;
        }
    };

    let warnings = codex_stage0::check_constitution_readiness(&db);

    if warnings.is_empty() {
        tracing::debug!("Constitution readiness gate: OK");
        return true;
    }

    // P92: Check if we should block execution
    let is_block_mode = config.phase1_gate_mode == codex_stage0::GateMode::Block;

    // Display warnings/errors to user
    let mut lines: Vec<ratatui::text::Line<'static>> = Vec::new();

    if is_block_mode {
        lines.push(ratatui::text::Line::from(
            "❌ Constitution Readiness Gate BLOCKED (P92):",
        ));
    } else {
        lines.push(ratatui::text::Line::from(
            "⚠ Constitution Readiness Check (P91):",
        ));
    }

    for warning in &warnings {
        lines.push(ratatui::text::Line::from(format!("  • {}", warning)));
    }
    lines.push(ratatui::text::Line::from(
        "  Run /speckit.constitution to configure.",
    ));

    if is_block_mode {
        lines.push(ratatui::text::Line::from(
            "  Pipeline aborted. Set phase1_gate_mode = \"warn\" to proceed without constitution.",
        ));
    }

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        lines,
        if is_block_mode {
            HistoryCellType::Error
        } else {
            HistoryCellType::Notice
        },
    ));

    if is_block_mode {
        tracing::error!(
            "Constitution readiness gate: BLOCKED ({} issue(s))",
            warnings.len()
        );
        return false;
    }

    tracing::warn!("Constitution readiness gate: {} warning(s)", warnings.len());
    true
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 2: Intake Presence Gate
// ─────────────────────────────────────────────────────────────────────────────

/// Pending intake backfill state - stored when backfill modal is shown
#[derive(Debug, Clone)]
pub struct PendingIntakeBackfill {
    pub spec_id: String,
    pub goal: String,
    pub resume_from: SpecStage,
    pub hal_mode: Option<HalMode>,
    pub cli_overrides: Option<PipelineOverrides>,
    pub stage0_config: super::stage0_integration::Stage0ExecutionConfig,
    pub started_at: std::time::Instant,
}

/// Check for IntakeCompleted event in capsule for this spec (Phase 2)
///
/// Returns true if intake is present (continue pipeline), false if backfill needed (pause).
///
/// ## Capsule Query Pattern
/// 1. Open capsule with default_capsule_config
/// 2. list_events() and filter for EventType::IntakeCompleted
/// 3. Check if any event has kind=Spec and spec_id matches
/// 4. Verify brief_uri resolves via capsule.get_bytes()
fn run_intake_presence_gate(
    widget: &mut ChatWidget,
    spec_id: &str,
    goal: &str,
    resume_from: SpecStage,
    hal_mode: Option<HalMode>,
    cli_overrides: Option<PipelineOverrides>,
    stage0_config: super::stage0_integration::Stage0ExecutionConfig,
) -> bool {
    use crate::memvid_adapter::{
        CapsuleHandle, EventType, IntakeCompletedPayload, IntakeKind, default_capsule_config,
    };

    // Open capsule - STRICT: capsule is SoR, fail if unavailable
    let capsule_config = default_capsule_config(&widget.config.cwd);
    let capsule = match CapsuleHandle::open(capsule_config) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(
                spec_id = %spec_id,
                error = %e,
                "IntakePresenceGate: Failed to open capsule - halting pipeline (capsule is SoR)"
            );
            // Halt pipeline with visible error - capsule SoR enforcement
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Pipeline halted: Capsule unavailable\n\n  Error: {}\n\n  \
                 The capsule is the System of Record for intake artifacts.\n  \
                 Fix the capsule issue and retry: /speckit.auto {}",
                e, spec_id
            )));
            widget.request_redraw();
            // Clear any pending pipeline state
            widget.spec_auto_state = None;
            return false;
        }
    };

    // Query for IntakeCompleted events (most recent first)
    let events = capsule.list_events();
    let intake_event = events.iter().rev().find(|ev| {
        if ev.event_type != EventType::IntakeCompleted {
            return false;
        }
        // Parse payload
        if let Ok(payload) = serde_json::from_value::<IntakeCompletedPayload>(ev.payload.clone()) {
            if payload.kind == IntakeKind::Spec {
                if let Some(ref ev_spec_id) = payload.spec_id {
                    return ev_spec_id == spec_id;
                }
            }
        }
        false
    });

    if let Some(event) = intake_event {
        // Parse payload to get brief_uri
        if let Ok(payload) = serde_json::from_value::<IntakeCompletedPayload>(event.payload.clone())
        {
            // Verify brief_uri resolves
            let uri = match payload
                .brief_uri
                .parse::<crate::memvid_adapter::LogicalUri>()
            {
                Ok(u) => u,
                Err(_) => {
                    tracing::warn!(
                        spec_id = %spec_id,
                        brief_uri = %payload.brief_uri,
                        "IntakePresenceGate: Invalid brief_uri format, triggering backfill"
                    );
                    return trigger_intake_backfill(
                        widget,
                        spec_id,
                        goal,
                        resume_from,
                        hal_mode,
                        cli_overrides,
                        stage0_config,
                    );
                }
            };

            match capsule.get_bytes(&uri, None, None) {
                Ok(_) => {
                    // Intake exists and brief is accessible
                    tracing::info!(
                        spec_id = %spec_id,
                        intake_id = %payload.intake_id,
                        "IntakePresenceGate: Intake found, continuing"
                    );
                    widget.history_push(crate::history_cell::PlainHistoryCell::new(
                        vec![ratatui::text::Line::from(format!(
                            "Intake: Found ({})",
                            &payload.intake_id[..8.min(payload.intake_id.len())]
                        ))],
                        HistoryCellType::Notice,
                    ));
                    return true; // Continue pipeline
                }
                Err(e) => {
                    tracing::warn!(
                        spec_id = %spec_id,
                        error = %e,
                        "IntakePresenceGate: Brief URI unresolvable, triggering backfill"
                    );
                }
            }
        }
    }

    // No valid intake found - trigger backfill
    tracing::info!(
        spec_id = %spec_id,
        "IntakePresenceGate: No intake found, triggering backfill"
    );
    trigger_intake_backfill(
        widget,
        spec_id,
        goal,
        resume_from,
        hal_mode,
        cli_overrides,
        stage0_config,
    )
}

/// Trigger intake backfill modal (returns false to pause pipeline)
fn trigger_intake_backfill(
    widget: &mut ChatWidget,
    spec_id: &str,
    goal: &str,
    resume_from: SpecStage,
    hal_mode: Option<HalMode>,
    cli_overrides: Option<PipelineOverrides>,
    stage0_config: super::stage0_integration::Stage0ExecutionConfig,
) -> bool {
    // Store pending state for resumption
    widget.pending_intake_backfill = Some(PendingIntakeBackfill {
        spec_id: spec_id.to_string(),
        goal: goal.to_string(),
        resume_from,
        hal_mode,
        cli_overrides,
        stage0_config,
        started_at: std::time::Instant::now(),
    });

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(
            "Intake: Required (backfill mode)",
        )],
        HistoryCellType::Notice,
    ));

    // Show backfill modal with existing spec_id
    widget.show_spec_intake_modal_backfill(spec_id.to_string());

    false // Pause pipeline
}

/// Resume pipeline after intake backfill completes
///
/// Called when SpecIntakeSubmitted event fires with existing_spec_id set.
/// This function continues the pipeline by calling the maieutic gate.
/// If maieutic passes (already completed), it re-invokes handle_spec_auto
/// to continue the pipeline from there.
pub fn resume_pipeline_after_intake_backfill(widget: &mut ChatWidget) {
    let Some(pending) = widget.pending_intake_backfill.take() else {
        // Not a backfill from pipeline gate - just a normal intake
        return;
    };

    let duration_ms = pending.started_at.elapsed().as_millis() as u64;
    tracing::info!(
        spec_id = %pending.spec_id,
        duration_ms = duration_ms,
        "IntakePresenceGate: Backfill completed, resuming pipeline"
    );

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Intake backfill complete ({}ms), continuing pipeline",
            duration_ms
        ))],
        HistoryCellType::Notice,
    ));

    // Re-invoke handle_spec_auto - it will skip constitution and intake gates
    // (constitution already passed, intake now present in capsule)
    // and continue with maieutic and the rest of the pipeline
    handle_spec_auto(
        widget,
        pending.spec_id,
        pending.goal,
        pending.resume_from,
        pending.hal_mode,
        pending.cli_overrides,
        pending.stage0_config,
    );
}

/// Cancel pipeline after intake backfill is cancelled
pub fn cancel_pipeline_after_intake_backfill(widget: &mut ChatWidget, spec_id: &str) {
    widget.pending_intake_backfill = None;
    tracing::info!(
        spec_id = %spec_id,
        "IntakePresenceGate: Backfill cancelled by user"
    );
    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Pipeline cancelled - intake backfill declined for {}",
            spec_id
        ))],
        HistoryCellType::Notice,
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// P93/D130: Maieutic Elicitation Gate
// ─────────────────────────────────────────────────────────────────────────────

/// Pending maieutic state - stored when maieutic modal is shown
#[derive(Debug, Clone)]
pub struct PendingMaieutic {
    pub spec_id: String,
    pub goal: String,
    pub resume_from: SpecStage,
    pub hal_mode: Option<HalMode>,
    pub cli_overrides: Option<PipelineOverrides>,
    pub stage0_config: super::stage0_integration::Stage0ExecutionConfig,
    pub started_at: std::time::Instant,
    /// Capture mode from Stage0 policy (D131: persistence follows capture mode)
    pub capture_mode: LLMCaptureMode,
}

/// Run the mandatory maieutic elicitation gate (D130)
///
/// Checks if maieutic elicitation is needed and shows the modal if so.
/// Returns true if maieutic is complete (continue pipeline), false if modal shown (pause pipeline).
///
/// # Arguments
/// * `widget` - ChatWidget
/// * `spec_id` - SPEC ID
/// * `goal` - Goal description
/// * `resume_from` - Stage to resume from
/// * `hal_mode` - HAL mode setting
/// * `cli_overrides` - CLI overrides for pipeline config
/// * `stage0_config` - Stage 0 execution config
fn run_maieutic_gate(
    widget: &mut ChatWidget,
    spec_id: &str,
    goal: &str,
    resume_from: SpecStage,
    hal_mode: Option<HalMode>,
    cli_overrides: Option<PipelineOverrides>,
    stage0_config: super::stage0_integration::Stage0ExecutionConfig,
) -> bool {
    // D133: Check for pre-supplied maieutic (headless mode - future PR)
    // For now, always require interactive elicitation

    // Check if maieutic was already completed (e.g., from previous aborted run)
    if super::maieutic::has_maieutic_completed(spec_id, "", &widget.config.cwd) {
        tracing::info!(
            spec_id = %spec_id,
            "Maieutic elicitation: Previously completed, skipping"
        );
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(
                "⚡ Maieutic: Using previous elicitation",
            )],
            HistoryCellType::Notice,
        ));
        return true; // Continue pipeline
    }

    // D131: Load capture mode from governance policy for maieutic persistence
    let capture_mode = codex_stage0::GovernancePolicy::load(None)
        .ok()
        .and_then(|gov| LLMCaptureMode::from_str(&gov.capture.mode))
        .unwrap_or(LLMCaptureMode::PromptsOnly);

    // Store pending maieutic state for resumption after modal completes
    widget.pending_maieutic = Some(PendingMaieutic {
        spec_id: spec_id.to_string(),
        goal: goal.to_string(),
        resume_from,
        hal_mode,
        cli_overrides,
        stage0_config,
        started_at: std::time::Instant::now(),
        capture_mode,
    });

    // Show maieutic modal
    tracing::info!(
        spec_id = %spec_id,
        "Maieutic elicitation: Showing modal"
    );

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(
            "📋 Maieutic: Pre-flight clarification required",
        )],
        HistoryCellType::Notice,
    ));

    // Get questions and show modal
    let questions = super::maieutic::default_fast_path_questions();
    widget.show_maieutic_modal(spec_id.to_string(), questions);

    false // Pause pipeline - will resume via MaieuticSubmitted event
}

/// Resume pipeline after maieutic elicitation completes
///
/// Called when MaieuticSubmitted event fires. Continues the pipeline
/// from where run_maieutic_gate paused.
///
/// # Arguments
/// * `widget` - ChatWidget
/// * `maieutic_spec` - Completed maieutic spec from modal
pub fn resume_pipeline_after_maieutic(
    widget: &mut ChatWidget,
    maieutic_spec: super::maieutic::MaieuticSpec,
) {
    // Retrieve pending maieutic state
    let Some(pending) = widget.pending_maieutic.take() else {
        tracing::error!("resume_pipeline_after_maieutic called with no pending maieutic state");
        widget.history_push(crate::history_cell::new_error_event(
            "Internal error: No pending maieutic state".to_string(),
        ));
        return;
    };

    let duration_ms = pending.started_at.elapsed().as_millis() as u64;
    tracing::info!(
        spec_id = %pending.spec_id,
        duration_ms = duration_ms,
        "Maieutic elicitation: Completed, resuming pipeline"
    );

    // Persist maieutic spec (D131: persistence follows capture mode)
    // capture_mode=None runs in-memory only, others persist to evidence
    match super::maieutic::persist_maieutic_spec(
        &pending.spec_id,
        &maieutic_spec,
        pending.capture_mode,
        &widget.config.cwd,
    ) {
        Ok(Some(path)) => {
            tracing::info!(
                spec_id = %pending.spec_id,
                path = %path.display(),
                "Maieutic spec persisted"
            );
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "✅ Maieutic: Elicitation complete ({}ms)",
                    duration_ms
                ))],
                HistoryCellType::Notice,
            ));
        }
        Ok(None) => {
            // capture_mode=None, in-memory only
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "✅ Maieutic: Elicitation complete ({}ms, in-memory)",
                    duration_ms
                ))],
                HistoryCellType::Notice,
            ));
        }
        Err(e) => {
            tracing::warn!(
                spec_id = %pending.spec_id,
                error = %e,
                "Failed to persist maieutic spec - continuing anyway"
            );
        }
    }

    // Continue pipeline with original parameters
    // Call handle_spec_auto_internal which skips the maieutic gate
    handle_spec_auto_after_maieutic(
        widget,
        pending.spec_id,
        pending.goal,
        pending.resume_from,
        pending.hal_mode,
        pending.cli_overrides,
        pending.stage0_config,
        maieutic_spec,
        pending.capture_mode, // D131: Capture mode for ship gate
    );
}

/// Cancel pipeline after maieutic elicitation is cancelled
///
/// Called when MaieuticCancelled event fires.
pub fn cancel_pipeline_after_maieutic(widget: &mut ChatWidget, spec_id: &str) {
    // Clear pending maieutic state
    widget.pending_maieutic = None;

    tracing::info!(
        spec_id = %spec_id,
        "Maieutic elicitation: Cancelled by user"
    );

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(
            "❌ Maieutic: Elicitation cancelled - pipeline aborted",
        )],
        HistoryCellType::Notice,
    ));
}

/// Continue handle_spec_auto after maieutic elicitation completes
///
/// This is the continuation of handle_spec_auto after the maieutic gate passes.
/// It duplicates the logic from handle_spec_auto starting after the maieutic gate.
fn handle_spec_auto_after_maieutic(
    widget: &mut ChatWidget,
    spec_id: String,
    goal: String,
    resume_from: SpecStage,
    hal_mode: Option<HalMode>,
    cli_overrides: Option<PipelineOverrides>,
    stage0_config: super::stage0_integration::Stage0ExecutionConfig,
    maieutic_spec: super::maieutic::MaieuticSpec,
    capture_mode: LLMCaptureMode, // D131: Capture mode for ship gate
) {
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

    let lifecycle = widget.ensure_validate_lifecycle(&spec_id);
    let mut state = super::state::SpecAutoState::new(
        spec_id.clone(),
        goal,
        resume_from,
        hal_mode,
        pipeline_config,
        capture_mode, // D131: Capture mode for ship gate
    );
    state.set_validate_lifecycle(lifecycle);

    // Set maieutic as completed
    state.maieutic_completed = true;
    state.maieutic_spec = Some(maieutic_spec);

    // SPEC-KIT-102: Set Stage 0 configuration from CLI flags
    state.stage0_disabled = stage0_config.disabled;
    state.stage0_explain = stage0_config.explain;

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

    // SPEC-KIT-977: Capture policy snapshot at run start for phase 4→5 gate
    if let Some(run_id) = &state.run_id {
        let capsule_config = default_capsule_config(&widget.config.cwd);

        match CapsuleHandle::open(capsule_config) {
            Ok(handle) => {
                if let Err(e) = handle.switch_branch(BranchId::for_run(run_id)) {
                    tracing::warn!(
                        run_id = %run_id,
                        error = %e,
                        "Failed to switch capsule branch for run"
                    );
                }

                let stage0_cfg = codex_stage0::Stage0Config::load().unwrap_or_default();
                match policy_capture::capture_and_store_policy(
                    &handle,
                    &stage0_cfg,
                    &spec_id,
                    run_id,
                ) {
                    Ok(snapshot) => {
                        state.policy_id = Some(snapshot.policy_id.clone());
                        state.policy_hash = Some(snapshot.hash.clone());
                        if let Some(info) = handle.current_policy() {
                            state.policy_uri = Some(info.uri.as_str().to_string());
                        }
                        tracing::info!(
                            policy_id = %snapshot.policy_id,
                            hash = %snapshot.hash,
                            "Policy snapshot captured at run start"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to capture policy snapshot - continuing without policy binding");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to open capsule for policy capture - continuing without policy binding");
            }
        }
    }

    // SPEC-KIT-102: Run Stage 0 context injection before pipeline starts
    if !stage0_config.disabled {
        let spec_path = widget.config.cwd.join(format!("docs/{}/spec.md", spec_id));
        let spec_content = std::fs::read_to_string(&spec_path).unwrap_or_default();

        if !spec_content.is_empty() {
            if let Some(run_id) = &state.run_id {
                state.execution_logger.log_event(
                    super::execution_logger::ExecutionEvent::Stage0Start {
                        run_id: run_id.clone(),
                        spec_id: spec_id.clone(),
                        tier2_enabled: codex_stage0::Stage0Config::load().ok().is_some_and(|cfg| {
                            let notebook_override = widget
                                .config
                                .stage0
                                .notebook
                                .clone()
                                .or(widget.config.stage0.notebook_url.clone())
                                .or(widget.config.stage0.notebook_id.clone())
                                .unwrap_or_default();
                            if !notebook_override.trim().is_empty() {
                                return true;
                            }
                            cfg.tier2.enabled && !cfg.tier2.notebook.trim().is_empty()
                        }),
                        explain_enabled: stage0_config.explain,
                        timestamp: super::execution_logger::ExecutionEvent::now(),
                    },
                );
            }

            // Spawn Stage0 async
            let pending = super::stage0_integration::spawn_stage0_async(
                widget.config.clone(),
                spec_id.clone(),
                spec_content.clone(),
                widget.config.cwd.clone(),
                stage0_config.clone(),
            );

            widget.stage0_pending = Some(pending);
            widget
                .app_event_tx
                .send(crate::app_event::AppEvent::StartCommitAnimation);

            state.phase = super::state::SpecAutoPhase::Stage0Pending {
                status: "Starting Stage0...".to_string(),
                started_at: std::time::Instant::now(),
            };
            widget.spec_auto_state = Some(state);

            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(
                    "⚙️  Stage 0: Starting (background)...",
                )],
                crate::history_cell::HistoryCellType::Notice,
            ));

            return;
        } else {
            let skip_reason = "spec.md is empty or not found";
            if let Some(run_id) = &state.run_id {
                state.execution_logger.log_event(
                    super::execution_logger::ExecutionEvent::Stage0Complete {
                        run_id: run_id.clone(),
                        spec_id: spec_id.clone(),
                        duration_ms: 0,
                        tier2_used: false,
                        cache_hit: false,
                        hybrid_used: false,
                        memories_used: 0,
                        task_brief_written: false,
                        skip_reason: Some(skip_reason.to_string()),
                        timestamp: super::execution_logger::ExecutionEvent::now(),
                    },
                );
            }
            state.stage0_skip_reason = Some(skip_reason.to_string());

            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "Stage 0: Skipped ({})",
                    skip_reason
                ))],
                crate::history_cell::HistoryCellType::Notice,
            ));
        }
    } else {
        let skip_reason = "Stage 0 disabled by flag";
        if let Some(run_id) = &state.run_id {
            state.execution_logger.log_event(
                super::execution_logger::ExecutionEvent::Stage0Complete {
                    run_id: run_id.clone(),
                    spec_id: spec_id.clone(),
                    duration_ms: 0,
                    tier2_used: false,
                    cache_hit: false,
                    hybrid_used: false,
                    memories_used: 0,
                    task_brief_written: false,
                    skip_reason: Some(skip_reason.to_string()),
                    timestamp: super::execution_logger::ExecutionEvent::now(),
                },
            );
        }
        state.stage0_skip_reason = Some(skip_reason.to_string());

        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "Stage 0: Skipped ({})",
                skip_reason
            ))],
            crate::history_cell::HistoryCellType::Notice,
        ));
    }

    widget.spec_auto_state = Some(state);
    advance_spec_auto(widget);
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memvid_adapter::{EventType, RetrievalRequestPayload};
    use tempfile::TempDir;

    // =========================================================================
    // SPEC-KIT-974 AC#4: should_auto_export_capsule unit tests
    // =========================================================================

    #[test]
    fn test_should_auto_export_capsule_always() {
        let config = CapsuleExportConfig {
            mode: "always".to_string(),
            high_risk: false,
            audit_handoff_required: false,
        };
        assert!(
            should_auto_export_capsule(&config),
            "always mode should always export"
        );
    }

    #[test]
    fn test_should_auto_export_capsule_manual() {
        let config = CapsuleExportConfig {
            mode: "manual".to_string(),
            high_risk: true, // Even with flags, manual = no export
            audit_handoff_required: true,
        };
        assert!(
            !should_auto_export_capsule(&config),
            "manual mode should never export regardless of flags"
        );
    }

    #[test]
    fn test_should_auto_export_capsule_risk_high_risk() {
        let config = CapsuleExportConfig {
            mode: "risk".to_string(),
            high_risk: true,
            audit_handoff_required: false,
        };
        assert!(
            should_auto_export_capsule(&config),
            "risk mode should export when high_risk=true"
        );
    }

    #[test]
    fn test_should_auto_export_capsule_risk_audit() {
        let config = CapsuleExportConfig {
            mode: "risk".to_string(),
            high_risk: false,
            audit_handoff_required: true,
        };
        assert!(
            should_auto_export_capsule(&config),
            "risk mode should export when audit_handoff_required=true"
        );
    }

    #[test]
    fn test_should_auto_export_capsule_risk_both_flags() {
        let config = CapsuleExportConfig {
            mode: "risk".to_string(),
            high_risk: true,
            audit_handoff_required: true,
        };
        assert!(
            should_auto_export_capsule(&config),
            "risk mode should export when both flags are true"
        );
    }

    #[test]
    fn test_should_auto_export_capsule_risk_no_flags() {
        let config = CapsuleExportConfig {
            mode: "risk".to_string(),
            high_risk: false,
            audit_handoff_required: false,
        };
        assert!(
            !should_auto_export_capsule(&config),
            "risk mode should NOT export when both flags are false"
        );
    }

    #[test]
    fn test_should_auto_export_capsule_unknown_mode() {
        let config = CapsuleExportConfig {
            mode: "unknown".to_string(),
            high_risk: true,
            audit_handoff_required: true,
        };
        assert!(
            !should_auto_export_capsule(&config),
            "unknown mode should default to manual (no export)"
        );
    }

    // =========================================================================
    // Tier2 degraded event emission tests
    // =========================================================================

    /// Helper to create a Tier2Trace for testing
    fn make_tier2_trace(enabled: bool, health_ok: bool, error: Option<&str>) -> Tier2Trace {
        Tier2Trace {
            base_url: "http://localhost:3456".to_string(),
            notebook_id: "test-notebook".to_string(),
            enabled,
            health_ok,
            latency_ms: Some(100),
            skip_reason: if error.is_some() {
                error.map(|e| e.to_string())
            } else if !health_ok {
                Some("intentional skip".to_string())
            } else {
                None
            },
            error: error.map(|e| e.to_string()),
        }
    }

    /// Count tier2:notebooklm events in capsule
    fn count_tier2_events(handle: &CapsuleHandle, run_id: &str) -> (usize, usize) {
        let events = handle.list_events_filtered(Some(&BranchId::for_run(run_id)));

        let requests = events
            .iter()
            .filter(|e| e.event_type == EventType::RetrievalRequest)
            .filter(|e| {
                // payload is serde_json::Value, check if it can be parsed as RetrievalRequestPayload
                if !e.payload.is_null() {
                    if let Ok(req) =
                        serde_json::from_value::<RetrievalRequestPayload>(e.payload.clone())
                    {
                        return req.source == "tier2:notebooklm";
                    }
                }
                false
            })
            .count();

        let responses = events
            .iter()
            .filter(|e| e.event_type == EventType::RetrievalResponse)
            .count();

        (requests, responses)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Negative cases: should NOT emit events
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_tier2_emission_precheck_skip_no_events() {
        let temp_dir = TempDir::new().unwrap();
        let run_id = "run-precheck-skip";

        // Pre-check hit: enabled=true, health_ok=false, error=None
        let trace = make_tier2_trace(true, false, None);
        assert!(
            !trace.should_emit_degraded_event(),
            "Pre-check skip should NOT emit"
        );

        emit_tier2_degraded_events_if_needed(temp_dir.path(), "SPEC-TEST", run_id, &trace);

        // Assert zero I/O: no capsule file created
        let capsule_path = default_capsule_config(temp_dir.path()).capsule_path;
        assert!(
            !capsule_path.exists(),
            "Helper must not touch capsule on non-emission paths"
        );
    }

    #[test]
    fn test_tier2_emission_no_notebook_no_events() {
        let temp_dir = TempDir::new().unwrap();
        let run_id = "run-no-notebook";

        // No notebook: enabled=true, health_ok=false, error=None
        let trace = make_tier2_trace(true, false, None);
        assert!(
            !trace.should_emit_degraded_event(),
            "No notebook configured should NOT emit"
        );

        emit_tier2_degraded_events_if_needed(temp_dir.path(), "SPEC-TEST", run_id, &trace);

        // Assert zero I/O: no capsule file created
        let capsule_path = default_capsule_config(temp_dir.path()).capsule_path;
        assert!(
            !capsule_path.exists(),
            "Helper must not touch capsule on non-emission paths"
        );
    }

    #[test]
    fn test_tier2_emission_disabled_no_events() {
        let temp_dir = TempDir::new().unwrap();
        let run_id = "run-disabled";

        // Disabled: enabled=false, health_ok=false, error=None
        let trace = make_tier2_trace(false, false, None);
        assert!(
            !trace.should_emit_degraded_event(),
            "Disabled Tier2 should NOT emit"
        );

        emit_tier2_degraded_events_if_needed(temp_dir.path(), "SPEC-TEST", run_id, &trace);

        // Assert zero I/O: no capsule file created
        let capsule_path = default_capsule_config(temp_dir.path()).capsule_path;
        assert!(
            !capsule_path.exists(),
            "Helper must not touch capsule on non-emission paths"
        );
    }

    #[test]
    fn test_tier2_emission_healthy_no_events() {
        let temp_dir = TempDir::new().unwrap();
        let run_id = "run-healthy";

        // Healthy: enabled=true, health_ok=true, error=None
        let trace = make_tier2_trace(true, true, None);
        assert!(
            !trace.should_emit_degraded_event(),
            "Healthy Tier2 should NOT emit"
        );

        emit_tier2_degraded_events_if_needed(temp_dir.path(), "SPEC-TEST", run_id, &trace);

        // Assert zero I/O: no capsule file created
        let capsule_path = default_capsule_config(temp_dir.path()).capsule_path;
        assert!(
            !capsule_path.exists(),
            "Helper must not touch capsule on non-emission paths"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Positive cases: SHOULD emit events
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_tier2_emission_unhealthy_emits_events() {
        let temp_dir = TempDir::new().unwrap();
        let run_id = "run-unhealthy";

        // Unhealthy (timeout): enabled=true, health_ok=false, error=Some
        let trace = make_tier2_trace(true, false, Some("NotebookLM service timeout"));
        assert!(
            trace.should_emit_degraded_event(),
            "Unhealthy Tier2 SHOULD emit"
        );

        // Call helper (opens and closes its own capsule handle)
        emit_tier2_degraded_events_if_needed(temp_dir.path(), "SPEC-TEST", run_id, &trace);

        // Reopen capsule to verify events
        let config = default_capsule_config(temp_dir.path());
        let handle = CapsuleHandle::open(config).expect("should reopen capsule");

        let (requests, responses) = count_tier2_events(&handle, run_id);
        assert_eq!(
            requests, 1,
            "Should emit exactly 1 RetrievalRequest for unhealthy Tier2"
        );
        assert_eq!(
            responses, 1,
            "Should emit exactly 1 RetrievalResponse for unhealthy Tier2"
        );
    }

    #[test]
    fn test_tier2_emission_unreachable_emits_events() {
        let temp_dir = TempDir::new().unwrap();
        let run_id = "run-unreachable";

        // Unreachable: enabled=true, health_ok=false, error=Some
        let trace = make_tier2_trace(true, false, Some("NotebookLM service unreachable"));

        // Call helper (opens and closes its own capsule handle)
        emit_tier2_degraded_events_if_needed(temp_dir.path(), "SPEC-TEST", run_id, &trace);

        // Reopen capsule to verify events
        let config = default_capsule_config(temp_dir.path());
        let handle = CapsuleHandle::open(config).expect("should reopen capsule");

        let (requests, responses) = count_tier2_events(&handle, run_id);
        assert_eq!(
            requests, 1,
            "Should emit exactly 1 RetrievalRequest for unreachable Tier2"
        );
        assert_eq!(
            responses, 1,
            "Should emit exactly 1 RetrievalResponse for unreachable Tier2"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Edge case: disabled with error (should NOT emit - enabled gate wins)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_tier2_emission_disabled_with_error_no_events() {
        let temp_dir = TempDir::new().unwrap();
        let run_id = "run-disabled-error";

        // Edge case: disabled but with error (enabled gate should win)
        let trace = Tier2Trace {
            base_url: "http://localhost:3456".to_string(),
            notebook_id: "test-notebook".to_string(),
            enabled: false, // Disabled
            health_ok: false,
            latency_ms: Some(100),
            skip_reason: Some("Tier2 disabled".to_string()),
            error: Some("This error should be ignored".to_string()), // Error present but irrelevant
        };

        assert!(
            !trace.should_emit_degraded_event(),
            "Disabled Tier2 should NOT emit even with error"
        );

        emit_tier2_degraded_events_if_needed(temp_dir.path(), "SPEC-TEST", run_id, &trace);

        // Assert zero I/O: no capsule file created
        let capsule_path = default_capsule_config(temp_dir.path()).capsule_path;
        assert!(
            !capsule_path.exists(),
            "Helper must not touch capsule on non-emission paths"
        );
    }
}
