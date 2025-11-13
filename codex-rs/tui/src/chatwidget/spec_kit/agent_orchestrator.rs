//! Agent orchestration and coordination
//!
//! This module handles multi-agent execution coordination:
//! - Auto-submitting prompts to agents with ACE routing
//! - Aggregator effort configuration (SPEC-KIT-070)
//! - Agent completion tracking with degraded mode continuation
//! - Cost tracking per agent
//! - Degraded follow-up scheduling

use super::super::ChatWidget;
use super::command_handlers::halt_spec_auto_with_error;
use super::consensus::expected_agents_for_stage;
use super::consensus_coordinator::block_on_sync;
use super::handler::check_consensus_and_advance_spec_auto;
use super::quality_gate_handler::on_quality_gate_agents_complete;
use super::state::{SpecAutoPhase, ValidateBeginOutcome, ValidateRunInfo};
use super::validation_lifecycle::{
    ValidateLifecycleEvent, ValidateMode, compute_validate_payload_hash,
    record_validate_lifecycle_event,
};
use crate::history_cell::HistoryCellType;
use crate::spec_prompts::{SpecAgent, SpecStage};
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
async fn build_individual_agent_prompt(
    spec_id: &str,
    stage: SpecStage,
    agent_name: &str,
    cwd: &Path,
) -> Result<String, String> {
    use serde_json::Value;

    // Load prompts.json
    let prompts_path = cwd.join("docs/spec-kit/prompts.json");
    let prompts_content = std::fs::read_to_string(&prompts_path)
        .map_err(|e| format!("Failed to read prompts.json: {}", e))?;

    let prompts: Value = serde_json::from_str(&prompts_content)
        .map_err(|e| format!("Failed to parse prompts.json: {}", e))?;

    // Get stage-specific prompts
    let stage_key = stage.key(); // "spec-plan", "spec-tasks", etc.
    let stage_prompts = prompts
        .get(stage_key)
        .ok_or_else(|| format!("No prompts found for stage {}", stage_key))?;

    // Get THIS agent's prompt template
    let prompt_template = stage_prompts
        .get(agent_name)
        .and_then(|v| v.get("prompt"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            format!(
                "No prompt found for agent {} in stage {}",
                agent_name, stage_key
            )
        })?;

    // Find SPEC directory using ACID-compliant resolver
    let spec_dir = super::spec_directory::find_spec_directory(cwd, spec_id)?;

    // Read SPEC files
    let spec_md = spec_dir.join("spec.md");
    let spec_content =
        std::fs::read_to_string(&spec_md).map_err(|e| format!("Failed to read spec.md: {}", e))?;

    // Build context (include prior stage outputs with size limits)
    // OS argument limit is ~2MB, but prior stage outputs can contain nested mega-bundles
    // Must be very conservative to prevent exponential growth
    // Total budget: ~100KB for all files combined
    const MAX_FILE_SIZE: usize = 20_000; // ~20KB per file (very conservative)

    let mut context = format!("SPEC: {}\n\n## spec.md\n", spec_id);

    // Add spec.md (truncate if too large)
    if spec_content.len() > MAX_FILE_SIZE {
        tracing::warn!(
            "  Truncating spec.md: {} → {} chars",
            spec_content.len(),
            MAX_FILE_SIZE
        );
        context.push_str(&spec_content.chars().take(MAX_FILE_SIZE).collect::<String>());
        context.push_str(&format!(
            "\n\n[...truncated {} chars...]\n\n",
            spec_content.len() - MAX_FILE_SIZE
        ));
    } else {
        context.push_str(&spec_content);
        context.push_str("\n\n");
    }

    // Add plan.md if available (for Tasks, Implement, Validate, etc.)
    // INTELLIGENT EXTRACTION: Skip debug sections and mega-bundles
    if stage != crate::spec_prompts::SpecStage::Plan {
        let plan_md = spec_dir.join("plan.md");
        if let Ok(plan_content) = std::fs::read_to_string(&plan_md) {
            context.push_str("## plan.md (summary)\n");

            // Extract only useful sections (skip debug output and embedded mega-bundles)
            let useful_content = extract_useful_content_from_stage_file(&plan_content);

            if useful_content.len() > MAX_FILE_SIZE {
                tracing::warn!(
                    "  📄 plan.md: {} total → {} useful → {} final (truncated)",
                    plan_content.len(),
                    useful_content.len(),
                    MAX_FILE_SIZE
                );
                context.push_str(
                    &useful_content
                        .chars()
                        .take(MAX_FILE_SIZE)
                        .collect::<String>(),
                );
                context.push_str(&format!("\n\n[...truncated...]\n\n"));
            } else {
                tracing::info!(
                    "  📄 plan.md: {} total → {} useful ({}% reduction)",
                    plan_content.len(),
                    useful_content.len(),
                    100 - (useful_content.len() * 100 / plan_content.len().max(1))
                );
                context.push_str(&useful_content);
                context.push_str("\n\n");
            }
        }
    }

    // Add tasks.md if available (for Implement, Validate, etc.)
    // INTELLIGENT EXTRACTION: Same filtering as plan.md
    if matches!(
        stage,
        crate::spec_prompts::SpecStage::Implement
            | crate::spec_prompts::SpecStage::Validate
            | crate::spec_prompts::SpecStage::Audit
            | crate::spec_prompts::SpecStage::Unlock
    ) {
        let tasks_md = spec_dir.join("tasks.md");
        if let Ok(tasks_content) = std::fs::read_to_string(&tasks_md) {
            context.push_str("## tasks.md (summary)\n");

            let useful_content = extract_useful_content_from_stage_file(&tasks_content);

            if useful_content.len() > MAX_FILE_SIZE {
                tracing::warn!(
                    "  📄 tasks.md: {} total → {} useful → {} final (truncated)",
                    tasks_content.len(),
                    useful_content.len(),
                    MAX_FILE_SIZE
                );
                context.push_str(
                    &useful_content
                        .chars()
                        .take(MAX_FILE_SIZE)
                        .collect::<String>(),
                );
                context.push_str(&format!("\n\n[...truncated...]\n\n"));
            } else {
                tracing::info!(
                    "  📄 tasks.md: {} total → {} useful ({}% reduction)",
                    tasks_content.len(),
                    useful_content.len(),
                    100 - (useful_content.len() * 100 / tasks_content.len().max(1))
                );
                context.push_str(&useful_content);
                context.push_str("\n\n");
            }
        }
    }

    // SPEC-KIT-924: Replace ALL template variables including metadata
    // Parse agent name to SpecAgent enum to get metadata
    let spec_agent = SpecAgent::from_string(agent_name)
        .ok_or_else(|| format!("Unknown agent name: {}", agent_name))?;

    // Get prompt version
    let prompt_version =
        crate::spec_prompts::stage_version_enum(stage).unwrap_or_else(|| "unversioned".to_string());

    // Get model metadata (MODEL_ID, MODEL_RELEASE, REASONING_MODE)
    let metadata = crate::spec_prompts::model_metadata(stage, spec_agent);
    let model_id = metadata
        .iter()
        .find(|(k, _)| k == "MODEL_ID")
        .map(|(_, v)| v.as_str())
        .unwrap_or("unknown");
    let model_release = metadata
        .iter()
        .find(|(k, _)| k == "MODEL_RELEASE")
        .map(|(_, v)| v.as_str())
        .unwrap_or("unknown");
    let reasoning_mode = metadata
        .iter()
        .find(|(k, _)| k == "REASONING_MODE")
        .map(|(_, v)| v.as_str())
        .unwrap_or("unknown");

    // Replace all placeholders (including metadata variables)
    let prompt = prompt_template
        .replace("${SPEC_ID}", spec_id)
        .replace("${CONTEXT}", &context)
        .replace("${PROMPT_VERSION}", &prompt_version)
        .replace("${MODEL_ID}", model_id)
        .replace("${MODEL_RELEASE}", model_release)
        .replace("${REASONING_MODE}", reasoning_mode);

    Ok(prompt)
}

/// Spawn and wait for a single agent to complete (sequential execution)
/// Returns the agent's output for injection into next agent's prompt
async fn spawn_and_wait_for_agent(
    agent_name: &str,
    config_name: &str,
    prompt: String,
    agent_configs: &[AgentConfig],
    batch_id: &str,
    spec_id: &str,
    stage: SpecStage,
    run_id: Option<&str>,
    timeout_secs: u64,
) -> Result<(String, String), String> {
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

    // SPEC-KIT-927: Enable observable agents by default for file stability protection
    // Set SPEC_KIT_OBSERVABLE_AGENTS=0 to disable if needed
    let tmux_enabled = std::env::var("SPEC_KIT_OBSERVABLE_AGENTS")
        .map(|v| v != "0" && v.to_lowercase() != "false")
        .unwrap_or(true);

    if tmux_enabled {
        tracing::info!("{}   🔍 Observable agents ENABLED (tmux mode)", run_tag);

        // SPEC-KIT-927: Check for and cleanup zombie processes before spawning new agents
        // This prevents orphaned agents from previous runs from interfering
        let session_name = format!("agents-{}", config_name);
        if let Ok(zombie_count) = codex_core::tmux::check_zombie_panes(&session_name).await {
            if zombie_count > 0 {
                tracing::warn!(
                    "{}   ⚠️ Found {} zombie panes in session {}, cleaning up...",
                    run_tag,
                    zombie_count,
                    session_name
                );
                if let Err(e) = codex_core::tmux::cleanup_zombie_panes(&session_name).await {
                    tracing::warn!("{}   ⚠️ Failed to cleanup zombie panes: {}", run_tag, e);
                } else {
                    tracing::info!("{}   ✅ Cleaned up {} zombie panes", run_tag, zombie_count);
                }
            }
        }
    }

    // Spawn agent
    let agent_id = {
        let mut manager = AGENT_MANAGER.write().await;
        manager
            .create_agent_from_config_name(
                config_name,
                agent_configs,
                prompt.clone(),
                false,
                Some(batch_id.to_string()),
                tmux_enabled, // SPEC-KIT-923
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

    // Record to SQLite with run_id for traceability
    if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
        let _ = db.record_agent_spawn(
            &agent_id,
            spec_id,
            stage,
            "regular_stage",
            agent_name,
            run_id,
        );
        if let Some(rid) = run_id {
            tracing::info!("    ✓ Recorded spawn with run_id: {}", rid);
        }
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

                        // CRITICAL: Record completion to SQLite for audit trail
                        if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
                            if let Err(e) = db.record_agent_completion(&agent_id, result) {
                                tracing::warn!(
                                    "    ⚠️ Failed to record completion to SQLite: {}",
                                    e
                                );
                            } else {
                                tracing::debug!("    ✓ Completion recorded to SQLite");
                            }
                        }

                        return Ok((agent_id, result.clone()));
                    } else {
                        return Err(format!("{} completed but no result", agent_name));
                    }
                }
                AgentStatus::Failed => {
                    // Check both error field and result field
                    let error_detail = agent
                        .error
                        .as_ref()
                        .or(agent.result.as_ref())
                        .map(|e| e.clone())
                        .unwrap_or_else(|| "no error message available".to_string());

                    tracing::error!("  ❌ {} FAILED - Status: {:?}", agent_name, agent.status);
                    tracing::error!("  ❌ Error detail: {}", error_detail);
                    tracing::error!("  ❌ Agent config: model={}", agent.model);

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

/// Spawn regular stage agents SEQUENTIALLY with output passing
/// Session 3: True sequential execution with agent collaboration
async fn spawn_regular_stage_agents_sequential(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
    run_id: Option<String>,
    expected_agents: &[String],
    agent_configs: &[AgentConfig],
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

    let agent_config_map: std::collections::HashMap<&str, &str> = [
        ("gemini", "gemini_flash"),
        ("claude", "claude_haiku"),
        ("gpt_pro", "gpt_pro"),
        ("gpt_codex", "gpt_codex"),
    ]
    .iter()
    .copied()
    .collect();

    // Spawn agents SEQUENTIALLY, each can use previous outputs
    for (idx, agent_name) in expected_agents.iter().enumerate() {
        tracing::warn!(
            "{} 🔄 SEQUENTIAL: Agent {}/{}: {}",
            run_tag,
            idx + 1,
            expected_agents.len(),
            agent_name
        );

        let config_name = agent_config_map
            .get(agent_name.as_str())
            .ok_or_else(|| format!("No config mapping for agent {}", agent_name))?;

        // Build prompt for THIS agent with previous agent outputs injected
        let mut prompt = build_individual_agent_prompt(spec_id, stage, agent_name, cwd).await?;

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
            config_name,
            prompt,
            agent_configs,
            &batch_id,
            spec_id,
            stage,
            run_id.as_deref(),
            1200, // 20min timeout per agent (Gemini can be slow)
        )
        .await?;

        // Store this agent's output for next agents to use
        agent_outputs.push((agent_name.clone(), agent_output.clone()));

        spawn_infos.push(AgentSpawnInfo {
            agent_id,
            agent_name: agent_name.clone(),
            model_name: config_name.to_string(),
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

/// Spawn regular stage agents in PARALLEL for consensus (no output passing)
/// Used for stages where independent perspectives are critical (Validate, Audit, Unlock)
async fn spawn_regular_stage_agents_parallel(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
    run_id: Option<String>,
    expected_agents: &[String],
    agent_configs: &[AgentConfig],
) -> Result<Vec<AgentSpawnInfo>, String> {
    let run_tag = run_id
        .as_ref()
        .map(|r| format!("[run:{}]", &r[..8]))
        .unwrap_or_else(|| "[run:none]".to_string());
    tracing::warn!(
        "{} 🎬 AUDIT: spawn_regular_stage_agents_parallel (independent consensus mode)",
        run_tag
    );
    tracing::warn!("  spec_id: {}", spec_id);
    tracing::warn!("  stage: {:?}", stage);
    tracing::warn!("  expected_agents: {:?}", expected_agents);

    let mut spawn_infos = Vec::new();
    let batch_id = uuid::Uuid::new_v4().to_string();

    // SPEC-KIT-923: Check for observable agents flag
    let tmux_enabled = std::env::var("SPEC_KIT_OBSERVABLE_AGENTS")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    if tmux_enabled {
        tracing::info!("{} 🔍 Observable agents ENABLED (tmux mode)", run_tag);
    }

    let agent_config_map: std::collections::HashMap<&str, &str> = [
        ("gemini", "gemini_flash"),
        ("claude", "claude_haiku"),
        ("gpt_pro", "gpt_pro"),
        ("gpt_codex", "gpt_codex"),
    ]
    .iter()
    .copied()
    .collect();

    // Spawn all agents in PARALLEL (no waiting, no output passing)
    for (idx, agent_name) in expected_agents.iter().enumerate() {
        tracing::warn!(
            "{} 🚀 PARALLEL: Spawning agent {}/{}: {}",
            run_tag,
            idx + 1,
            expected_agents.len(),
            agent_name
        );

        let config_name = agent_config_map
            .get(agent_name.as_str())
            .ok_or_else(|| format!("No config mapping for agent {}", agent_name))?;

        // Build individual prompt (no previous outputs)
        let prompt = build_individual_agent_prompt(spec_id, stage, agent_name, cwd).await?;

        // Spawn without waiting
        let mut manager = AGENT_MANAGER.write().await;
        let agent_id = manager
            .create_agent_from_config_name(
                config_name,
                agent_configs,
                prompt,
                false,
                Some(batch_id.clone()),
                tmux_enabled, // SPEC-KIT-923
            )
            .await
            .map_err(|e| format!("Failed to spawn {}: {}", agent_name, e))?;

        // Record to SQLite with run_id
        if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
            let _ = db.record_agent_spawn(
                &agent_id,
                spec_id,
                stage,
                "regular_stage",
                agent_name,
                run_id.as_deref(),
            );
        }

        spawn_infos.push(AgentSpawnInfo {
            agent_id,
            agent_name: agent_name.clone(),
            model_name: config_name.to_string(),
            result: None, // Parallel execution doesn't have result yet
        });

        tracing::warn!("{}   ✓ {} spawned (not waiting)", run_tag, agent_name);
    }

    tracing::warn!(
        "{} ✅ PARALLEL: All {} agents spawned, executing independently",
        run_tag,
        expected_agents.len()
    );

    Ok(spawn_infos)
}

/// Spawn regular stage agents natively (SPEC-KIT-900 Session 3)
/// Routes to appropriate execution pattern based on stage type
async fn spawn_regular_stage_agents_native(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
    _prompt: &str, // Deprecated: no longer used (was mega-bundle)
    run_id: Option<String>,
    expected_agents: &[String],
    agent_configs: &[AgentConfig],
) -> Result<Vec<AgentSpawnInfo>, String> {
    let run_tag = run_id
        .as_ref()
        .map(|r| format!("[run:{}]", &r[..8]))
        .unwrap_or_else(|| "[run:none]".to_string());

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
                expected_agents,
                agent_configs,
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
            spawn_regular_stage_agents_sequential(
                cwd,
                spec_id,
                stage,
                run_id,
                expected_agents,
                agent_configs,
            )
            .await
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
                expected_agents,
                agent_configs,
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
                expected_agents,
                agent_configs,
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
                    if matches!(status, AgentStatus::Completed) {
                        if let Some(agent) = manager.get_agent(agent_id) {
                            if let Some(result_text) = &agent.result {
                                let _ = db.record_agent_completion(agent_id, result_text);
                            }
                        }
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
            use super::ace_prompt_injector::{command_to_scope, should_use_ace};
            use super::routing::{get_current_branch, get_repo_root};

            let command_name = format!("speckit.{}", stage.command_name());

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
            if let Some(state) = widget.spec_auto_state.as_mut() {
                if let Some(bullets) = &state.ace_bullets_cache {
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
            let agent_count = expected_agents_for_stage(stage).len();
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
                expected_agents_for_stage(stage)
                    .iter()
                    .map(|a| a.canonical_name())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));

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
                if let (
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
                                model: agent_name.clone(), // Best guess at this point
                                prompt_preview: prompt_preview.clone(),
                                timestamp: super::execution_logger::ExecutionEvent::now(),
                            },
                        );
                    }
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

            let spawn_result = block_on_sync(|| async move {
                spawn_regular_stage_agents_native(
                    &cwd,
                    &spec_id_owned,
                    stage,
                    &prompt_owned,
                    run_id_for_spawn,
                    &expected_agents_owned,
                    &agent_configs_owned,
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

                                    let _ = event_tx.send(
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

                        let _ = widget.app_event_tx.send(
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
    if let Some(state) = widget.spec_auto_state.as_ref() {
        if let Some(current_stage) = state.current_stage() {
            if let Some(run_id) = &state.run_id {
                if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
                    for (agent_name, response_text) in &agent_results {
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
                                "{} Failed to store {} artifact: {}",
                                run_tag,
                                agent_name,
                                e
                            );
                        } else {
                            tracing::warn!(
                                "{} ✓ Stored {} artifact to SQLite",
                                run_tag,
                                agent_name
                            );
                        }
                    }
                }
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
            if let Some(ref database) = db {
                if let Ok(Some((phase_type, _))) = database.get_agent_spawn_info(&agent_info.id) {
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
            }

            // Log agent complete event
            if let Some(state) = widget.spec_auto_state.as_ref() {
                if let Some(run_id) = &state.run_id {
                    if let Some(current_stage) = state.current_stage() {
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
                            if let Some(ref database) = db {
                                if let Ok(Some((phase_type, _))) =
                                    database.get_agent_spawn_info(&a.id)
                                {
                                    return phase_type == "regular_stage";
                                }
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
                // GPT-5 validation phase - single agent (GPT-5)
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
        };
        phase_type
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
                                        tracing::warn!(
                                            "Failed to store {} artifact to SQLite: {}",
                                            agent_name,
                                            e
                                        );
                                    } else {
                                        tracing::warn!(
                                            "DEBUG: Stored {} artifact to SQLite",
                                            agent_name
                                        );

                                        // Note: Memory cleanup removed - SQLite-based consensus doesn't use local-memory
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
