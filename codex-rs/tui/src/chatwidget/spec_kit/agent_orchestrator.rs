//! Agent orchestration and coordination
//!
//! This module handles multi-agent execution coordination:
//! - Auto-submitting prompts to agents with ACE routing
//! - Aggregator effort configuration (SPEC-KIT-070)
//! - Agent completion tracking with degraded mode continuation
//! - Cost tracking per agent
//! - Degraded follow-up scheduling

use super::super::ChatWidget;
use crate::history_cell::HistoryCellType;
use crate::spec_prompts::SpecStage;
use codex_core::protocol::{AgentInfo, InputItem};
use codex_core::slash_commands::format_subagent_command;
use super::command_handlers::halt_spec_auto_with_error;
use super::consensus::expected_agents_for_stage;
use super::consensus_coordinator::block_on_sync;
use super::handler::check_consensus_and_advance_spec_auto;
use super::quality_gate_handler::on_quality_gate_agents_complete;
use super::state::{SpecAutoPhase, ValidateBeginOutcome, ValidateRunInfo};
use super::validation_lifecycle::{
    compute_validate_payload_hash, record_validate_lifecycle_event,
    ValidateLifecycleEvent, ValidateMode,
};

pub fn auto_submit_spec_stage_prompt(widget: &mut ChatWidget, stage: SpecStage, spec_id: &str) {
    let goal = widget
        .spec_auto_state
        .as_ref()
        .map(|s| s.goal.clone())
        .unwrap_or_default();

    // ACE Framework Integration: Pre-fetch playbook bullets for this stage
    // This solves the async/sync boundary issue by fetching BEFORE prompt assembly
    let ace_bullets = {
        let ace_config = &widget.config.ace;
        if ace_config.enabled {
            use super::ace_prompt_injector::{should_use_ace, command_to_scope};
            use super::routing::{get_repo_root, get_current_branch};

            let command_name = format!("speckit.{}", stage.command_name());

            if should_use_ace(ace_config, &command_name) {
                if let Some(scope) = command_to_scope(&command_name) {
                    // Convert scope to owned String for use across async boundary
                    let scope_string = scope.to_string();

                    // Use block_on_sync for sync/async bridge
                    let repo_root_opt = get_repo_root(&widget.config.cwd);
                    let branch_opt = get_current_branch(&widget.config.cwd);

                    // Fallback to defaults if git commands fail
                    let repo_root = repo_root_opt.unwrap_or_else(|| {
                        widget.config.cwd.to_string_lossy().to_string()
                    });
                    let branch = branch_opt.unwrap_or_else(|| "main".to_string());
                    let slice_size = ace_config.slice_size;
                    let stage_name = stage.display_name().to_string();

                    let result = block_on_sync(|| {
                        let scope_clone = scope_string.clone();
                        async move {
                            super::ace_client::playbook_slice(
                                repo_root,
                                branch,
                                scope_clone,
                                slice_size,
                                false, // exclude_neutral
                            )
                            .await
                        }
                    });

                    match result {
                        super::ace_client::AceResult::Ok(response) => {
                            tracing::info!(
                                "ACE pre-fetch successful: {} bullets for {} ({})",
                                response.bullets.len(),
                                stage_name,
                                scope_string
                            );
                            Some(response.bullets)
                        }
                        super::ace_client::AceResult::Disabled => {
                            tracing::debug!("ACE disabled");
                            None
                        }
                        super::ace_client::AceResult::Error(e) => {
                            tracing::warn!("ACE pre-fetch failed for {}: {}", stage_name, e);
                            None
                        }
                    }
                } else {
                    tracing::debug!("No ACE scope mapping for {}", command_name);
                    None
                }
            } else {
                tracing::debug!("ACE not enabled for {}", command_name);
                None
            }
        } else {
            None
        }
    };

    // Cache bullets in state for synchronous injection later
    if let Some(state) = widget.spec_auto_state.as_mut() {
        state.ace_bullets_cache = ace_bullets;
        state.ace_bullet_ids_used = None; // Reset for new stage
    }

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
        Ok(mut prompt) => {
            // ACE Framework Integration (2025-10-29): Inject cached bullets
            if let Some(state) = widget.spec_auto_state.as_mut() {
                if let Some(bullets) = &state.ace_bullets_cache {
                    use super::ace_prompt_injector::format_ace_section;
                    let (ace_section, bullet_ids) = format_ace_section(bullets);
                    if !ace_section.is_empty() {
                        prompt.push_str("\n\n");
                        prompt.push_str(&ace_section);
                        state.ace_bullet_ids_used = Some(bullet_ids);
                        tracing::info!("ACE: Injected {} bullets into {} prompt", bullets.len(), stage.display_name());
                    }
                }
            }

            // SPEC-KIT-070: ACE-aligned routing — set aggregator effort per stage
            // Estimate tokens ~ chars/4
            // Always use standard routing (no retry logic)
            let routing = super::ace_route_selector::decide_stage_routing(
                stage,
                prompt.len(),
                false,
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
                state.transition_phase(
                    SpecAutoPhase::ExecutingAgents {
                        expected_agents: stage_expected,
                        completed_agents: std::collections::HashSet::new(),
                    },
                    "agents_spawned"
                );

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

            // Log agent spawn events for each expected agent (before consuming prompt)
            let prompt_preview = prompt[..200.min(prompt.len())].to_string();
            if let Some(state) = widget.spec_auto_state.as_ref() {
                if let (Some(run_id), SpecAutoPhase::ExecutingAgents { expected_agents, .. }) =
                    (&state.run_id, &state.phase)
                {
                    for agent_name in expected_agents {
                        // Note: Agent IDs not yet available at submission time
                        // Log with placeholder ID - will be updated when agents actually spawn
                        state.execution_logger.log_event(
                            super::execution_logger::ExecutionEvent::AgentSpawn {
                                run_id: run_id.clone(),
                                stage: stage.display_name().to_string(),
                                agent_name: agent_name.clone(),
                                agent_id: format!("pending-{}", agent_name), // Placeholder
                                model: agent_name.clone(), // Best guess at this point
                                prompt_preview: prompt_preview.clone(),
                                timestamp: super::execution_logger::ExecutionEvent::now(),
                            }
                        );
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


pub fn on_spec_auto_agents_complete(widget: &mut ChatWidget) {
    tracing::warn!("DEBUG: on_spec_auto_agents_complete called");
    let Some(state) = widget.spec_auto_state.as_ref() else {
        tracing::warn!("DEBUG: No spec_auto_state");
        return;
    };

    let current_stage_name = state.current_stage().map(|s| s.display_name()).unwrap_or("unknown");
    tracing::warn!("DEBUG: Current stage={}, phase={:?}", current_stage_name, state.phase);
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

    // Collect which agents completed successfully and log completion events
    // Also check SQLite to determine if these are quality gate or regular stage agents
    let db = super::consensus_db::ConsensusDb::init_default().ok();
    let mut completed_names = std::collections::HashSet::new();
    let mut quality_gate_agent_ids = std::collections::HashSet::new();

    for agent_info in &widget.active_agents {
        if matches!(agent_info.status, super::super::AgentStatus::Completed) {
            completed_names.insert(agent_info.name.to_lowercase());

            // Check if this agent was spawned as a quality gate agent
            if let Some(ref database) = db {
                if let Ok(Some((phase_type, _))) = database.get_agent_spawn_info(&agent_info.id) {
                    tracing::warn!("DEBUG: Agent {} ({}) was spawned as phase_type={}",
                        agent_info.name, agent_info.id, phase_type);
                    if phase_type == "quality_gate" {
                        quality_gate_agent_ids.insert(agent_info.id.clone());
                    }
                }
            }

            // Log agent complete event
            if let Some(state) = widget.spec_auto_state.as_ref() {
                if let Some(run_id) = &state.run_id {
                    if let Some(current_stage) = state.current_stage() {
                        // Calculate output lines from agent result (if available)
                        let output_lines = agent_info.result
                            .as_ref()
                            .map(|r| r.lines().count())
                            .unwrap_or(0);

                        state.execution_logger.log_event(
                            super::execution_logger::ExecutionEvent::AgentComplete {
                                run_id: run_id.clone(),
                                stage: current_stage.display_name().to_string(),
                                agent_name: agent_info.name.clone(),
                                agent_id: agent_info.id.clone(),
                                duration_sec: 0.0, // TODO: Calculate from agent start time if available
                                status: "completed".to_string(),
                                output_lines,
                                timestamp: super::execution_logger::ExecutionEvent::now(),
                            }
                        );
                    }
                }
            }
        }
    }

    // Update completed agents in state and determine phase type
    let phase_type = if let Some(state) = widget.spec_auto_state.as_mut() {
        let phase_type = match &mut state.phase {
            SpecAutoPhase::ExecutingAgents {
                completed_agents,
                expected_agents: phase_expected,
                ..
            } => {
                *completed_agents = completed_names.clone();
                tracing::warn!("DEBUG: Phase match → ExecutingAgents, routing to 'regular'");

                // Definitive check: Are these quality gate agents completing late?
                // Query SQLite to see if any completed agents were spawned as quality_gate phase_type
                if !quality_gate_agent_ids.is_empty() {
                    tracing::warn!("DEBUG: Found {} quality gate agents in completion set - these are stale completions",
                        quality_gate_agent_ids.len());
                    tracing::warn!("DEBUG: Quality gate agent IDs: {:?}", quality_gate_agent_ids);
                    tracing::warn!("DEBUG: Skipping - quality gates have their own completion handler");
                    return;
                }

                tracing::warn!("DEBUG: No quality gate agents detected - these are regular stage agents");
                "regular"
            }
            SpecAutoPhase::QualityGateExecuting {
                completed_agents, ..
            } => {
                *completed_agents = completed_names.clone();
                tracing::warn!("DEBUG: Phase match → QualityGateExecuting, routing to 'quality_gate'");
                "quality_gate"
            }
            SpecAutoPhase::QualityGateValidating { .. } => {
                // GPT-5 validation phase - single agent (GPT-5)
                tracing::warn!("DEBUG: Phase match → QualityGateValidating, routing to 'gpt5_validation'");
                "gpt5_validation"
            }
            _ => {
                tracing::warn!("DEBUG: Phase match → Other ({:?}), routing to 'none'", state.phase);
                "none"
            }
        };
        phase_type
    } else {
        "none"
    };

    // Handle different phase types
    tracing::warn!("DEBUG: on_spec_auto_agents_complete - phase_type={}", phase_type);
    match phase_type {
        "quality_gate" => {
            tracing::warn!("DEBUG: Quality gate path - calling on_quality_gate_agents_complete");
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
            tracing::warn!("DEBUG: Regular agent phase, checking completion");
            tracing::warn!("DEBUG: Expected agents: {:?}", expected_agents);
            tracing::warn!("DEBUG: Completed agents: {:?}", completed_names);

            // Check completion with agent name normalization
            // Handles aliases like "code" (command) vs "gpt_pro" (config name)
            let all_complete = expected_agents.iter().all(|expected| {
                let exp_lower = expected.to_lowercase();
                // Direct match
                if completed_names.contains(&exp_lower) {
                    return true;
                }
                // Special case: gpt_pro config uses "code" command
                if exp_lower == "gpt_pro" && (completed_names.contains("code") || completed_names.contains("gpt5") || completed_names.contains("gpt-5")) {
                    return true;
                }
                // Special case: code config might report as gpt_pro
                if exp_lower == "code" && completed_names.contains("gpt_pro") {
                    return true;
                }
                false
            });

            tracing::warn!("DEBUG: All complete: {}", all_complete);
            if all_complete {
                tracing::warn!("DEBUG: All regular stage agents complete, collecting responses for consensus");

                // Collect agent responses from widget.active_agents
                let agent_responses: Vec<(String, String)> = widget.active_agents.iter()
                    .filter_map(|agent| {
                        if matches!(agent.status, super::super::AgentStatus::Completed) {
                            agent.result.as_ref().map(|result| (agent.name.clone(), result.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();

                tracing::warn!("DEBUG: Collected {} agent responses for consensus", agent_responses.len());

                // SPEC-KIT-072: Store to SQLite for persistent consensus artifacts
                if let Some(state) = widget.spec_auto_state.as_ref() {
                    if let Some(current_stage) = state.current_stage() {
                        if let Some(run_id) = &state.run_id {
                            // Initialize SQLite database
                            if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
                                for (agent_name, response_text) in &agent_responses {
                                    // Try to extract JSON content for structured storage
                                    let json_str = super::pipeline_coordinator::extract_json_from_agent_response(response_text)
                                        .unwrap_or_else(|| response_text.clone());

                                    if let Err(e) = db.store_artifact(
                                        &state.spec_id,
                                        current_stage,
                                        agent_name,
                                        &json_str,
                                        Some(response_text),
                                        Some(run_id),
                                    ) {
                                        tracing::warn!("Failed to store {} artifact to SQLite: {}", agent_name, e);
                                    } else {
                                        tracing::warn!("DEBUG: Stored {} artifact to SQLite", agent_name);
                                    }
                                }
                            }
                        }
                    }
                }

                // Store responses in state for consensus to use (REGULAR stages only, not quality gates)
                if let Some(state) = widget.spec_auto_state.as_mut() {
                    state.agent_responses_cache = Some(agent_responses);
                    state.transition_phase(SpecAutoPhase::CheckingConsensus, "all_agents_complete");
                }

                tracing::warn!("DEBUG: Calling check_consensus_and_advance_spec_auto");
                check_consensus_and_advance_spec_auto(widget);
                tracing::warn!("DEBUG: Returned from check_consensus_and_advance_spec_auto");
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
            // NEW: Continue in degraded mode (no retries)
            // Consensus will handle 2/3 majority if enough agents succeeded
            let missing: Vec<_> = expected_agents
                .iter()
                .filter(|exp| !completed_names.contains(&exp.to_lowercase()))
                .map(|s| s.as_str())
                .collect();

            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "⚠ Agent failures detected. Missing/failed: {:?}",
                    missing
                ))],
                crate::history_cell::HistoryCellType::Notice,
            ));

            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(
                    "Continuing in degraded mode (2/3 consensus). Scheduling follow-up checklist.",
                )],
                crate::history_cell::HistoryCellType::Notice,
            ));

            // Mark for degraded follow-up
            let followup_data = widget.spec_auto_state.as_ref().and_then(|state| {
                state.current_stage().map(|stage| (state.spec_id.clone(), stage))
            });
            if let Some((spec_id, stage)) = followup_data {
                schedule_degraded_follow_up(widget, stage, &spec_id);
            }
        }
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
