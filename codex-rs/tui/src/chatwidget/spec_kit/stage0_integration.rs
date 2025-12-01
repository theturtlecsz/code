//! Stage 0 integration for spec-kit pipeline
//!
//! SPEC-KIT-102: Stage 0 context injection for /speckit.auto
//!
//! This module handles:
//! - Running Stage0Engine before the main pipeline
//! - Creating adapters from MCP connections
//! - Injecting Divine Truth + TASK_BRIEF into agent prompts

use crate::stage0_adapters::{
    create_stage0_adapters, has_local_memory_server, has_notebooklm_server,
    LocalMemoryMcpAdapter, LlmStubAdapter, NoopTier2Client, Tier2McpAdapter,
};
use codex_core::mcp_connection_manager::McpConnectionManager;
use codex_stage0::dcc::EnvCtx;
use codex_stage0::Stage0Engine;
use std::path::Path;
use std::sync::Arc;

/// Result of Stage 0 execution for pipeline consumption
#[derive(Debug, Clone)]
pub struct Stage0ExecutionResult {
    /// Stage0Result if successful
    pub result: Option<codex_stage0::Stage0Result>,
    /// Skip reason if Stage 0 didn't run
    pub skip_reason: Option<String>,
    /// Execution time in milliseconds
    pub duration_ms: u64,
    /// Whether Tier 2 (NotebookLM) was used
    pub tier2_used: bool,
    /// Whether cache was hit
    pub cache_hit: bool,
}

/// Configuration for Stage 0 execution
#[derive(Debug, Clone, Default)]
pub struct Stage0ExecutionConfig {
    /// Disable Stage 0 entirely
    pub disabled: bool,
    /// Include score breakdown in TASK_BRIEF
    pub explain: bool,
}

/// Run Stage 0 context injection for a spec
///
/// This is called synchronously from handle_spec_auto before the pipeline starts.
/// Uses block_on_sync internally to run async Stage0 code.
pub fn run_stage0_for_spec(
    mcp_manager: &Arc<tokio::sync::Mutex<Option<Arc<McpConnectionManager>>>>,
    spec_id: &str,
    spec_content: &str,
    cwd: &Path,
    config: &Stage0ExecutionConfig,
) -> Stage0ExecutionResult {
    // Check if disabled
    if config.disabled {
        return Stage0ExecutionResult {
            result: None,
            skip_reason: Some("Stage 0 disabled by configuration".to_string()),
            duration_ms: 0,
            tier2_used: false,
            cache_hit: false,
        };
    }

    let start = std::time::Instant::now();

    // Try to get MCP manager synchronously
    let mcp_manager_clone = mcp_manager.clone();
    let mcp_opt = super::consensus_coordinator::block_on_sync(|| async move {
        mcp_manager_clone.lock().await.clone()
    });

    let Some(mcp) = mcp_opt else {
        return Stage0ExecutionResult {
            result: None,
            skip_reason: Some("MCP manager not available".to_string()),
            duration_ms: start.elapsed().as_millis() as u64,
            tier2_used: false,
            cache_hit: false,
        };
    };

    // Check for required MCP servers
    if !has_local_memory_server(&mcp) {
        return Stage0ExecutionResult {
            result: None,
            skip_reason: Some("local-memory MCP server not available".to_string()),
            duration_ms: start.elapsed().as_millis() as u64,
            tier2_used: false,
            cache_hit: false,
        };
    }

    let _tier2_available = has_notebooklm_server(&mcp);

    // Create adapters
    let (local_memory_opt, llm, tier2_opt) = create_stage0_adapters(mcp);

    let Some(local_memory) = local_memory_opt else {
        return Stage0ExecutionResult {
            result: None,
            skip_reason: Some("Failed to create local-memory adapter".to_string()),
            duration_ms: start.elapsed().as_millis() as u64,
            tier2_used: false,
            cache_hit: false,
        };
    };

    // Build environment context
    let env = EnvCtx {
        cwd: cwd.to_string_lossy().to_string(),
        branch: Some(get_git_branch(cwd)),
        recent_files: get_recent_files(cwd),
    };

    // Run Stage 0 engine
    // Note: Stage0Engine contains rusqlite::Connection which is not Send,
    // so we need to run everything in a dedicated single-threaded runtime
    let stage0_result = run_stage0_blocking(
        spec_id.to_string(),
        spec_content.to_string(),
        env,
        local_memory,
        llm,
        tier2_opt,
        config.explain,
    );

    let duration_ms = start.elapsed().as_millis() as u64;

    match stage0_result {
        Ok(result) => {
            let tier2_used = result.tier2_used;
            let cache_hit = result.cache_hit;

            tracing::info!(
                "Stage 0 completed for {}: tier2={}, cache_hit={}, duration={}ms",
                spec_id,
                tier2_used,
                cache_hit,
                duration_ms
            );

            Stage0ExecutionResult {
                result: Some(result),
                skip_reason: None,
                duration_ms,
                tier2_used,
                cache_hit,
            }
        }
        Err(e) => {
            tracing::warn!("Stage 0 failed for {}: {}", spec_id, e);
            Stage0ExecutionResult {
                result: None,
                skip_reason: Some(format!("Stage 0 error: {e}")),
                duration_ms,
                tier2_used: false,
                cache_hit: false,
            }
        }
    }
}

/// Blocking implementation of Stage 0 execution
///
/// Uses a dedicated single-threaded runtime because Stage0Engine
/// contains rusqlite::Connection which is not Send/Sync.
fn run_stage0_blocking(
    spec_id: String,
    spec_content: String,
    env: EnvCtx,
    local_memory: LocalMemoryMcpAdapter,
    llm: LlmStubAdapter,
    tier2: Option<Tier2McpAdapter>,
    explain: bool,
) -> Result<codex_stage0::Stage0Result, String> {
    // Create a dedicated runtime for Stage0 (single-threaded to avoid Send requirements)
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create Stage0 runtime: {e}"))?;

    rt.block_on(async {
        // Create Stage0Engine inside the async block
        let engine =
            Stage0Engine::new().map_err(|e| format!("Failed to create Stage0Engine: {e}"))?;

        // Run Stage 0 with appropriate Tier2 client
        if let Some(tier2_client) = tier2 {
            // Use real NotebookLM adapter
            engine
                .run_stage0(
                    &local_memory,
                    &llm,
                    &tier2_client,
                    &spec_id,
                    &spec_content,
                    &env,
                    explain,
                )
                .await
                .map_err(|e| format!("Stage 0 execution failed: {e}"))
        } else {
            // Use noop Tier2 client (will trigger fallback to Tier1-only mode)
            let noop_tier2 = NoopTier2Client::new();
            engine
                .run_stage0(
                    &local_memory,
                    &llm,
                    &noop_tier2,
                    &spec_id,
                    &spec_content,
                    &env,
                    explain,
                )
                .await
                .map_err(|e| format!("Stage 0 execution failed: {e}"))
        }
    })
}

/// Get current git branch
fn get_git_branch(cwd: &Path) -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(cwd)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "main".to_string())
}

/// Get recently modified files in the cwd
fn get_recent_files(cwd: &Path) -> Vec<String> {
    std::process::Command::new("git")
        .args(["ls-files", "-m"])
        .current_dir(cwd)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.lines().take(20).map(String::from).collect())
        .unwrap_or_default()
}

/// Build context injection prefix for agent prompts
///
/// Returns a string to prepend to agent system prompts containing
/// Divine Truth and Task Brief from Stage 0.
pub fn build_stage0_context_prefix(stage0_result: &codex_stage0::Stage0Result) -> String {
    // Use the built-in combined_context_md() helper
    stage0_result.combined_context_md()
}

/// Write TASK_BRIEF.md to spec evidence directory
pub fn write_task_brief_to_evidence(
    spec_id: &str,
    cwd: &Path,
    task_brief: &str,
) -> std::io::Result<std::path::PathBuf> {
    let evidence_dir = cwd.join("docs").join(spec_id).join("evidence");
    std::fs::create_dir_all(&evidence_dir)?;

    let path = evidence_dir.join("TASK_BRIEF.md");
    std::fs::write(&path, task_brief)?;

    tracing::debug!("Wrote TASK_BRIEF.md to {}", path.display());
    Ok(path)
}

/// SPEC-KIT-102: Write DIVINE_TRUTH.md to evidence directory
/// Contains the Tier 2 (NotebookLM) synthesis - high-level guidance, risks, and framing
pub fn write_divine_truth_to_evidence(
    spec_id: &str,
    cwd: &Path,
    divine_truth: &str,
) -> std::io::Result<std::path::PathBuf> {
    let evidence_dir = cwd.join("docs").join(spec_id).join("evidence");
    std::fs::create_dir_all(&evidence_dir)?;

    let path = evidence_dir.join("DIVINE_TRUTH.md");
    std::fs::write(&path, divine_truth)?;

    tracing::debug!("Wrote DIVINE_TRUTH.md to {}", path.display());
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage0_config_default() {
        let config = Stage0ExecutionConfig::default();
        assert!(!config.disabled);
        assert!(!config.explain);
    }

    #[test]
    fn test_get_git_branch_fallback() {
        // Test with non-existent path
        let branch = get_git_branch(std::path::Path::new("/nonexistent"));
        assert_eq!(branch, "main");
    }
}
