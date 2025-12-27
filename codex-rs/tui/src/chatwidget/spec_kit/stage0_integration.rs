//! Stage 0 integration for spec-kit pipeline
//!
//! SPEC-KIT-102: Stage 0 context injection for /speckit.auto
//!
//! This module handles:
//! - Running Stage0Engine before the main pipeline
//! - Creating adapters from local services (CLI/REST + HTTP)
//! - Injecting Divine Truth + TASK_BRIEF into agent prompts
//! - V2.5b: Hybrid retrieval using shared TfIdfBackend
//! - SPEC-DOGFOOD-001 S30: Progress callbacks for UX feedback

use crate::stage0_adapters::{
    LlmStubAdapter, LocalMemoryCliAdapter, NoopTier2Client, Tier2HttpAdapter,
};
use crate::vector_state::VECTOR_STATE;
use codex_stage0::Stage0Engine;
use codex_stage0::dcc::EnvCtx;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

/// Stage0 progress updates for UX feedback (SPEC-DOGFOOD-001 S30)
#[derive(Debug, Clone)]
pub enum Stage0Progress {
    /// Starting Stage0 execution
    Starting,
    /// Checking local-memory daemon health
    CheckingLocalMemory,
    /// Loading Stage0 configuration
    LoadingConfig,
    /// Checking Tier2 (NotebookLM) health
    CheckingTier2Health,
    /// Compiling DCC context
    CompilingContext,
    /// Querying Tier2 (NotebookLM)
    QueryingTier2,
    /// Tier2 query completed (with duration in ms)
    Tier2Complete(u64),
    /// Finished with result summary
    Finished {
        success: bool,
        tier2_used: bool,
        duration_ms: u64,
    },
}

/// Channel sender for Stage0 progress updates
pub type Stage0ProgressSender = mpsc::Sender<Stage0Progress>;

/// Pending Stage0 operation for async execution (SPEC-DOGFOOD-001 S30)
///
/// This allows Stage0 to run in the background while the TUI remains responsive.
pub struct Stage0PendingOperation {
    /// Receiver for progress updates
    pub progress_rx: mpsc::Receiver<Stage0Progress>,
    /// Receiver for final result
    pub result_rx: mpsc::Receiver<Stage0ExecutionResult>,
    /// Spec ID being processed
    pub spec_id: String,
    /// Spec content
    pub spec_content: String,
    /// Stage0 execution config
    pub config: Stage0ExecutionConfig,
}

/// Spawn Stage0 in a background thread and return channels for polling.
///
/// SPEC-DOGFOOD-001 S30: This allows the TUI to remain responsive while Stage0 runs.
/// Call `poll_stage0_progress` periodically to check for updates.
pub fn spawn_stage0_async(
    planner_config: codex_core::config::Config,
    spec_id: String,
    spec_content: String,
    cwd: std::path::PathBuf,
    config: Stage0ExecutionConfig,
) -> Stage0PendingOperation {
    let (progress_tx, progress_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();

    let spec_id_clone = spec_id.clone();
    let spec_content_clone = spec_content.clone();
    let config_clone = config.clone();

    std::thread::spawn(move || {
        let result = run_stage0_for_spec(
            &planner_config,
            &spec_id_clone,
            &spec_content_clone,
            &cwd,
            &config_clone,
            Some(progress_tx),
        );

        // S33: Trace before sending result over channel
        {
            use std::io::Write;
            let trace_msg = format!(
                "[{}] Stage0 ASYNC RESULT: tier2={}, has_result={}, sending to channel...\n",
                chrono::Utc::now().format("%H:%M:%S%.3f"),
                result.tier2_used,
                result.result.is_some(),
            );
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/speckit-trace.log")
            {
                let _ = f.write_all(trace_msg.as_bytes());
            }
        }

        match result_tx.send(result) {
            Ok(_) => {
                use std::io::Write;
                let trace_msg = format!(
                    "[{}] Stage0 CHANNEL SEND: success\n",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                );
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/speckit-trace.log")
                {
                    let _ = f.write_all(trace_msg.as_bytes());
                }
            }
            Err(_) => {
                use std::io::Write;
                let trace_msg = format!(
                    "[{}] Stage0 CHANNEL SEND: FAILED (receiver dropped)\n",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                );
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/speckit-trace.log")
                {
                    let _ = f.write_all(trace_msg.as_bytes());
                }
            }
        }
    });

    Stage0PendingOperation {
        progress_rx,
        result_rx,
        spec_id,
        spec_content,
        config,
    }
}

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
    /// CONVERGENCE: Tier2 skip reason (for diagnostics and pointer memory)
    pub tier2_skip_reason: Option<String>,
}

/// Configuration for Stage 0 execution
#[derive(Debug, Clone, Default)]
pub struct Stage0ExecutionConfig {
    /// Disable Stage 0 entirely
    pub disabled: bool,
    /// Include score breakdown in TASK_BRIEF
    pub explain: bool,
}

/// Helper to send progress update (ignores send failures)
fn send_progress(tx: &Option<Stage0ProgressSender>, progress: Stage0Progress) {
    if let Some(sender) = tx {
        let _ = sender.send(progress);
    }
}

/// Run Stage 0 context injection for a spec
///
/// This is called synchronously from handle_spec_auto before the pipeline starts.
/// Uses block_on_sync internally to run async Stage0 code.
///
/// SPEC-DOGFOOD-001 S30: Added optional progress_tx for UX feedback during execution.
pub fn run_stage0_for_spec(
    planner_config: &codex_core::config::Config,
    spec_id: &str,
    spec_content: &str,
    cwd: &Path,
    config: &Stage0ExecutionConfig,
    progress_tx: Option<Stage0ProgressSender>,
) -> Stage0ExecutionResult {
    send_progress(&progress_tx, Stage0Progress::Starting);

    // Check if disabled
    if config.disabled {
        send_progress(
            &progress_tx,
            Stage0Progress::Finished {
                success: false,
                tier2_used: false,
                duration_ms: 0,
            },
        );
        return Stage0ExecutionResult {
            result: None,
            skip_reason: Some("Stage 0 disabled by configuration".to_string()),
            duration_ms: 0,
            tier2_used: false,
            cache_hit: false,
            hybrid_retrieval_used: false,
            tier2_skip_reason: Some("Stage0 disabled".to_string()),
        };
    }

    let start = std::time::Instant::now();

    send_progress(&progress_tx, Stage0Progress::CheckingLocalMemory);
    if !crate::local_memory_cli::local_memory_daemon_healthy_blocking(Duration::from_millis(750)) {
        let duration_ms = start.elapsed().as_millis() as u64;
        send_progress(
            &progress_tx,
            Stage0Progress::Finished {
                success: false,
                tier2_used: false,
                duration_ms,
            },
        );
        return Stage0ExecutionResult {
            result: None,
            skip_reason: Some(
                "local-memory daemon not available at http://localhost:3002".to_string(),
            ),
            duration_ms,
            tier2_used: false,
            cache_hit: false,
            hybrid_retrieval_used: false,
            tier2_skip_reason: Some("local-memory unavailable".to_string()),
        };
    }

    send_progress(&progress_tx, Stage0Progress::LoadingConfig);
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
                tier2_skip_reason: Some("config load failed".to_string()),
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
    let (tier2_opt, tier2_skip_reason) = if stage0_cfg.tier2.enabled
        && !stage0_cfg.tier2.notebook.trim().is_empty()
    {
        // Check NotebookLM service health before creating adapter
        send_progress(&progress_tx, Stage0Progress::CheckingTier2Health);
        let base_url = stage0_cfg
            .tier2
            .base_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:3456".to_string());

        match check_tier2_service_health(&base_url) {
            Ok(()) => (
                Some(Tier2HttpAdapter::new(
                    base_url,
                    stage0_cfg.tier2.notebook.clone(),
                )),
                None,
            ),
            Err(reason) => {
                // Tier2 fail-closed: skip with diagnostic
                tracing::warn!(
                    "Stage0 Tier2 skipped: {}. Run 'code doctor' for details.",
                    reason
                );
                (None, Some(reason))
            }
        }
    } else {
        // No notebook configured - emit diagnostic
        let reason = if stage0_cfg.tier2.enabled {
            let msg = "No notebook configured".to_string();
            tracing::info!(
                "Stage0 Tier2 skipped: {}. Add tier2.notebook to stage0.toml",
                msg
            );
            Some(msg)
        } else {
            Some("Tier2 disabled".to_string())
        };
        (None, reason)
    };

    // Build environment context
    let env = EnvCtx {
        cwd: cwd.to_string_lossy().to_string(),
        branch: Some(get_git_branch(cwd)),
        recent_files: get_recent_files(cwd),
    };

    // Send progress: Querying Tier2 if enabled, otherwise just compiling context
    if tier2_opt.is_some() {
        send_progress(&progress_tx, Stage0Progress::QueryingTier2);
    } else {
        send_progress(&progress_tx, Stage0Progress::CompilingContext);
    }

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

            // Send final progress
            send_progress(
                &progress_tx,
                Stage0Progress::Finished {
                    success: true,
                    tier2_used,
                    duration_ms,
                },
            );

            // If tier2 was used, clear the skip reason
            let final_tier2_skip = if tier2_used {
                None
            } else {
                tier2_skip_reason
            };

            Stage0ExecutionResult {
                result: Some(result),
                skip_reason: None,
                duration_ms,
                tier2_used,
                cache_hit,
                hybrid_retrieval_used: hybrid_used,
                tier2_skip_reason: final_tier2_skip,
            }
        }
        Err(e) => {
            tracing::warn!("Stage 0 failed for {}: {}", spec_id, e);

            // Send final progress
            send_progress(
                &progress_tx,
                Stage0Progress::Finished {
                    success: false,
                    tier2_used: false,
                    duration_ms,
                },
            );

            Stage0ExecutionResult {
                result: None,
                skip_reason: Some(format!("Stage 0 error: {e}")),
                duration_ms,
                tier2_used: false,
                cache_hit: false,
                hybrid_retrieval_used: false,
                tier2_skip_reason: tier2_skip_reason.or(Some("Stage0 error".to_string())),
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

    // FILE-BASED TRACE: Tier2 health check (SPEC-DOGFOOD-001 S29)
    {
        use std::io::Write;
        let trace_msg = format!(
            "[{}] Tier2 HEALTH CHECK: url={}\n",
            chrono::Utc::now().format("%H:%M:%S%.3f"),
            health_url
        );
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/speckit-trace.log")
        {
            let _ = f.write_all(trace_msg.as_bytes());
        }
    }

    // SPEC-KIT-900 FIX: Use block_in_place to allow blocking reqwest calls
    // within an async tokio context.
    let result = tokio::task::block_in_place(|| {
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
    });

    // FILE-BASED TRACE: Health check result
    {
        use std::io::Write;
        let trace_msg = format!(
            "[{}] Tier2 HEALTH RESULT: {:?}\n",
            chrono::Utc::now().format("%H:%M:%S%.3f"),
            result
        );
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/speckit-trace.log")
        {
            let _ = f.write_all(trace_msg.as_bytes());
        }
    }

    result
}

/// CONVERGENCE: Store Stage0 system pointer memory after artifacts are written
///
/// This is a best-effort operation that logs errors but never fails.
/// Call this after `write_task_brief_to_evidence` and `write_divine_truth_to_evidence`.
///
/// # Arguments
/// * `spec_id` - SPEC identifier
/// * `execution_result` - The Stage0ExecutionResult from run_stage0_for_spec
/// * `task_brief_path` - Path where TASK_BRIEF.md was written
/// * `divine_truth_path` - Path where DIVINE_TRUTH.md was written (if applicable)
/// * `notebook_id` - Optional NotebookLM notebook ID used for Tier2
pub fn store_stage0_system_pointer(
    spec_id: &str,
    execution_result: &Stage0ExecutionResult,
    task_brief_path: Option<&std::path::Path>,
    divine_truth_path: Option<&std::path::Path>,
    notebook_id: Option<&str>,
) {
    // FILE-BASED TRACE: System pointer storage entry (SPEC-DOGFOOD-001 S29)
    {
        use std::io::Write;
        let trace_msg = format!(
            "[{}] SYSTEM POINTER: entry for spec_id={}, has_result={}\n",
            chrono::Utc::now().format("%H:%M:%S%.3f"),
            spec_id,
            execution_result.result.is_some()
        );
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/speckit-trace.log")
        {
            let _ = f.write_all(trace_msg.as_bytes());
        }
    }

    // Check if we should store system pointers (from Stage0Config)
    let store_enabled = match codex_stage0::Stage0Config::load() {
        Ok(cfg) => cfg.store_system_pointers,
        Err(_) => true, // Default to enabled if config fails
    };

    if !store_enabled {
        // FILE-BASED TRACE
        {
            use std::io::Write;
            let trace_msg = format!(
                "[{}] SYSTEM POINTER: disabled in config\n",
                chrono::Utc::now().format("%H:%M:%S%.3f")
            );
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/speckit-trace.log")
            {
                let _ = f.write_all(trace_msg.as_bytes());
            }
        }
        tracing::debug!(
            spec_id = spec_id,
            "System pointer storage disabled in config"
        );
        return;
    }

    // Skip if no result (Stage0 didn't run)
    let result = match &execution_result.result {
        Some(r) => r,
        None => {
            // FILE-BASED TRACE
            {
                use std::io::Write;
                let trace_msg = format!(
                    "[{}] SYSTEM POINTER: no Stage0 result, skipping\n",
                    chrono::Utc::now().format("%H:%M:%S%.3f")
                );
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/speckit-trace.log")
                {
                    let _ = f.write_all(trace_msg.as_bytes());
                }
            }
            tracing::debug!(
                spec_id = spec_id,
                "Skipping system pointer: no Stage0 result"
            );
            return;
        }
    };

    // Build Tier2 status
    let tier2_status = if execution_result.tier2_used {
        codex_stage0::Tier2Status::Success
    } else if let Some(ref reason) = execution_result.tier2_skip_reason {
        codex_stage0::Tier2Status::Skipped(reason.clone())
    } else {
        codex_stage0::Tier2Status::Skipped("unknown".to_string())
    };

    // Compute hashes
    let task_brief_hash = codex_stage0::compute_content_hash(&result.task_brief_md);
    let divine_truth_hash = if execution_result.tier2_used {
        Some(codex_stage0::compute_content_hash(
            &result.divine_truth.raw_markdown,
        ))
    } else {
        None
    };

    // Extract summary bullets from divine truth
    let summary_bullets =
        codex_stage0::extract_summary_bullets(&result.divine_truth.raw_markdown, 5);

    // Get current git commit SHA
    let commit_sha = get_git_commit_sha();

    // Build pointer info
    let info = codex_stage0::Stage0PointerInfo {
        spec_id: spec_id.to_string(),
        task_brief_path: task_brief_path.map(|p| p.to_string_lossy().to_string()),
        divine_truth_path: divine_truth_path.map(|p| p.to_string_lossy().to_string()),
        task_brief_hash,
        divine_truth_hash,
        summary_bullets,
        tier2_status,
        notebook_id: notebook_id.map(|s| s.to_string()),
        commit_sha,
    };

    // Store pointer in background (non-blocking)
    let api_base = "http://localhost:3002/api/v1".to_string();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::warn!(
                    spec_id = %info.spec_id,
                    error = %e,
                    "Failed to create runtime for system pointer storage"
                );
                return;
            }
        };

        rt.block_on(async {
            match codex_stage0::store_stage0_pointer(&api_base, &info).await {
                Some(id) => tracing::info!(
                    spec_id = %info.spec_id,
                    memory_id = %id,
                    "System pointer memory stored"
                ),
                None => tracing::debug!(
                    spec_id = %info.spec_id,
                    "System pointer storage completed (best-effort)"
                ),
            }
        });
    });
}

/// Get current git commit SHA (short form)
fn get_git_commit_sha() -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
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
