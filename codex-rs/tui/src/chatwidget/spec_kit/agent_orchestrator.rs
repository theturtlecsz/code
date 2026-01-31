//! Agent orchestration and coordination
//!
//! This module handles multi-agent execution coordination:
//! - Auto-submitting prompts to agents with ACE routing
//! - Aggregator effort configuration (SPEC-KIT-070)
//! - Agent completion tracking with degraded mode continuation
//! - Cost tracking per agent
//! - Degraded follow-up scheduling

#![allow(dead_code, unused_variables)] // Some coordination helpers unused

use super::super::ChatWidget;
use super::ace_client::PlaybookBullet;
use super::command_handlers::halt_spec_auto_with_error;
use super::consensus_coordinator::block_on_sync;
use super::gate_evaluation::expected_agents_for_stage;
use super::handler::check_consensus_and_advance_spec_auto;
use super::maieutic::MaieuticSpec;
use super::quality_gate_handler::on_quality_gate_agents_complete;
use super::state::{SpecAutoPhase, ValidateBeginOutcome, ValidateRunInfo};
use super::validation_lifecycle::{
    ValidateLifecycleEvent, ValidateMode, compute_validate_payload_hash,
    record_validate_lifecycle_event,
};
use crate::history_cell::HistoryCellType;
use crate::memvid_adapter::{CapsuleHandle, default_capsule_config};
use crate::spec_prompts::{SpecAgent, SpecStage};
// P6-SYNC Phase 6: Token metrics UI integration
use crate::token_metrics_widget::{TokenMetricsWidget, model_context_window};
// SPEC-KIT-978: Reflex routing and metrics
use super::reflex_client::{ChatMessage, ReflexClient};
use super::reflex_metrics::get_metrics_db;
use super::reflex_router::{RoutingDecision, decide_implementer_routing, emit_routing_event};
use crate::memvid_adapter::RoutingMode;

/// SPEC-KIT-978: JSON schema for agent output enforcement.
///
/// Requires a "stage" field (consistent with json_extractor.rs validation)
/// while allowing any additional properties for flexibility.
fn agent_output_schema() -> serde_json::Value {
    serde_json::json!({
        "name": "agent_output",
        "strict": false,
        "schema": {
            "type": "object",
            "properties": {
                "stage": {
                    "type": "string",
                    "description": "The stage this agent output belongs to (e.g., plan, implement, validate)"
                },
                "confidence": {
                    "type": "number",
                    "description": "Agent's confidence in the output (0.0 to 1.0)"
                },
                "decision": {
                    "type": "string",
                    "description": "Agent's decision or recommendation"
                },
                "reasoning": {
                    "type": "string",
                    "description": "Explanation of the agent's reasoning"
                }
            },
            "required": ["stage"],
            "additionalProperties": true
        }
    })
}
use codex_core::agent_tool::AGENT_MANAGER;
use codex_core::config_types::AgentConfig;
use codex_core::protocol::{AgentInfo, InputItem};
use codex_core::slash_commands::format_subagent_command;
use std::path::Path;

/// Agent spawn info (matches native_quality_gate_orchestrator)
pub struct AgentSpawnInfo {
    pub agent_id: String,
    pub agent_name: String,
    pub model_name: String,
    pub result: Option<String>, // For sequential execution, includes agent result
}

/// Extract useful content from stage files (plan.md, tasks.md)
/// Skips debug sections, mega-bundles, and raw JSON dumps
fn extract_useful_content_from_stage_file(content: &str) -> String {
    // Split by common debug section markers
    let sections_to_skip = [
        "## Debug:",
        "## Raw JSON",
        "## code\n", // Debug section sometimes starts with "## code"
        "## Debug: code",
        "Raw JSON from agents",
        "[2025-", // Timestamp lines indicate debug output
    ];

    // Find the earliest debug section marker
    let cut_pos = sections_to_skip
        .iter()
        .filter_map(|marker| content.find(marker))
        .min()
        .unwrap_or(content.len());

    content[..cut_pos].trim().to_string()
}

// SPEC-KIT-927: extract_clean_json_from_agent_output() removed - replaced by json_extractor.rs

/// Build individual agent prompt with context (matches quality gate pattern)
/// SPEC-KIT-900 Session 3: Fix architectural mismatch - each agent gets unique prompt
/// SPEC-KIT-102: Added stage0_context parameter for combined Divine Truth + Task Brief injection
/// D113/D133: Now uses unified prompt-source API for TUI/headless parity
/// SPEC-KIT-982: Added maieutic_spec and ace_bullets for contract injection
async fn build_individual_agent_prompt(
    spec_id: &str,
    stage: SpecStage,
    agent_name: &str,
    cwd: &Path,
    stage0_context: Option<&str>, // SPEC-KIT-102: Pre-computed combined_context_md()
    maieutic_spec: Option<&MaieuticSpec>, // SPEC-KIT-982: Delegation contract
    ace_bullets: Option<&[PlaybookBullet]>, // SPEC-KIT-982: Project heuristics
) -> Result<String, String> {
    // D113/D133: Parse agent name to SpecAgent enum
    let spec_agent = SpecAgent::from_string(agent_name)
        .ok_or_else(|| format!("Unknown agent name: {}", agent_name))?;

    // D113/D133: Use unified prompt-source API (project-local with embedded fallback)
    let stage_key = stage.key();
    let (prompt_template, prompt_version) =
        crate::spec_prompts::get_prompt_with_version(stage_key, spec_agent, Some(cwd)).ok_or_else(
            || {
                format!(
                    "No prompt found for agent {} in stage {}",
                    agent_name, stage_key
                )
            },
        )?;

    // Find SPEC directory using ACID-compliant resolver
    let spec_dir = super::spec_directory::find_spec_directory(cwd, spec_id)?;

    // SPEC-KIT-982: Use unified prompt context builder for TUI/headless parity
    // This provides deterministic section order, budget enforcement, and ACE/maieutic support
    let prompt_context = super::prompt_vars::build_prompt_context(
        spec_id,
        stage,
        &spec_dir,
        stage0_context,
        maieutic_spec,
        ace_bullets,
    )?;

    // Log context stats for debugging
    tracing::debug!(
        "prompt_vars: context {} chars, {} ACE bullets used",
        prompt_context.context.len(),
        prompt_context.ace_bullet_ids_used.len()
    );

    // D113/D133: Use unified render_prompt_text() for all substitutions
    // This ensures ${TEMPLATE:*} expansion and consistent variable handling
    let prompt = crate::spec_prompts::render_prompt_text(
        &prompt_template,
        &prompt_version,
        &[("SPEC_ID", spec_id), ("CONTEXT", &prompt_context.context)],
        stage,
        spec_agent,
    );

    // D113/D133: Debug assertion - no template tokens should leak
    debug_assert!(
        !prompt.contains("${TEMPLATE:"),
        "Template token leaked in build_individual_agent_prompt: {}",
        prompt.chars().take(200).collect::<String>()
    );

    Ok(prompt)
}

/// Spawn and wait for a single agent to complete (sequential execution)
/// Returns the agent's output for injection into next agent's prompt
///
/// SPEC-938: Wrapped with exponential backoff retry (max 3 attempts)
/// Handles transient errors: timeouts, rate limits, service unavailable
async fn spawn_and_wait_for_agent(
    agent_name: &str,
    config_name: &str,
    prompt: String,
    agent_configs: &[AgentConfig],
    batch_id: &str,
    spec_id: &str,
    stage: SpecStage,
    run_id: Option<&str>,
    branch_id: Option<&str>, // P6-SYNC Phase 4: Branch tracking for resume filtering
    timeout_secs: u64,
) -> Result<(String, String), String> {
    use super::agent_retry::spawn_agent_with_retry;
    use codex_core::agent_tool::{AGENT_MANAGER, AgentStatus};

    let run_tag = run_id
        .map(|r| format!("[run:{}]", &r[..8.min(r.len())]))
        .unwrap_or_else(|| "[run:none]".to_string());
    tracing::warn!(
        "{} 🎬 SEQUENTIAL: Spawning {} and waiting for completion...",
        run_tag,
        agent_name
    );
    tracing::warn!("{}   Config: {}", run_tag, config_name);
    tracing::warn!("{}   Prompt size: {} chars", run_tag, prompt.len());
    tracing::warn!(
        "{}   Prompt preview: {}",
        run_tag,
        &prompt.chars().take(300).collect::<String>()
    );

    // SPEC-938: Wrap spawn+wait operation with retry logic
    // Closure captures all necessary context for retryable operation
    let prompt_clone = prompt.clone();
    let config_name_clone = config_name.to_string();
    let batch_id_clone = batch_id.to_string();
    let agent_configs_clone = agent_configs.to_vec();
    let spec_id_clone = spec_id.to_string();
    let stage_clone = stage;
    let run_id_clone = run_id.map(|s| s.to_string());
    let branch_id_clone = branch_id.map(|s| s.to_string()); // P6-SYNC Phase 4
    let agent_name_clone = agent_name.to_string();
    let run_tag_clone = run_tag.clone();

    // Define retryable spawn+wait operation
    let spawn_operation = || {
        let prompt = prompt_clone.clone();
        let config_name = config_name_clone.clone();
        let batch_id = batch_id_clone.clone();
        let agent_configs = agent_configs_clone.clone();
        let spec_id = spec_id_clone.clone();
        let run_id_opt = run_id_clone.clone();
        let branch_id_opt = branch_id_clone.clone(); // P6-SYNC Phase 4
        let agent_name = agent_name_clone.clone();
        let run_tag = run_tag_clone.clone();

        async move {
            // Spawn agent
            let agent_id = {
                let mut manager = AGENT_MANAGER.write().await;
                manager
                    .create_agent_from_config_name(
                        &config_name,
                        &agent_configs,
                        prompt,
                        false,
                        Some(batch_id),
                    )
                    .await
                    .map_err(|e| {
                        tracing::error!("  ❌ Spawn error for {}: {}", agent_name, e);
                        format!("Failed to spawn {}: {}", agent_name, e)
                    })?
            };

            tracing::warn!(
                "  ✓ {} spawned successfully: {}",
                agent_name,
                &agent_id[..8]
            );

            // Record to SQLite (idempotent, safe to retry)
            // P6-SYNC Phase 4: branch_id now wired from SpecAutoState
            if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
                let _ = db.record_agent_spawn(
                    &agent_id,
                    &spec_id,
                    stage_clone,
                    "regular_stage",
                    &agent_name,
                    run_id_opt.as_deref(),
                    branch_id_opt.as_deref(),
                );
            }

            // Wait for completion
            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(timeout_secs);
            let poll_interval = std::time::Duration::from_millis(500);

            loop {
                if start.elapsed() > timeout {
                    return Err(format!("{} timeout after {}s", agent_name, timeout_secs));
                }

                let manager = AGENT_MANAGER.read().await;
                if let Some(agent) = manager.get_agent(&agent_id) {
                    match agent.status {
                        AgentStatus::Completed => {
                            if let Some(result) = &agent.result {
                                tracing::warn!(
                                    "{}   ✅ {} completed: {} chars",
                                    run_tag,
                                    agent_name,
                                    result.len()
                                );

                                // Record completion (idempotent)
                                if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
                                    let _ = db.record_agent_completion(&agent_id, result);
                                }

                                return Ok((agent_id.clone(), result.clone()));
                            } else {
                                return Err(format!("{} completed but no result", agent_name));
                            }
                        }
                        AgentStatus::Failed => {
                            let error_detail = agent
                                .error
                                .as_ref()
                                .or(agent.result.as_ref())
                                .cloned()
                                .unwrap_or_else(|| "no error message available".to_string());

                            tracing::error!(
                                "  ❌ {} FAILED - Status: {:?}",
                                agent_name,
                                agent.status
                            );
                            tracing::error!("  ❌ Error detail: {}", error_detail);

                            return Err(format!("{} failed: {}", agent_name, error_detail));
                        }
                        AgentStatus::Cancelled => {
                            return Err(format!("{} cancelled", agent_name));
                        }
                        _ => {
                            // Still running, continue polling
                        }
                    }
                }

                tokio::time::sleep(poll_interval).await;
            }
        }
    };

    // Execute with retry (SPEC-938: exponential backoff, max 3 attempts)
    spawn_agent_with_retry(agent_name, spawn_operation)
        .await
        .map_err(|e| e.to_string())
}

/// Spawn regular stage agents SEQUENTIALLY with output passing
/// Session 3: True sequential execution with agent collaboration
/// SPEC-KIT-982: Added maieutic_spec and ace_bullets for contract injection
async fn spawn_regular_stage_agents_sequential(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
    run_id: Option<String>,
    branch_id: Option<String>, // P6-SYNC Phase 4: Branch tracking for resume filtering
    expected_agents: &[String],
    agent_configs: &[AgentConfig],
    stage0_context: Option<&str>, // SPEC-KIT-102: Combined context from Stage 0
    maieutic_spec: Option<&MaieuticSpec>, // SPEC-KIT-982: Delegation contract
    ace_bullets: Option<&[PlaybookBullet]>, // SPEC-KIT-982: Project heuristics
) -> Result<Vec<AgentSpawnInfo>, String> {
    let run_tag = run_id
        .as_ref()
        .map(|r| format!("[run:{}]", &r[..8]))
        .unwrap_or_else(|| "[run:none]".to_string());
    tracing::warn!(
        "{} 🎬 AUDIT: spawn_regular_stage_agents_sequential (true sequential mode)",
        run_tag
    );
    tracing::warn!("{}   spec_id: {}", run_tag, spec_id);
    tracing::warn!("{}   stage: {:?}", run_tag, stage);
    tracing::warn!("{}   expected_agents: {:?}", run_tag, expected_agents);

    let mut spawn_infos = Vec::new();
    let mut agent_outputs: Vec<(String, String)> = Vec::new(); // (agent_name, output)
    let batch_id = uuid::Uuid::new_v4().to_string();

    // Spawn agents SEQUENTIALLY, each can use previous outputs
    for (idx, agent_name) in expected_agents.iter().enumerate() {
        tracing::warn!(
            "{} 🔄 SEQUENTIAL: Agent {}/{}: {}",
            run_tag,
            idx + 1,
            expected_agents.len(),
            agent_name
        );

        // SPEC-KIT-981: Use shared resolver for TUI/headless parity
        let config_name =
            super::agent_resolver::resolve_agent_config_name(agent_name, agent_configs)?;

        // Build prompt for THIS agent with previous agent outputs injected
        // SPEC-KIT-982: Include maieutic contract and ACE heuristics
        let mut prompt = build_individual_agent_prompt(
            spec_id,
            stage,
            agent_name,
            cwd,
            stage0_context,
            maieutic_spec,
            ace_bullets,
        )
        .await?;

        // Inject previous agent outputs into prompt (INTELLIGENT EXTRACTION)
        for (prev_agent_name, prev_output) in &agent_outputs {
            let placeholder = format!("${{PREVIOUS_OUTPUTS.{}}}", prev_agent_name);

            // SPEC-KIT-927: Extract JSON using robust cascade
            let output_to_inject = super::json_extractor::extract_stage_agent_json(prev_output)
                .map(|result| {
                    // Re-serialize to compact JSON
                    result.json.to_string()
                })
                .unwrap_or_else(|_| {
                    tracing::warn!(
                        "  ⚠️ Failed to extract JSON from {}, using truncated raw output",
                        prev_agent_name
                    );
                    // Fallback: Truncate raw output to prevent explosion
                    if prev_output.len() > 5000 {
                        format!(
                            "{}...[truncated {} chars]",
                            &prev_output.chars().take(5000).collect::<String>(),
                            prev_output.len() - 5000
                        )
                    } else {
                        prev_output.to_string()
                    }
                });

            tracing::warn!(
                "  ✅ Injecting {} output ({} chars, extracted from {} raw) into {} prompt",
                prev_agent_name,
                output_to_inject.len(),
                prev_output.len(),
                agent_name
            );

            prompt = prompt.replace(&placeholder, &output_to_inject);
        }

        // Also handle generic ${PREVIOUS_OUTPUTS} (all previous)
        if prompt.contains("${PREVIOUS_OUTPUTS}") {
            let all_outputs = agent_outputs
                .iter()
                .map(|(name, output)| {
                    // SPEC-KIT-927: Use robust extraction
                    let clean = super::json_extractor::extract_stage_agent_json(output)
                        .map(|r| r.json.to_string())
                        .unwrap_or_else(|_| output.chars().take(5000).collect::<String>());
                    format!("## {}\n{}", name, clean)
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            prompt = prompt.replace("${PREVIOUS_OUTPUTS}", &all_outputs);
        }

        // Spawn and WAIT for this agent to complete
        let (agent_id, agent_output) = spawn_and_wait_for_agent(
            agent_name,
            &config_name,
            prompt,
            agent_configs,
            &batch_id,
            spec_id,
            stage,
            run_id.as_deref(),
            branch_id.as_deref(), // P6-SYNC Phase 4
            1200,                 // 20min timeout per agent (Gemini can be slow)
        )
        .await?;

        // Store this agent's output for next agents to use
        agent_outputs.push((agent_name.clone(), agent_output.clone()));

        spawn_infos.push(AgentSpawnInfo {
            agent_id,
            agent_name: agent_name.clone(),
            model_name: config_name.clone(),
            result: Some(agent_output), // Store result for direct access
        });
    }

    tracing::warn!(
        "{} ✅ SEQUENTIAL: All {} agents completed",
        run_tag,
        expected_agents.len()
    );

    Ok(spawn_infos)
}

/// SPEC-KIT-978: Spawn stage agents using local reflex inference
///
/// Uses ReflexClient for OpenAI-compatible local inference (e.g., SGLang, vLLM).
/// Mirrors `spawn_regular_stage_agents_sequential` but routes through reflex endpoint.
///
/// ## Fallback
/// If this function fails, the caller should fall back to cloud mode using
/// `spawn_regular_stage_agents_sequential`.
/// SPEC-KIT-982: Added maieutic_spec and ace_bullets for contract injection
#[allow(clippy::too_many_arguments)]
async fn spawn_reflex_stage_agents_sequential(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
    run_id: Option<String>,
    _branch_id: Option<String>,
    expected_agents: &[String],
    _agent_configs: &[AgentConfig],
    stage0_context: Option<&str>,
    maieutic_spec: Option<&MaieuticSpec>, // SPEC-KIT-982: Delegation contract
    ace_bullets: Option<&[PlaybookBullet]>, // SPEC-KIT-982: Project heuristics
    routing_decision: &RoutingDecision,
    run_tag: &str,
) -> Result<Vec<AgentSpawnInfo>, String> {
    use std::time::Instant;

    tracing::info!(
        "{} 🚀 REFLEX: spawn_reflex_stage_agents_sequential (local inference mode)",
        run_tag
    );
    tracing::info!("{}   spec_id: {}", run_tag, spec_id);
    tracing::info!("{}   stage: {:?}", run_tag, stage);
    tracing::info!("{}   expected_agents: {:?}", run_tag, expected_agents);

    // Get reflex config from routing decision
    let reflex_config = routing_decision
        .reflex_config
        .as_ref()
        .ok_or_else(|| "Reflex mode selected but no reflex config available".to_string())?;

    tracing::info!("{}   reflex endpoint: {}", run_tag, reflex_config.endpoint);
    tracing::info!("{}   reflex model: {}", run_tag, reflex_config.model);

    // Create reflex client
    let client = ReflexClient::new(reflex_config)
        .map_err(|e| format!("Failed to create reflex client: {}", e))?;

    let mut spawn_infos = Vec::new();
    let mut agent_outputs: Vec<(String, String)> = Vec::new();
    let batch_id = uuid::Uuid::new_v4().to_string();

    // Spawn agents SEQUENTIALLY using reflex
    for (idx, agent_name) in expected_agents.iter().enumerate() {
        tracing::info!(
            "{} 🔄 REFLEX SEQUENTIAL: Agent {}/{}: {}",
            run_tag,
            idx + 1,
            expected_agents.len(),
            agent_name
        );

        // Build prompt for THIS agent with previous agent outputs injected
        // SPEC-KIT-982: Include maieutic contract and ACE heuristics
        let mut prompt = build_individual_agent_prompt(
            spec_id,
            stage,
            agent_name,
            cwd,
            stage0_context,
            maieutic_spec,
            ace_bullets,
        )
        .await?;

        // Inject previous agent outputs into prompt (same as regular spawner)
        for (prev_agent_name, prev_output) in &agent_outputs {
            let placeholder = format!("${{PREVIOUS_OUTPUTS.{}}}", prev_agent_name);

            let output_to_inject = super::json_extractor::extract_stage_agent_json(prev_output)
                .map(|result| result.json.to_string())
                .unwrap_or_else(|_| {
                    if prev_output.len() > 5000 {
                        format!(
                            "{}...[truncated {} chars]",
                            &prev_output.chars().take(5000).collect::<String>(),
                            prev_output.len() - 5000
                        )
                    } else {
                        prev_output.to_string()
                    }
                });

            tracing::info!(
                "{}   ✅ Injecting {} output ({} chars) into {} prompt",
                run_tag,
                prev_agent_name,
                output_to_inject.len(),
                agent_name
            );

            prompt = prompt.replace(&placeholder, &output_to_inject);
        }

        // Handle generic ${PREVIOUS_OUTPUTS}
        if prompt.contains("${PREVIOUS_OUTPUTS}") {
            let all_outputs = agent_outputs
                .iter()
                .map(|(name, output)| {
                    let clean = super::json_extractor::extract_stage_agent_json(output)
                        .map(|r| r.json.to_string())
                        .unwrap_or_else(|_| output.chars().take(5000).collect::<String>());
                    format!("## {}\n{}", name, clean)
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            prompt = prompt.replace("${PREVIOUS_OUTPUTS}", &all_outputs);
        }

        // Build chat messages for reflex
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: format!(
                    "You are an expert {} agent working on SPEC {}. \
                     Return your analysis as valid JSON.",
                    agent_name, spec_id
                ),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt.clone(),
            },
        ];

        // SPEC-KIT-978: Call reflex endpoint with JSON schema enforcement
        let schema = agent_output_schema();
        let start = Instant::now();
        let result = client.chat_completion_json(&messages, &schema).await;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        // Record metrics
        if let Ok(db) = get_metrics_db() {
            let (success, json_compliant) = match &result {
                Ok(r) => (true, r.json_compliant),
                Err(_) => (false, false),
            };
            let _ = db.record_reflex_attempt(
                spec_id,
                run_id.as_deref().unwrap_or("unknown"),
                elapsed_ms,
                success,
                json_compliant,
            );
        }

        let agent_output = match result {
            Ok(response) => {
                tracing::info!(
                    "{} ✅ REFLEX: {} completed in {}ms ({} chars)",
                    run_tag,
                    agent_name,
                    response.latency_ms,
                    response.content.len()
                );
                response.content
            }
            Err(e) => {
                // Propagate error for fallback handling
                return Err(format!(
                    "Reflex call failed for agent {}: {}",
                    agent_name, e
                ));
            }
        };

        // Store output for next agents
        agent_outputs.push((agent_name.clone(), agent_output.clone()));

        // Generate a pseudo-agent ID for tracking
        let agent_id = format!("reflex-{}-{}", batch_id, idx);

        spawn_infos.push(AgentSpawnInfo {
            agent_id,
            agent_name: agent_name.clone(),
            model_name: reflex_config.model.clone(),
            result: Some(agent_output),
        });
    }

    tracing::info!(
        "{} ✅ REFLEX SEQUENTIAL: All {} agents completed via local inference",
        run_tag,
        expected_agents.len()
    );

    Ok(spawn_infos)
}

/// Spawn regular stage agents in PARALLEL for consensus (no output passing)
/// Used for stages where independent perspectives are critical (Validate, Audit, Unlock)
///
/// SPEC-933 Component 3: Optimized parallel spawning with tokio::JoinSet
/// Target: 150ms → 50ms (3× speedup) via true concurrent initialization
/// SPEC-KIT-982: Added maieutic_spec and ace_bullets for contract injection
async fn spawn_regular_stage_agents_parallel(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
    run_id: Option<String>,
    branch_id: Option<String>, // P6-SYNC Phase 4: Branch tracking for resume filtering
    expected_agents: &[String],
    agent_configs: &[AgentConfig],
    stage0_context: Option<String>, // SPEC-KIT-102: Combined context from Stage 0 (owned for async)
    maieutic_spec: Option<MaieuticSpec>, // SPEC-KIT-982: Delegation contract (owned for async)
    ace_bullets: Option<Vec<PlaybookBullet>>, // SPEC-KIT-982: Project heuristics (owned for async)
) -> Result<Vec<AgentSpawnInfo>, String> {
    use std::time::Instant;
    use tokio::task::JoinSet;

    let run_tag = run_id
        .as_ref()
        .map(|r| format!("[run:{}]", &r[..8]))
        .unwrap_or_else(|| "[run:none]".to_string());

    tracing::warn!(
        "{} 🎬 PARALLEL-OPTIMIZED: spawn_regular_stage_agents_parallel (true concurrent mode)",
        run_tag
    );
    tracing::warn!("  spec_id: {}", spec_id);
    tracing::warn!("  stage: {:?}", stage);
    tracing::warn!("  expected_agents: {:?}", expected_agents);

    // SPEC-933: Track total spawn time
    let total_start = Instant::now();

    let batch_id = uuid::Uuid::new_v4().to_string();

    // SPEC-933 Component 3: Use JoinSet for TRUE parallel spawning
    let mut join_set = JoinSet::new();
    let mut individual_durations = Vec::new();

    for agent_name in expected_agents {
        // Clone data for async move
        let agent_name = agent_name.clone();
        let cwd = cwd.to_path_buf();
        let spec_id = spec_id.to_string();
        let stage = stage;
        let run_id = run_id.clone();
        let branch_id = branch_id.clone();
        let batch_id = batch_id.clone();
        let agent_configs_cloned = agent_configs.to_vec();
        let stage0_ctx = stage0_context.clone(); // SPEC-KIT-102: Clone for async move
        let maieutic_cloned = maieutic_spec.clone(); // SPEC-KIT-982: Clone for async move
        let ace_cloned = ace_bullets.clone(); // SPEC-KIT-982: Clone for async move

        // SPEC-KIT-981: Use shared resolver for TUI/headless parity
        let config_name =
            super::agent_resolver::resolve_agent_config_name(&agent_name, agent_configs)?;

        // Spawn concurrent task
        join_set.spawn(async move {
            let spawn_start = Instant::now();

            // Build individual prompt (no previous outputs)
            // SPEC-KIT-102: Pass Stage 0 context to agent prompt builder
            // SPEC-KIT-982: Pass maieutic contract and ACE heuristics
            let prompt = build_individual_agent_prompt(
                &spec_id,
                stage,
                &agent_name,
                &cwd,
                stage0_ctx.as_deref(),
                maieutic_cloned.as_ref(),
                ace_cloned.as_deref(),
            )
            .await?;

            // Spawn agent (critical section - AGENT_MANAGER write lock)
            let agent_id = {
                let mut manager = AGENT_MANAGER.write().await;
                manager
                    .create_agent_from_config_name(
                        &config_name,
                        &agent_configs_cloned,
                        prompt,
                        false,
                        Some(batch_id.clone()),
                    )
                    .await
                    .map_err(|e| format!("Failed to spawn {}: {}", agent_name, e))?
            };

            let spawn_duration = spawn_start.elapsed();

            // Record to SQLite with run_id and branch_id (has built-in retry logic)
            if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
                let _ = db.record_agent_spawn(
                    &agent_id,
                    &spec_id,
                    stage,
                    "regular_stage",
                    &agent_name,
                    run_id.as_deref(),
                    branch_id.as_deref(),
                );
            }

            // SPEC-933: Record spawn metrics
            super::spawn_metrics::record_agent_spawn(&agent_name, spawn_duration, true);

            tracing::warn!(
                "{}   ✓ {} spawned in {:?} ({})",
                run_id
                    .as_ref()
                    .map(|r| format!("[run:{}]", &r[..8]))
                    .unwrap_or_else(|| "[run:none]".to_string()),
                agent_name,
                spawn_duration,
                &agent_id[..8]
            );

            Ok::<(AgentSpawnInfo, std::time::Duration), String>((
                AgentSpawnInfo {
                    agent_id,
                    agent_name: agent_name.clone(),
                    model_name: config_name.clone(),
                    result: None, // Parallel execution doesn't have result yet
                },
                spawn_duration,
            ))
        });
    }

    // SPEC-933: Collect results from concurrent spawns
    let mut spawn_infos = Vec::new();

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok((spawn_info, duration))) => {
                spawn_infos.push(spawn_info);
                individual_durations.push(duration);
            }
            Ok(Err(e)) => {
                // Spawn failed, but continue with other agents (degraded mode)
                tracing::error!("{} ❌ Agent spawn failed: {}", run_tag, e);
                // Record failure metric
                super::spawn_metrics::record_agent_spawn(
                    "unknown",
                    std::time::Duration::from_secs(0),
                    false,
                );
            }
            Err(join_error) => {
                tracing::error!("{} ❌ Join error: {}", run_tag, join_error);
                super::spawn_metrics::record_agent_spawn(
                    "unknown",
                    std::time::Duration::from_secs(0),
                    false,
                );
            }
        }
    }

    let total_duration = total_start.elapsed();

    // SPEC-933: Record batch metrics
    super::spawn_metrics::record_batch_spawn(
        expected_agents.len(),
        spawn_infos.len(),
        total_duration,
        &individual_durations,
    );

    tracing::warn!(
        "{} ✅ PARALLEL-OPTIMIZED: {} agents spawned in {:?} (avg: {:?})",
        run_tag,
        spawn_infos.len(),
        total_duration,
        if !individual_durations.is_empty() {
            individual_durations.iter().sum::<std::time::Duration>()
                / individual_durations.len() as u32
        } else {
            std::time::Duration::from_secs(0)
        }
    );

    Ok(spawn_infos)
}

/// SPEC-KIT-978: Emit routing decision event for Implementer role
///
/// Records the routing decision (reflex vs cloud) to:
/// 1. Capsule event track (audit trail)
/// 2. SQLite metrics database (bakeoff analysis)
///
/// Returns the routing decision so caller can act on it.
fn emit_implementer_routing_decision(
    cwd: &Path,
    spec_id: &str,
    run_id: Option<&str>,
    cloud_model: &str,
    run_tag: &str,
) -> super::reflex_router::RoutingDecision {
    // Make routing decision
    let decision = decide_implementer_routing("implement", cloud_model, None);

    tracing::info!(
        "{} SPEC-KIT-978: Implementer routing decision: mode={}, is_fallback={}",
        run_tag,
        decision.mode.as_str(),
        decision.is_fallback
    );

    if let Some(reason) = &decision.fallback_reason {
        tracing::info!(
            "{} SPEC-KIT-978: Fallback reason: {}",
            run_tag,
            reason.as_str()
        );
    }

    let run_id_str = run_id.unwrap_or("unknown");

    // Record bakeoff metric for the routing decision
    // This captures EVERY routing decision for later analysis
    if let Ok(db) = get_metrics_db() {
        let latency_ms = decision
            .health_result
            .as_ref()
            .and_then(|h| h.latency_ms)
            .unwrap_or(0);

        let success = !decision.is_fallback;
        let json_compliant = true; // Routing decisions don't involve JSON output yet

        let result = match decision.mode {
            RoutingMode::Reflex => {
                db.record_reflex_attempt(spec_id, run_id_str, latency_ms, success, json_compliant)
            }
            RoutingMode::Cloud => {
                db.record_cloud_attempt(spec_id, run_id_str, latency_ms, success, json_compliant)
            }
        };

        if let Err(e) = result {
            tracing::warn!(
                "{} SPEC-KIT-978: Failed to record bakeoff metric: {}",
                run_tag,
                e
            );
        } else {
            tracing::debug!(
                "{} SPEC-KIT-978: Bakeoff metric recorded: mode={}, latency={}ms",
                run_tag,
                decision.mode.as_str(),
                latency_ms
            );
        }
    }

    // Try to emit event to capsule (non-blocking)
    // Use canonical capsule config (SPEC-KIT-971/977 alignment)
    let config = default_capsule_config(cwd);

    match CapsuleHandle::open(config) {
        Ok(handle) => {
            if let Err(e) = emit_routing_event(
                &handle,
                spec_id,
                run_id_str,
                "implement",
                "Implementer",
                &decision,
            ) {
                tracing::warn!(
                    "{} SPEC-KIT-978: Failed to emit routing event: {}",
                    run_tag,
                    e
                );
            } else {
                tracing::debug!(
                    "{} SPEC-KIT-978: Routing decision event emitted to capsule",
                    run_tag
                );
            }
        }
        Err(e) => {
            // Capsule may not exist yet or be locked - just log and continue
            tracing::debug!(
                "{} SPEC-KIT-978: Could not open capsule for routing event: {}",
                run_tag,
                e
            );
        }
    }

    decision
}

/// Spawn regular stage agents natively (SPEC-KIT-900 Session 3)
/// Routes to appropriate execution pattern based on stage type
/// SPEC-KIT-982: Added maieutic_spec and ace_bullets for contract injection
async fn spawn_regular_stage_agents_native(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
    _prompt: &str, // Deprecated: no longer used (was mega-bundle)
    run_id: Option<String>,
    branch_id: Option<String>, // P6-SYNC Phase 4: Branch tracking for resume filtering
    expected_agents: &[String],
    agent_configs: &[AgentConfig],
    stage0_context: Option<String>, // SPEC-KIT-102: Combined context from Stage 0
    maieutic_spec: Option<MaieuticSpec>, // SPEC-KIT-982: Delegation contract
    ace_bullets: Option<Vec<PlaybookBullet>>, // SPEC-KIT-982: Project heuristics
) -> Result<Vec<AgentSpawnInfo>, String> {
    let run_tag = run_id
        .as_ref()
        .map(|r| format!("[run:{}]", &r[..8]))
        .unwrap_or_else(|| "[run:none]".to_string());

    // SPEC-KIT-964 Phase 6: Validate hermetic isolation before spawning agents
    if let Err(e) = super::isolation_validator::validate_agent_isolation_with_skip(cwd) {
        tracing::warn!(
            "{} SPEC-KIT-964: Isolation validation failed: {}",
            run_tag,
            e
        );
        // Log warning but don't block execution - some projects may not have all instruction files
        // This provides visibility while allowing existing workflows to continue
    }

    // Stage-specific execution patterns (Option 4)
    match stage {
        // Sequential pipeline: Research → Synthesis → QA
        crate::spec_prompts::SpecStage::Plan | crate::spec_prompts::SpecStage::Tasks => {
            tracing::warn!(
                "{} 🔄 Using SEQUENTIAL execution for {} stage (progressive refinement)",
                run_tag,
                stage.display_name()
            );
            spawn_regular_stage_agents_sequential(
                cwd,
                spec_id,
                stage,
                run_id,
                branch_id, // P6-SYNC Phase 4
                expected_agents,
                agent_configs,
                stage0_context.as_deref(), // SPEC-KIT-102
                maieutic_spec.as_ref(),    // SPEC-KIT-982
                ace_bullets.as_deref(),    // SPEC-KIT-982
            )
            .await
        }

        // Hybrid: Parallel research → Sequential implementation
        crate::spec_prompts::SpecStage::Implement => {
            tracing::warn!(
                "{} 🔀 Using SEQUENTIAL execution for {} stage (code generation pipeline)",
                run_tag,
                stage.display_name()
            );

            // SPEC-KIT-978: Get routing decision (reflex vs cloud)
            let cloud_model = agent_configs
                .first()
                .map(|c| c.name.as_str())
                .unwrap_or("claude-3-opus");
            let routing_decision = emit_implementer_routing_decision(
                cwd,
                spec_id,
                run_id.as_deref(),
                cloud_model,
                &run_tag,
            );

            // SPEC-KIT-978: Route based on decision mode
            match routing_decision.mode {
                RoutingMode::Reflex => {
                    tracing::info!(
                        "{} SPEC-KIT-978: Using REFLEX mode for Implement stage",
                        run_tag
                    );
                    // Try reflex first, fall back to cloud on failure
                    match spawn_reflex_stage_agents_sequential(
                        cwd,
                        spec_id,
                        stage,
                        run_id.clone(),
                        branch_id.clone(),
                        expected_agents,
                        agent_configs,
                        stage0_context.as_deref(),
                        maieutic_spec.as_ref(), // SPEC-KIT-982
                        ace_bullets.as_deref(), // SPEC-KIT-982
                        &routing_decision,
                        &run_tag,
                    )
                    .await
                    {
                        Ok(infos) => Ok(infos),
                        Err(e) => {
                            tracing::warn!(
                                "{} SPEC-KIT-978: Reflex failed, falling back to cloud: {}",
                                run_tag,
                                e
                            );
                            // Fallback to cloud
                            spawn_regular_stage_agents_sequential(
                                cwd,
                                spec_id,
                                stage,
                                run_id,
                                branch_id, // P6-SYNC Phase 4
                                expected_agents,
                                agent_configs,
                                stage0_context.as_deref(), // SPEC-KIT-102
                                maieutic_spec.as_ref(),    // SPEC-KIT-982
                                ace_bullets.as_deref(),    // SPEC-KIT-982
                            )
                            .await
                        }
                    }
                }
                RoutingMode::Cloud => {
                    tracing::info!(
                        "{} SPEC-KIT-978: Using CLOUD mode for Implement stage",
                        run_tag
                    );
                    spawn_regular_stage_agents_sequential(
                        cwd,
                        spec_id,
                        stage,
                        run_id,
                        branch_id, // P6-SYNC Phase 4
                        expected_agents,
                        agent_configs,
                        stage0_context.as_deref(), // SPEC-KIT-102
                        maieutic_spec.as_ref(),    // SPEC-KIT-982
                        ace_bullets.as_deref(),    // SPEC-KIT-982
                    )
                    .await
                }
            }
        }

        // Parallel consensus: Independent validation critical
        crate::spec_prompts::SpecStage::Validate
        | crate::spec_prompts::SpecStage::Audit
        | crate::spec_prompts::SpecStage::Unlock => {
            tracing::warn!(
                "{} ⚡ Using PARALLEL execution for {} stage (independent consensus)",
                run_tag,
                stage.display_name()
            );
            spawn_regular_stage_agents_parallel(
                cwd,
                spec_id,
                stage,
                run_id,
                branch_id, // P6-SYNC Phase 4
                expected_agents,
                agent_configs,
                stage0_context, // SPEC-KIT-102
                maieutic_spec,  // SPEC-KIT-982
                ace_bullets,    // SPEC-KIT-982
            )
            .await
        }

        // Fallback to sequential for other stages
        _ => {
            tracing::warn!(
                "{} 🔄 Using SEQUENTIAL execution for {} stage (default)",
                run_tag,
                stage.display_name()
            );
            spawn_regular_stage_agents_sequential(
                cwd,
                spec_id,
                stage,
                run_id,
                branch_id, // P6-SYNC Phase 4
                expected_agents,
                agent_configs,
                stage0_context.as_deref(), // SPEC-KIT-102
                maieutic_spec.as_ref(),    // SPEC-KIT-982
                ace_bullets.as_deref(),    // SPEC-KIT-982
            )
            .await
        }
    }
}

/// Wait for regular stage agents to complete (mirrors quality gate polling)
/// Returns when all agents reach terminal state or timeout expires
async fn wait_for_regular_stage_agents(
    agent_ids: &[String],
    timeout_secs: u64,
    run_id: Option<&str>,
) -> Result<(), String> {
    use codex_core::agent_tool::AGENT_MANAGER;

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);
    let poll_interval = std::time::Duration::from_millis(500);
    let run_tag = run_id
        .map(|r| format!("[run:{}]", &r[..8.min(r.len())]))
        .unwrap_or_else(|| "[run:none]".to_string());

    tracing::warn!(
        "{} 🔍 AUDIT: Starting to poll {} regular stage agents (timeout={}s)",
        run_tag,
        agent_ids.len(),
        timeout_secs
    );

    let mut poll_count = 0;
    loop {
        poll_count += 1;
        let elapsed = start.elapsed();

        if elapsed > timeout {
            tracing::warn!(
                "{} ❌ AUDIT: Agent polling timeout after {} polls ({}s)",
                run_tag,
                poll_count,
                elapsed.as_secs()
            );
            return Err(format!(
                "Timeout waiting for {} agents after {}s",
                agent_ids.len(),
                elapsed.as_secs()
            ));
        }

        // Check if all agents are complete
        let manager = AGENT_MANAGER.read().await;
        let mut all_done = true;
        let mut status_summary = Vec::new();

        for agent_id in agent_ids {
            if let Some(agent) = manager.get_agent(agent_id) {
                use codex_core::agent_tool::AgentStatus;
                let is_terminal = matches!(
                    agent.status,
                    AgentStatus::Completed | AgentStatus::Failed | AgentStatus::Cancelled
                );
                status_summary.push((agent_id.clone(), agent.status.clone(), is_terminal));

                if !is_terminal {
                    all_done = false;
                }
            } else {
                tracing::warn!(
                    "{} ⚠ AUDIT: Agent {} not found in AGENT_MANAGER (poll #{})",
                    run_tag,
                    agent_id,
                    poll_count
                );
                all_done = false;
            }
        }

        if poll_count % 10 == 1 {
            // Log every 10th poll (every 5 seconds)
            tracing::warn!(
                "{} 📊 AUDIT: Poll #{} @ {}s - Status:",
                run_tag,
                poll_count,
                elapsed.as_secs()
            );
            for (id, status, terminal) in &status_summary {
                tracing::warn!(
                    "  {} {}: {:?}",
                    if *terminal { "✓" } else { "⏳" },
                    &id[..8],
                    status
                );
            }
        }

        if all_done {
            tracing::warn!(
                "{} ✅ AUDIT: All {} agents reached terminal state after {} polls ({}s)",
                run_tag,
                agent_ids.len(),
                poll_count,
                elapsed.as_secs()
            );

            // Record all completions to SQLite for audit trail
            use codex_core::agent_tool::AgentStatus;
            if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
                for (agent_id, status, _) in &status_summary {
                    if matches!(status, AgentStatus::Completed)
                        && let Some(agent) = manager.get_agent(agent_id)
                        && let Some(result_text) = &agent.result
                    {
                        let _ = db.record_agent_completion(agent_id, result_text);
                    }
                }
                tracing::debug!(
                    "  ✓ Recorded {} agent completions to SQLite",
                    agent_ids.len()
                );
            }

            return Ok(());
        }

        tokio::time::sleep(poll_interval).await;
    }
}

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
            use super::ace_prompt_injector::{command_to_scope, should_use_ace, stage_to_ace_command};
            use super::routing::{get_current_branch, get_repo_root};

            // SPEC-KIT-982: Use normalized ACE command name for consistent matching
            let command_name = stage_to_ace_command(stage.command_name());

            if should_use_ace(ace_config, &command_name) {
                if let Some(scope) = command_to_scope(&command_name) {
                    // Convert scope to owned String for use across async boundary
                    let scope_string = scope.to_string();

                    // Use block_on_sync for sync/async bridge
                    let repo_root_opt = get_repo_root(&widget.config.cwd);
                    let branch_opt = get_current_branch(&widget.config.cwd);

                    // Fallback to defaults if git commands fail
                    let repo_root = repo_root_opt
                        .unwrap_or_else(|| widget.config.cwd.to_string_lossy().to_string());
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
            if let Some(state) = widget.spec_auto_state.as_mut()
                && let Some(bullets) = &state.ace_bullets_cache
            {
                use super::ace_prompt_injector::format_ace_section;
                let (ace_section, bullet_ids) = format_ace_section(bullets);
                if !ace_section.is_empty() {
                    prompt.push_str("\n\n");
                    prompt.push_str(&ace_section);
                    state.ace_bullet_ids_used = Some(bullet_ids);
                    tracing::info!(
                        "ACE: Injected {} bullets into {} prompt",
                        bullets.len(),
                        stage.display_name()
                    );
                }
            }

            // SPEC-KIT-070: ACE-aligned routing — set aggregator effort per stage
            // Estimate tokens ~ chars/4
            // Always use standard routing (no retry logic)
            let routing =
                super::ace_route_selector::decide_stage_routing(stage, prompt.len(), false);

            // Apply aggregator effort by updating gpt_pro args in-session
            apply_aggregator_effort(widget, routing.aggregator_effort);

            // Persist notes in state for cost summary sidecar
            if let Some(state) = widget.spec_auto_state.as_mut() {
                state
                    .aggregator_effort_notes
                    .insert(stage, routing.aggregator_effort.as_str().to_string());
                if let Some(reason) = routing.escalation_reason.as_ref() {
                    state.escalation_reason_notes.insert(stage, reason.clone());
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
            // Build clear agent list based on stage
            // SPEC-KIT-981: Use config-aware agent selection
            // Clone the config to avoid borrow conflicts with widget.history_push()
            let stage_agents_config_owned = widget.config.speckit_stage_agents.clone();
            let expected_agents_list =
                expected_agents_for_stage(stage, Some(&stage_agents_config_owned));
            let agent_count = expected_agents_list.len();
            let agent_names_display = expected_agents_list
                .iter()
                .map(|a| a.canonical_name())
                .collect::<Vec<_>>()
                .join(", ");

            let execution_mode = if matches!(
                stage,
                crate::spec_prompts::SpecStage::Validate
                    | crate::spec_prompts::SpecStage::Audit
                    | crate::spec_prompts::SpecStage::Unlock
            ) {
                "parallel consensus"
            } else {
                "sequential pipeline"
            };

            lines.push(ratatui::text::Line::from(format!(
                "🚀 Launching {} agents in {} mode...",
                agent_count, execution_mode
            )));
            lines.push(ratatui::text::Line::from(format!(
                "   Agents: {}",
                agent_names_display
            )));

            widget.history_push(crate::history_cell::PlainHistoryCell::new(
                lines,
                HistoryCellType::Notice,
            ));

            // SPEC-KIT-981: Match agents by either name OR canonical_name (case-insensitive)
            // This allows AgentConfig entries like name="gpt-5.2-architect", canonical_name="gpt_pro"
            // to match when the stage expects "gpt_pro".
            let stage_expected: Vec<String> = expected_agents_list
                .into_iter()
                .filter_map(|agent| {
                    let canonical = agent.canonical_name().to_string();
                    widget
                        .config
                        .agents
                        .iter()
                        .find(|cfg| {
                            cfg.enabled && agent_matches_canonical_key(cfg, &canonical)
                        })
                        .map(|_| canonical)
                })
                .collect();

            // Clone for later use (before move into phase transition)
            let stage_expected_for_spawn = stage_expected.clone();

            if let Some(state) = widget.spec_auto_state.as_mut() {
                state.transition_phase(
                    SpecAutoPhase::ExecutingAgents {
                        expected_agents: stage_expected,
                        completed_agents: std::collections::HashSet::new(),
                    },
                    "agents_spawned",
                );

                if stage == SpecStage::Validate
                    && let Some((info, payload_hash)) = validate_context.as_mut()
                    && let Some(updated) = state.mark_validate_dispatched(&info.run_id)
                {
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

            // Log agent spawn events for each expected agent (before consuming prompt)
            let prompt_preview = prompt[..200.min(prompt.len())].to_string();
            if let Some(state) = widget.spec_auto_state.as_ref()
                && let (
                    Some(run_id),
                    SpecAutoPhase::ExecutingAgents {
                        expected_agents, ..
                    },
                ) = (&state.run_id, &state.phase)
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
                            model: agent_name.clone(),                   // Best guess at this point
                            prompt_preview: prompt_preview.clone(),
                            timestamp: super::execution_logger::ExecutionEvent::now(),
                        },
                    );
                }
            }

            // SPEC-KIT-900 FIX: Spawn agents directly (like quality gates) instead of via text prompt
            // This ensures AgentStatusUpdate events are sent, enabling SQLite tracking
            let cwd = widget.config.cwd.clone();
            let spec_id_owned = spec_id.to_string();
            let prompt_owned = prompt.clone();
            let agent_configs_owned = widget.config.agents.clone();
            let expected_agents_owned = stage_expected_for_spawn.clone();
            let run_id_owned = widget
                .spec_auto_state
                .as_ref()
                .and_then(|s| s.run_id.clone());
            let run_id_for_spawn = run_id_owned.clone(); // Clone for async move

            // P6-SYNC Phase 4: Extract branch_id for resume filtering
            let branch_id_owned = widget
                .spec_auto_state
                .as_ref()
                .and_then(|s| s.branch_id().map(|b| b.to_string()));
            let branch_id_for_spawn = branch_id_owned.clone();

            // SPEC-KIT-102: Extract Stage 0 combined context from state
            let stage0_context_owned = widget
                .spec_auto_state
                .as_ref()
                .and_then(|s| s.stage0_result.as_ref())
                .map(|r| r.combined_context_md());

            // SPEC-KIT-982: Extract maieutic and ACE from state
            let maieutic_owned = widget
                .spec_auto_state
                .as_ref()
                .and_then(|s| s.maieutic_spec.clone());
            let ace_owned = widget
                .spec_auto_state
                .as_ref()
                .and_then(|s| s.ace_bullets_cache.clone());

            let spawn_result = block_on_sync(|| async move {
                spawn_regular_stage_agents_native(
                    &cwd,
                    &spec_id_owned,
                    stage,
                    &prompt_owned,
                    run_id_for_spawn,
                    branch_id_for_spawn, // P6-SYNC Phase 4
                    &expected_agents_owned,
                    &agent_configs_owned,
                    stage0_context_owned, // SPEC-KIT-102
                    maieutic_owned,       // SPEC-KIT-982
                    ace_owned,            // SPEC-KIT-982
                )
                .await
            });

            match spawn_result {
                Ok(spawn_infos) => {
                    tracing::warn!(
                        "🚀 AUDIT: Spawned {} agents for stage={:?}",
                        spawn_infos.len(),
                        stage
                    );
                    for info in &spawn_infos {
                        tracing::warn!(
                            "  ✓ {} ({}): model={}",
                            info.agent_name,
                            &info.agent_id[..8],
                            info.model_name
                        );
                    }

                    let agent_ids: Vec<String> =
                        spawn_infos.iter().map(|i| i.agent_id.clone()).collect();

                    // For PARALLEL stages, use background polling
                    // For SEQUENTIAL stages, agents are already complete - send event immediately
                    let is_parallel_stage = matches!(
                        stage,
                        crate::spec_prompts::SpecStage::Validate
                            | crate::spec_prompts::SpecStage::Audit
                            | crate::spec_prompts::SpecStage::Unlock
                    );

                    // Create run_tag before branching (to avoid borrow issues)
                    let run_tag_display = run_id_owned
                        .as_ref()
                        .map(|r| format!("[run:{}]", &r[..8]))
                        .unwrap_or_else(|| "[run:none]".to_string());

                    if is_parallel_stage {
                        // Start background polling task for parallel execution
                        let event_tx = widget.app_event_tx.clone();
                        let spec_id_clone = spec_id.to_string();
                        let stage_clone = stage;
                        let run_id_for_poll = run_id_owned.clone();

                        tracing::warn!(
                            "{} 🔄 PARALLEL: Starting background polling for {} agents",
                            run_tag_display,
                            agent_ids.len()
                        );

                        let _poll_handle = tokio::spawn(async move {
                            let run_tag_bg = run_id_for_poll
                                .as_ref()
                                .map(|r| format!("[run:{}]", &r[..8]))
                                .unwrap_or_else(|| "[run:none]".to_string());
                            tracing::warn!("{} 📡 PARALLEL: Background task started", run_tag_bg);

                            match wait_for_regular_stage_agents(
                                &agent_ids,
                                600,
                                run_id_for_poll.as_deref(),
                            )
                            .await
                            {
                                Ok(()) => {
                                    tracing::warn!(
                                        "{} ✅ PARALLEL: All agents completed",
                                        run_tag_bg
                                    );

                                    event_tx.send(
                                        crate::app_event::AppEvent::RegularStageAgentsComplete {
                                            stage: stage_clone,
                                            spec_id: spec_id_clone,
                                            agent_ids: agent_ids.clone(),
                                            agent_results: vec![], // Parallel: results collected from active_agents later
                                        },
                                    );
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "{} ❌ PARALLEL: Polling failed: {}",
                                        run_tag_bg,
                                        e
                                    );
                                }
                            }

                            tracing::warn!("{} 🏁 PARALLEL: Polling task complete", run_tag_bg);
                        });
                    } else {
                        // SEQUENTIAL execution - agents already complete, send event immediately
                        tracing::warn!(
                            "{} ✅ SEQUENTIAL: All {} agents already completed, sending event now",
                            run_tag_display,
                            agent_ids.len()
                        );

                        // Extract results from spawn_infos (sequential execution has results)
                        let agent_results: Vec<(String, String)> = spawn_infos
                            .iter()
                            .filter_map(|info| {
                                info.result
                                    .as_ref()
                                    .map(|r| (info.agent_name.clone(), r.clone()))
                            })
                            .collect();

                        tracing::warn!(
                            "{} 📋 SEQUENTIAL: Extracted {} results from spawn_infos",
                            run_tag_display,
                            agent_results.len()
                        );

                        // Show completion status in TUI
                        widget.history_push(crate::history_cell::PlainHistoryCell::new(
                            vec![
                                ratatui::text::Line::from(format!(
                                    "✅ All {} agents completed for {} stage",
                                    agent_ids.len(),
                                    stage.display_name()
                                )),
                                ratatui::text::Line::from(
                                    "   Building consensus and generating output...",
                                ),
                            ],
                            crate::history_cell::HistoryCellType::Notice,
                        ));
                        widget.request_redraw();

                        let result_count = agent_results.len();

                        widget.app_event_tx.send(
                            crate::app_event::AppEvent::RegularStageAgentsComplete {
                                stage,
                                spec_id: spec_id.to_string(),
                                agent_ids: agent_ids.clone(),
                                agent_results, // Pass results directly, no widget.active_agents dependency!
                            },
                        );

                        tracing::warn!(
                            "{} 📬 SEQUENTIAL: RegularStageAgentsComplete event sent with {} results",
                            run_tag_display,
                            result_count
                        );
                    }
                }
                Err(e) => {
                    tracing::error!("❌ AUDIT: Failed to spawn agents for {:?}: {}", stage, e);
                    halt_spec_auto_with_error(
                        widget,
                        format!("Failed to spawn agents for {}: {}", stage.display_name(), e),
                    );
                }
            }
        }
        Err(err) => {
            halt_spec_auto_with_error(
                widget,
                format!("Failed to build {} prompt: {}", stage.display_name(), err),
            );
        }
    }
}

fn apply_aggregator_effort(
    widget: &mut ChatWidget,
    effort: super::ace_route_selector::AggregatorEffort,
) {
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
    if let Some(state) = widget.spec_auto_state.as_mut()
        && !state.degraded_followups.insert(stage)
    {
        return;
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

/// Wrapper for backward compatibility
pub fn on_spec_auto_agents_complete(widget: &mut ChatWidget) {
    on_spec_auto_agents_complete_with_ids(widget, vec![]);
}

/// Handle agent completion with DIRECT results (SPEC-KIT-900 Session 3 refactor)
/// For SEQUENTIAL execution: uses results directly from spawn, eliminates active_agents dependency
pub fn on_spec_auto_agents_complete_with_results(
    widget: &mut ChatWidget,
    agent_results: Vec<(String, String)>,
) {
    let run_tag = widget
        .spec_auto_state
        .as_ref()
        .and_then(|s| s.run_id.as_ref())
        .map(|r| format!("[run:{}]", &r[..8]))
        .unwrap_or_else(|| "[run:none]".to_string());

    tracing::warn!(
        "{} 🎯 DIRECT RESULTS: Processing {} agent results from spawn",
        run_tag,
        agent_results.len()
    );
    for (name, result) in &agent_results {
        tracing::warn!("{}   - {}: {} chars", run_tag, name, result.len());
    }

    // SPEC-KIT-072: Store to SQLite for persistent consensus artifacts
    if let Some(state) = widget.spec_auto_state.as_ref()
        && let Some(current_stage) = state.current_stage()
        && let Some(run_id) = &state.run_id
        && let Ok(db) = super::consensus_db::ConsensusDb::init_default()
    {
        for (agent_name, response_text) in &agent_results {
            let json_str =
                super::pipeline_coordinator::extract_json_from_agent_response(response_text)
                    .unwrap_or_else(|| response_text.clone());

            if let Err(e) = db.store_artifact(
                &state.spec_id,
                current_stage,
                agent_name,
                &json_str,
                Some(response_text),
                Some(run_id),
            ) {
                tracing::warn!("{} Failed to store {} artifact: {}", run_tag, agent_name, e);
            } else {
                tracing::warn!("{} ✓ Stored {} artifact to SQLite", run_tag, agent_name);
            }
        }
    }

    // Store responses in state cache for synthesis
    if let Some(state) = widget.spec_auto_state.as_mut() {
        state.agent_responses_cache = Some(agent_results);
        state.transition_phase(
            SpecAutoPhase::CheckingConsensus,
            "all_agents_complete_direct",
        );
    }

    tracing::warn!(
        "{} DEBUG: Calling check_consensus_and_advance_spec_auto",
        run_tag
    );
    check_consensus_and_advance_spec_auto(widget);
    tracing::warn!(
        "{} DEBUG: Returned from check_consensus_and_advance_spec_auto",
        run_tag
    );
}

/// Handle agent completion with specific agent IDs (prevents collecting ALL historical agents)
/// For PARALLEL execution: collects from active_agents using agent_ids filter
pub fn on_spec_auto_agents_complete_with_ids(
    widget: &mut ChatWidget,
    specific_agent_ids: Vec<String>,
) {
    let run_tag = widget
        .spec_auto_state
        .as_ref()
        .and_then(|s| s.run_id.as_ref())
        .map(|r| format!("[run:{}]", &r[..8]))
        .unwrap_or_else(|| "[run:none]".to_string());

    tracing::warn!(
        "{} DEBUG: on_spec_auto_agents_complete_with_ids called with {} specific IDs",
        run_tag,
        specific_agent_ids.len()
    );
    if !specific_agent_ids.is_empty() {
        tracing::warn!(
            "{}   Specific agent IDs: {:?}",
            run_tag,
            specific_agent_ids
                .iter()
                .map(|id| &id[..8])
                .collect::<Vec<_>>()
        );
    }
    let Some(state) = widget.spec_auto_state.as_ref() else {
        tracing::warn!("{} DEBUG: No spec_auto_state", run_tag);
        return;
    };

    let current_stage_name = state
        .current_stage()
        .map(|s| s.display_name())
        .unwrap_or("unknown");
    tracing::warn!(
        "{} DEBUG: Current stage={}, phase={:?}",
        run_tag,
        current_stage_name,
        state.phase
    );
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
            if let Some(ref database) = db
                && let Ok(Some((phase_type, _))) = database.get_agent_spawn_info(&agent_info.id)
            {
                tracing::warn!(
                    "{}   DEBUG: Agent {} ({}) was spawned as phase_type={}",
                    run_tag,
                    agent_info.name,
                    agent_info.id,
                    phase_type
                );
                if phase_type == "quality_gate" {
                    quality_gate_agent_ids.insert(agent_info.id.clone());
                }
            }

            // Log agent complete event
            if let Some(state) = widget.spec_auto_state.as_ref()
                && let Some(run_id) = &state.run_id
                && let Some(current_stage) = state.current_stage()
            {
                // Calculate output lines from agent result (if available)
                let output_lines = agent_info
                    .result
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
                    },
                );
            }
        }
    }

    // Update completed agents in state and determine phase type
    let phase_type = if let Some(state) = widget.spec_auto_state.as_mut() {
        match &mut state.phase {
            SpecAutoPhase::ExecutingAgents {
                completed_agents,
                expected_agents: phase_expected,
                ..
            } => {
                *completed_agents = completed_names.clone();
                tracing::warn!(
                    "{} DEBUG: Phase match → ExecutingAgents, routing to 'regular'",
                    run_tag
                );

                // Definitive check: Are these quality gate agents completing late?
                // Query SQLite to see if any completed agents were spawned as quality_gate phase_type
                //
                // SPEC-KIT-900 Session 2 FIX: Don't skip if we have BOTH quality gates AND regular agents
                // Only skip if completion set contains ONLY quality gate agents (all stale)
                if !quality_gate_agent_ids.is_empty() {
                    // Count how many regular_stage agents are in the completion set
                    let regular_stage_count = widget
                        .active_agents
                        .iter()
                        .filter(|a| matches!(a.status, super::super::AgentStatus::Completed))
                        .filter(|a| {
                            if let Some(ref database) = db
                                && let Ok(Some((phase_type, _))) =
                                    database.get_agent_spawn_info(&a.id)
                            {
                                return phase_type == "regular_stage";
                            }
                            false
                        })
                        .count();

                    tracing::warn!(
                        "{} DEBUG: Found {} quality gate agents and {} regular_stage agents in completion set",
                        run_tag,
                        quality_gate_agent_ids.len(),
                        regular_stage_count
                    );

                    if regular_stage_count == 0 {
                        // ONLY quality gates (all stale) - skip processing
                        tracing::warn!(
                            "{} DEBUG: Completion set contains ONLY quality gate agents - skipping",
                            run_tag
                        );
                        tracing::warn!(
                            "{}   DEBUG: Quality gate agent IDs: {:?}",
                            run_tag,
                            quality_gate_agent_ids
                        );
                        return;
                    } else {
                        // Mixed: quality gates + regular agents
                        // Continue processing, but filter out quality gates
                        tracing::warn!(
                            "{} DEBUG: Mixed completion: {} regular + {} quality gates - processing regular agents only",
                            run_tag,
                            regular_stage_count,
                            quality_gate_agent_ids.len()
                        );
                    }
                }

                tracing::warn!("{} DEBUG: Processing regular stage agents", run_tag);
                "regular"
            }
            SpecAutoPhase::QualityGateExecuting {
                completed_agents, ..
            } => {
                *completed_agents = completed_names.clone();
                tracing::warn!(
                    "{} DEBUG: Phase match → QualityGateExecuting, routing to 'quality_gate'",
                    run_tag
                );
                "quality_gate"
            }
            SpecAutoPhase::QualityGateValidating { .. } => {
                // GPT-5.1 validation phase - single agent (GPT-5.1)
                tracing::warn!(
                    "{} DEBUG: Phase match → QualityGateValidating, routing to 'gpt5_validation'",
                    run_tag
                );
                "gpt5_validation"
            }
            _ => {
                tracing::warn!(
                    "{} DEBUG: Phase match → Other ({:?}), routing to 'none'",
                    run_tag,
                    state.phase
                );
                "none"
            }
        }
    } else {
        "none"
    };

    // Handle different phase types
    tracing::warn!(
        "{} DEBUG: on_spec_auto_agents_complete - phase_type={}",
        run_tag,
        phase_type
    );
    match phase_type {
        "quality_gate" => {
            tracing::warn!(
                "{} DEBUG: Quality gate path - calling on_quality_gate_agents_complete",
                run_tag
            );
            if !completed_names.is_empty() {
                on_quality_gate_agents_complete(widget);
            }
        }
        "gpt5_validation" => {
            if let Some(state) = widget.spec_auto_state.as_ref()
                && let SpecAutoPhase::QualityGateValidating { checkpoint, .. } = state.phase
            {
                widget
                    .quality_gate_broker
                    .fetch_validation_payload(state.spec_id.clone(), checkpoint);
            }
        }
        "regular" => {
            // Regular stage agents
            tracing::warn!(
                "{} DEBUG: Regular agent phase, checking completion",
                run_tag
            );
            tracing::warn!(
                "{}   DEBUG: Expected agents: {:?}",
                run_tag,
                expected_agents
            );
            tracing::warn!(
                "{}   DEBUG: Completed agents: {:?}",
                run_tag,
                completed_names
            );

            // Check completion with agent name normalization
            // Handles aliases like "code" (command) vs "gpt_pro"/"gpt_codex" (config names)
            let all_complete = expected_agents.iter().all(|expected| {
                let exp_lower = expected.to_lowercase();
                // Direct match
                if completed_names.contains(&exp_lower) {
                    return true;
                }
                // Special case: gpt_pro and gpt_codex both use "code" command
                if (exp_lower == "gpt_pro" || exp_lower == "gpt_codex")
                    && (completed_names.contains("code")
                        || completed_names.contains("gpt5")
                        || completed_names.contains("gpt-5"))
                {
                    return true;
                }
                // Special case: code config might report as gpt_pro or gpt_codex
                if exp_lower == "code"
                    && (completed_names.contains("gpt_pro")
                        || completed_names.contains("gpt_codex"))
                {
                    return true;
                }
                false
            });

            let run_tag_collection = widget
                .spec_auto_state
                .as_ref()
                .and_then(|s| s.run_id.as_ref())
                .map(|r| format!("[run:{}]", &r[..8]))
                .unwrap_or_else(|| "[run:none]".to_string());

            tracing::warn!(
                "{} DEBUG: All complete: {}",
                run_tag_collection,
                all_complete
            );
            if all_complete {
                tracing::warn!(
                    "{} DEBUG: All regular stage agents complete, collecting responses for consensus",
                    run_tag_collection
                );

                // Build agent_id → expected_name mapping from database
                // This handles agent name mismatches (e.g., "code" command vs "gpt_codex"/"gpt_pro" config names)
                let agent_name_map: std::collections::HashMap<String, String> =
                    if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
                        specific_agent_ids
                            .iter()
                            .filter_map(|agent_id| {
                                db.get_agent_name(agent_id)
                                    .ok()
                                    .flatten()
                                    .map(|name| (agent_id.clone(), name))
                            })
                            .collect()
                    } else {
                        std::collections::HashMap::new()
                    };

                tracing::warn!(
                    "{}   📋 Agent name mapping: {} entries",
                    run_tag_collection,
                    agent_name_map.len()
                );
                for (id, name) in &agent_name_map {
                    tracing::warn!("{}     {} → {}", run_tag_collection, &id[..8], name);
                }

                // Collect agent responses - ONLY from specific agent IDs if provided
                let agent_responses: Vec<(String, String)> = if !specific_agent_ids.is_empty() {
                    // FILTERED collection - only these specific agents (prevents collecting ALL history)
                    tracing::warn!(
                        "{}   🎯 FILTERED collection: {} specific agent IDs",
                        run_tag_collection,
                        specific_agent_ids.len()
                    );
                    widget
                        .active_agents
                        .iter()
                        .filter(|agent| specific_agent_ids.contains(&agent.id))
                        .filter_map(|agent| {
                            if matches!(agent.status, super::super::AgentStatus::Completed) {
                                // Use expected name from database, fallback to agent.name
                                let expected_name = agent_name_map
                                    .get(&agent.id)
                                    .cloned()
                                    .unwrap_or_else(|| agent.name.clone());
                                tracing::warn!(
                                    "{}     Collecting: {} → {} ({})",
                                    run_tag_collection,
                                    agent.name,
                                    expected_name,
                                    &agent.id[..8]
                                );
                                agent
                                    .result
                                    .as_ref()
                                    .map(|result| (expected_name, result.clone()))
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    // FALLBACK: Collect all completed (for backward compatibility)
                    tracing::warn!(
                        "{}   ⚠️ FALLBACK: No specific IDs provided, collecting ALL completed agents",
                        run_tag_collection
                    );
                    widget
                        .active_agents
                        .iter()
                        .filter_map(|agent| {
                            if matches!(agent.status, super::super::AgentStatus::Completed) {
                                agent
                                    .result
                                    .as_ref()
                                    .map(|result| (agent.name.clone(), result.clone()))
                            } else {
                                None
                            }
                        })
                        .collect()
                };

                tracing::warn!(
                    "{} ✅ Collected {} agent responses for consensus (expected: {})",
                    run_tag_collection,
                    agent_responses.len(),
                    expected_agents.len()
                );

                // SPEC-KIT-072: Store to SQLite for persistent consensus artifacts
                if let Some(state) = widget.spec_auto_state.as_ref()
                    && let Some(current_stage) = state.current_stage()
                    && let Some(run_id) = &state.run_id
                {
                    // Initialize SQLite database
                    if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
                        for (agent_name, response_text) in &agent_responses {
                            // Try to extract JSON content for structured storage
                            let json_str =
                                super::pipeline_coordinator::extract_json_from_agent_response(
                                    response_text,
                                )
                                .unwrap_or_else(|| response_text.clone());

                            if let Err(e) = db.store_artifact(
                                &state.spec_id,
                                current_stage,
                                agent_name,
                                &json_str,
                                Some(response_text),
                                Some(run_id),
                            ) {
                                tracing::warn!(
                                    "Failed to store {} artifact to SQLite: {}",
                                    agent_name,
                                    e
                                );
                            } else {
                                tracing::warn!("DEBUG: Stored {} artifact to SQLite", agent_name);

                                // Note: Memory cleanup removed - SQLite-based consensus doesn't use local-memory
                            }
                        }
                    }
                }

                // Store responses in state for consensus to use (REGULAR stages only, not quality gates)
                if let Some(state) = widget.spec_auto_state.as_mut() {
                    state.agent_responses_cache = Some(agent_responses);
                    state.transition_phase(SpecAutoPhase::CheckingConsensus, "all_agents_complete");
                }

                tracing::warn!(
                    "{} DEBUG: Calling check_consensus_and_advance_spec_auto",
                    run_tag_collection
                );
                check_consensus_and_advance_spec_auto(widget);
                tracing::warn!(
                    "{} DEBUG: Returned from check_consensus_and_advance_spec_auto",
                    run_tag_collection
                );
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
                state
                    .current_stage()
                    .map(|stage| (state.spec_id.clone(), stage))
            });
            if let Some((spec_id, stage)) = followup_data {
                schedule_degraded_follow_up(widget, stage, &spec_id);
            }
        }
    }
}

pub fn record_agent_costs(widget: &mut ChatWidget, agents: &[AgentInfo]) {
    let tracker = widget.spec_cost_tracker();
    #[allow(unused_assignments)]
    let mut spec_id: Option<String> = None;
    #[allow(unused_assignments)]
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

    // P6-SYNC Phase 2: Track if we recorded any agent costs for session metrics
    let recorded_any = !to_record.is_empty();

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

    // P6-SYNC Phase 2: Record token usage in SessionMetrics for predictive estimation
    if recorded_any {
        if let Some(state) = widget.spec_auto_state.as_mut() {
            let usage = widget.last_token_usage.clone();
            state.session_metrics.record_turn(&usage);
            tracing::debug!(
                "SessionMetrics: turn={}, total_input={}, estimated_next={}",
                state.session_metrics.turn_count(),
                state.session_metrics.running_total().input_tokens,
                state.session_metrics.estimated_next_prompt_tokens()
            );

            // P6-SYNC Phase 6: Update UI with spec-kit token metrics
            let model_id = state.current_model.as_deref().unwrap_or("gpt-5");
            let context_window = model_context_window(model_id);
            let metrics = TokenMetricsWidget::from_session_metrics(
                &state.session_metrics,
                context_window,
                model_id,
            );
            widget.bottom_pane.set_spec_auto_metrics(Some(metrics));
        }
    }
}

/// SPEC-KIT-981: Check if an AgentConfig matches a canonical agent key.
///
/// Matches if either:
/// - cfg.name equals the canonical key (case-insensitive), OR
/// - cfg.canonical_name equals the canonical key (case-insensitive)
#[inline]
pub fn agent_matches_canonical_key(
    cfg: &codex_core::config_types::AgentConfig,
    canonical_key: &str,
) -> bool {
    cfg.name.eq_ignore_ascii_case(canonical_key)
        || cfg
            .canonical_name
            .as_ref()
            .is_some_and(|cn| cn.eq_ignore_ascii_case(canonical_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_core::config_types::AgentConfig;

    /// SPEC-KIT-981: Test that canonical_name-based matching works
    #[test]
    fn test_agent_matches_canonical_key_via_canonical_name() {
        let agent = AgentConfig {
            name: "gpt-5.2-architect".to_string(),
            canonical_name: Some("gpt_pro".to_string()),
            enabled: true,
            command: "some-command".to_string(),
            ..Default::default()
        };

        // Should match via canonical_name
        assert!(
            agent_matches_canonical_key(&agent, "gpt_pro"),
            "Agent with canonical_name='gpt_pro' should match key 'gpt_pro'"
        );
        assert!(
            agent_matches_canonical_key(&agent, "GPT_PRO"),
            "Matching should be case-insensitive"
        );

        // Should not match via name (different from canonical_name)
        assert!(
            !agent_matches_canonical_key(&agent, "gemini"),
            "Agent should not match unrelated key 'gemini'"
        );
    }

    /// SPEC-KIT-981: Test that name-based matching still works
    #[test]
    fn test_agent_matches_canonical_key_via_name() {
        let agent = AgentConfig {
            name: "gemini".to_string(),
            canonical_name: None, // No canonical_name set
            enabled: true,
            command: "some-command".to_string(),
            ..Default::default()
        };

        // Should match via name
        assert!(
            agent_matches_canonical_key(&agent, "gemini"),
            "Agent with name='gemini' should match key 'gemini'"
        );
        assert!(
            agent_matches_canonical_key(&agent, "GEMINI"),
            "Matching should be case-insensitive"
        );
    }
}
