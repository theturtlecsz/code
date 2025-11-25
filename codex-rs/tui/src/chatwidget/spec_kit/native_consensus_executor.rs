//! Native consensus orchestration without orchestrator agent
//!
//! DEPRECATED: This module implements direct agent spawning and result aggregation
//!
//! **STATUS**: Legacy code, NOT used in current architecture
//! **REPLACED BY**: pipeline_coordinator.rs (check_consensus_and_advance_spec_auto)
//!
//! This file is kept for reference but execute_native_consensus() is never called.
//! The active flow uses:
//! 1. agent_orchestrator.rs - Direct spawning via AGENT_MANAGER
//! 2. pipeline_coordinator.rs - Consensus synthesis with SQLite storage
//!
//! **CLEANUP TODO**: Remove this file after confirming no dependencies
//! to eliminate the wasteful "orchestrator agent" pattern that spawns
//! unnecessary meta-agents.
//!
//! **KEY ARCHITECTURE**:
//! - Directly calls AgentManager::create_agent_with_config()
//! - Polls agent status natively (no AgentWait tool)
//! - Aggregates results using single gpt_pro call or native logic
//! - Stores to local-memory using native MCP client
//!
//! **ELIMINATES**:
//! - Orchestrator agent (usually "code") that interprets instructions
//! - Meta-agent spawning ("fetch results", "get result", etc.)
//! - Unpredictable agent count (now exactly N agents, no more)

use super::super::ChatWidget;
use super::consensus_coordinator::block_on_sync;
use super::error::{Result, SpecKitError};
use crate::history_cell::HistoryCellType;
use crate::spec_prompts::{SpecAgent, SpecStage};
use codex_core::agent_tool::{AGENT_MANAGER, AgentStatus};
use std::sync::Arc;
use std::time::Duration;

/// Execute native consensus orchestration
///
/// Spawns agents directly using AgentManager, waits for completion,
/// aggregates results, and stores to local-memory.
///
/// **Arguments**:
/// - widget: ChatWidget for state and configuration
/// - stage: SpecStage being executed
/// - spec_id: SPEC-ID being processed
/// - agents: List of agent canonical names to spawn (e.g., ["gemini", "claude"])
///
/// **Returns**: Ok(()) on success, Err on failure
pub fn execute_native_consensus(
    widget: &mut ChatWidget,
    stage: SpecStage,
    spec_id: &str,
    agents: &[String],
) -> Result<()> {
    // Display starting message
    let mut lines: Vec<ratatui::text::Line<'static>> = Vec::new();
    lines.push(ratatui::text::Line::from(format!(
        "Native consensus: {} for {} ({} agents)",
        stage.display_name(),
        spec_id,
        agents.len()
    )));
    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        lines,
        HistoryCellType::Notice,
    ));

    // 1. Read context files natively
    let context = read_context_files(widget, spec_id, stage)?;

    // 2. Build prompt from template
    let mcp_manager = block_on_sync(|| {
        let manager = widget.mcp_manager.clone();
        async move { manager.lock().await.as_ref().cloned() }
    });

    let prompt_base =
        crate::spec_prompts::build_stage_prompt_with_mcp(stage, spec_id, mcp_manager.clone())
            .map_err(|e| SpecKitError::from_string(format!("Failed to build prompt: {}", e)))?;

    // 3. Spawn agents in parallel using AgentManager
    let batch_id = uuid::Uuid::new_v4().to_string();
    let agent_ids =
        spawn_agents_natively(widget, stage, agents, &prompt_base, &context, &batch_id)?;

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Spawned {} agents: {:?}",
            agent_ids.len(),
            agents
        ))],
        HistoryCellType::Notice,
    ));

    // 4. Wait for completion (poll agent status)
    let results = wait_for_agents_natively(&agent_ids, Duration::from_secs(600))?;

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Collected {} results",
            results.len()
        ))],
        HistoryCellType::Notice,
    ));

    // 5. Aggregate results (native or single gpt_pro call)
    let synthesis = aggregate_results_natively(widget, stage, spec_id, &results)?;

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from("Synthesis complete")],
        HistoryCellType::Notice,
    ));

    // 6. Store to local-memory using native MCP client
    // DEPRECATED: Disabled - violates SPEC-KIT-072 (SQLite for consensus)
    // Pipeline_coordinator.rs:1050 handles SQLite storage properly
    // store_consensus_to_memory(widget, spec_id, stage, &synthesis, mcp_manager)?;

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Native consensus complete for {}",
            spec_id
        ))],
        HistoryCellType::Notice,
    ));

    Ok(())
}

/// Read context files natively
fn read_context_files(_widget: &ChatWidget, spec_id: &str, stage: SpecStage) -> Result<String> {
    use std::fs;
    use std::path::PathBuf;

    let mut context_parts = Vec::new();

    // Read spec.md
    let spec_path = PathBuf::from(format!("docs/SPEC-{}/spec.md", spec_id));
    if spec_path.exists() {
        if let Ok(content) = fs::read_to_string(&spec_path) {
            context_parts.push(format!("# spec.md\n{}", content));
        }
    }

    // Read plan.md for tasks/implement/validate/audit/unlock
    if stage != SpecStage::Plan {
        let plan_path = PathBuf::from(format!("docs/SPEC-{}/plan.md", spec_id));
        if plan_path.exists() {
            if let Ok(content) = fs::read_to_string(&plan_path) {
                context_parts.push(format!("# plan.md\n{}", content));
            }
        }
    }

    // Read tasks.md for implement/validate/audit/unlock
    if matches!(
        stage,
        SpecStage::Implement | SpecStage::Validate | SpecStage::Audit | SpecStage::Unlock
    ) {
        let tasks_path = PathBuf::from(format!("docs/SPEC-{}/tasks.md", spec_id));
        if tasks_path.exists() {
            if let Ok(content) = fs::read_to_string(&tasks_path) {
                context_parts.push(format!("# tasks.md\n{}", content));
            }
        }
    }

    // Read constitution
    let constitution_path = PathBuf::from("memory/constitution.md");
    if constitution_path.exists() {
        if let Ok(content) = fs::read_to_string(&constitution_path) {
            context_parts.push(format!("# constitution.md\n{}", content));
        }
    }

    Ok(context_parts.join("\n\n---\n\n"))
}

/// Spawn agents natively using AgentManager
fn spawn_agents_natively(
    widget: &ChatWidget,
    stage: SpecStage,
    agents: &[String],
    prompt_base: &str,
    context: &str,
    batch_id: &str,
) -> Result<Vec<String>> {
    let mut agent_ids = Vec::new();

    for agent_name in agents {
        let spec_agent = SpecAgent::from_string(agent_name)
            .ok_or_else(|| SpecKitError::from_string(format!("Unknown agent: {}", agent_name)))?;

        // Get agent config
        let agent_config = widget
            .config
            .agents
            .iter()
            .find(|cfg| cfg.enabled && cfg.name.eq_ignore_ascii_case(agent_name))
            .ok_or_else(|| {
                SpecKitError::from_string(format!("Agent {} not configured", agent_name))
            })?
            .clone();

        // Get agent prompt from prompts.json
        let agent_prompt =
            crate::spec_prompts::agent_prompt(stage.key(), spec_agent).ok_or_else(|| {
                SpecKitError::from_string(format!(
                    "No prompt for {} in stage {}",
                    agent_name,
                    stage.display_name()
                ))
            })?;

        // Build full prompt (agent-specific prompt + base prompt + context)
        let full_prompt = format!(
            "{}\n\n---\n\n{}\n\n---\n\nContext:\n{}",
            agent_prompt.prompt, prompt_base, context
        );

        // Spawn agent using AgentManager
        let agent_id = block_on_sync(|| {
            let model = agent_config.command.clone();
            let prompt = full_prompt.clone();
            let batch = Some(batch_id.to_string());
            let config = agent_config.clone();
            async move {
                let mut manager = AGENT_MANAGER.write().await;
                manager
                    .create_agent_with_config(
                        model,
                        prompt,
                        None,       // context (already in prompt)
                        None,       // output_goal
                        Vec::new(), // files
                        true,       // read_only
                        batch,
                        config,
                    )
                    .await
            }
        });

        agent_ids.push(agent_id);
    }

    Ok(agent_ids)
}

/// Wait for agents to complete natively (poll status)
fn wait_for_agents_natively(agent_ids: &[String], timeout: Duration) -> Result<Vec<AgentResult>> {
    let start = std::time::Instant::now();
    let poll_interval = Duration::from_millis(500);

    loop {
        // Check if timeout exceeded
        if start.elapsed() > timeout {
            return Err(SpecKitError::from_string("Agent timeout exceeded"));
        }

        // Poll agent statuses
        let statuses = block_on_sync(|| {
            let ids = agent_ids.to_vec();
            async move {
                let manager = AGENT_MANAGER.read().await;
                ids.iter()
                    .filter_map(|id| manager.get_agent(id))
                    .collect::<Vec<_>>()
            }
        });

        // Check if all agents completed
        let all_done = statuses.iter().all(|agent| {
            matches!(
                agent.status,
                AgentStatus::Completed | AgentStatus::Failed | AgentStatus::Cancelled
            )
        });

        if all_done {
            // Collect results
            let results: Vec<AgentResult> = statuses
                .into_iter()
                .map(|agent| AgentResult {
                    agent_id: agent.id.clone(),
                    model: agent.model.clone(),
                    status: agent.status.clone(),
                    result: agent.result.clone(),
                    error: agent.error.clone(),
                })
                .collect();

            return Ok(results);
        }

        // Sleep before next poll
        std::thread::sleep(poll_interval);
    }
}

/// Agent execution result
#[derive(Debug, Clone)]
struct AgentResult {
    agent_id: String,
    model: String,
    status: AgentStatus,
    result: Option<String>,
    error: Option<String>,
}

/// Aggregate results natively or with single gpt_pro call
fn aggregate_results_natively(
    widget: &ChatWidget,
    stage: SpecStage,
    spec_id: &str,
    results: &[AgentResult],
) -> Result<String> {
    // Check if all agents succeeded
    let all_success = results
        .iter()
        .all(|r| r.status == AgentStatus::Completed && r.result.is_some());

    if !all_success {
        // Build error summary
        let mut error_parts = Vec::new();
        for result in results {
            if result.status != AgentStatus::Completed || result.error.is_some() {
                error_parts.push(format!(
                    "- {}: {:?} - {}",
                    result.model,
                    result.status,
                    result.error.as_deref().unwrap_or("no error message")
                ));
            }
        }
        return Err(SpecKitError::from_string(format!(
            "Some agents failed:\n{}",
            error_parts.join("\n")
        )));
    }

    // Collect successful results
    let agent_outputs: Vec<String> = results.iter().filter_map(|r| r.result.clone()).collect();

    // Simple native aggregation: concatenate outputs
    // TODO: Use gpt_pro for synthesis if needed
    let synthesis = format!(
        "# Consensus Synthesis for {} ({})\n\n## Agent Outputs\n\n{}",
        spec_id,
        stage.display_name(),
        agent_outputs.join("\n\n---\n\n")
    );

    // Optional: Use gpt_pro for synthesis
    if widget
        .config
        .agents
        .iter()
        .any(|cfg| cfg.enabled && cfg.name.eq_ignore_ascii_case("gpt_pro"))
    {
        // TODO: Spawn gpt_pro agent to synthesize results
        // For now, use simple concatenation
    }

    Ok(synthesis)
}

/// Store consensus to SQLite (SPEC-934)
///
/// Replaces MCP local-memory storage with SQLite consensus_db.
fn store_consensus_to_memory(
    widget: &ChatWidget,
    spec_id: &str,
    stage: SpecStage,
    synthesis: &str,
    _mcp_manager: Option<Arc<codex_core::mcp_connection_manager::McpConnectionManager>>,
) -> Result<()> {
    // SPEC-934: Store to SQLite instead of MCP local-memory
    let db = super::consensus_db::ConsensusDb::init_default().map_err(|e| {
        SpecKitError::from_string(format!("Failed to initialize consensus DB: {}", e))
    })?;

    db.store_synthesis(
        spec_id, stage, synthesis,
        None,      // output_path (not written to file for native consensus)
        "success", // status
        1,         // artifacts_count (native synthesis is single artifact)
        None,      // agreements
        None,      // conflicts
        false,     // degraded
        None,      // run_id
    )
    .map_err(|e| {
        SpecKitError::from_string(format!("Failed to store consensus synthesis: {}", e))
    })?;

    tracing::debug!(
        "Stored native consensus synthesis to SQLite: spec={}, stage={}",
        spec_id,
        stage.command_name()
    );

    Ok(())
}
