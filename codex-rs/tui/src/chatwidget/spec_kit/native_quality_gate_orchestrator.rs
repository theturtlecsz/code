//! Native Quality Gate Orchestration (SPEC-KIT-900, I-003 fix)
//!
//! Eliminates LLM orchestrator for quality gate plumbing.
//! LLMs do quality analysis ONLY. Native code handles spawning, waiting, collection.
//!
//! Architecture:
//! - Native spawns 3 agents (gemini, claude, code) with gate-specific prompts
//! - Native polls for completion
//! - Native broker collects results from filesystem
//! - No Python scripts, no orchestrator LLM

use super::state::{QualityCheckpoint, QualityGateType};
use crate::spec_prompts::SpecStage;
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

/// Spawn quality gate agents natively (no LLM orchestrator)
pub async fn spawn_quality_gate_agents_native(
    cwd: &Path,
    spec_id: &str,
    checkpoint: QualityCheckpoint,
    agent_configs: &[AgentConfig],
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

    let gate_prompts = prompts.get(gate_key)
        .ok_or_else(|| format!("No prompts found for {}", gate_key))?;

    // Define the 3 agents to spawn (SPEC-KIT-070 Tier 2: cheapest models for quality gates)
    let agent_spawn_configs = vec![
        ("gemini", "gemini_flash"),  // gemini-2.5-flash (cheapest)
        ("claude", "claude_haiku"),  // claude-haiku (cheapest)
        ("code", "gpt_low"),         // gpt-5 low reasoning (cheapest, matches flash/haiku tier)
    ];

    let mut spawn_infos = Vec::new();
    let batch_id = uuid::Uuid::new_v4().to_string();

    // Spawn each agent
    for (agent_name, config_name) in agent_spawn_configs {
        let prompt_template = gate_prompts.get(agent_name)
            .and_then(|v| v.get("prompt"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("No prompt found for {} in {}", agent_name, gate_key))?;

        // Build prompt with SPEC context
        let prompt = build_quality_gate_prompt(
            spec_id,
            gate,
            prompt_template,
            cwd,
        ).await?;

        // Get prompt preview (first 200 chars)
        let prompt_preview = if prompt.len() > 200 {
            format!("{}...", &prompt[..200])
        } else {
            prompt.clone()
        };

        // Spawn agent via AgentManager using config lookup
        let mut manager = AGENT_MANAGER.write().await;
        let agent_id = manager.create_agent_from_config_name(
            config_name, // Config name (e.g., "gemini_flash", "claude_haiku")
            agent_configs,
            prompt,
            true, // read_only
            Some(batch_id.clone()),
        ).await.map_err(|e| format!("Failed to spawn {}: {}", config_name, e))?;

        spawn_infos.push(AgentSpawnInfo {
            agent_id,
            agent_name: agent_name.to_string(),
            model_name: config_name.to_string(),
            prompt_preview,
        });
    }

    Ok(spawn_infos)
}

/// Build quality gate prompt with SPEC context
async fn build_quality_gate_prompt(
    spec_id: &str,
    gate: QualityGateType,
    prompt_template: &str,
    cwd: &Path,
) -> Result<String, String> {
    // Find SPEC directory
    let spec_dir = find_spec_directory(cwd, spec_id)
        .ok_or_else(|| format!("SPEC directory not found for {}", spec_id))?;

    // Read SPEC files
    let spec_md = spec_dir.join("spec.md");
    let spec_content = std::fs::read_to_string(&spec_md)
        .map_err(|e| format!("Failed to read spec.md: {}", e))?;

    let prd_md = spec_dir.join("PRD.md");
    let prd_content = std::fs::read_to_string(&prd_md)
        .map_err(|e| format!("Failed to read PRD.md: {}", e))?;

    // Build context
    let context = format!(
        r#"SPEC: {}

## spec.md
{}

## PRD.md
{}
"#,
        spec_id,
        spec_content,
        prd_content
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

    loop {
        if start.elapsed() > timeout {
            return Err(format!("Timeout waiting for {} agents", agent_ids.len()));
        }

        // Check if all agents are complete
        let manager = AGENT_MANAGER.read().await;
        let mut all_done = true;

        for agent_id in agent_ids {
            if let Some(agent) = manager.get_agent(agent_id) {
                use codex_core::agent_tool::AgentStatus;
                match agent.status {
                    AgentStatus::Completed | AgentStatus::Failed | AgentStatus::Cancelled => {
                        // Agent done
                    }
                    _ => {
                        all_done = false;
                        break;
                    }
                }
            } else {
                all_done = false;
                break;
            }
        }

        if all_done {
            return Ok(());
        }

        // Sleep before next poll
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}

// === Helper Functions ===

fn find_spec_directory(cwd: &Path, spec_id: &str) -> Option<std::path::PathBuf> {
    let docs_dir = cwd.join("docs");
    if !docs_dir.exists() {
        return None;
    }

    let entries = std::fs::read_dir(&docs_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with(spec_id) && entry.path().is_dir() {
            return Some(entry.path());
        }
    }

    None
}
