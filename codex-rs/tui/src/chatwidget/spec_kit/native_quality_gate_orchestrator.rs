//! Native Quality Gate Orchestration (SPEC-KIT-900, I-003 fix)
//!
//! Eliminates LLM orchestrator for quality gate plumbing.
//! LLMs do quality analysis ONLY. Native code handles spawning, waiting, collection.
//!
//! Architecture (GR-001 compliant):
//! - Single-agent "critic sidecar" mode only (no multi-agent consensus)
//! - Native spawns exactly 1 agent with gate-specific prompt
//! - Native polls for completion
//! - Native broker collects results from filesystem
//! - No Python scripts, no orchestrator LLM

#![allow(dead_code)] // Native orchestration pending full integration

use super::state::{QualityCheckpoint, QualityGateType};
use codex_core::agent_tool::AGENT_MANAGER;
use codex_core::config_types::AgentConfig;
use serde_json::Value;
use std::path::Path;

/// Agent spawn info for logging
pub struct AgentSpawnInfo {
    pub agent_id: String,
    pub agent_name: String,
    pub model_name: String,
    pub prompt_preview: String,
}

/// Spawn quality gate agent natively (GR-001 compliant: single agent only)
///
/// # Arguments
/// * `quality_gate_agent` - The single configured quality gate agent name
pub async fn spawn_quality_gate_agents_native(
    cwd: &Path,
    spec_id: &str,
    checkpoint: QualityCheckpoint,
    agent_configs: &[AgentConfig],
    quality_gate_agent: &str, // GR-001: Single agent only
    run_id: Option<String>,
    branch_id: Option<String>, // P6-SYNC Phase 4: Branch tracking for resume filtering
) -> Result<Vec<AgentSpawnInfo>, String> {
    let gates = checkpoint.gates();

    // For now, we only handle single-gate checkpoints
    // Multiple gates would need sequential or parallel spawning
    if gates.len() != 1 {
        return Err(format!(
            "Native orchestration currently supports single-gate checkpoints only (got {} gates)",
            gates.len()
        ));
    }

    let gate = gates[0];

    // P7-SPEC: Create run_tag for consistent log tagging
    let run_tag = run_id
        .as_ref()
        .map(|r| format!("[run:{}]", &r[..8.min(r.len())]))
        .unwrap_or_else(|| "[run:none]".to_string());

    // Load prompts from prompts.json
    let prompts_path = cwd.join("docs/spec-kit/prompts.json");
    let prompts_content = std::fs::read_to_string(&prompts_path)
        .map_err(|e| format!("Failed to read prompts.json: {}", e))?;

    let prompts: Value = serde_json::from_str(&prompts_content)
        .map_err(|e| format!("Failed to parse prompts.json: {}", e))?;

    // Get gate-specific prompts
    let gate_key = match gate {
        QualityGateType::Clarify => "quality-gate-clarify",
        QualityGateType::Checklist => "quality-gate-checklist",
        QualityGateType::Analyze => "quality-gate-analyze",
    };

    let gate_prompts = prompts
        .get(gate_key)
        .ok_or_else(|| format!("No prompts found for {}", gate_key))?;

    // GR-001: Use the single configured agent (no hardcoded multi-agent list)
    // Map common agent names to config names
    let agent_lower = quality_gate_agent.to_lowercase();
    let config_name: &str = match agent_lower.as_str() {
        "gemini" | "gemini-flash" => "gemini_flash",
        "claude" | "claude-haiku" => "claude_haiku",
        "code" | "gpt" | "gpt-low" => "gpt_low",
        _ => quality_gate_agent, // Use as-is if not a known alias
    };

    let mut spawn_infos = Vec::new();
    let batch_id = uuid::Uuid::new_v4().to_string();

    // SPEC-KIT-928: Log concurrent agent check BEFORE spawning
    {
        let manager_pre_check = AGENT_MANAGER.read().await;
        let running_agents = manager_pre_check.get_running_agents();

        if !running_agents.is_empty() {
            let running_list: Vec<String> = running_agents
                .iter()
                .map(|(id, model, _)| format!("{} ({})", model, &id[..8]))
                .collect();
            tracing::info!(
                "{} 📊 Pre-spawn check for {}: {} agents currently running: {}",
                run_tag,
                spec_id,
                running_list.len(),
                running_list.join(", ")
            );
        } else {
            tracing::info!(
                "{} 📊 Pre-spawn check for {}: No agents currently running",
                run_tag,
                spec_id
            );
        }
    }

    // GR-001: Spawn the single configured agent
    let agent_name = quality_gate_agent;
    {
        // Try to find agent-specific prompt, fall back to generic "critic" prompt
        let prompt_template = gate_prompts
            .get(agent_name)
            .and_then(|v| v.get("prompt"))
            .and_then(|v| v.as_str())
            .or_else(|| {
                // Fallback: try "critic" or first available prompt
                gate_prompts
                    .get("critic")
                    .and_then(|v| v.get("prompt"))
                    .and_then(|v| v.as_str())
            })
            .ok_or_else(|| {
                format!(
                    "No prompt found for {} or 'critic' in {}",
                    agent_name, gate_key
                )
            })?;

        // Build prompt with SPEC context
        let prompt = build_quality_gate_prompt(spec_id, gate, prompt_template, cwd).await?;

        // Get prompt preview (first 200 chars)
        let prompt_preview = if prompt.len() > 200 {
            format!("{}...", &prompt[..200])
        } else {
            prompt.clone()
        };

        // Spawn agent via AgentManager using config lookup
        let mut manager = AGENT_MANAGER.write().await;
        let agent_id = manager
            .create_agent_from_config_name(
                config_name, // Config name (e.g., "gemini_flash", "claude_haiku")
                agent_configs,
                prompt,
                true, // read_only
                Some(batch_id.clone()),
            )
            .await
            .map_err(|e| format!("Failed to spawn {}: {}", config_name, e))?;

        // Record agent spawn to SQLite for definitive routing at completion
        // P6-SYNC Phase 4: branch_id now wired from SpecAutoState
        if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
            let stage = crate::spec_prompts::SpecStage::Plan; // Quality gates run before Plan
            if let Err(e) = db.record_agent_spawn(
                &agent_id,
                spec_id,
                stage,
                "quality_gate",
                agent_name,
                run_id.as_deref(),
                branch_id.as_deref(),
            ) {
                tracing::warn!(
                    "{} Failed to record agent spawn for {}: {}",
                    run_tag,
                    agent_name,
                    e
                );
            } else {
                tracing::info!(
                    "{} Recorded quality gate agent spawn: {} ({})",
                    run_tag,
                    agent_name,
                    agent_id
                );
            }
        }

        spawn_infos.push(AgentSpawnInfo {
            agent_id,
            agent_name: agent_name.to_string(),
            model_name: config_name.to_string(),
            prompt_preview,
        });
    }

    // SPEC-KIT-928: Log post-spawn state to detect concurrent execution
    {
        let manager_post_check = AGENT_MANAGER.read().await;
        let now_running = manager_post_check.get_running_agents();

        let running_list: Vec<String> = now_running
            .iter()
            .map(|(id, model, _)| format!("{} ({})", model, &id[..8]))
            .collect();

        tracing::info!(
            "{} 📊 Post-spawn check for {}: {} agents now running (spawned {}): {}",
            run_tag,
            spec_id,
            now_running.len(),
            spawn_infos.len(),
            running_list.join(", ")
        );

        // Detect if we have duplicates using the helper function
        let concurrent = manager_post_check.check_concurrent_agents();
        for (model, count) in concurrent {
            tracing::warn!(
                "{} 🚨 CONCURRENT AGENTS DETECTED: {} instances of '{}' running simultaneously!",
                run_tag,
                count,
                model
            );
        }
    }

    Ok(spawn_infos)
}

/// Build quality gate prompt with SPEC context
async fn build_quality_gate_prompt(
    spec_id: &str,
    _gate: QualityGateType,
    prompt_template: &str,
    cwd: &Path,
) -> Result<String, String> {
    // Find SPEC directory using central ACID-compliant resolver
    let spec_dir = super::spec_directory::find_spec_directory(cwd, spec_id)?;

    // Read SPEC files
    let spec_md = spec_dir.join("spec.md");
    let spec_content =
        std::fs::read_to_string(&spec_md).map_err(|e| format!("Failed to read spec.md: {}", e))?;

    let prd_md = spec_dir.join("PRD.md");
    let prd_content =
        std::fs::read_to_string(&prd_md).map_err(|e| format!("Failed to read PRD.md: {}", e))?;

    // Build context
    let context = format!(
        r#"SPEC: {}

## spec.md
{}

## PRD.md
{}
"#,
        spec_id, spec_content, prd_content
    );

    // Replace placeholders
    let mut prompt = prompt_template
        .replace("${SPEC_ID}", spec_id)
        .replace("SPEC ${SPEC_ID}", &context); // Replace inline SPEC reference with full content

    // CRITICAL: Enforce JSON-only output (agents sometimes respond in prose)
    prompt.push_str("\n\nCRITICAL: You MUST output ONLY valid JSON matching the schema above. Do NOT add commentary, explanations, or prose. Start your response with { and end with }. No markdown fences, no preamble, just pure JSON.");

    Ok(prompt)
}

/// Wait for all quality gate agents to complete
pub async fn wait_for_quality_gate_agents(
    agent_ids: &[String],
    timeout_secs: u64,
) -> Result<(), String> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);
    let mut recorded_completions = std::collections::HashSet::new();

    loop {
        if start.elapsed() > timeout {
            return Err(format!("Timeout waiting for {} agents", agent_ids.len()));
        }

        // Check if all agents are complete
        let manager = AGENT_MANAGER.read().await;
        let mut all_done = true;
        let mut still_running = Vec::new();

        for agent_id in agent_ids {
            if let Some(agent) = manager.get_agent(agent_id) {
                use codex_core::agent_tool::AgentStatus;
                match agent.status {
                    AgentStatus::Completed | AgentStatus::Failed | AgentStatus::Cancelled => {
                        // Agent done - record completion to SQLite (once)
                        // SPEC-KIT-928: Record BOTH Completed and Failed (Failed agents now store output)
                        if (matches!(agent.status, AgentStatus::Completed | AgentStatus::Failed))
                            && !recorded_completions.contains(agent_id)
                            && let Ok(db) = super::consensus_db::ConsensusDb::init_default()
                            && let Some(result) = &agent.result
                        {
                            let _ = db.record_agent_completion(agent_id, result);
                            let status_str = match agent.status {
                                AgentStatus::Completed => "completion",
                                AgentStatus::Failed => "failure (with output)",
                                _ => "other",
                            };
                            tracing::info!(
                                "Recorded quality gate {}: {} ({} bytes)",
                                status_str,
                                agent_id,
                                result.len()
                            );
                            recorded_completions.insert(agent_id.clone());
                        }
                    }
                    _ => {
                        all_done = false;
                        still_running.push((agent_id.clone(), format!("{:?}", agent.status)));
                    }
                }
            } else {
                all_done = false;
                still_running.push((agent_id.clone(), "NotFound".to_string()));
            }
        }

        // SPEC-KIT-928: Log which agents are still running (every 10 seconds)
        if !still_running.is_empty() && start.elapsed().as_secs().is_multiple_of(10) {
            let running_summary: Vec<String> = still_running
                .iter()
                .map(|(id, status)| format!("{}... ({})", &id[..8], status))
                .collect();
            tracing::info!(
                "⏳ Waiting for {} agents: {}",
                still_running.len(),
                running_summary.join(", ")
            );
        }

        if all_done {
            return Ok(());
        }

        // Sleep before next poll
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}

// === Helper Functions ===

// Removed: Use super::spec_directory::find_spec_directory instead
