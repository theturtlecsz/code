//! Quality Gate Handlers (T85)
//!
//! Autonomous quality assurance system integrated into /speckit.auto pipeline.
//! Implements 3-checkpoint validation with confidence-based auto-resolution.
//!
//! MAINT-2: Extracted from handler.rs (925 LOC) for maintainability

use super::super::ChatWidget;
use super::ace_learning::{ExecutionFeedback, send_learning_feedback_sync};
use super::ace_orchestrator;
use super::ace_reflector;
use super::evidence::{EvidenceRepository, FilesystemEvidence};
use super::quality_gate_broker::{
    QualityGateAgentPayload, QualityGateBrokerResult, QualityGateValidationResult,
};
use super::routing::{get_current_branch, get_repo_root};
use super::state::SpecAutoPhase;
use crate::chatwidget::AgentStatus;
use crate::history_cell::HistoryCellType;
use crate::spec_prompts::SpecStage;
use codex_core::mcp_connection_manager::McpConnectionManager;
use serde_json::json;
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Handle quality gate agents completing
pub fn on_quality_gate_agents_complete(widget: &mut ChatWidget) {
    let (spec_id, checkpoint, expected_agents, gate_names, native_agent_ids) = {
        let Some(state) = widget.spec_auto_state.as_ref() else {
            return;
        };

        match &state.phase {
            SpecAutoPhase::QualityGateExecuting {
                checkpoint,
                expected_agents,
                gates,
                native_agent_ids,
                ..
            } => {
                if state.completed_checkpoints.contains(checkpoint)
                    || state.quality_gate_processing == Some(*checkpoint)
                {
                    return;
                }
                (
                    state.spec_id.clone(),
                    *checkpoint,
                    expected_agents.clone(),
                    gates
                        .iter()
                        .map(|gate| gate.command_name().to_string())
                        .collect::<Vec<_>>(),
                    native_agent_ids.clone(),
                )
            }
            _ => return,
        }
    };

    let gate_count = gate_names.len();

    // Mark processing active IMMEDIATELY to prevent recursion via history_push → mod.rs:4167
    // Clear any stale results before attempting storage
    if let Some(state) = widget.spec_auto_state.as_mut() {
        state.quality_gate_processing = Some(checkpoint);
        if let SpecAutoPhase::QualityGateExecuting { results, .. } = &mut state.phase {
            results.clear();
        }
    }

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Quality Gate: {} - storing agent artifacts to local-memory ({} gates)...",
            checkpoint.name(),
            gate_count
        ))],
        crate::history_cell::HistoryCellType::Notice,
    ));

    // STEP 3: Read agent result files and store to local-memory (SPEC-KIT-068 fix)
    // This was previously expected from orchestrator but never executed reliably.
    // Now implemented in Rust for deterministic execution.
    //
    // Note: Storage happens synchronously via spawn_blocking to ensure completion
    // before broker searches. Small delay acceptable (typically <500ms for 3 agents).
    let stored_count = store_quality_gate_artifacts_sync(widget, &spec_id, checkpoint, &gate_names);

    if stored_count > 0 {
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "Stored {}/{} agent artifacts to local-memory",
                stored_count,
                expected_agents.len()
            ))],
            crate::history_cell::HistoryCellType::Notice,
        ));
    } else {
        widget.history_push(crate::history_cell::new_error_event(
            "Warning: No agent artifacts stored - agents may not have completed yet".to_string(),
        ));
    }

    // Add small delay to ensure async storage tasks complete
    std::thread::sleep(std::time::Duration::from_millis(200));

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Quality Gate: {} - retrieving agent responses{}...",
            checkpoint.name(),
            if native_agent_ids.is_some() {
                " from memory (native)"
            } else {
                " from filesystem"
            }
        ))],
        crate::history_cell::HistoryCellType::Notice,
    ));

    // Use memory-based collection for native orchestrator, filesystem for legacy
    if let Some(agent_ids) = native_agent_ids {
        widget.quality_gate_broker.fetch_agent_payloads_from_memory(
            spec_id,
            checkpoint,
            expected_agents,
            agent_ids,
        );
    } else {
        widget.quality_gate_broker.fetch_agent_payloads(
            spec_id,
            checkpoint,
            expected_agents,
            gate_names,
        );
    }
}

/// Handle asynchronous broker callbacks delivering agent payloads.
pub fn on_quality_gate_broker_result(
    widget: &mut ChatWidget,
    broker_result: QualityGateBrokerResult,
) {
    let QualityGateBrokerResult {
        spec_id,
        checkpoint,
        attempts,
        info_lines,
        missing_agents,
        found_agents,
        payload,
    } = broker_result;

    let Some(state) = widget.spec_auto_state.as_ref() else {
        tracing::warn!("quality gate broker result received with no spec auto state");
        return;
    };

    if state.spec_id != spec_id {
        tracing::warn!(
            "quality gate broker result spec mismatch: expected {}, got {}",
            state.spec_id,
            spec_id
        );
        return;
    }

    let (expected_agents, current_stage) = match &state.phase {
        SpecAutoPhase::QualityGateExecuting {
            checkpoint: phase_ckpt,
            expected_agents,
            ..
        } if *phase_ckpt == checkpoint => (expected_agents.clone(), state.current_stage()),
        _ => {
            tracing::warn!("quality gate broker result received outside executing phase");
            return;
        }
    };

    if !info_lines.is_empty() {
        let mut lines = Vec::with_capacity(info_lines.len() + 1);
        lines.push(ratatui::text::Line::from(format!(
            "Quality Gate: {} broker attempts ({} total)",
            checkpoint.name(),
            attempts
        )));
        for entry in &info_lines {
            lines.push(ratatui::text::Line::from(entry.clone()));
        }
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            lines,
            crate::history_cell::HistoryCellType::Notice,
        ));
    }

    match payload {
        Err(message) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Quality Gate: {} broker error — {}",
                checkpoint.name(),
                message
            )));

            if let Some(state) = widget.spec_auto_state.as_mut() {
                state.quality_gate_processing = None;
            }

            super::handler::halt_spec_auto_with_error(
                widget,
                format!(
                    "Quality gate {} failed – missing artefacts after {} attempts",
                    checkpoint.name(),
                    attempts
                ),
            );
        }
        Ok(agent_payloads) => {
            let payload_clone = agent_payloads.clone();

            if let Some(state) = widget.spec_auto_state.as_mut() {
                if let SpecAutoPhase::QualityGateExecuting {
                    completed_agents,
                    results,
                    ..
                } = &mut state.phase
                {
                    completed_agents.clear();
                    completed_agents.extend(found_agents.iter().cloned());
                    results.clear();
                    for payload in &payload_clone {
                        results.insert(payload.agent.clone(), payload.content.clone());
                    }
                }

                if missing_agents.is_empty() {
                    state.quality_checkpoint_degradations.remove(&checkpoint);
                } else {
                    state
                        .quality_checkpoint_degradations
                        .insert(checkpoint, missing_agents.clone());
                }
            }

            if !missing_agents.is_empty() {
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from(format!(
                        "Quality Gate: {} degraded consensus – missing agents: {} (expected {})",
                        checkpoint.name(),
                        missing_agents.join(", "),
                        expected_agents.join(", ")
                    ))],
                    crate::history_cell::HistoryCellType::Notice,
                ));

                if let Some(stage) = current_stage {
                    super::handler::schedule_degraded_follow_up(widget, stage, &spec_id);
                }
            }

            process_quality_gate_agent_results(widget, checkpoint, &agent_payloads);
        }
    }
}

fn process_quality_gate_agent_results(
    widget: &mut ChatWidget,
    checkpoint: super::state::QualityCheckpoint,
    agent_payloads: &[QualityGateAgentPayload],
) {
    let Some(state) = widget.spec_auto_state.as_ref() else {
        return;
    };

    let spec_id = state.spec_id.clone();
    let cwd = widget.config.cwd.clone();
    let gates = match &state.phase {
        SpecAutoPhase::QualityGateExecuting { gates, .. } => gates.clone(),
        _ => return,
    };

    if agent_payloads.is_empty() {
        widget.history_push(crate::history_cell::new_error_event(
            "Quality gate broker returned no agent payloads".to_string(),
        ));
        if let Some(state) = widget.spec_auto_state.as_mut() {
            state.quality_gate_processing = None;
        }
        return;
    }
    let mut all_agent_issues = Vec::new();

    for gate in &gates {
        let gate_name = gate.command_name();

        let mut entries: Vec<&QualityGateAgentPayload> = agent_payloads
            .iter()
            .filter(|payload| payload.gate.as_deref() == Some(gate_name))
            .collect();

        if entries.is_empty() {
            // Fallback: include payloads without explicit gate tagging.
            entries = agent_payloads
                .iter()
                .filter(|payload| payload.gate.is_none())
                .collect();
        }

        if entries.is_empty() {
            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "Quality Gate: {} - no artefacts found for gate {}",
                    checkpoint.name(),
                    gate_name
                ))],
                crate::history_cell::HistoryCellType::Notice,
            ));
            continue;
        }

        for payload in entries {
            match super::quality::parse_quality_issue_from_agent(
                &payload.agent,
                &payload.content,
                *gate,
            ) {
                Ok(issues) => all_agent_issues.push(issues),
                Err(err) => {
                    widget.history_push(crate::history_cell::new_error_event(format!(
                        "Failed to parse {} results from {}: {}",
                        gate_name, payload.agent, err
                    )));
                }
            }
        }
    }

    let merged_issues = super::quality::merge_agent_issues(all_agent_issues);

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Quality Gate: {} - found {} issues from {} gates",
            checkpoint.name(),
            merged_issues.len(),
            gates.len()
        ))],
        crate::history_cell::HistoryCellType::Notice,
    ));

    if merged_issues.is_empty() {
        // FORK-SPECIFIC: Send ACE learning feedback on successful validation
        send_ace_learning_on_checkpoint_pass(widget, checkpoint);

        if let Some(state) = widget.spec_auto_state.as_mut() {
            state.completed_checkpoints.insert(checkpoint);
            state.quality_gate_processing = None;
            state.phase = SpecAutoPhase::Guardrail;
        }
        super::handler::advance_spec_auto(widget);
        return;
    }

    let mut auto_resolvable = Vec::new();
    let mut needs_validation = Vec::new();
    let mut escalate_to_human = Vec::new();

    // ACE Framework Integration: Get cached bullets for ACE-enhanced resolution
    let ace_bullets = widget
        .spec_auto_state
        .as_ref()
        .and_then(|s| s.ace_bullets_cache.as_ref())
        .map(|b| b.as_slice())
        .unwrap_or(&[]);

    for issue in merged_issues {
        if super::quality::should_auto_resolve_with_ace(&issue, ace_bullets) {
            auto_resolvable.push(issue);
        } else if matches!(issue.confidence, super::state::Confidence::Medium) {
            needs_validation.push(issue);
        } else {
            escalate_to_human.push(issue);
        }
    }

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Quality Gate: {} - {} auto-resolvable, {} need GPT-5 validation, {} escalated",
            checkpoint.name(),
            auto_resolvable.len(),
            needs_validation.len(),
            escalate_to_human.len()
        ))],
        crate::history_cell::HistoryCellType::Notice,
    ));

    let mut auto_resolved_list = Vec::new();

    for issue in auto_resolvable {
        let (_, majority_answer, _) =
            super::quality::classify_issue_agreement(&issue.agent_answers);
        let answer = majority_answer.unwrap_or_else(|| "unknown".to_string());

        let spec_dir = cwd.join(format!("docs/{}", spec_id));
        match super::quality::apply_auto_resolution(&issue, &answer, &spec_dir) {
            Ok(outcome) => {
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from(format!(
                        "✅ Auto-resolved: {} → {}",
                        issue.description, answer
                    ))],
                    crate::history_cell::HistoryCellType::Notice,
                ));

                auto_resolved_list.push((issue.clone(), answer.clone()));

                if let Some(state) = widget.spec_auto_state.as_mut() {
                    let file_name = outcome
                        .file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    if !state.quality_modifications.contains(&file_name) {
                        state.quality_modifications.push(file_name);
                    }
                }
            }
            Err(err) => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "Failed to apply auto-resolution for '{}': {}",
                    issue.description, err
                )));
            }
        }
    }

    if let Some(state) = widget.spec_auto_state.as_mut() {
        state
            .quality_auto_resolved
            .extend(auto_resolved_list.clone());
    }

    if !needs_validation.is_empty() {
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "Submitting {} medium-confidence issues to GPT-5 for validation...",
                needs_validation.len()
            ))],
            crate::history_cell::HistoryCellType::Notice,
        ));

        submit_gpt5_validations(widget, &needs_validation, &spec_id, &cwd, checkpoint);

        if let Some(state) = widget.spec_auto_state.as_mut() {
            let auto_resolved_issues: Vec<_> = auto_resolved_list
                .iter()
                .map(|(issue, _)| issue.clone())
                .collect();

            state.quality_gate_processing = None;
            state.phase = SpecAutoPhase::QualityGateValidating {
                checkpoint,
                auto_resolved: auto_resolved_issues,
                pending_validations: needs_validation
                    .into_iter()
                    .map(|issue| {
                        let (_, majority, _) =
                            super::quality::classify_issue_agreement(&issue.agent_answers);
                        (issue, majority.unwrap_or_default())
                    })
                    .collect(),
                completed_validations: std::collections::HashMap::new(),
            };
        }

        return;
    }

    let mut all_escalations = Vec::new();
    for issue in escalate_to_human {
        all_escalations.push((issue, None));
    }

    if !all_escalations.is_empty() {
        let (escalated_issues, escalated_questions): (Vec<_>, Vec<_>) = all_escalations
            .into_iter()
            .map(|(issue, validation_opt)| {
                let question = super::state::EscalatedQuestion {
                    id: issue.id.clone(),
                    gate_type: issue.gate_type,
                    question: issue.description.clone(),
                    context: issue.context.clone(),
                    agent_answers: issue.agent_answers.clone(),
                    gpt5_reasoning: validation_opt
                        .as_ref()
                        .map(|v: &super::state::GPT5ValidationResult| v.reasoning.clone()),
                    magnitude: issue.magnitude,
                    suggested_options: validation_opt
                        .and_then(|v: super::state::GPT5ValidationResult| v.recommended_answer)
                        .into_iter()
                        .collect(),
                };
                (issue, question)
            })
            .unzip();

        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "Quality Gate: {} - {} auto-resolved, {} need your input",
                checkpoint.name(),
                auto_resolved_list.len(),
                escalated_questions.len()
            ))],
            crate::history_cell::HistoryCellType::Notice,
        ));

        widget
            .bottom_pane
            .show_quality_gate_modal(checkpoint, escalated_questions.clone());

        if let Some(state) = widget.spec_auto_state.as_mut() {
            state.quality_gate_processing = None;
            state.phase = SpecAutoPhase::QualityGateAwaitingHuman {
                checkpoint,
                escalated_issues,
                escalated_questions,
                answers: std::collections::HashMap::new(),
            };
        }
    } else {
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "Quality Gate: {} complete - all issues auto-resolved",
                checkpoint.name()
            ))],
            crate::history_cell::HistoryCellType::Notice,
        ));

        if let Some(state) = widget.spec_auto_state.as_mut() {
            state.completed_checkpoints.insert(checkpoint);
            state.quality_gate_processing = None;
            state
                .quality_checkpoint_outcomes
                .push((checkpoint, auto_resolved_list.len(), 0));
            state.phase = SpecAutoPhase::Guardrail;
        }

        super::handler::advance_spec_auto(widget);
    }
}

/// Handle quality gate answers submitted by user
pub fn on_quality_gate_answers(
    widget: &mut ChatWidget,
    checkpoint: super::state::QualityCheckpoint,
    answers: std::collections::HashMap<String, String>,
) {
    let Some(state) = widget.spec_auto_state.as_ref() else {
        return;
    };

    let spec_id = state.spec_id.clone();
    let cwd = widget.config.cwd.clone();

    // Get escalated issues from state
    let escalated_issues = match &state.phase {
        SpecAutoPhase::QualityGateAwaitingHuman {
            escalated_issues, ..
        } => escalated_issues.clone(),
        _ => {
            widget.history_push(crate::history_cell::new_error_event(
                "Not in QualityGateAwaitingHuman phase".to_string(),
            ));
            return;
        }
    };

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Quality Gate: {} - applying {} human answers",
            checkpoint.name(),
            answers.len()
        ))],
        crate::history_cell::HistoryCellType::Notice,
    ));

    // Apply each answer to its corresponding issue
    let mut applied_answers = Vec::new();

    for issue in &escalated_issues {
        if let Some(answer) = answers.get(&issue.id) {
            let spec_dir = cwd.join(format!("docs/{}", spec_id));

            match super::quality::apply_auto_resolution(issue, answer, &spec_dir) {
                Ok(outcome) => {
                    widget.history_push(crate::history_cell::PlainHistoryCell::new(
                        vec![ratatui::text::Line::from(format!(
                            "✅ Applied: {} → {}",
                            issue.description, answer
                        ))],
                        crate::history_cell::HistoryCellType::Notice,
                    ));

                    applied_answers.push((issue.clone(), answer.clone()));

                    // Track modified file
                    if let Some(state) = widget.spec_auto_state.as_mut() {
                        let file_name = outcome
                            .file_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        if !state.quality_modifications.contains(&file_name) {
                            state.quality_modifications.push(file_name);
                        }
                    }
                }
                Err(err) => {
                    widget.history_push(crate::history_cell::new_error_event(format!(
                        "Failed to apply answer for '{}': {}",
                        issue.description, err
                    )));
                }
            }
        }
    }

    // Track answered questions in state
    if let Some(state) = widget.spec_auto_state.as_mut() {
        state.quality_escalated.extend(applied_answers);
    }

    // Mark checkpoint complete and transition to next stage
    if let Some(state) = widget.spec_auto_state.as_mut() {
        state.completed_checkpoints.insert(checkpoint);
        state.phase = SpecAutoPhase::Guardrail;
    }

    // Continue pipeline
    super::handler::advance_spec_auto(widget);
}

/// Handle GPT-5 validation artefacts delivered by the broker.
pub fn on_quality_gate_validation_result(
    widget: &mut ChatWidget,
    broker_result: QualityGateValidationResult,
) {
    let QualityGateValidationResult {
        spec_id,
        checkpoint,
        attempts,
        info_lines,
        payload,
    } = broker_result;

    let Some(state) = widget.spec_auto_state.as_ref() else {
        tracing::warn!("quality gate validation result received with no spec auto state");
        return;
    };

    if state.spec_id != spec_id {
        tracing::warn!(
            "quality gate validation result spec mismatch: expected {}, got {}",
            state.spec_id,
            spec_id
        );
        return;
    }

    let (auto_resolved, pending_validations) = match &state.phase {
        SpecAutoPhase::QualityGateValidating {
            checkpoint: phase_ckpt,
            auto_resolved,
            pending_validations,
            ..
        } if *phase_ckpt == checkpoint => (auto_resolved.clone(), pending_validations.clone()),
        _ => {
            tracing::warn!("quality gate validation result received outside validating phase");
            return;
        }
    };

    if !info_lines.is_empty() {
        let mut lines = Vec::with_capacity(info_lines.len() + 1);
        lines.push(ratatui::text::Line::from(format!(
            "GPT-5 validation broker attempts: {}",
            attempts
        )));
        for entry in info_lines {
            lines.push(ratatui::text::Line::from(entry));
        }
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            lines,
            crate::history_cell::HistoryCellType::Notice,
        ));
    }

    let validation_json = match payload {
        Ok(value) => value,
        Err(err) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "GPT-5 validation broker error: {}",
                err
            )));
            super::handler::halt_spec_auto_with_error(
                widget,
                format!(
                    "Quality gate {} failed to retrieve GPT-5 validation artefact",
                    checkpoint.name()
                ),
            );
            return;
        }
    };

    process_validation_response(
        widget,
        checkpoint,
        &validation_json,
        auto_resolved,
        pending_validations,
    );
}

fn process_validation_response(
    widget: &mut ChatWidget,
    checkpoint: super::state::QualityCheckpoint,
    validation_json: &serde_json::Value,
    auto_resolved: Vec<super::state::QualityIssue>,
    pending_validations: Vec<(super::state::QualityIssue, String)>,
) {
    let Some(state) = widget.spec_auto_state.as_ref() else {
        return;
    };

    let spec_id = state.spec_id.clone();
    let cwd = widget.config.cwd.clone();

    let validation_array = match validation_json.as_array() {
        Some(arr) => arr,
        None => {
            widget.history_push(crate::history_cell::new_error_event(
                "GPT-5 validation response was not an array".to_string(),
            ));
            return;
        }
    };

    let mut validated_auto_resolved = Vec::new();
    let mut validation_rejected = Vec::new();

    for validation_item in validation_array {
        let issue_index = validation_item["issue_index"].as_u64().unwrap_or(0) as usize;
        if issue_index == 0 || issue_index > pending_validations.len() {
            continue;
        }

        let (issue, majority_answer) = &pending_validations[issue_index - 1];
        let agrees = validation_item["agrees_with_majority"]
            .as_bool()
            .unwrap_or(false);

        let validation = super::state::GPT5ValidationResult {
            agrees_with_majority: agrees,
            reasoning: validation_item["reasoning"]
                .as_str()
                .unwrap_or("No reasoning")
                .to_string(),
            recommended_answer: validation_item["recommended_answer"]
                .as_str()
                .map(String::from),
            confidence: match validation_item["confidence"].as_str() {
                Some("high") => super::state::Confidence::High,
                Some("medium") => super::state::Confidence::Medium,
                _ => super::state::Confidence::Low,
            },
        };

        if agrees {
            let spec_dir = cwd.join(format!("docs/{}", spec_id));
            match super::quality::apply_auto_resolution(issue, majority_answer, &spec_dir) {
                Ok(outcome) => {
                    widget.history_push(crate::history_cell::PlainHistoryCell::new(
                        vec![ratatui::text::Line::from(format!(
                            "✅ GPT-5 validated: {} → {}",
                            issue.description, majority_answer
                        ))],
                        crate::history_cell::HistoryCellType::Notice,
                    ));

                    validated_auto_resolved.push((issue.clone(), majority_answer.clone()));

                    if let Some(state) = widget.spec_auto_state.as_mut() {
                        let file_name = outcome
                            .file_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        if !state.quality_modifications.contains(&file_name) {
                            state.quality_modifications.push(file_name);
                        }
                    }
                }
                Err(err) => {
                    widget.history_push(crate::history_cell::new_error_event(format!(
                        "Failed to apply GPT-5 validated resolution: {}",
                        err
                    )));
                }
            }
        } else {
            validation_rejected.push((issue.clone(), validation));
        }
    }

    if let Some(state) = widget.spec_auto_state.as_mut() {
        state
            .quality_auto_resolved
            .extend(validated_auto_resolved.clone());
    }

    if validation_rejected.is_empty() {
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "Quality Gate: {} complete - all validations accepted",
                checkpoint.name()
            ))],
            crate::history_cell::HistoryCellType::Notice,
        ));

        if let Some(state) = widget.spec_auto_state.as_mut() {
            state.completed_checkpoints.insert(checkpoint);
            state.quality_gate_processing = None;
            state.quality_checkpoint_outcomes.push((
                checkpoint,
                auto_resolved.len() + validated_auto_resolved.len(),
                0,
            ));
            state.phase = SpecAutoPhase::Guardrail;
        }

        super::handler::advance_spec_auto(widget);
    } else {
        let escalated_questions: Vec<_> = validation_rejected
            .iter()
            .map(|(issue, validation)| super::state::EscalatedQuestion {
                id: issue.id.clone(),
                gate_type: issue.gate_type,
                question: issue.description.clone(),
                context: issue.context.clone(),
                agent_answers: issue.agent_answers.clone(),
                gpt5_reasoning: Some(validation.reasoning.clone()),
                magnitude: issue.magnitude,
                suggested_options: validation.recommended_answer.clone().into_iter().collect(),
            })
            .collect();

        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "Quality Gate: {} - {} auto-resolved, {} require review",
                checkpoint.name(),
                auto_resolved.len() + validated_auto_resolved.len(),
                escalated_questions.len()
            ))],
            crate::history_cell::HistoryCellType::Notice,
        ));

        widget
            .bottom_pane
            .show_quality_gate_modal(checkpoint, escalated_questions.clone());

        if let Some(state) = widget.spec_auto_state.as_mut() {
            state.quality_gate_processing = None;
            state.phase = SpecAutoPhase::QualityGateAwaitingHuman {
                checkpoint,
                escalated_issues: validation_rejected
                    .into_iter()
                    .map(|(issue, _)| issue)
                    .collect(),
                escalated_questions,
                answers: std::collections::HashMap::new(),
            };
        }
    }
}

fn submit_gpt5_validations(
    widget: &mut ChatWidget,
    majority_issues: &[super::state::QualityIssue],
    spec_id: &str,
    cwd: &std::path::Path,
    checkpoint: super::state::QualityCheckpoint,
) {
    let spec_path = cwd.join(format!("docs/{}/spec.md", spec_id));
    let spec_content = std::fs::read_to_string(&spec_path).unwrap_or_default();

    let prd_path = cwd.join(format!("docs/{}/PRD.md", spec_id));
    let prd_content = std::fs::read_to_string(&prd_path).unwrap_or_default();

    let mut validation_prompts = Vec::new();
    for (idx, issue) in majority_issues.iter().enumerate() {
        let (_, majority_answer, dissent) =
            super::quality::classify_issue_agreement(&issue.agent_answers);
        validation_prompts.push(format!(
            "Issue {}: {}\nMajority answer: {}\nDissent: {}\n",
            idx + 1,
            issue.description,
            majority_answer.as_deref().unwrap_or("unknown"),
            dissent.as_deref().unwrap_or("N/A")
        ));
    }

    let storage_tags = format!(
        "[\\\"quality-gate\\\", \\\"spec:{}\\\", \\\"checkpoint:{}\\\", \\\"stage:gpt5-validation\\\", \\\"agent:gpt_pro\\\"]",
        spec_id,
        checkpoint.name()
    );

    let combined_prompt = format!(
        "You are validating {} quality gate issues for SPEC {}.\n\nSPEC Content:\n{}\n\n{}\n\nFor each issue respond in JSON array format:\n[\n  {{\n    \\\"issue_index\\\": 1,\n    \\\"agrees_with_majority\\\": boolean,\n    \\\"reasoning\\\": string,\n    \\\"recommended_answer\\\": string|null,\n    \\\"confidence\\\": \\\"high\\\"|\\\"medium\\\"|\\\"low\\\"\n  }}\n]\n\nIssues:\n{}\n\nAfter producing the JSON array, store it to local-memory using remember with:\n- domain: spec-kit\n- importance: 8\n- tags: {}\n- content: JSON array only\n\nIf the JSON is empty, return an empty array.",
        majority_issues.len(),
        spec_id,
        spec_content,
        prd_content,
        validation_prompts.join("\n"),
        storage_tags
    );

    // SPEC-KIT-927: Spawn gpt_pro agent DIRECTLY instead of submitting to main LLM
    // This prevents the main LLM from calling run_agent tool with 18 models
    let agent_configs = widget.config.agents.clone();
    let spec_id_clone = spec_id.to_string();
    let checkpoint_clone = checkpoint;

    tokio::spawn(async move {
        use codex_core::agent_tool::AGENT_MANAGER;

        let batch_id = uuid::Uuid::new_v4().to_string();

        // Spawn gpt_pro (gpt-5 with medium reasoning) directly
        let agent_id = {
            let mut manager = AGENT_MANAGER.write().await;
            match manager
                .create_agent_from_config_name(
                    "gpt5-medium", // Use gpt5-medium for validation
                    &agent_configs,
                    combined_prompt,
                    true, // read_only
                    Some(batch_id),
                    false, // No tmux for single validation agent
                )
                .await
            {
                Ok(id) => {
                    info!("✅ Spawned GPT-5 validation agent: {}", &id[..8]);
                    id
                }
                Err(e) => {
                    warn!("❌ Failed to spawn GPT-5 validation: {}", e);
                    return;
                }
            }
        };

        // Wait for completion (max 5 minutes)
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(300);

        loop {
            if start.elapsed() > timeout {
                warn!("❌ GPT-5 validation timeout after 5 minutes");
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let manager = AGENT_MANAGER.read().await;
            if let Some(agent) = manager.get_agent(&agent_id) {
                if agent.status == codex_core::agent_tool::AgentStatus::Completed {
                    info!("✅ GPT-5 validation completed");
                    break;
                } else if agent.status == codex_core::agent_tool::AgentStatus::Failed {
                    warn!("❌ GPT-5 validation failed");
                    break;
                }
            }
        }
    });

    // Trigger broker to collect validation results
    widget
        .quality_gate_broker
        .fetch_validation_payload(spec_id.to_string(), checkpoint);
}

/// Handle quality gate cancelled by user
pub fn on_quality_gate_cancelled(
    widget: &mut ChatWidget,
    checkpoint: super::state::QualityCheckpoint,
) {
    super::handler::halt_spec_auto_with_error(
        widget,
        format!("Quality gate {} cancelled by user", checkpoint.name()),
    );
}

/// Determine which quality checkpoint should run before the given stage
pub(super) fn determine_quality_checkpoint(
    stage: SpecStage,
    completed: &std::collections::HashSet<super::state::QualityCheckpoint>,
) -> Option<super::state::QualityCheckpoint> {
    // Option A: Strategic placement - one gate per stage type
    // BeforeSpecify: Clarify (BEFORE plan - assumes PRD exists from /speckit.specify)
    // AfterSpecify: Checklist (BEFORE tasks - validate PRD+plan quality)
    // AfterTasks: Analyze (BEFORE implement - full consistency check)
    let checkpoint = match stage {
        SpecStage::Plan => super::state::QualityCheckpoint::BeforeSpecify, // Clarify before planning
        SpecStage::Tasks => super::state::QualityCheckpoint::AfterSpecify, // Checklist after plan
        SpecStage::Implement => super::state::QualityCheckpoint::AfterTasks, // Analyze after tasks
        _ => return None,
    };

    if completed.contains(&checkpoint) {
        None
    } else {
        Some(checkpoint)
    }
}

/// Get quality gate agents from config, falling back to defaults
fn get_quality_gate_agents(
    widget: &ChatWidget,
    checkpoint: super::state::QualityCheckpoint,
) -> Vec<String> {
    // Try to get from config first
    if let Some(quality_gates) = &widget.config.quality_gates {
        let agents = match checkpoint {
            super::state::QualityCheckpoint::BeforeSpecify => &quality_gates.plan,
            super::state::QualityCheckpoint::AfterSpecify => &quality_gates.tasks,
            super::state::QualityCheckpoint::AfterTasks => &quality_gates.validate,
        };
        if !agents.is_empty() {
            return agents.clone();
        }
    }
    // Fallback to default agents
    vec!["gemini".to_string(), "claude".to_string(), "code".to_string()]
}

/// Execute quality checkpoint by starting quality gate agents
pub(super) fn execute_quality_checkpoint(
    widget: &mut ChatWidget,
    checkpoint: super::state::QualityCheckpoint,
) {
    let Some(state) = widget.spec_auto_state.as_ref() else {
        return;
    };

    let spec_id = state.spec_id.clone();
    let cwd = widget.config.cwd.clone();

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Starting Quality Checkpoint: {}",
            checkpoint.name()
        ))],
        crate::history_cell::HistoryCellType::Notice,
    ));

    // SPEC-KIT-900, I-003: Native quality gate orchestration
    // LLM orchestrator eliminated - native code spawns agents directly
    let gates = checkpoint.gates();
    let gate_names: Vec<String> = gates.iter().map(|g| g.command_name().to_string()).collect();

    // Get quality gate agents from config (SPEC-939: configurable quality gates)
    let quality_gate_agents = get_quality_gate_agents(widget, checkpoint);

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Spawning {} quality gate agents ({}) for gates: {}",
            quality_gate_agents.len(),
            quality_gate_agents.join(", "),
            gate_names.join(", ")
        ))],
        crate::history_cell::HistoryCellType::Notice,
    ));

    // Spawn agents natively (no LLM orchestrator)
    let cwd_clone = cwd.clone();
    let spec_id_clone = spec_id.clone();
    let checkpoint_clone = checkpoint;

    // Log quality gate start event
    if let Some(state) = widget.spec_auto_state.as_ref() {
        if let Some(run_id) = &state.run_id {
            state.execution_logger.log_event(
                super::execution_logger::ExecutionEvent::QualityGateStart {
                    run_id: run_id.clone(),
                    checkpoint: checkpoint.name().to_string(),
                    gates: gate_names.clone(),
                    timestamp: super::execution_logger::ExecutionEvent::now(),
                },
            );
        }
    }

    // Get execution logger, agent configs, and event sender for spawning
    let logger = widget
        .spec_auto_state
        .as_ref()
        .map(|s| s.execution_logger.clone());
    let run_id = widget
        .spec_auto_state
        .as_ref()
        .and_then(|s| s.run_id.clone());
    let agent_configs = widget.config.agents.clone();
    let event_tx = widget.app_event_tx.clone();

    // SPEC-KIT-928: Check for already-running quality gate agents (single-flight guard)
    // Use configured agents instead of hardcoded list (SPEC-939)
    let expected_agents = quality_gate_agents.clone();
    let already_running = {
        if let Ok(manager_check) = codex_core::agent_tool::AGENT_MANAGER.try_read() {
            let running_agents = manager_check.get_running_agents();
            let mut matched = Vec::new();

            for (agent_id, model, _status) in running_agents {
                for expected in &expected_agents {
                    if model.to_lowercase().contains(expected) {
                        matched.push((expected.to_string(), agent_id));
                        break;
                    }
                }
            }
            matched
        } else {
            Vec::new() // Failed to acquire lock, skip check
        }
    };

    if !already_running.is_empty() {
        let running_list: Vec<String> = already_running
            .iter()
            .map(|(name, id)| format!("{} ({})", name, &id[..8]))
            .collect();

        tracing::warn!(
            "🚨 DUPLICATE SPAWN DETECTED: {} quality gate agents already running for {}: {}",
            already_running.len(),
            spec_id,
            running_list.join(", ")
        );

        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![
                ratatui::text::Line::from(format!(
                    "⚠ Quality gate agents already running: {}",
                    already_running
                        .iter()
                        .map(|(n, _)| n.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                ratatui::text::Line::from(
                    "Skipping duplicate spawn. Waiting for current run to complete.",
                ),
            ],
            crate::history_cell::HistoryCellType::Notice,
        ));
        return;
    }

    // Spawn agents in background task
    let spawn_handle = tokio::spawn(async move {
        match super::native_quality_gate_orchestrator::spawn_quality_gate_agents_native(
            &cwd_clone,
            &spec_id_clone,
            checkpoint_clone,
            &agent_configs,
            run_id.clone(),
        )
        .await
        {
            Ok(spawn_infos) => {
                info!("Spawned {} quality gate agents", spawn_infos.len());

                // Log each agent spawn event (CRITICAL for SPEC-KIT-070 validation)
                if let (Some(logger), Some(run_id)) = (&logger, &run_id) {
                    for spawn_info in &spawn_infos {
                        logger.log_event(super::execution_logger::ExecutionEvent::AgentSpawn {
                            run_id: run_id.clone(),
                            stage: format!("quality-gate-{}", checkpoint_clone.name()),
                            agent_name: spawn_info.agent_name.clone(),
                            agent_id: spawn_info.agent_id.clone(),
                            model: spawn_info.model_name.clone(),
                            prompt_preview: spawn_info.prompt_preview.clone(),
                            timestamp: super::execution_logger::ExecutionEvent::now(),
                        });
                    }
                }

                // Extract agent IDs for waiting
                let agent_ids: Vec<String> = spawn_infos
                    .iter()
                    .map(|info| info.agent_id.clone())
                    .collect();

                // Wait for completion (5 minute timeout)
                match super::native_quality_gate_orchestrator::wait_for_quality_gate_agents(
                    &agent_ids, 300, // 5 minutes
                )
                .await
                {
                    Ok(()) => {
                        info!("Quality gate agents completed successfully");
                        // Send completion event to trigger broker collection
                        let _ = event_tx.send(
                            crate::app_event::AppEvent::QualityGateNativeAgentsComplete {
                                checkpoint: checkpoint_clone,
                                agent_ids: agent_ids.clone(),
                            },
                        );
                    }
                    Err(e) => {
                        warn!("Quality gate agents timeout: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to spawn quality gate agents: {}", e);
            }
        }
    });

    // Store spawn handle (optional - for tracking)
    drop(spawn_handle);

    // Transition to quality gate executing phase
    if let Some(state) = widget.spec_auto_state.as_mut() {
        tracing::warn!(
            "DEBUG: Setting phase to QualityGateExecuting for checkpoint={:?}",
            checkpoint
        );
        let old_phase = format!("{:?}", state.phase);
        state.phase = SpecAutoPhase::QualityGateExecuting {
            checkpoint,
            gates: gates.to_vec(),
            active_gates: gates.iter().copied().collect(),
            expected_agents: vec![
                "gemini".to_string(),
                "claude".to_string(),
                "code".to_string(),
            ],
            completed_agents: std::collections::HashSet::new(),
            results: std::collections::HashMap::new(),
            native_agent_ids: None, // Will be set by completion event
        };
        tracing::warn!(
            "DEBUG: Phase transition: {} → QualityGateExecuting",
            old_phase
        );
    }
}

/// Update phase with native agent IDs when event arrives
pub fn set_native_agent_ids(widget: &mut ChatWidget, agent_ids: Vec<String>) {
    if let Some(state) = widget.spec_auto_state.as_mut() {
        if let SpecAutoPhase::QualityGateExecuting {
            native_agent_ids, ..
        } = &mut state.phase
        {
            *native_agent_ids = Some(agent_ids);
        }
    }
}

/// Build quality gate prompt for a specific gate
fn build_quality_gate_prompt(
    spec_id: &str,
    gate: super::state::QualityGateType,
    checkpoint: super::state::QualityCheckpoint,
) -> String {
    // FORK-SPECIFIC: Add JSON schema and examples (just-every/code)

    let gate_name = match gate {
        super::state::QualityGateType::Clarify => "quality-gate-clarify",
        super::state::QualityGateType::Checklist => "quality-gate-checklist",
        super::state::QualityGateType::Analyze => "quality-gate-analyze",
    };

    // Add schema and examples
    let schema_json = super::schemas::quality_gate_response_schema();
    let schema_str =
        serde_json::to_string_pretty(&schema_json["schema"]).unwrap_or_else(|_| "{}".to_string());

    // Few-shot example
    let example = r#"{
  "issues": [
    {
      "id": "Q1",
      "question": "Authentication method not specified in requirements",
      "answer": "Add OAuth2 authentication section specifying provider and scopes",
      "confidence": "high",
      "magnitude": "important",
      "resolvability": "auto-fix",
      "context": "Security requirements section is missing auth details",
      "suggested_fix": "Add OAuth2 section with provider and scopes",
      "reasoning": "Authentication is critical for security and must be specified before implementation"
    }
  ]
}"#;

    format!(
        r#"Quality Gate: {} at checkpoint {}

Analyze SPEC {} for issues.

CRITICAL: Return ONLY valid JSON matching this exact schema:
{}

Example correct output:
{}

Instructions:
- Find all ambiguities, inconsistencies, or missing requirements
- Each issue needs: id, question, answer, confidence, magnitude, resolvability
- confidence: "high" (certain), "medium" (likely), "low" (unsure)
- magnitude: "critical" (blocks progress), "important" (significant), "minor" (nice-to-have)
- resolvability: "auto-fix" (safe to apply), "suggest-fix" (needs review), "need-human" (judgment required)
- Store this analysis in local-memory using remember command
- If no issues found, return: {{"issues": []}}

See prompts.json["{}"] for detailed context."#,
        gate.command_name(),
        checkpoint.name(),
        spec_id,
        schema_str,
        example,
        gate_name
    )
}

/// Finalize quality gates at pipeline completion
pub(super) fn finalize_quality_gates(widget: &mut ChatWidget) {
    let Some(state) = widget.spec_auto_state.as_ref() else {
        return;
    };

    let spec_id = state.spec_id.clone();
    let cwd = widget.config.cwd.clone();
    let auto_resolved = state.quality_auto_resolved.clone();
    let escalated = state.quality_escalated.clone();
    let modified_files = state.quality_modifications.clone();
    let checkpoint_outcomes = state.quality_checkpoint_outcomes.clone();
    let degradations = state.quality_checkpoint_degradations.clone();

    // Step 1: Persist telemetry for each checkpoint
    let repo = FilesystemEvidence::new(cwd.clone(), None);

    for (checkpoint, auto_count, esc_count) in &checkpoint_outcomes {
        // Build telemetry JSON
        let degraded_agents = degradations.get(checkpoint).map(|agents| agents.as_slice());
        let telemetry = super::quality::build_quality_checkpoint_telemetry(
            &spec_id,
            *checkpoint,
            &auto_resolved,
            &escalated,
            degraded_agents,
        );

        match repo.write_quality_checkpoint_telemetry(&spec_id, *checkpoint, &telemetry) {
            Ok(path) => {
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from(format!(
                        "📊 Telemetry: {}",
                        path.display()
                    ))],
                    HistoryCellType::Notice,
                ));
            }
            Err(err) => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "Failed to write telemetry for {}: {}",
                    checkpoint.name(),
                    err
                )));
            }
        }

        // Log quality gate complete event
        if let Some(state) = widget.spec_auto_state.as_ref() {
            if let Some(run_id) = &state.run_id {
                let degraded_agent_vec = degraded_agents
                    .map(|agents| agents.to_vec())
                    .unwrap_or_default();

                state.execution_logger.log_event(
                    super::execution_logger::ExecutionEvent::QualityGateComplete {
                        run_id: run_id.clone(),
                        checkpoint: checkpoint.name().to_string(),
                        status: "passed".to_string(),
                        auto_resolved: *auto_count,
                        escalated: *esc_count,
                        degraded_agents: degraded_agent_vec,
                        timestamp: super::execution_logger::ExecutionEvent::now(),
                    },
                );
            }
        }

        if let Some(missing) = degradations.get(checkpoint) {
            if !missing.is_empty() {
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from(format!(
                        "Quality Gate: {} ran in degraded mode (missing agents: {})",
                        checkpoint.name(),
                        missing.join(", ")
                    ))],
                    HistoryCellType::Notice,
                ));
            }
        }
    }

    // Step 2: Create git commit if there are modifications
    if !modified_files.is_empty() {
        let commit_msg = super::quality::build_quality_gate_commit_message(
            &spec_id,
            &checkpoint_outcomes,
            &modified_files,
        );

        // Execute git commit
        let git_result = std::process::Command::new("git")
            .current_dir(&cwd)
            .args(&["add", "docs/"])
            .output();

        if let Ok(add_output) = git_result {
            if add_output.status.success() {
                let commit_result = std::process::Command::new("git")
                    .current_dir(&cwd)
                    .args(&["commit", "-m", &commit_msg])
                    .output();

                match commit_result {
                    Ok(output) if output.status.success() => {
                        widget.history_push(crate::history_cell::PlainHistoryCell::new(
                            vec![ratatui::text::Line::from(
                                "✅ Quality gate changes committed",
                            )],
                            HistoryCellType::Notice,
                        ));
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        widget.history_push(crate::history_cell::new_error_event(format!(
                            "Git commit failed: {}",
                            stderr
                        )));
                    }
                    Err(err) => {
                        widget.history_push(crate::history_cell::new_error_event(format!(
                            "Failed to run git commit: {}",
                            err
                        )));
                    }
                }
            }
        }
    }

    // Step 3: Show review summary
    let summary_lines =
        super::quality::build_quality_gate_summary(&auto_resolved, &escalated, &modified_files);

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        summary_lines,
        HistoryCellType::Notice,
    ));
}

/// FORK-SPECIFIC: Send ACE learning feedback when checkpoint passes
///
/// Uses full Reflector/Curator cycle if enabled, otherwise simple scoring
fn send_ace_learning_on_checkpoint_pass(
    widget: &ChatWidget,
    checkpoint: super::state::QualityCheckpoint,
) {
    let Some(state) = widget.spec_auto_state.as_ref() else {
        return;
    };

    // Map checkpoint to scope based on current stage
    let scope = match state.current_stage() {
        Some(SpecStage::Implement) => "implement",
        Some(SpecStage::Validate) | Some(SpecStage::Audit) => "test",
        _ => "implement", // Default fallback
    };

    // Build success feedback (checkpoint passed)
    let feedback = ExecutionFeedback::new()
        .with_compile_ok(true)
        .with_tests_passed(true)
        .with_lint_issues(0);

    // Get git context
    let repo_root = get_repo_root(&widget.config.cwd).unwrap_or_else(|| ".".to_string());
    let branch = get_current_branch(&widget.config.cwd).unwrap_or_else(|| "main".to_string());

    // Use spec_id as task title
    let task_title = &state.spec_id;

    // Check if we should use full reflection-curation cycle
    if ace_reflector::should_reflect(&feedback) {
        // FULL ACE CYCLE: Reflector → Curator → Apply
        info!("ACE: Starting full reflection-curation cycle");

        match ace_orchestrator::run_ace_cycle_sync(
            &widget.config.ace,
            repo_root,
            branch,
            scope,
            task_title,
            feedback,
            Vec::new(), // TODO: Track bullet IDs from injection
        ) {
            Ok(result) => {
                info!(
                    "ACE cycle complete: {}ms, {} patterns, +{} bullets",
                    result.elapsed_ms,
                    result.reflection.patterns.len(),
                    result.bullets_added
                );
            }
            Err(e) => {
                warn!("ACE cycle failed: {}", e);
                // Fall back to simple learning if reflection fails
                send_learning_feedback_sync(
                    &widget.config.ace,
                    get_repo_root(&widget.config.cwd).unwrap_or_else(|| ".".to_string()),
                    get_current_branch(&widget.config.cwd).unwrap_or_else(|| "main".to_string()),
                    scope,
                    task_title,
                    ExecutionFeedback::new()
                        .with_compile_ok(true)
                        .with_tests_passed(true),
                    None,
                );
            }
        }
    } else {
        // SIMPLE LEARNING: Just update scores (routine success)
        debug!("ACE: Using simple learning (routine success)");
        send_learning_feedback_sync(
            &widget.config.ace,
            repo_root,
            branch,
            scope,
            task_title,
            feedback,
            None,
        );
    }
}

// ============================================================================
// STEP 3 Implementation: Store quality gate artifacts to local-memory
// ============================================================================

/// Store quality gate agent artifacts to local-memory (synchronously)
///
/// Implements STEP 3 from execute_quality_checkpoint orchestrator prompt:
/// Read .code/agents/{agent_id}/result.txt and store via local-memory MCP.
///
/// Uses spawn_blocking to avoid tokio runtime nesting panic while still
/// waiting for all storage operations to complete before returning.
///
/// Returns count of successfully stored artifacts.
fn store_quality_gate_artifacts_sync(
    widget: &mut ChatWidget,
    spec_id: &str,
    checkpoint: super::state::QualityCheckpoint,
    gate_names: &[String],
) -> usize {
    // Get completed quality gate agents from filesystem
    let completed_agents = get_completed_quality_gate_agents(widget);

    if completed_agents.is_empty() {
        warn!("No quality gate agents found in filesystem scan");
        return 0;
    }

    info!(
        "Found {} quality gate agents to store: {:?}",
        completed_agents.len(),
        completed_agents
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
    );

    let mcp_manager = widget.mcp_manager.clone();
    let spec_id_owned = spec_id.to_string();
    let checkpoint_owned = checkpoint;
    let stage_name = gate_names
        .first()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "clarify".to_string());

    // Create storage tasks for each agent
    let mut handles = Vec::new();

    for (agent_name, agent_id) in completed_agents {
        // Read agent result file
        let result_path = format!(".code/agents/{}/result.txt", agent_id);
        let content = match fs::read_to_string(&result_path) {
            Ok(c) => c,
            Err(e) => {
                debug!("Failed to read agent result file {}: {}", result_path, e);
                continue;
            }
        };

        // SPEC-KIT-927: Use robust JSON extraction with validation
        let json_str =
            match super::json_extractor::extract_and_validate_quality_gate(&content, &agent_name) {
                Ok(extraction_result) => {
                    debug!(
                        "Extracted {} via {:?} (confidence: {:.2})",
                        agent_name, extraction_result.method, extraction_result.confidence
                    );
                    // Re-serialize to string for storage
                    extraction_result.json.to_string()
                }
                Err(e) => {
                    warn!(
                        "Extraction failed for agent result file {} ({}): {}",
                        result_path, agent_name, e
                    );
                    continue;
                }
            };

        // Clone for async task
        let mcp_clone = mcp_manager.clone();
        let spec_clone = spec_id_owned.clone();
        let checkpoint_clone = checkpoint_owned;
        let agent_clone = agent_name.clone();
        let stage_clone = stage_name.clone();
        let json_clone = json_str.clone();

        // Spawn async storage task
        let handle = tokio::spawn(async move {
            store_artifact_async(
                mcp_clone,
                &spec_clone,
                checkpoint_clone,
                &agent_clone,
                &stage_clone,
                &json_clone,
            )
            .await
        });

        handles.push((agent_name, handle));
    }

    // Wait for all storage tasks to complete (with timeout)
    let stored_count = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            let mut count = 0;
            for (agent_name, handle) in handles {
                match tokio::time::timeout(std::time::Duration::from_secs(15), handle).await {
                    Ok(Ok(Ok(_))) => {
                        debug!("Successfully stored artifact for {}", agent_name);
                        count += 1;
                    }
                    Ok(Ok(Err(e))) => {
                        warn!("Failed to store artifact for {}: {}", agent_name, e);
                    }
                    Ok(Err(e)) => {
                        warn!("Task join error for {}: {}", agent_name, e);
                    }
                    Err(_) => {
                        warn!("Timeout storing artifact for {}", agent_name);
                    }
                }
            }
            count
        })
    });

    stored_count
}

/// Get completed quality gate agents by scanning .code/agents/ directory
///
/// Returns Vec<(agent_name, agent_id)> for quality gate agents found in filesystem.
///
/// This approach scans the filesystem instead of widget.active_agents because
/// quality gate agents are spawned as sub-agents by the orchestrator via agent_run,
/// and are not tracked in widget.active_agents (only the orchestrator itself is).
fn get_completed_quality_gate_agents(_widget: &ChatWidget) -> Vec<(String, String)> {
    let agents_dir = std::path::Path::new(".code/agents");

    let mut quality_gate_agents = Vec::new();

    // Scan all agent directories
    let entries = match std::fs::read_dir(agents_dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to read .code/agents directory: {}", e);
            return quality_gate_agents;
        }
    };

    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }

        let agent_id = entry.file_name().to_string_lossy().to_string();
        let result_path = entry.path().join("result.txt");

        // Try to read result file
        let content = match std::fs::read_to_string(&result_path) {
            Ok(c) => c,
            Err(_) => continue, // No result file yet
        };

        // SPEC-KIT-927: Use robust JSON extraction with validation
        match super::json_extractor::extract_and_validate_quality_gate(&content, "scanner") {
            Ok(extraction_result) => {
                let json_val = extraction_result.json;

                // Already validated by extractor - just get agent name
                if let Some(agent) = json_val.get("agent").and_then(|v| v.as_str()) {
                    let stage = json_val
                        .get("stage")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    quality_gate_agents.push((agent.to_lowercase().to_string(), agent_id.clone()));
                    debug!(
                        "Found quality gate agent: {} (id: {}, stage: {}) via {:?}",
                        agent, agent_id, stage, extraction_result.method
                    );
                }
            }
            Err(_) => {
                // Not a quality gate agent or extraction failed
                continue;
            }
        }
    }

    if quality_gate_agents.is_empty() {
        debug!("No quality gate agents found in .code/agents directory");
    } else {
        info!(
            "Found {} quality gate agents: {:?}",
            quality_gate_agents.len(),
            quality_gate_agents
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>()
        );
    }

    quality_gate_agents
}

// SPEC-KIT-927: extract_json_from_markdown() removed - replaced by json_extractor.rs

/// Store quality gate artifact to SQLite (SPEC-934)
///
/// Async function called from spawned tasks. Returns Result for error propagation.
/// Replaces MCP local-memory storage with SQLite consensus_db.
async fn store_artifact_async(
    _mcp_manager: Arc<Mutex<Option<Arc<McpConnectionManager>>>>,
    spec_id: &str,
    checkpoint: super::state::QualityCheckpoint,
    agent_name: &str,
    _stage_name: &str,
    json_content: &str,
) -> Result<(), String> {
    // SPEC-934: Store to SQLite instead of MCP local-memory
    // Quality gate checkpoints use checkpoint name as stage (e.g., "before-specify", "after-specify")
    let stage_for_db = checkpoint.name();

    let db = super::consensus_db::ConsensusDb::init_default()
        .map_err(|e| format!("Failed to initialize consensus DB: {}", e))?;

    db.store_artifact_with_stage_name(
        spec_id,
        stage_for_db,
        agent_name,
        json_content,
        None, // run_id not available for quality gates
    )
    .map_err(|e| format!("SQLite storage failed: {}", e))?;

    debug!(
        "Successfully stored artifact to SQLite: agent={}, spec={}, checkpoint={}",
        agent_name, spec_id, checkpoint.name()
    );
    Ok(())
}
