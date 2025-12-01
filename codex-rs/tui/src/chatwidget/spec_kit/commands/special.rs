//! Special command implementations (auto, new, specify, consensus, constitution)
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit multi-agent automation framework

use super::super::super::ChatWidget;
use super::super::ace_constitution;
use super::super::command_registry::SpecKitCommand;
use super::super::handler;
use super::super::routing::{get_current_branch, get_repo_root};

/// Command: /speckit.auto
/// Full 6-stage pipeline with auto-advancement
/// Note: Legacy /spec-auto alias removed to prevent confusion with subagent routing
pub struct SpecKitAutoCommand;

impl SpecKitCommand for SpecKitAutoCommand {
    fn name(&self) -> &'static str {
        "speckit.auto"
    }

    fn aliases(&self) -> &[&'static str] {
        &[] // No aliases - use /speckit.auto explicitly
    }

    fn description(&self) -> &'static str {
        "full 6-stage pipeline with auto-advancement (supports --configure for interactive modal)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        // Parse spec-auto args and delegate to handler
        match crate::slash_command::parse_spec_auto_args(&args) {
            Ok(invocation) => {
                widget.handle_spec_auto_command(invocation);
            }
            Err(err) => {
                let error_msg = match err {
                    crate::slash_command::SpecAutoParseError::MissingSpecId => {
                        "Missing SPEC ID. Usage: /speckit.auto SPEC-KIT-### [--configure] [--from stage]"
                            .to_string()
                    }
                    crate::slash_command::SpecAutoParseError::MissingFromStage => {
                        "`--from` flag requires a stage name".to_string()
                    }
                    crate::slash_command::SpecAutoParseError::UnknownStage(stage) => {
                        format!(
                            "Unknown stage '{}'. Valid stages: plan, tasks, implement, validate, audit, unlock",
                            stage
                        )
                    }
                    crate::slash_command::SpecAutoParseError::UnknownHalMode(mode) => {
                        format!("Unknown HAL mode '{}'. Expected 'mock' or 'live'", mode)
                    }
                };
                widget.history_push(crate::history_cell::new_error_event(error_msg));
                widget.request_redraw();
            }
        }
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.new (and /new-spec)
/// Create new SPEC from description with interactive Q&A - FULLY NATIVE (zero agents, $0)
/// SPEC-KIT-970: Now shows modal with 3 required questions before generating PRD
/// SPEC-KIT-971: Questions customized based on detected project type
pub struct SpecKitNewCommand;

impl SpecKitCommand for SpecKitNewCommand {
    fn name(&self) -> &'static str {
        "speckit.new"
    }

    fn aliases(&self) -> &[&'static str] {
        &["new-spec"]
    }

    fn description(&self) -> &'static str {
        "create new SPEC with project-aware Q&A (INSTANT, zero agents, $0)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        use super::super::project_detector::{detect_project_type, get_project_questions};

        // SPEC-KIT-971: Detect project type and customize questions
        let project_type = detect_project_type(&widget.config.cwd);
        let project_questions = get_project_questions(project_type);

        // Convert project_detector questions to prd_builder_modal format
        let modal_questions: Vec<crate::bottom_pane::prd_builder_modal::PrdQuestion> =
            project_questions
                .into_iter()
                .map(|q| crate::bottom_pane::prd_builder_modal::PrdQuestion {
                    category: q.category,
                    question: q.question,
                    options: q
                        .options
                        .into_iter()
                        .map(|o| crate::bottom_pane::prd_builder_modal::PrdOption {
                            label: o.label,
                            text: o.text,
                            is_custom: o.is_custom,
                        })
                        .collect(),
                })
                .collect();

        let project_display = format!("{} {}", project_type.icon(), project_type.display_name());

        // SPEC-KIT-970: Show interactive PRD builder modal with project-aware questions
        widget.show_prd_builder_with_context(
            args.trim().to_string(),
            project_display,
            modal_questions,
        );
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.specify
/// Generate PRD with single-agent refinement (SPEC-KIT-957: Tier 1)
pub struct SpecKitSpecifyCommand;

impl SpecKitCommand for SpecKitSpecifyCommand {
    fn name(&self) -> &'static str {
        "speckit.specify"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "refine PRD with single-agent (Tier 1, ~$0.10)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        // SPEC-KIT-957: Direct execution, no longer uses orchestrator pattern
        super::plan::execute_stage_command(
            widget,
            args,
            crate::spec_prompts::SpecStage::Specify,
            "speckit.specify",
        );
    }

    fn expand_prompt(&self, _args: &str) -> Option<String> {
        None // SPEC-KIT-957: No longer uses orchestrator pattern
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /spec-consensus
/// Check multi-agent consensus via local-memory
pub struct SpecConsensusCommand;

impl SpecKitCommand for SpecConsensusCommand {
    fn name(&self) -> &'static str {
        "spec-consensus"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "check multi-agent consensus via local-memory (requires SPEC ID & stage)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        handler::handle_spec_consensus(widget, args);
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.constitution
/// Extract and pin constitution bullets to ACE
pub struct SpecKitConstitutionCommand;

impl SpecKitCommand for SpecKitConstitutionCommand {
    fn name(&self) -> &'static str {
        "speckit.constitution"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "extract and pin constitution bullets to ACE playbook"
    }

    fn execute(&self, widget: &mut ChatWidget, _args: String) {
        tracing::info!("SpecKitConstitution: execute() called");

        // Find constitution.md in the repository
        let constitution_path = widget.config.cwd.join("memory").join("constitution.md");

        tracing::info!(
            "SpecKitConstitution: Looking for constitution at: {:?}",
            constitution_path
        );

        if !constitution_path.exists() {
            widget.history_push(crate::history_cell::new_error_event(
                "Constitution not found at memory/constitution.md".to_string(),
            ));
            widget.request_redraw();
            return;
        }

        // Read constitution
        let markdown = match std::fs::read_to_string(&constitution_path) {
            Ok(content) => content,
            Err(e) => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "Failed to read constitution: {}",
                    e
                )));
                widget.request_redraw();
                return;
            }
        };

        // Extract bullets
        let bullets = ace_constitution::extract_bullets(&markdown);

        if bullets.is_empty() {
            widget.history_push(crate::history_cell::new_error_event(
                "No valid bullets extracted from constitution".to_string(),
            ));
            widget.request_redraw();
            return;
        }

        // Show detailed extraction info
        let scope_counts: std::collections::HashMap<String, usize> = bullets
            .iter()
            .flat_map(|b| b.scopes.iter())
            .fold(std::collections::HashMap::new(), |mut acc, scope| {
                *acc.entry(scope.clone()).or_insert(0) += 1;
                acc
            });

        let scope_summary = scope_counts
            .iter()
            .map(|(scope, count)| format!("{}: {}", scope, count))
            .collect::<Vec<_>>()
            .join(", ");

        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![
                ratatui::text::Line::from(format!(
                    "📋 Extracted {} bullets from constitution",
                    bullets.len()
                )),
                ratatui::text::Line::from(format!("   Scopes: {}", scope_summary)),
                ratatui::text::Line::from("   Pinning to ACE playbook..."),
            ],
            crate::history_cell::HistoryCellType::Notice,
        ));

        // Get git context
        let repo_root = get_repo_root(&widget.config.cwd).unwrap_or_else(|| ".".to_string());
        let branch = get_current_branch(&widget.config.cwd).unwrap_or_else(|| "main".to_string());

        // Pin to ACE
        match ace_constitution::pin_constitution_to_ace_sync(
            &widget.config.ace,
            repo_root,
            branch,
            bullets,
        ) {
            Ok(pinned_count) => {
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![
                        ratatui::text::Line::from(format!(
                            "✅ Successfully pinned {} bullets to ACE playbook",
                            pinned_count
                        )),
                        ratatui::text::Line::from(
                            "   Database: ~/.code/ace/playbooks_normalized.sqlite3",
                        ),
                        ratatui::text::Line::from("   Use /speckit.ace-status to view playbook"),
                    ],
                    crate::history_cell::HistoryCellType::Notice,
                ));
            }
            Err(e) => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "❌ Failed to pin bullets to ACE: {}",
                    e
                )));
            }
        }

        widget.request_redraw();
    }

    fn requires_args(&self) -> bool {
        false
    }
}

/// Command: /speckit.seed
/// Generate NotebookLM-ready Markdown files from local-memory and codebase
/// SPEC-KIT-102: Shadow Notebook Seeder V1
pub struct SpecKitSeedCommand;

impl SpecKitCommand for SpecKitSeedCommand {
    fn name(&self) -> &'static str {
        "speckit.seed"
    }

    fn aliases(&self) -> &[&'static str] {
        &["notebooklm-seed"]
    }

    fn description(&self) -> &'static str {
        "generate NotebookLM-ready Markdown files from local-memory (Stage0 Seeder)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        use super::super::stage0_seeding::{run_shadow_seeding, SeedingConfig};
        use crate::stage0_adapters::{has_local_memory_server, LocalMemoryMcpAdapter};
        use std::sync::Arc;

        // Parse optional arguments
        let mut max_memories = 50usize;
        let mut output_dir = widget.config.cwd.join("evidence").join("notebooklm");

        for arg in args.split_whitespace() {
            if arg.starts_with("--max=") {
                if let Ok(n) = arg.trim_start_matches("--max=").parse() {
                    max_memories = n;
                }
            } else if arg.starts_with("--output=") {
                output_dir = std::path::PathBuf::from(arg.trim_start_matches("--output="));
            }
        }

        // Show starting message
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![
                ratatui::text::Line::from("🌱 Stage0 NotebookLM Seeder"),
                ratatui::text::Line::from(format!(
                    "   Output: {}",
                    output_dir.display()
                )),
                ratatui::text::Line::from(format!("   Max memories per artifact: {}", max_memories)),
                ratatui::text::Line::from("   Scanning local-memory and codebase..."),
            ],
            crate::history_cell::HistoryCellType::Notice,
        ));
        widget.request_redraw();

        // Get MCP manager
        let mcp_manager = widget.mcp_manager.clone();
        let cwd = widget.config.cwd.clone();

        // Run seeding in async context
        let result = super::super::consensus_coordinator::block_on_sync(|| async move {
            let mcp_lock = mcp_manager.lock().await;
            let Some(mcp) = mcp_lock.as_ref() else {
                return Err("MCP manager not available".to_string());
            };

            if !has_local_memory_server(mcp) {
                return Err("local-memory MCP server not available".to_string());
            }

            let local_mem = LocalMemoryMcpAdapter::new(Arc::clone(mcp));
            let config = SeedingConfig {
                max_memories_per_artifact: max_memories,
                output_dir,
                project_root: cwd,
            };

            Ok(run_shadow_seeding(&local_mem, &config).await)
        });

        match result {
            Ok(seeding_result) => {
                let mut lines = vec![
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(format!(
                        "✅ Stage0 NotebookLM seeding complete ({} ms)",
                        seeding_result.duration_ms
                    )),
                ];

                for artifact in &seeding_result.artifacts {
                    let status = if artifact.written { "✓" } else { "✗" };
                    lines.push(ratatui::text::Line::from(format!(
                        "   {} {} ({} sources)",
                        status,
                        artifact.kind.filename(),
                        artifact.count
                    )));
                }

                if !seeding_result.errors.is_empty() {
                    lines.push(ratatui::text::Line::from(""));
                    lines.push(ratatui::text::Line::from("⚠ Warnings:"));
                    for err in &seeding_result.errors {
                        lines.push(ratatui::text::Line::from(format!("   - {}", err)));
                    }
                }

                lines.push(ratatui::text::Line::from(""));
                lines.push(ratatui::text::Line::from(
                    "📚 Upload these files to your NotebookLM notebook:",
                ));
                lines.push(ratatui::text::Line::from(
                    "   \"codex-rs – Shadow Stage 0\"",
                ));

                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    lines,
                    crate::history_cell::HistoryCellType::Notice,
                ));
            }
            Err(e) => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "Stage0 seeding failed: {}",
                    e
                )));
            }
        }

        widget.request_redraw();
    }

    fn requires_args(&self) -> bool {
        false
    }
}

/// Command: /speckit.ace-status
/// Show ACE playbook status and statistics
pub struct SpecKitAceStatusCommand;

impl SpecKitCommand for SpecKitAceStatusCommand {
    fn name(&self) -> &'static str {
        "speckit.ace-status"
    }

    fn aliases(&self) -> &[&'static str] {
        &["ace-status"]
    }

    fn description(&self) -> &'static str {
        "show ACE playbook status and bullet statistics"
    }

    fn execute(&self, widget: &mut ChatWidget, _args: String) {
        use std::process::Command;

        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from("📊 ACE Playbook Status")],
            crate::history_cell::HistoryCellType::Notice,
        ));

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let db_path = std::path::PathBuf::from(home).join(".code/ace/playbooks_normalized.sqlite3");

        // Check if database exists
        if !db_path.exists() {
            widget.history_push(crate::history_cell::new_error_event(
                "ACE database not found. Run /speckit.constitution to initialize.".to_string(),
            ));
            widget.request_redraw();
            return;
        }

        // Get statistics
        let query = "SELECT scope, COUNT(*), SUM(pinned), AVG(score), MAX(score) FROM playbook_bullet GROUP BY scope ORDER BY scope;";

        match Command::new("sqlite3").arg(&db_path).arg(query).output() {
            Ok(result) if result.status.success() => {
                let stats = String::from_utf8_lossy(&result.stdout);

                let mut lines = vec![
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(
                        "Scope      | Total | Pinned | Avg Score | Max Score",
                    ),
                    ratatui::text::Line::from(
                        "-----------|-------|--------|-----------|----------",
                    ),
                ];

                for line in stats.lines() {
                    let parts: Vec<&str> = line.split('|').collect();
                    if parts.len() >= 5 {
                        lines.push(ratatui::text::Line::from(format!(
                            "{:<10} | {:<5} | {:<6} | {:<9.2} | {:.2}",
                            parts[0],
                            parts[1],
                            parts[2],
                            parts[3].parse::<f64>().unwrap_or(0.0),
                            parts[4].parse::<f64>().unwrap_or(0.0)
                        )));
                    }
                }

                lines.push(ratatui::text::Line::from(""));
                lines.push(ratatui::text::Line::from(format!(
                    "Database: {}",
                    db_path.display()
                )));

                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    lines,
                    crate::history_cell::HistoryCellType::Notice,
                ));
            }
            _ => {
                widget.history_push(crate::history_cell::new_error_event(
                    "Failed to query ACE database. Is sqlite3 installed?".to_string(),
                ));
            }
        }

        widget.request_redraw();
    }

    fn requires_args(&self) -> bool {
        false
    }
}

/// Command: /stage0.index
/// Index local-memory contents into the shared TF-IDF vector backend
/// SPEC-KIT-102 V2.5b: Vector backend indexing command with shared state
pub struct Stage0IndexCommand;

impl SpecKitCommand for Stage0IndexCommand {
    fn name(&self) -> &'static str {
        "stage0.index"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "index local-memory contents into Stage0 vector backend"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        use crate::stage0_adapters::{has_local_memory_server, LocalMemoryMcpAdapter};
        use crate::vector_state::{IndexingStats, VECTOR_STATE};
        use codex_stage0::{
            DocumentKind, Iqo, LocalMemoryClient, LocalMemorySearchParams,
            TfIdfBackend, VectorBackend, VectorDocument,
        };
        use std::sync::Arc;

        // Parse optional arguments
        let mut max_results = 100usize;

        for arg in args.split_whitespace() {
            if arg.starts_with("--max=") {
                if let Ok(n) = arg.trim_start_matches("--max=").parse() {
                    max_results = n;
                }
            }
        }

        // Show starting message
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![
                ratatui::text::Line::from("🔍 Stage0 Vector Backend Indexing"),
                ratatui::text::Line::from(format!("   Max memories: {}", max_results)),
                ratatui::text::Line::from("   Fetching memories from local-memory..."),
            ],
            crate::history_cell::HistoryCellType::Notice,
        ));
        widget.request_redraw();

        let mcp_manager = widget.mcp_manager.clone();

        // Run indexing in async context
        let result = super::super::consensus_coordinator::block_on_sync(|| async move {
            let mcp_lock = mcp_manager.lock().await;
            let Some(mcp) = mcp_lock.as_ref() else {
                return Err("MCP manager not available".to_string());
            };

            if !has_local_memory_server(mcp) {
                return Err("local-memory MCP server not available".to_string());
            }

            let local_mem = LocalMemoryMcpAdapter::new(Arc::clone(mcp));

            // Search for memories using wildcard IQO
            let iqo = Iqo {
                keywords: vec!["*".to_string()],
                domains: vec![],
                max_candidates: max_results,
                ..Default::default()
            };
            let params = LocalMemorySearchParams {
                iqo,
                max_results,
            };

            let memories = local_mem
                .search_memories(params)
                .await
                .map_err(|e| format!("Failed to fetch memories: {}", e))?;

            if memories.is_empty() {
                return Ok((0, 0, 0, 0));
            }

            // Convert to VectorDocuments
            let docs: Vec<VectorDocument> = memories
                .iter()
                .map(|m| {
                    let mut doc = VectorDocument::new(
                        m.id.clone(),
                        DocumentKind::Memory,
                        m.snippet.clone(),
                    );

                    if let Some(domain) = &m.domain {
                        doc = doc.with_domain(domain.as_str());
                    }

                    for tag in &m.tags {
                        doc = doc.with_tag(tag.as_str());
                    }

                    doc
                })
                .collect();

            let doc_count = docs.len();

            // Create backend and index - V2.5b: Store in shared state
            let backend = TfIdfBackend::new();
            let stats = backend
                .index_documents(docs)
                .await
                .map_err(|e| format!("Indexing failed: {}", e))?;

            // Store in shared VECTOR_STATE for use by run_stage0_blocking
            let indexing_stats = IndexingStats {
                doc_count,
                unique_tokens: stats.unique_tokens,
                total_tokens: stats.total_tokens,
                duration_ms: stats.duration_ms,
                indexed_at: chrono::Utc::now(),
            };
            VECTOR_STATE.set_backend(backend, indexing_stats).await;

            Ok((
                doc_count,
                stats.unique_tokens,
                stats.total_tokens,
                stats.duration_ms,
            ))
        });

        match result {
            Ok((doc_count, unique_tokens, total_tokens, duration_ms)) => {
                if doc_count == 0 {
                    widget.history_push(crate::history_cell::PlainHistoryCell::new(
                        vec![ratatui::text::Line::from(
                            "⚠ No memories found in local-memory",
                        )],
                        crate::history_cell::HistoryCellType::Notice,
                    ));
                } else {
                    widget.history_push(crate::history_cell::PlainHistoryCell::new(
                        vec![
                            ratatui::text::Line::from(""),
                            ratatui::text::Line::from(format!(
                                "✅ Stage0 indexing complete ({} ms)",
                                duration_ms
                            )),
                            ratatui::text::Line::from(format!(
                                "   Documents indexed: {}",
                                doc_count
                            )),
                            ratatui::text::Line::from(format!(
                                "   Unique tokens: {}",
                                unique_tokens
                            )),
                            ratatui::text::Line::from(format!(
                                "   Total tokens: {}",
                                total_tokens
                            )),
                            ratatui::text::Line::from(""),
                            ratatui::text::Line::from(
                                "   Backend stored in shared state. Run /speckit.auto to use hybrid retrieval.",
                            ),
                        ],
                        crate::history_cell::HistoryCellType::Notice,
                    ));
                }
            }
            Err(e) => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "Stage0 indexing failed: {}",
                    e
                )));
            }
        }

        widget.request_redraw();
    }

    fn requires_args(&self) -> bool {
        false
    }
}

/// Command: /stage0.eval-backend
/// Run evaluation harness comparing baseline vs hybrid retrieval
/// SPEC-KIT-102 V2.5b: Baseline vs Hybrid DCC comparison
pub struct Stage0EvalBackendCommand;

impl SpecKitCommand for Stage0EvalBackendCommand {
    fn name(&self) -> &'static str {
        "stage0.eval-backend"
    }

    fn aliases(&self) -> &[&'static str] {
        &["stage0.eval"]
    }

    fn description(&self) -> &'static str {
        "compare baseline vs hybrid retrieval using eval cases"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        use crate::vector_state::VECTOR_STATE;
        use codex_stage0::{
            built_in_eval_cases, built_in_test_documents, evaluate_backend,
            TfIdfBackend, VectorBackend, VectorFilters,
        };
        use std::path::PathBuf;

        // Parse optional arguments
        let mut top_k = 10usize;
        let mut output_json = false;
        let mut cases_file: Option<PathBuf> = None;

        for arg in args.split_whitespace() {
            if arg.starts_with("--top-k=") || arg.starts_with("--k=") {
                if let Ok(n) = arg
                    .trim_start_matches("--top-k=")
                    .trim_start_matches("--k=")
                    .parse()
                {
                    top_k = n;
                }
            } else if arg == "--json" {
                output_json = true;
            } else if arg.starts_with("--cases=") {
                cases_file = Some(PathBuf::from(arg.trim_start_matches("--cases=")));
            }
        }

        // Show starting message
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![
                ratatui::text::Line::from("📊 Stage0 Baseline vs Hybrid Evaluation"),
                ratatui::text::Line::from(format!("   Top K: {}", top_k)),
                ratatui::text::Line::from(match &cases_file {
                    Some(p) => format!("   Cases: {}", p.display()),
                    None => "   Cases: Built-in test cases".to_string(),
                }),
                ratatui::text::Line::from("   Running evaluation..."),
            ],
            crate::history_cell::HistoryCellType::Notice,
        ));
        widget.request_redraw();

        // Run evaluation in async context
        let result = super::super::consensus_coordinator::block_on_sync(|| async move {
            // Load eval cases
            let cases = match cases_file {
                Some(ref path) => codex_stage0::load_eval_cases_from_file(path)
                    .map_err(|e| format!("Failed to load eval cases: {}", e))?,
                None => built_in_eval_cases(),
            };

            if cases.is_empty() {
                return Err("No eval cases to run".to_string());
            }

            // Index test documents in a fresh backend (for baseline)
            let baseline_backend = TfIdfBackend::new();
            let docs = built_in_test_documents();
            baseline_backend
                .index_documents(docs)
                .await
                .map_err(|e| format!("Indexing failed: {}", e))?;

            // Run baseline evaluation
            let baseline_result =
                evaluate_backend(&baseline_backend, &cases, &VectorFilters::new(), top_k)
                    .await
                    .map_err(|e| format!("Baseline evaluation failed: {}", e))?;

            // Check for shared backend (hybrid)
            let backend_handle = VECTOR_STATE.backend_handle();
            let backend_lock = backend_handle.read().await;

            let hybrid_result = if let Some(ref hybrid_backend) = *backend_lock {
                // Run hybrid evaluation with shared backend
                let result =
                    evaluate_backend(hybrid_backend, &cases, &VectorFilters::new(), top_k)
                        .await
                        .map_err(|e| format!("Hybrid evaluation failed: {}", e))?;
                Some(result)
            } else {
                None
            };

            Ok((baseline_result, hybrid_result))
        });

        match result {
            Ok((baseline, hybrid_opt)) => {
                if output_json {
                    // JSON output for CI automation
                    let json_output = serde_json::json!({
                        "top_k": top_k,
                        "baseline": {
                            "mean_precision": baseline.mean_precision,
                            "mean_recall": baseline.mean_recall,
                            "mrr": baseline.mrr,
                            "cases_passed": baseline.cases_passed,
                            "total_cases": baseline.total_cases,
                            "pass_rate": baseline.pass_rate(),
                        },
                        "hybrid": hybrid_opt.as_ref().map(|h| serde_json::json!({
                            "mean_precision": h.mean_precision,
                            "mean_recall": h.mean_recall,
                            "mrr": h.mrr,
                            "cases_passed": h.cases_passed,
                            "total_cases": h.total_cases,
                            "pass_rate": h.pass_rate(),
                        })),
                        "improvement": hybrid_opt.as_ref().map(|h| serde_json::json!({
                            "precision_delta": h.mean_precision - baseline.mean_precision,
                            "recall_delta": h.mean_recall - baseline.mean_recall,
                            "mrr_delta": h.mrr - baseline.mrr,
                        })),
                    });

                    widget.history_push(crate::history_cell::PlainHistoryCell::new(
                        vec![
                            ratatui::text::Line::from(""),
                            ratatui::text::Line::from(
                                serde_json::to_string_pretty(&json_output).unwrap_or_default(),
                            ),
                        ],
                        crate::history_cell::HistoryCellType::Notice,
                    ));
                } else {
                    // Text table output
                    let mut lines = vec![
                        ratatui::text::Line::from(""),
                        ratatui::text::Line::from(format!(
                            "{:<20} {:>12} {:>12}",
                            "Metric",
                            "Baseline",
                            hybrid_opt.as_ref().map(|_| "Hybrid").unwrap_or("N/A")
                        )),
                        ratatui::text::Line::from("-".repeat(48)),
                    ];

                    let format_delta = |baseline_val: f64, hybrid_val: Option<f64>| -> String {
                        match hybrid_val {
                            Some(h) => {
                                let delta = h - baseline_val;
                                let sign = if delta >= 0.0 { "+" } else { "" };
                                format!("{:.2} ({}{:.2})", h, sign, delta)
                            }
                            None => "N/A".to_string(),
                        }
                    };

                    lines.push(ratatui::text::Line::from(format!(
                        "{:<20} {:>12.2} {:>12}",
                        "Mean P@k",
                        baseline.mean_precision,
                        format_delta(baseline.mean_precision, hybrid_opt.as_ref().map(|h| h.mean_precision))
                    )));

                    lines.push(ratatui::text::Line::from(format!(
                        "{:<20} {:>12.2} {:>12}",
                        "Mean R@k",
                        baseline.mean_recall,
                        format_delta(baseline.mean_recall, hybrid_opt.as_ref().map(|h| h.mean_recall))
                    )));

                    lines.push(ratatui::text::Line::from(format!(
                        "{:<20} {:>12.2} {:>12}",
                        "MRR",
                        baseline.mrr,
                        format_delta(baseline.mrr, hybrid_opt.as_ref().map(|h| h.mrr))
                    )));

                    lines.push(ratatui::text::Line::from(format!(
                        "{:<20} {:>12} {:>12}",
                        "Pass Rate",
                        format!("{:.1}%", baseline.pass_rate() * 100.0),
                        hybrid_opt
                            .as_ref()
                            .map(|h| format!("{:.1}%", h.pass_rate() * 100.0))
                            .unwrap_or_else(|| "N/A".to_string())
                    )));

                    lines.push(ratatui::text::Line::from("-".repeat(48)));

                    if hybrid_opt.is_none() {
                        lines.push(ratatui::text::Line::from(
                            "⚠ No hybrid backend. Run /stage0.index first.",
                        ));
                    }

                    widget.history_push(crate::history_cell::PlainHistoryCell::new(
                        lines,
                        crate::history_cell::HistoryCellType::Notice,
                    ));
                }
            }
            Err(e) => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "Stage0 evaluation failed: {}",
                    e
                )));
            }
        }

        widget.request_redraw();
    }

    fn requires_args(&self) -> bool {
        false
    }
}

