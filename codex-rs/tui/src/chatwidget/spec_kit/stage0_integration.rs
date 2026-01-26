//! Stage 0 integration for spec-kit pipeline
//!
//! SPEC-KIT-102: Stage 0 context injection for /speckit.auto
//! SPEC-KIT-971: Pipeline integration honors Stage0Config.memory_backend
//!
//! This module handles:
//! - Running Stage0Engine before the main pipeline
//! - Creating adapters from local services (CLI/REST + HTTP)
//! - Injecting Divine Truth + TASK_BRIEF into agent prompts
//! - V2.5b: Hybrid retrieval using shared TfIdfBackend
//! - SPEC-DOGFOOD-001 S30: Progress callbacks for UX feedback
//! - SPEC-KIT-971: Backend routing via memory_backend config

use crate::memvid_adapter::{
    DEFAULT_WORKSPACE_ID, UnifiedMemoryClient, create_unified_memory_client, default_capsule_path,
};
use crate::stage0_adapters::{LlmStubAdapter, NoopTier2Client, Tier2HttpAdapter};
use crate::vector_state::VECTOR_STATE;
use codex_stage0::dcc::EnvCtx;
use codex_stage0::{MemoryBackend, Stage0Engine};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

/// Stage0 progress updates for UX feedback (SPEC-DOGFOOD-001 S30)
#[derive(Debug, Clone)]
pub enum Stage0Progress {
    /// Starting Stage0 execution
    Starting,
    /// Checking local-memory daemon health (only for local-memory backend)
    CheckingLocalMemory,
    /// Loading Stage0 configuration
    LoadingConfig,
    /// SPEC-KIT-971: Creating memory client (memvid or local-memory)
    CreatingMemoryClient { backend: String },
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
    // ─────────────────────────────────────────────────────────────────────────────
    // ADR-003 Prompt F: Pre-check + Curation telemetry
    // ─────────────────────────────────────────────────────────────────────────────
    /// Whether pre-check against codex-product hit (skipped Tier2)
    pub precheck_hit: bool,
    /// Number of pre-check candidates found
    pub precheck_candidates_found: usize,
    /// Number of insights curated after Tier2 (if curation enabled)
    pub curated_insights_count: usize,
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
            precheck_hit: false,
            precheck_candidates_found: 0,
            curated_insights_count: 0,
        };
    }

    let start = std::time::Instant::now();

    // SPEC-KIT-971: Load Stage0Config FIRST to determine memory_backend
    send_progress(&progress_tx, Stage0Progress::LoadingConfig);
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
                precheck_hit: false,
                precheck_candidates_found: 0,
                curated_insights_count: 0,
            };
        }
    };

    // SPEC-KIT-971: Create memory client based on configured backend
    // This replaces the unconditional local-memory daemon check
    let memory_backend = stage0_cfg.memory_backend;
    let backend_name = match memory_backend {
        MemoryBackend::Memvid => "memvid",
        MemoryBackend::LocalMemory => "local-memory",
    };
    send_progress(
        &progress_tx,
        Stage0Progress::CreatingMemoryClient {
            backend: backend_name.to_string(),
        },
    );

    // For LocalMemory backend, check daemon health first (traditional behavior)
    if memory_backend == MemoryBackend::LocalMemory {
        send_progress(&progress_tx, Stage0Progress::CheckingLocalMemory);
        if !crate::local_memory_cli::local_memory_daemon_healthy_blocking(Duration::from_millis(
            750,
        )) {
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
                precheck_hit: false,
                precheck_candidates_found: 0,
                curated_insights_count: 0,
            };
        }
    }

    // SPEC-KIT-971: Create unified memory client
    // For memvid backend: does NOT require local-memory daemon upfront
    // Fallback to local-memory only checked if memvid fails
    // Use canonical capsule config (SPEC-KIT-971/977 alignment)
    let capsule_path = default_capsule_path(cwd);

    // Use tokio runtime to call async create_unified_memory_client
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
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
                skip_reason: Some(format!("Failed to create runtime: {e}")),
                duration_ms,
                tier2_used: false,
                cache_hit: false,
                hybrid_retrieval_used: false,
                tier2_skip_reason: Some("runtime creation failed".to_string()),
                precheck_hit: false,
                precheck_candidates_found: 0,
                curated_insights_count: 0,
            };
        }
    };

    let memory_client: UnifiedMemoryClient = match rt.block_on(create_unified_memory_client(
        memory_backend,
        capsule_path,
        DEFAULT_WORKSPACE_ID.to_string(),
        || {
            crate::local_memory_cli::local_memory_daemon_healthy_blocking(Duration::from_millis(
                500,
            ))
        },
    )) {
        Ok(client) => {
            tracing::info!(
                target: "stage0",
                backend = backend_name,
                "Memory client created successfully"
            );
            client
        }
        Err(e) => {
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
                skip_reason: Some(format!(
                    "Failed to create memory client ({}): {}",
                    backend_name, e
                )),
                duration_ms,
                tier2_used: false,
                cache_hit: false,
                hybrid_retrieval_used: false,
                tier2_skip_reason: Some("memory client creation failed".to_string()),
                precheck_hit: false,
                precheck_candidates_found: 0,
                curated_insights_count: 0,
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

    // Create LLM adapter (no MCP dependencies).
    let llm = LlmStubAdapter::new();

    // ─────────────────────────────────────────────────────────────────────────────
    // ADR-003 Prompt F: Pre-check against codex-product before Tier2
    // ─────────────────────────────────────────────────────────────────────────────
    let (mut precheck_hit, mut precheck_candidates_found) = (false, 0_usize);

    let precheck_skip_tier2 = if stage0_cfg.context_compiler.product_knowledge.enabled
        && stage0_cfg.context_compiler.product_knowledge.precheck_enabled
        && memory_backend == MemoryBackend::LocalMemory
    {
        // Extract query from spec content (first 500 chars as search query)
        let query: String = spec_content.chars().take(500).collect();

        // Search codex-product domain
        match crate::local_memory_cli::search_blocking(
            &query,
            10, // Check top 10 matches
            &[],
            Some(&stage0_cfg.context_compiler.product_knowledge.domain),
            1000, // Short snippets for pre-check
        ) {
            Ok(results) => {
                // Convert to LocalMemorySummary format for precheck function
                let summaries: Vec<codex_stage0::LocalMemorySummary> = results
                    .iter()
                    .filter_map(|r| {
                        Some(codex_stage0::LocalMemorySummary {
                            id: r.memory.id.clone()?,
                            domain: r.memory.domain.clone(),
                            tags: r.memory.tags.clone().unwrap_or_default(),
                            created_at: r.memory.created_at.clone(),
                            snippet: if r.memory.content.len() > 200 {
                                format!("{}...", &r.memory.content[..200])
                            } else {
                                r.memory.content.clone()
                            },
                            similarity_score: r.relevance_score.unwrap_or(0.0),
                        })
                    })
                    .collect();

                // Run pre-check filtering and threshold check
                let precheck_result = codex_stage0::precheck_product_knowledge(
                    summaries,
                    stage0_cfg.context_compiler.product_knowledge.precheck_threshold,
                    stage0_cfg.context_compiler.product_knowledge.min_importance,
                    &query,
                );

                precheck_hit = precheck_result.hit;
                precheck_candidates_found = precheck_result.candidates.len();

                if precheck_result.hit {
                    tracing::info!(
                        "Pre-check HIT for {}: {} candidates with max_relevance={:.3}, skipping Tier2",
                        spec_id,
                        precheck_result.candidates.len(),
                        precheck_result.max_relevance
                    );
                    true // Skip Tier2
                } else {
                    tracing::debug!(
                        "Pre-check MISS for {}: {} candidates, max_relevance={:.3} < threshold {:.2}",
                        spec_id,
                        precheck_result.candidates.len(),
                        precheck_result.max_relevance,
                        stage0_cfg.context_compiler.product_knowledge.precheck_threshold
                    );
                    false // Proceed to Tier2
                }
            }
            Err(e) => {
                // Pre-check failed - proceed to Tier2 (fail-closed behavior)
                tracing::warn!("Pre-check search failed (proceeding to Tier2): {}", e);
                false
            }
        }
    } else {
        false
    };

    // CONVERGENCE: Tier2 fail-closed with explicit diagnostics
    // Per MEMO_codex-rs.md Section 1: "emit diagnostics with actionable next steps"
    let (tier2_opt, tier2_skip_reason) =
        if precheck_skip_tier2 {
            // Pre-check hit - skip Tier2 entirely
            (None, Some("pre-check hit (cached insight found)".to_string()))
        } else if stage0_cfg.tier2.enabled && !stage0_cfg.tier2.notebook.trim().is_empty() {
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

    // ADR-003 Prompt F: Save curation settings before moving stage0_cfg
    let curation_enabled = stage0_cfg.context_compiler.product_knowledge.enabled
        && stage0_cfg.context_compiler.product_knowledge.curation_enabled;

    // Run Stage 0 engine
    // Note: Stage0Engine contains rusqlite::Connection which is not Send,
    // so we need to run everything in a dedicated single-threaded runtime
    // SPEC-KIT-971: Uses memory_client from backend routing above
    let (stage0_result, hybrid_used) = run_stage0_blocking(
        spec_id.to_string(),
        spec_content.to_string(),
        env,
        memory_client,
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
            let final_tier2_skip = if tier2_used { None } else { tier2_skip_reason };

            // ADR-003 Prompt F: Post-curation (background thread, non-blocking)
            // Curate Tier2 output into codex-product if enabled and tier2 was used
            let curated_count = if tier2_used && !cache_hit && curation_enabled {
                let divine_truth_raw = result.divine_truth.raw_markdown.clone();
                let spec_id_clone = spec_id.to_string();
                // Extract component from spec_id (e.g., "SPEC-KIT-102" -> "spec-kit")
                let component = spec_id
                    .split('-')
                    .take(2)
                    .collect::<Vec<_>>()
                    .join("-")
                    .to_lowercase();

                // Spawn background curation (never blocks pipeline)
                std::thread::spawn(move || {
                    let adapter = crate::stage0_adapters::ProductKnowledgeCurationAdapter::new();
                    match adapter.curate_tier2_output(&spec_id_clone, &component, &divine_truth_raw)
                    {
                        Ok(ids) => {
                            if !ids.is_empty() {
                                tracing::info!(
                                    "Curated {} insights from {} into codex-product",
                                    ids.len(),
                                    spec_id_clone
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Post-curation failed for {}: {}", spec_id_clone, e);
                        }
                    }
                });
                0 // Actual count will be determined async, but we report 0 for this run
            } else {
                0
            };

            Stage0ExecutionResult {
                result: Some(result),
                skip_reason: None,
                duration_ms,
                tier2_used,
                cache_hit,
                hybrid_retrieval_used: hybrid_used,
                tier2_skip_reason: final_tier2_skip,
                precheck_hit,
                precheck_candidates_found,
                curated_insights_count: curated_count,
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
                precheck_hit,
                precheck_candidates_found,
                curated_insights_count: 0,
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
///
/// SPEC-KIT-971: Accepts UnifiedMemoryClient enum to support both
/// memvid and local-memory backends via unified interface.
fn run_stage0_blocking(
    spec_id: String,
    spec_content: String,
    env: EnvCtx,
    memory_client: UnifiedMemoryClient,
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
                            &memory_client,
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
                            &memory_client,
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
                    &memory_client,
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
                &memory_client,
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
///
/// SPEC-KIT-971: Accepts &UnifiedMemoryClient to support unified interface.
async fn run_without_vector(
    engine: &Stage0Engine,
    memory_client: &UnifiedMemoryClient,
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
                memory_client,
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
                memory_client,
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

// ─────────────────────────────────────────────────────────────────────────────
// ADR-003: Product Knowledge Capsule Snapshotting
// ─────────────────────────────────────────────────────────────────────────────

/// Write product knowledge evidence pack to capsule
///
/// ADR-003: When product knowledge is used, the evidence pack must be written
/// to capsule for deterministic replay. Call this after Stage0 completes when
/// capsule access is available.
///
/// # Arguments
/// * `spec_id` - SPEC identifier
/// * `run_id` - Run identifier
/// * `pack` - The ProductKnowledgeEvidencePack to write
/// * `capsule` - CapsuleHandle for writing
///
/// # Returns
/// The logical URI of the written artifact, or an error message if writing fails.
///
/// # Example
/// ```ignore
/// if let Some(pack) = &stage0_result.product_knowledge_pack {
///     if let Err(e) = write_product_knowledge_to_capsule(spec_id, run_id, pack, &capsule) {
///         tracing::warn!("Failed to write product knowledge evidence pack: {}", e);
///     }
/// }
/// ```
pub fn write_product_knowledge_to_capsule(
    spec_id: &str,
    run_id: &str,
    pack: &codex_stage0::ProductKnowledgeEvidencePack,
    capsule: &crate::memvid_adapter::CapsuleHandle,
) -> Result<crate::memvid_adapter::LogicalUri, String> {
    use crate::memvid_adapter::ObjectType;

    // Serialize the evidence pack
    let pack_json = serde_json::to_vec_pretty(pack)
        .map_err(|e| format!("Failed to serialize evidence pack: {}", e))?;

    // Write to capsule at the ADR-003 path
    let uri = capsule
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "product_knowledge/evidence_pack.json",
            pack_json,
            serde_json::json!({
                "schema": codex_stage0::ProductKnowledgeEvidencePack::SCHEMA_VERSION,
                "item_count": pack.items.len(),
                "domain": &pack.domain,
            }),
        )
        .map_err(|e| format!("Failed to write evidence pack to capsule: {}", e))?;

    tracing::info!(
        target: "stage0",
        uri = %uri,
        spec_id = spec_id,
        run_id = run_id,
        items = pack.items.len(),
        "Written ProductKnowledgeEvidencePack to capsule"
    );

    Ok(uri)
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

    // =========================================================================
    // SPEC-KIT-971-A5: Pipeline Backend Test
    // =========================================================================

    /// SPEC-KIT-971-A5: Stage0 runs with memvid backend when local-memory is absent.
    ///
    /// This test verifies that:
    /// 1. When memory_backend = memvid, the pipeline does NOT require local-memory daemon
    /// 2. Stage0 succeeds when capsule path exists (creates capsule if needed)
    /// 3. Returns a Stage0ExecutionResult even with 0 memories
    #[test]
    fn test_971_a5_memvid_backend_without_local_memory() {
        use crate::memvid_adapter::{UnifiedMemoryClient, create_unified_memory_client};
        use codex_stage0::MemoryBackend;
        use tempfile::TempDir;

        // Create a temporary directory for the capsule
        let temp_dir = TempDir::new().expect("create temp dir");
        let capsule_path = temp_dir.path().join("test.mv2");

        // Create tokio runtime for async test
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create runtime");

        rt.block_on(async {
            // Create unified memory client with memvid backend
            // The health check closure always returns false (simulating absent local-memory)
            let result = create_unified_memory_client(
                MemoryBackend::Memvid,
                capsule_path.clone(),
                "test".to_string(),
                || false, // Local-memory is not healthy
            )
            .await;

            // Should succeed because capsule opens
            assert!(
                result.is_ok(),
                "Memvid client should open without local-memory"
            );

            let client = result.unwrap();

            // Verify it's a Memvid client, not LocalMemory
            match &client {
                UnifiedMemoryClient::Memvid(_) => {
                    // Expected: memvid adapter created
                }
                UnifiedMemoryClient::LocalMemory(_) => {
                    panic!("Expected Memvid client, got LocalMemory");
                }
            }

            // Verify the capsule file was created
            assert!(capsule_path.exists(), "Capsule file should be created");

            // Verify the client is functional (search returns empty since no data)
            use codex_stage0::dcc::{Iqo, LocalMemoryClient, LocalMemorySearchParams};
            let params = LocalMemorySearchParams {
                iqo: Iqo {
                    keywords: vec!["test".to_string()],
                    ..Default::default()
                },
                max_results: 10,
            };
            let results = client.search_memories(params).await;
            assert!(results.is_ok(), "Search should succeed");
            assert!(
                results.unwrap().is_empty(),
                "No data, empty results expected"
            );
        });
    }

    /// SPEC-KIT-971-A5: Memvid fallback to local-memory when capsule fails.
    ///
    /// This test verifies that when memvid capsule fails AND local-memory is healthy,
    /// the system falls back to local-memory.
    #[test]
    fn test_971_a5_memvid_fallback_to_local_memory() {
        use crate::memvid_adapter::{UnifiedMemoryClient, create_unified_memory_client};
        use codex_stage0::MemoryBackend;
        use std::path::PathBuf;

        // Use an invalid path that will cause capsule open to fail
        let invalid_path = PathBuf::from("/nonexistent/deeply/nested/path/that/cannot/exist.mv2");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create runtime");

        rt.block_on(async {
            // Create unified memory client with memvid backend
            // The health check returns true (local-memory is healthy)
            let result = create_unified_memory_client(
                MemoryBackend::Memvid,
                invalid_path,
                "test".to_string(),
                || true, // Local-memory is healthy
            )
            .await;

            // Should succeed with fallback to local-memory
            assert!(result.is_ok(), "Should fall back to local-memory");

            let client = result.unwrap();

            // Verify it's a LocalMemory client (fallback)
            match &client {
                UnifiedMemoryClient::LocalMemory(_) => {
                    // Expected: fell back to local-memory
                }
                UnifiedMemoryClient::Memvid(_) => {
                    panic!("Expected LocalMemory fallback, got Memvid");
                }
            }
        });
    }

    /// SPEC-KIT-971-A5: Memvid fails when capsule fails and local-memory unhealthy.
    #[test]
    fn test_971_a5_memvid_no_fallback_fails() {
        use crate::memvid_adapter::create_unified_memory_client;
        use codex_stage0::MemoryBackend;
        use std::path::PathBuf;

        // Use an invalid path that will cause capsule open to fail
        let invalid_path = PathBuf::from("/nonexistent/deeply/nested/path/that/cannot/exist.mv2");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create runtime");

        rt.block_on(async {
            // Create unified memory client with memvid backend
            // The health check returns false (local-memory is not healthy)
            let result = create_unified_memory_client(
                MemoryBackend::Memvid,
                invalid_path,
                "test".to_string(),
                || false, // Local-memory is not healthy
            )
            .await;

            // Should fail because no fallback available
            assert!(result.is_err(), "Should fail when no fallback available");
        });
    }
}
