//! Librarian command implementations
//!
//! SPEC-KIT-103: Memory corpus quality engine commands
//!
//! P99: Integrated audit trail, relationships client, and history command.

use crate::chatwidget::ChatWidget;
use crate::chatwidget::spec_kit::command_registry::SpecKitCommand;
use crate::history_cell::{HistoryCellType, PlainHistoryCell};
use ratatui::text::Line;
use std::str::FromStr;

/// Command: /stage0.librarian sweep
pub struct Stage0LibrarianCommand;

impl SpecKitCommand for Stage0LibrarianCommand {
    fn name(&self) -> &'static str {
        "stage0.librarian"
    }

    fn aliases(&self) -> &[&'static str] {
        &["librarian"]
    }

    fn description(&self) -> &'static str {
        "memory corpus quality sweep (classify, template, causal inference)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let parts: Vec<&str> = args.split_whitespace().collect();
        let subcommand = parts.first().copied().unwrap_or("help");

        match subcommand {
            "sweep" => execute_sweep(widget, &parts[1..]),
            "status" => execute_status(widget),
            "history" => execute_history(widget, &parts[1..]),
            "help" | _ => show_help(widget),
        }
    }
}

struct SweepFlags {
    apply: bool,
    domains: Vec<String>,
    limit: usize,
    min_importance: i32,
    json_report: bool,
    verbose: bool,
}

impl Default for SweepFlags {
    fn default() -> Self {
        Self {
            apply: false,
            domains: Vec::new(),
            limit: 100,
            min_importance: 0,
            json_report: false,
            verbose: false,
        }
    }
}

fn parse_sweep_flags(args: &[&str]) -> SweepFlags {
    let mut flags = SweepFlags::default();

    for arg in args {
        if *arg == "--apply" {
            flags.apply = true;
        } else if *arg == "--dry-run" {
            flags.apply = false;
        } else if *arg == "--json" || *arg == "--json-report" {
            flags.json_report = true;
        } else if *arg == "--verbose" || *arg == "-v" {
            flags.verbose = true;
        } else if let Some(domains_str) = arg.strip_prefix("--domains=") {
            flags.domains = domains_str.split(',').map(|s| s.trim().to_string()).collect();
        } else if let Some(limit_str) = arg.strip_prefix("--limit=") {
            flags.limit = limit_str.parse().unwrap_or(100);
        } else if let Some(min_str) = arg.strip_prefix("--min-importance=") {
            flags.min_importance = min_str.parse().unwrap_or(0);
        }
    }

    flags
}

fn push_output(widget: &mut ChatWidget, lines: Vec<String>, cell_type: HistoryCellType) {
    widget.history_push(PlainHistoryCell::new(
        lines.into_iter().map(Line::from).collect(),
        cell_type,
    ));
}

fn execute_sweep(widget: &mut ChatWidget, args: &[&str]) {
    use crate::stage0_adapters::{create_librarian_memory_client, create_relationships_client};
    use codex_stage0::librarian::{
        apply_template, classify_memory, detect_causal_language, infer_relationships,
        ChangeInput, ChangeType, EdgeInput, LibrarianAudit, LocalMemoryClient, MemoryChange,
        MemoryType, RelationshipInput, RelationshipsClient, SweepConfig, SweepResult,
    };

    let flags = parse_sweep_flags(args);
    let dry_run = !flags.apply;

    let config = SweepConfig {
        dry_run,
        domains: flags.domains.clone(),
        limit: flags.limit,
        min_importance: flags.min_importance,
        json_report: flags.json_report,
        ..Default::default()
    };

    let start_time = std::time::Instant::now();

    // P99: Connect to OverlayDb for audit trail
    let overlay_db = match codex_stage0::Stage0Config::load()
        .and_then(|cfg| codex_stage0::OverlayDb::connect_and_init(&cfg))
    {
        Ok(db) => Some(db),
        Err(e) => {
            tracing::warn!("OverlayDb unavailable for audit trail: {}", e);
            if flags.verbose {
                push_output(
                    widget,
                    vec![format!("Note: Audit trail unavailable - {}", e)],
                    HistoryCellType::Notice,
                );
            }
            None
        }
    };

    // P99: Generate sweep_id from audit trail or fallback to random
    let sweep_id = overlay_db
        .as_ref()
        .and_then(|db| LibrarianAudit::new(db.conn()).generate_run_id().ok())
        .unwrap_or_else(|| {
            format!(
                "LRB-{}-{:03}",
                chrono::Utc::now().format("%Y%m%d"),
                rand::random::<u16>() % 1000
            )
        });

    // Try to get MCP manager for live data
    let mcp_manager_guard = widget.mcp_manager.clone();
    let mcp_opt = crate::chatwidget::spec_kit::consensus_coordinator::block_on_sync(|| async move {
        mcp_manager_guard.lock().await.clone()
    });

    let mut result = SweepResult::new(&sweep_id, dry_run);
    result.config = Some(config.clone());

    // Output header
    let mode_str = if dry_run { " (DRY RUN)" } else { " (APPLYING)" };
    let header = vec![
        format!("=== Librarian Sweep{} ===", mode_str),
        format!("Sweep ID: {}", sweep_id),
        format!("Limit: {} memories", flags.limit),
        format!("Min Importance: {}", flags.min_importance),
        format!(
            "Domains: {}",
            if flags.domains.is_empty() { "all".to_string() } else { flags.domains.join(", ") }
        ),
        String::new(),
    ];
    push_output(widget, header, HistoryCellType::Notice);

    // P99: Start audit trail sweep (if db available)
    let audit_sweep_id = overlay_db.as_ref().and_then(|db| {
        let audit = LibrarianAudit::new(db.conn());
        let args_json = serde_json::to_string(&serde_json::json!({
            "dry_run": dry_run,
            "domains": &flags.domains,
            "limit": flags.limit,
            "min_importance": flags.min_importance,
        }))
        .unwrap_or_default();

        match audit.start_sweep(&sweep_id, &args_json) {
            Ok(id) => Some(id),
            Err(e) => {
                tracing::warn!("Failed to start audit sweep: {}", e);
                None
            }
        }
    });

    // Get memory client (live or fallback to sample data)
    let memory_client = mcp_opt.as_ref().and_then(|mcp| create_librarian_memory_client(mcp.clone()));

    // P99: Get relationships client for writing causal edges
    let relationships_client = mcp_opt
        .as_ref()
        .and_then(|mcp| create_relationships_client(mcp.clone()));

    let memories = if let Some(ref client) = memory_client {
        let list_params = codex_stage0::librarian::ListParams {
            domains: flags.domains.clone(),
            limit: flags.limit,
            min_importance: if flags.min_importance > 0 { Some(flags.min_importance) } else { None },
        };

        match client.list_memories(&list_params) {
            Ok(mems) => {
                if flags.verbose {
                    push_output(widget, vec![format!("Fetched {} memories from local-memory", mems.len())], HistoryCellType::Notice);
                }
                mems.into_iter().map(|m| (m.id, m.content, m.tags)).collect::<Vec<_>>()
            }
            Err(e) => {
                push_output(widget, vec![format!("MCP error: {}. Using sample data.", e)], HistoryCellType::Error);
                get_sample_memories().into_iter().map(|(id, content)| (id, content, Vec::new())).collect()
            }
        }
    } else {
        if flags.verbose {
            push_output(widget, vec!["No MCP connection, using sample data...".to_string()], HistoryCellType::Notice);
        }
        get_sample_memories().into_iter().map(|(id, content)| (id, content, Vec::new())).collect()
    };

    // Process each memory
    let mut changes_applied = 0;
    let mut changes_skipped = 0;

    for (memory_id, content, existing_tags) in memories.iter().take(flags.limit) {
        result.summary.memories_scanned += 1;

        // Classify
        let classification = classify_memory(content);

        // Check if already has correct type tag (idempotency)
        let existing_type = existing_tags.iter()
            .find(|t| t.starts_with("type:"))
            .and_then(|t| t.strip_prefix("type:"))
            .and_then(|t| MemoryType::from_str(t).ok());

        let needs_retype = existing_type != Some(classification.memory_type)
            && classification.memory_type != MemoryType::Unknown;

        // Template
        let templated = apply_template(content, classification.memory_type);
        let needs_template = !templated.preserved_original && classification.memory_type != MemoryType::Unknown;

        // Skip if already correct
        if !needs_retype && !needs_template {
            changes_skipped += 1;
            continue;
        }

        // Record changes
        if needs_retype {
            let old_type_str = existing_type.map(|t| t.as_str());
            result.add_retype(memory_id, old_type_str, &classification.memory_type, classification.confidence);
        }
        if needs_template {
            result.add_template(memory_id, &classification.memory_type, templated.preserved_original, templated.warnings.clone());
        }

        // Apply changes if not dry-run
        let was_applied = if !dry_run {
            if let Some(ref client) = memory_client {
                let mut new_tags = existing_tags.clone();
                new_tags.retain(|t| !t.starts_with("type:"));
                new_tags.push(format!("type:{}", classification.memory_type.as_str()));

                let change = MemoryChange {
                    content: if needs_template { Some(templated.content.clone()) } else { None },
                    tags: Some(new_tags),
                    importance: None,
                };

                match client.update_memory(memory_id, &change) {
                    Ok(()) => {
                        changes_applied += 1;
                        if flags.verbose {
                            push_output(widget, vec![format!("  Applied: {} -> type:{}", memory_id, classification.memory_type.as_str())], HistoryCellType::Notice);
                        }
                        true
                    }
                    Err(e) => {
                        tracing::warn!("Failed to update memory {}: {}", memory_id, e);
                        if flags.verbose {
                            push_output(widget, vec![format!("  Failed: {} - {}", memory_id, e)], HistoryCellType::Error);
                        }
                        false
                    }
                }
            } else {
                false
            }
        } else {
            false
        };

        // P99: Log changes to audit trail
        if let (Some(db), Some(sweep_db_id)) = (&overlay_db, audit_sweep_id) {
            let audit = LibrarianAudit::new(db.conn());
            let change_type = match (needs_retype, needs_template) {
                (true, true) => ChangeType::Both,
                (true, false) => ChangeType::Retype,
                (false, true) => ChangeType::Template,
                _ => continue, // shouldn't happen due to earlier skip
            };

            let input = ChangeInput {
                memory_id: memory_id.clone(),
                change_type,
                old_type: existing_type.map(|t| t.as_str().to_string()),
                new_type: Some(classification.memory_type.as_str().to_string()),
                old_content: if needs_template { Some(content.clone()) } else { None },
                new_content: if needs_template { Some(templated.content.clone()) } else { None },
                confidence: Some(classification.confidence as f64),
                applied: was_applied,
            };

            if let Err(e) = audit.log_change(sweep_db_id, &input) {
                tracing::warn!("Failed to log change to audit: {}", e);
            }
        }

        // Detect causal language
        let causal_patterns = detect_causal_language(content);
        if !causal_patterns.is_empty() && flags.verbose {
            tracing::debug!(memory_id = memory_id, patterns = causal_patterns.len(), "Detected causal language");
        }
    }

    // Process causal relationships
    // For each memory with causal language, find relationships to other memories
    let candidates: Vec<_> = memories.iter()
        .map(|(id, content, _)| (id.clone(), content.clone()))
        .collect();

    let mut edges_applied = 0;
    for (memory_id, content, _) in memories.iter() {
        let causal_patterns = detect_causal_language(content);
        if !causal_patterns.is_empty() && candidates.len() > 1 {
            let edges = infer_relationships(memory_id, content, &candidates);
            for edge in &edges {
                result.add_causal_edge(edge);

                // P99: Write causal edge to local-memory when --apply
                let edge_applied = if !dry_run {
                    if let Some(ref rel_client) = relationships_client {
                        let input = RelationshipInput::new(
                            &edge.source_id,
                            &edge.target_id,
                            edge.relation.to_string(),
                            edge.confidence,
                        )
                        .with_context(&edge.trigger_phrase);

                        match rel_client.create_relationship(&input) {
                            Ok(()) => {
                                edges_applied += 1;
                                if flags.verbose {
                                    push_output(
                                        widget,
                                        vec![format!(
                                            "  Edge: {} -> {} ({})",
                                            edge.source_id, edge.target_id, edge.relation
                                        )],
                                        HistoryCellType::Notice,
                                    );
                                }
                                true
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to create relationship {} -> {}: {}",
                                    edge.source_id, edge.target_id, e
                                );
                                false
                            }
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                // P99: Log edge to audit trail
                if let (Some(db), Some(sweep_db_id)) = (&overlay_db, audit_sweep_id) {
                    let audit = LibrarianAudit::new(db.conn());
                    let input = EdgeInput {
                        from_id: edge.source_id.clone(),
                        to_id: edge.target_id.clone(),
                        relation_type: edge.relation.to_string(),
                        reason: Some(edge.trigger_phrase.clone()),
                        applied: edge_applied,
                    };

                    if let Err(e) = audit.log_edge(sweep_db_id, &input) {
                        tracing::warn!("Failed to log edge to audit: {}", e);
                    }
                }
            }
        }
    }

    result.summary.duration_ms = start_time.elapsed().as_millis() as u64;

    // Output result
    if flags.json_report {
        match result.to_json() {
            Ok(json) => push_output(widget, vec![json], HistoryCellType::Notice),
            Err(e) => push_output(widget, vec![format!("Error generating JSON: {}", e)], HistoryCellType::Error),
        }
    } else {
        let mut summary_lines = vec![String::new(), "=== Results ===".to_string(), result.summary_text()];
        if !dry_run {
            summary_lines.push(format!("Changes applied: {}", changes_applied));
            if edges_applied > 0 {
                summary_lines.push(format!("Edges created: {}", edges_applied));
            }
        }
        if changes_skipped > 0 { summary_lines.push(format!("Skipped (already correct): {}", changes_skipped)); }
        push_output(widget, summary_lines, HistoryCellType::Notice);

        if !result.changes.is_empty() && (flags.verbose || result.changes.len() <= 10) {
            let mut change_lines = vec![String::new(), "Changes:".to_string()];
            let max_show = if flags.verbose { result.changes.len() } else { 5 };

            for change in result.changes.iter().take(max_show) {
                match change {
                    codex_stage0::librarian::SweepChange::Retype { memory_id, new_type, confidence, .. } => {
                        change_lines.push(format!("  - {}: {} (confidence: {:.2})", memory_id, new_type, confidence));
                    }
                    codex_stage0::librarian::SweepChange::FlaggedForReview { memory_id, reason } => {
                        change_lines.push(format!("  - {}: Flagged - {}", memory_id, reason));
                    }
                    _ => {}
                }
            }

            if result.changes.len() > max_show {
                change_lines.push(format!("  ... and {} more changes", result.changes.len() - max_show));
            }
            push_output(widget, change_lines, HistoryCellType::Notice);
        }

        if dry_run {
            push_output(widget, vec![String::new(), "Note: This was a dry run. Use --apply to write changes.".to_string()], HistoryCellType::Notice);
        }
    }

    // P99: Complete audit sweep
    if let (Some(db), Some(sweep_db_id)) = (&overlay_db, audit_sweep_id) {
        let audit = LibrarianAudit::new(db.conn());
        let stats_json = serde_json::to_string(&serde_json::json!({
            "memories_scanned": result.summary.memories_scanned,
            "memories_retyped": result.summary.memories_retyped,
            "memories_templated": result.summary.memories_templated,
            "causal_edges_created": result.summary.causal_edges_created,
            "unknown_flagged": result.summary.unknown_flagged,
            "changes_applied": changes_applied,
            "edges_applied": edges_applied,
            "changes_skipped": changes_skipped,
            "duration_ms": result.summary.duration_ms,
        }))
        .unwrap_or_default();

        if let Err(e) = audit.complete_sweep(sweep_db_id, &stats_json) {
            tracing::warn!("Failed to complete audit sweep: {}", e);
        }
    }

    tracing::info!(
        target: "stage0.librarian",
        sweep_id = %sweep_id,
        dry_run = dry_run,
        memories_scanned = result.summary.memories_scanned,
        memories_retyped = result.summary.memories_retyped,
        memories_templated = result.summary.memories_templated,
        causal_edges_created = result.summary.causal_edges_created,
        unknown_flagged = result.summary.unknown_flagged,
        duration_ms = result.summary.duration_ms,
        changes_applied = changes_applied,
        edges_applied = edges_applied,
        changes_skipped = changes_skipped,
        "LibrarianSweepRun"
    );
}

fn execute_status(widget: &mut ChatWidget) {
    let lines = vec![
        "=== Librarian Status ===".to_string(),
        String::new(),
        "The Librarian module is available for memory corpus quality operations.".to_string(),
        String::new(),
        "Capabilities:".to_string(),
        "  - Memory type classification (Pattern, Decision, Problem, Insight, Exception, Reference)".to_string(),
        "  - Template restructuring (CONTEXT/REASONING/OUTCOME/TAGS)".to_string(),
        "  - Causal relationship inference (Causes, Blocks, Enables)".to_string(),
        String::new(),
        "Run /stage0.librarian sweep to preview changes (dry-run by default).".to_string(),
        "Run /stage0.librarian sweep --apply to write changes.".to_string(),
        "Run /stage0.librarian history to view past sweeps.".to_string(),
    ];
    push_output(widget, lines, HistoryCellType::Notice);
}

/// P99: Execute history subcommand
fn execute_history(widget: &mut ChatWidget, args: &[&str]) {
    use codex_stage0::librarian::LibrarianAudit;

    // Parse limit argument
    let limit: usize = args
        .iter()
        .find_map(|a| a.strip_prefix("--limit="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let show_detail = args.iter().any(|a| *a == "--detail" || *a == "-d");

    // Connect to OverlayDb
    let db = match codex_stage0::Stage0Config::load()
        .and_then(|cfg| codex_stage0::OverlayDb::connect_and_init(&cfg))
    {
        Ok(d) => d,
        Err(e) => {
            push_output(
                widget,
                vec![format!("Failed to connect to overlay DB: {}", e)],
                HistoryCellType::Error,
            );
            return;
        }
    };

    let audit = LibrarianAudit::new(db.conn());

    // List recent sweeps
    let sweeps = match audit.list_sweeps(limit) {
        Ok(s) => s,
        Err(e) => {
            push_output(
                widget,
                vec![format!("Failed to list sweeps: {}", e)],
                HistoryCellType::Error,
            );
            return;
        }
    };

    if sweeps.is_empty() {
        push_output(
            widget,
            vec![
                "=== Librarian History ===".to_string(),
                String::new(),
                "No sweep history found.".to_string(),
                String::new(),
                "Run /stage0.librarian sweep to perform your first sweep.".to_string(),
            ],
            HistoryCellType::Notice,
        );
        return;
    }

    let mut lines = vec![
        "=== Librarian History ===".to_string(),
        String::new(),
        format!("Showing {} most recent sweep(s):", sweeps.len()),
        String::new(),
    ];

    for sweep in &sweeps {
        let status_emoji = match sweep.status {
            codex_stage0::librarian::SweepStatus::Completed => "✓",
            codex_stage0::librarian::SweepStatus::Running => "⏳",
            codex_stage0::librarian::SweepStatus::Failed => "✗",
        };

        let duration = sweep
            .finished_at
            .map(|end| (end - sweep.started_at).num_milliseconds())
            .unwrap_or(0);

        lines.push(format!(
            "{} {} | {} | {}ms",
            status_emoji,
            sweep.run_id,
            sweep.started_at.format("%Y-%m-%d %H:%M"),
            duration
        ));

        if show_detail {
            // Parse and show stats if available
            if let Some(ref stats) = sweep.stats_json {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(stats) {
                    let scanned = json.get("memories_scanned").and_then(|v| v.as_u64()).unwrap_or(0);
                    let retyped = json.get("memories_retyped").and_then(|v| v.as_u64()).unwrap_or(0);
                    let edges = json.get("causal_edges_created").and_then(|v| v.as_u64()).unwrap_or(0);
                    let applied = json.get("changes_applied").and_then(|v| v.as_u64()).unwrap_or(0);

                    lines.push(format!(
                        "    Scanned: {} | Retyped: {} | Edges: {} | Applied: {}",
                        scanned, retyped, edges, applied
                    ));
                }
            }

            // Show change/edge counts from audit tables
            if let (Ok(changes), Ok(edges)) = (
                audit.count_changes(sweep.id),
                audit.count_edges(sweep.id),
            ) {
                if changes > 0 || edges > 0 {
                    lines.push(format!("    Audit: {} changes, {} edges logged", changes, edges));
                }
            }
        }
    }

    if !show_detail {
        lines.push(String::new());
        lines.push("Use --detail to show stats for each sweep.".to_string());
    }

    push_output(widget, lines, HistoryCellType::Notice);
}

fn show_help(widget: &mut ChatWidget) {
    let lines = vec![
        "=== /stage0.librarian - Memory Corpus Quality Engine ===".to_string(),
        String::new(),
        "Usage: /stage0.librarian <subcommand> [flags]".to_string(),
        String::new(),
        "Subcommands:".to_string(),
        "  sweep    Run classification, templating, and causal inference".to_string(),
        "  history  View past sweep runs and audit trail".to_string(),
        "  status   Show librarian module status".to_string(),
        "  help     Show this help".to_string(),
        String::new(),
        "Sweep Flags:".to_string(),
        "  --apply              Apply changes (default: dry-run)".to_string(),
        "  --dry-run            Preview changes without writing (default)".to_string(),
        "  --domains=<list>     Filter by domain (comma-separated)".to_string(),
        "  --limit=<N>          Process max N memories (default: 100)".to_string(),
        "  --min-importance=<N> Only process memories >= importance".to_string(),
        "  --json               Output diff as JSON for CI".to_string(),
        "  --verbose, -v        Show detailed progress".to_string(),
        String::new(),
        "History Flags:".to_string(),
        "  --limit=<N>          Show last N sweeps (default: 10)".to_string(),
        "  --detail, -d         Show statistics for each sweep".to_string(),
        String::new(),
        "Examples:".to_string(),
        "  /stage0.librarian sweep                        # Dry-run with defaults".to_string(),
        "  /stage0.librarian sweep --apply                # Apply changes".to_string(),
        "  /stage0.librarian sweep --domains=spec-kit    # Filter to domain".to_string(),
        "  /stage0.librarian sweep --json --verbose      # Detailed JSON output".to_string(),
        "  /stage0.librarian history --detail            # View sweep history".to_string(),
        String::new(),
        "SPEC-KIT-103 | P99 Session".to_string(),
    ];
    push_output(widget, lines, HistoryCellType::Notice);
}

/// Sample memories for MVP demonstration
fn get_sample_memories() -> Vec<(String, String)> {
    vec![
        ("mem-sample-001".to_string(), "Decision: Use SQLite for the overlay database because we need embedded storage with ACID guarantees.".to_string()),
        ("mem-sample-002".to_string(), "Pattern: Always validate spec IDs before processing. Check format matches SPEC-XXX-NNN pattern.".to_string()),
        ("mem-sample-003".to_string(), "Bug: Cache invalidation failed when TTL exceeded 24 hours. Fixed by adding explicit timestamp comparison.".to_string()),
        ("mem-sample-004".to_string(), "TIL: The Rust borrow checker caught a potential data race in our async code. Lesson: trust the compiler.".to_string()),
        ("mem-sample-005".to_string(), "Constitution Exception: SPEC-KIT-102 bypasses the 'no direct SQL' guardrail because we need custom aggregation queries.".to_string()),
        ("mem-sample-006".to_string(), "See: https://docs.rust-lang.org/book/ for Rust language reference.".to_string()),
        ("mem-sample-007".to_string(), "Random implementation note without clear category.".to_string()),
        ("mem-sample-008".to_string(), "The cache bug caused memory leaks which blocked the deployment pipeline.".to_string()),
        ("mem-sample-009".to_string(), "Decision: Chose chrono over time crate because of better timezone support.".to_string()),
        ("mem-sample-010".to_string(), "Pattern: Use builder pattern for complex configuration structs. Standard approach across codebase.".to_string()),
    ]
}
