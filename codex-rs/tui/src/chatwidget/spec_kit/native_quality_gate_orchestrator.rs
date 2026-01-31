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
//!
//! D113/D133: Now uses unified prompt-source API for consistency.

#![allow(dead_code)] // Native orchestration pending full integration

use super::state::{QualityCheckpoint, QualityGateType};
use crate::spec_prompts::{SpecAgent, SpecStage, get_prompt_with_version, render_prompt_text};
use codex_core::agent_tool::AGENT_MANAGER;
use codex_core::config_types::AgentConfig;
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

    // D113/D133: Get gate key for prompt lookup
    let gate_key = match gate {
        QualityGateType::Clarify => "quality-gate-clarify",
        QualityGateType::Checklist => "quality-gate-checklist",
        QualityGateType::Analyze => "quality-gate-analyze",
    };

    // D113/D133: Compute spec_agent and spec_stage for unified API
    let spec_agent = SpecAgent::from_string(quality_gate_agent).unwrap_or(SpecAgent::Claude);
    let spec_stage = match gate {
        QualityGateType::Clarify => SpecStage::Clarify,
        QualityGateType::Checklist => SpecStage::Checklist,
        QualityGateType::Analyze => SpecStage::Analyze,
    };

    // D113/D133: Use unified prompt API (project-local → embedded fallback)
    let (prompt_template, prompt_version) =
        get_prompt_with_version(gate_key, spec_agent, Some(cwd)).ok_or_else(|| {
            format!(
                "No prompt found for {} (agent: {})",
                gate_key, quality_gate_agent
            )
        })?;

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
        // D113/D133: Build prompt with SPEC context using unified render_prompt_text
        let prompt = build_quality_gate_prompt(
            spec_id,
            &prompt_template,
            &prompt_version,
            spec_stage,
            spec_agent,
            cwd,
        )
        .await?;

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

/// Maximum size for individual artifact files (~20KB per file)
const MAX_ARTIFACT_FILE_SIZE: usize = 20_000;

/// Build artifacts string from SPEC directory files with truncation
///
/// Includes: spec.md, PRD.md, plan.md (if exists), tasks.md (if exists)
/// Each file is truncated to MAX_ARTIFACT_FILE_SIZE to prevent prompt explosion.
fn build_artifacts_string(spec_dir: &std::path::Path) -> String {
    let mut artifacts = String::new();

    // Helper to read and truncate a file
    let read_and_truncate = |path: &std::path::Path| -> Option<String> {
        std::fs::read_to_string(path).ok().map(|content| {
            if content.len() > MAX_ARTIFACT_FILE_SIZE {
                let truncated: String = content.chars().take(MAX_ARTIFACT_FILE_SIZE).collect();
                format!(
                    "{}\n\n[...truncated {} chars...]\n",
                    truncated,
                    content.len() - MAX_ARTIFACT_FILE_SIZE
                )
            } else {
                content
            }
        })
    };

    // spec.md (required)
    if let Some(content) = read_and_truncate(&spec_dir.join("spec.md")) {
        artifacts.push_str("## spec.md\n\n");
        artifacts.push_str(&content);
        artifacts.push_str("\n\n");
    }

    // PRD.md (optional but common)
    if let Some(content) = read_and_truncate(&spec_dir.join("PRD.md")) {
        artifacts.push_str("## PRD.md\n\n");
        artifacts.push_str(&content);
        artifacts.push_str("\n\n");
    }

    // plan.md (optional, present post-plan stage)
    if let Some(content) = read_and_truncate(&spec_dir.join("plan.md")) {
        artifacts.push_str("## plan.md\n\n");
        artifacts.push_str(&content);
        artifacts.push_str("\n\n");
    }

    // tasks.md (optional, present post-tasks stage)
    if let Some(content) = read_and_truncate(&spec_dir.join("tasks.md")) {
        artifacts.push_str("## tasks.md\n\n");
        artifacts.push_str(&content);
        artifacts.push_str("\n\n");
    }

    artifacts
}

/// Build quality gate prompt with SPEC context
///
/// D113/D133: Uses unified render_prompt_text for all substitutions.
async fn build_quality_gate_prompt(
    spec_id: &str,
    prompt_template: &str,
    prompt_version: &str,
    spec_stage: SpecStage,
    spec_agent: SpecAgent,
    cwd: &Path,
) -> Result<String, String> {
    // Find SPEC directory using central ACID-compliant resolver
    let spec_dir = super::spec_directory::find_spec_directory(cwd, spec_id)?;

    // Read SPEC files
    let spec_md = spec_dir.join("spec.md");
    let spec_content =
        std::fs::read_to_string(&spec_md).map_err(|e| format!("Failed to read spec.md: {}", e))?;

    let prd_md = spec_dir.join("PRD.md");
    let prd_content = std::fs::read_to_string(&prd_md).unwrap_or_default();

    // D113/D133: Build context using standard ${CONTEXT} pattern
    let context = format!(
        r#"SPEC: {}

## spec.md
{}

## PRD.md
{}
"#,
        spec_id, spec_content, prd_content
    );

    // MAINT-14: Build artifacts string for ${ARTIFACTS} substitution
    // Quality gate prompts need access to all artifacts for consistency checking
    let artifacts = build_artifacts_string(&spec_dir);

    // D113/D133: Use unified render_prompt_text for all substitutions
    // This handles: user vars, model metadata, prompt version, and template expansion
    let mut prompt = render_prompt_text(
        prompt_template,
        prompt_version,
        &[
            ("SPEC_ID", spec_id),
            ("CONTEXT", &context),
            ("ARTIFACTS", &artifacts),
        ],
        spec_stage,
        spec_agent,
    );

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn quality_gate_prompt_no_template_leakage() {
        // D113/D133: Verify render_prompt_text substitutes all tokens correctly

        // Create temp dir with spec.md and PRD.md
        let temp = tempfile::TempDir::new().unwrap();
        let spec_dir = temp.path().join("docs/SPEC-TEST");
        std::fs::create_dir_all(&spec_dir).unwrap();
        std::fs::write(spec_dir.join("spec.md"), "# Test Spec\nTest content").unwrap();
        std::fs::write(spec_dir.join("PRD.md"), "# Test PRD").unwrap();

        // Build prompt using unified API (will use embedded prompts since no prompts.json)
        let prompt = build_quality_gate_prompt(
            "SPEC-TEST",
            "Test template: ${SPEC_ID}, Model: ${MODEL_ID}, Version: ${PROMPT_VERSION}, Template: ${TEMPLATE:plan}",
            "v1.0.0-test",
            SpecStage::Clarify,
            SpecAgent::Claude,
            temp.path(),
        )
        .await
        .unwrap();

        // Assert no template/model tokens leaked
        assert!(
            !prompt.contains("${TEMPLATE:"),
            "Template token leaked: {}",
            &prompt[..prompt.len().min(300)]
        );
        assert!(
            !prompt.contains("${MODEL_ID}"),
            "MODEL_ID token leaked: {}",
            &prompt[..prompt.len().min(300)]
        );
        assert!(
            !prompt.contains("${MODEL_RELEASE}"),
            "MODEL_RELEASE token leaked"
        );
        assert!(
            !prompt.contains("${REASONING_MODE}"),
            "REASONING_MODE token leaked"
        );
        assert!(
            !prompt.contains("${PROMPT_VERSION}"),
            "PROMPT_VERSION token leaked"
        );
        assert!(!prompt.contains("${SPEC_ID}"), "SPEC_ID token leaked");

        // Verify substitutions occurred
        assert!(prompt.contains("SPEC-TEST"), "SPEC_ID not substituted");
        assert!(
            prompt.contains("v1.0.0-test") || !prompt.contains("${"),
            "Version not substituted or tokens remain"
        );
    }

    #[tokio::test]
    async fn quality_gate_prompt_no_artifacts_leakage() {
        // MAINT-14: Verify ${ARTIFACTS} placeholder is substituted correctly
        // and contains expected artifact headers

        // Create temp dir with spec.md, PRD.md, plan.md, tasks.md
        let temp = tempfile::TempDir::new().unwrap();
        let spec_dir = temp.path().join("docs/SPEC-ARTIFACTS-TEST");
        std::fs::create_dir_all(&spec_dir).unwrap();
        std::fs::write(spec_dir.join("spec.md"), "# Test Spec\nSpec content here").unwrap();
        std::fs::write(spec_dir.join("PRD.md"), "# Test PRD\nPRD content here").unwrap();
        std::fs::write(spec_dir.join("plan.md"), "# Plan\nPlan content here").unwrap();
        std::fs::write(spec_dir.join("tasks.md"), "# Tasks\nTasks content here").unwrap();

        // Use a template that includes ${ARTIFACTS} placeholder
        let template = "Analyze artifacts for ${SPEC_ID}:\n\n${ARTIFACTS}\n\nProvide analysis.";

        let prompt = build_quality_gate_prompt(
            "SPEC-ARTIFACTS-TEST",
            template,
            "v1.0.0-artifacts",
            SpecStage::Analyze,
            SpecAgent::Claude,
            temp.path(),
        )
        .await
        .unwrap();

        // Assert ${ARTIFACTS} placeholder was substituted (no leak)
        assert!(
            !prompt.contains("${ARTIFACTS}"),
            "ARTIFACTS token leaked to model: {}",
            &prompt[..prompt.len().min(500)]
        );

        // Assert artifact headers are present (proves substitution worked)
        assert!(
            prompt.contains("## spec.md"),
            "Expected ## spec.md header in artifacts"
        );
        assert!(
            prompt.contains("## PRD.md"),
            "Expected ## PRD.md header in artifacts"
        );
        assert!(
            prompt.contains("## plan.md"),
            "Expected ## plan.md header in artifacts"
        );
        assert!(
            prompt.contains("## tasks.md"),
            "Expected ## tasks.md header in artifacts"
        );

        // Assert actual content is present
        assert!(
            prompt.contains("Spec content here"),
            "Expected spec.md content in artifacts"
        );
        assert!(
            prompt.contains("Plan content here"),
            "Expected plan.md content in artifacts"
        );
    }

    #[test]
    fn build_artifacts_string_truncates_large_files() {
        // MAINT-14: Verify truncation works correctly for large files

        let temp = tempfile::TempDir::new().unwrap();
        let spec_dir = temp.path();

        // Create a file larger than MAX_ARTIFACT_FILE_SIZE (20KB)
        let large_content = "x".repeat(25_000);
        std::fs::write(spec_dir.join("spec.md"), &large_content).unwrap();

        let artifacts = build_artifacts_string(spec_dir);

        // Should contain the header
        assert!(artifacts.contains("## spec.md"));

        // Should be truncated (less than original size + some overhead for header/marker)
        assert!(
            artifacts.len() < 22_000,
            "Artifacts should be truncated, got {} chars",
            artifacts.len()
        );

        // Should contain truncation marker
        assert!(
            artifacts.contains("[...truncated"),
            "Expected truncation marker in output"
        );
    }
}
