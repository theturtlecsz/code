//! Stage 0 integration for spec-kit pipeline
//!
//! SPEC-KIT-102: Stage 0 context injection for /speckit.auto
//!
//! This module handles:
//! - Running Stage0Engine before the main pipeline
//! - Creating adapters from local services (CLI/REST + HTTP)
//! - Injecting Divine Truth + TASK_BRIEF into agent prompts
//! - V2.5b: Hybrid retrieval using shared TfIdfBackend

use crate::stage0_adapters::{
    LlmStubAdapter, LocalMemoryCliAdapter, NoopTier2Client, Tier2HttpAdapter,
};
use crate::vector_state::VECTOR_STATE;
use codex_stage0::Stage0Engine;
use codex_stage0::dcc::EnvCtx;
use std::path::Path;
use std::time::Duration;

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
    /// V2.5b: Whether hybrid retrieval was used (TfIdfBackend available)
    pub hybrid_retrieval_used: bool,
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
    planner_config: &codex_core::config::Config,
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
            hybrid_retrieval_used: false,
        };
    }

    let start = std::time::Instant::now();

    if !crate::local_memory_cli::local_memory_daemon_healthy_blocking(Duration::from_millis(750)) {
        return Stage0ExecutionResult {
            result: None,
            skip_reason: Some(
                "local-memory daemon not available at http://localhost:3002".to_string(),
            ),
            duration_ms: start.elapsed().as_millis() as u64,
            tier2_used: false,
            cache_hit: false,
            hybrid_retrieval_used: false,
        };
    }

    // Load Stage0Config and apply per-project Tier2 overrides from Planner config.
    let mut stage0_cfg = match codex_stage0::Stage0Config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            return Stage0ExecutionResult {
                result: None,
                skip_reason: Some(format!("Failed to load Stage0 config: {e}")),
                duration_ms: start.elapsed().as_millis() as u64,
                tier2_used: false,
                cache_hit: false,
                hybrid_retrieval_used: false,
            };
        }
    };

    let (tier2_notebook, tier2_base_url) = resolve_tier2_overrides(planner_config);
    if let Some(notebook) = tier2_notebook.clone() {
        stage0_cfg.tier2.enabled = true;
        stage0_cfg.tier2.notebook = notebook;
    } else if stage0_cfg.tier2.notebook.trim().is_empty() {
        stage0_cfg.tier2.enabled = false;
    }
    if let Some(base_url) = tier2_base_url {
        stage0_cfg.tier2.base_url = Some(base_url);
    }

    // Create adapters (no MCP dependencies).
    let local_memory = LocalMemoryCliAdapter::new();
    let llm = LlmStubAdapter::new();

    // CONVERGENCE: Tier2 fail-closed with explicit diagnostics
    // Per MEMO_codex-rs.md Section 1: "emit diagnostics with actionable next steps"
    let tier2_opt = if stage0_cfg.tier2.enabled && !stage0_cfg.tier2.notebook.trim().is_empty() {
        // Check NotebookLM service health before creating adapter
        let base_url = stage0_cfg
            .tier2
            .base_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:3456".to_string());

        match check_tier2_service_health(&base_url) {
            Ok(()) => Some(Tier2HttpAdapter::new(base_url, stage0_cfg.tier2.notebook.clone())),
            Err(reason) => {
                // Tier2 fail-closed: skip with diagnostic
                tracing::warn!(
                    "Stage0 Tier2 skipped: {}. Run 'code doctor' for details.",
                    reason
                );
                None
            }
        }
    } else {
        // No notebook configured - emit diagnostic
        if stage0_cfg.tier2.enabled {
            tracing::info!(
                "Stage0 Tier2 skipped: No notebook configured. Add tier2.notebook to stage0.toml"
            );
        }
        None
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
    let (stage0_result, hybrid_used) = run_stage0_blocking(
        spec_id.to_string(),
        spec_content.to_string(),
        env,
        local_memory,
        llm,
        stage0_cfg,
        tier2_opt,
        config.explain,
    );

    let duration_ms = start.elapsed().as_millis() as u64;

    match stage0_result {
        Ok(result) => {
            let tier2_used = result.tier2_used;
            let cache_hit = result.cache_hit;

            tracing::info!(
                "Stage 0 completed for {}: tier2={}, cache_hit={}, hybrid={}, duration={}ms",
                spec_id,
                tier2_used,
                cache_hit,
                hybrid_used,
                duration_ms
            );

            Stage0ExecutionResult {
                result: Some(result),
                skip_reason: None,
                duration_ms,
                tier2_used,
                cache_hit,
                hybrid_retrieval_used: hybrid_used,
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
                hybrid_retrieval_used: false,
            }
        }
    }
}

/// Blocking implementation of Stage 0 execution
///
/// Uses a dedicated single-threaded runtime because Stage0Engine
/// contains rusqlite::Connection which is not Send/Sync.
///
/// V2.5b: Returns (result, hybrid_used) tuple. Uses shared VECTOR_STATE
/// if available for hybrid retrieval.
fn run_stage0_blocking(
    spec_id: String,
    spec_content: String,
    env: EnvCtx,
    local_memory: LocalMemoryCliAdapter,
    llm: LlmStubAdapter,
    stage0_cfg: codex_stage0::Stage0Config,
    tier2: Option<Tier2HttpAdapter>,
    explain: bool,
) -> (Result<codex_stage0::Stage0Result, String>, bool) {
    // Create a dedicated runtime for Stage0 (single-threaded to avoid Send requirements)
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return (Err(format!("Failed to create Stage0 runtime: {e}")), false),
    };

    rt.block_on(async {
        // Create Stage0Engine inside the async block
        let engine = match Stage0Engine::with_config(stage0_cfg) {
            Ok(e) => e,
            Err(e) => return (Err(format!("Failed to create Stage0Engine: {e}")), false),
        };

        // V2.5b: Check if shared TfIdfBackend is available
        let backend_handle = VECTOR_STATE.backend_handle();
        let backend_lock = backend_handle.read().await;

        if backend_lock.is_some() {
            // Use shared TfIdfBackend for hybrid retrieval
            tracing::debug!("Using shared TfIdfBackend for hybrid retrieval");
            drop(backend_lock);

            // Re-acquire lock for the actual operation
            let backend_handle = VECTOR_STATE.backend_handle();
            let backend_lock = backend_handle.read().await;
            let backend_ref = backend_lock.as_ref();

            if let Some(backend) = backend_ref {
                let result = if let Some(tier2_client) = tier2 {
                    engine
                        .run_stage0(
                            &local_memory,
                            &llm,
                            Some(backend),
                            &tier2_client,
                            &spec_id,
                            &spec_content,
                            &env,
                            explain,
                        )
                        .await
                        .map_err(|e| format!("Stage 0 execution failed: {e}"))
                } else {
                    let noop_tier2 = NoopTier2Client::new();
                    engine
                        .run_stage0(
                            &local_memory,
                            &llm,
                            Some(backend),
                            &noop_tier2,
                            &spec_id,
                            &spec_content,
                            &env,
                            explain,
                        )
                        .await
                        .map_err(|e| format!("Stage 0 execution failed: {e}"))
                };
                (result, true)
            } else {
                // Backend disappeared, fall back to noop
                run_without_vector(
                    &engine,
                    &local_memory,
                    &llm,
                    tier2,
                    &spec_id,
                    &spec_content,
                    &env,
                    explain,
                )
                .await
            }
        } else {
            // No backend available, run without hybrid retrieval
            drop(backend_lock);
            tracing::debug!("No TfIdfBackend available, running without hybrid retrieval");
            run_without_vector(
                &engine,
                &local_memory,
                &llm,
                tier2,
                &spec_id,
                &spec_content,
                &env,
                explain,
            )
            .await
        }
    })
}

/// Helper to run Stage0 without vector backend
async fn run_without_vector(
    engine: &Stage0Engine,
    local_memory: &LocalMemoryCliAdapter,
    llm: &LlmStubAdapter,
    tier2: Option<Tier2HttpAdapter>,
    spec_id: &str,
    spec_content: &str,
    env: &EnvCtx,
    explain: bool,
) -> (Result<codex_stage0::Stage0Result, String>, bool) {
    let noop_vector: Option<&codex_stage0::NoopVectorBackend> = None;

    let result = if let Some(tier2_client) = tier2 {
        engine
            .run_stage0(
                local_memory,
                llm,
                noop_vector,
                &tier2_client,
                spec_id,
                spec_content,
                env,
                explain,
            )
            .await
            .map_err(|e| format!("Stage 0 execution failed: {e}"))
    } else {
        let noop_tier2 = NoopTier2Client::new();
        engine
            .run_stage0(
                local_memory,
                llm,
                noop_vector,
                &noop_tier2,
                spec_id,
                spec_content,
                env,
                explain,
            )
            .await
            .map_err(|e| format!("Stage 0 execution failed: {e}"))
    };

    (result, false)
}

fn resolve_tier2_overrides(
    planner_config: &codex_core::config::Config,
) -> (Option<String>, Option<String>) {
    let tier2 = &planner_config.stage0;
    let notebook = tier2
        .notebook
        .clone()
        .or(tier2.notebook_url.clone())
        .or(tier2.notebook_id.clone())
        .filter(|v| !v.trim().is_empty());
    (notebook, tier2.notebooklm_base_url.clone())
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

/// CONVERGENCE: Check NotebookLM service health before attempting Tier2
///
/// Returns Ok(()) if service is healthy, Err(reason) if not.
/// Per MEMO_codex-rs.md: fail-closed means skip Tier2, not fail the pipeline.
fn check_tier2_service_health(base_url: &str) -> Result<(), String> {
    let health_url = format!("{}/health", base_url.trim_end_matches('/'));

    // Use blocking client since we're in sync context
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    match client.get(&health_url).send() {
        Ok(resp) if resp.status().is_success() => Ok(()),
        Ok(resp) => Err(format!("NotebookLM service unhealthy: {}", resp.status())),
        Err(e) if e.is_timeout() => Err("NotebookLM service timeout".to_string()),
        Err(e) if e.is_connect() => Err("NotebookLM service not running".to_string()),
        Err(e) => Err(format!("NotebookLM service unreachable: {e}")),
    }
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
