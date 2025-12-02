//! Librarian command implementations
//!
//! SPEC-KIT-103: Memory corpus quality engine commands

use crate::chatwidget::ChatWidget;
use crate::chatwidget::spec_kit::command_registry::SpecKitCommand;
use crate::history_cell::{HistoryCellType, PlainHistoryCell};
use ratatui::text::Line;

/// Command: /stage0.librarian sweep
///
/// Runs the librarian sweep operation to classify, template, and infer
/// causal relationships in the memory corpus.
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
        // Parse subcommand and flags
        let parts: Vec<&str> = args.split_whitespace().collect();

        let subcommand = parts.first().copied().unwrap_or("help");

        match subcommand {
            "sweep" => execute_sweep(widget, &parts[1..]),
            "status" => execute_status(widget),
            "help" | _ => show_help(widget),
        }
    }
}

/// Parse sweep flags from arguments
struct SweepFlags {
    dry_run: bool,
    domains: Vec<String>,
    limit: usize,
    min_importance: i32,
    json_report: bool,
}

impl Default for SweepFlags {
    fn default() -> Self {
        Self {
            dry_run: false,
            domains: Vec::new(),
            limit: 100, // Default limit for safety
            min_importance: 0,
            json_report: false,
        }
    }
}

fn parse_sweep_flags(args: &[&str]) -> SweepFlags {
    let mut flags = SweepFlags::default();

    for arg in args {
        if *arg == "--dry-run" {
            flags.dry_run = true;
        } else if *arg == "--json-report" {
            flags.json_report = true;
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
    use codex_stage0::librarian::{
        SweepConfig, SweepResult, classify_memory, apply_template, detect_causal_language,
        MemoryType,
    };

    let flags = parse_sweep_flags(args);

    // Build sweep configuration
    let config = SweepConfig {
        dry_run: flags.dry_run,
        domains: flags.domains.clone(),
        limit: flags.limit,
        min_importance: flags.min_importance,
        json_report: flags.json_report,
        ..Default::default()
    };

    // Create sweep result to track changes
    let sweep_id = format!(
        "sweep-{}-{:03}",
        chrono::Utc::now().format("%Y%m%d"),
        rand::random::<u16>() % 1000
    );
    let mut result = SweepResult::new(&sweep_id, config.dry_run);
    result.config = Some(config.clone());

    let start_time = std::time::Instant::now();

    // Output header
    let mode_str = if flags.dry_run { " (DRY RUN)" } else { "" };
    let header = vec![
        format!("=== Librarian Sweep{} ===", mode_str),
        format!("Sweep ID: {}", sweep_id),
        format!("Limit: {} memories", flags.limit),
        format!("Min Importance: {}", flags.min_importance),
        format!(
            "Domains: {}",
            if flags.domains.is_empty() {
                "all".to_string()
            } else {
                flags.domains.join(", ")
            }
        ),
        String::new(),
    ];
    push_output(widget, header, HistoryCellType::Notice);

    // For MVP, process sample data to demonstrate the pipeline works
    // In production, this would query local-memory MCP
    let sample_memories = get_sample_memories();

    for (memory_id, content) in sample_memories.iter().take(flags.limit) {
        result.summary.memories_scanned += 1;

        // Classify
        let classification = classify_memory(content);

        // Check if we should process based on flags
        if flags.min_importance > 0 {
            let type_priority = classification.memory_type.default_priority();
            if type_priority < flags.min_importance {
                continue;
            }
        }

        // Template
        let templated = apply_template(content, classification.memory_type);

        // Detect causal language
        let causal_patterns = detect_causal_language(content);

        // Record changes
        if classification.memory_type != MemoryType::Unknown {
            result.add_retype(
                memory_id,
                None,
                &classification.memory_type,
                classification.confidence,
            );
        } else {
            result.flag_for_review(memory_id, "No clear type signals detected");
        }

        if !templated.preserved_original {
            result.add_template(
                memory_id,
                &classification.memory_type,
                templated.preserved_original,
                templated.warnings.clone(),
            );
        }

        // Note causal patterns (edge creation would require cross-memory analysis)
        if !causal_patterns.is_empty() {
            tracing::debug!(
                memory_id = memory_id,
                patterns = causal_patterns.len(),
                "Detected causal language"
            );
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
        // Summary output
        let summary_lines = vec![
            String::new(),
            "=== Results ===".to_string(),
            result.summary_text(),
        ];
        push_output(widget, summary_lines, HistoryCellType::Notice);

        // Show sample changes
        if !result.changes.is_empty() {
            let mut change_lines = vec![String::new(), "Sample Changes:".to_string()];

            for change in result.changes.iter().take(5) {
                match change {
                    codex_stage0::librarian::SweepChange::Retype {
                        memory_id,
                        new_type,
                        confidence,
                        ..
                    } => {
                        change_lines.push(format!(
                            "  - {}: {} (confidence: {:.2})",
                            memory_id, new_type, confidence
                        ));
                    }
                    codex_stage0::librarian::SweepChange::FlaggedForReview {
                        memory_id,
                        reason,
                    } => {
                        change_lines.push(format!(
                            "  - {}: Flagged - {}",
                            memory_id, reason
                        ));
                    }
                    _ => {}
                }
            }

            if result.changes.len() > 5 {
                change_lines.push(format!("  ... and {} more changes", result.changes.len() - 5));
            }

            push_output(widget, change_lines, HistoryCellType::Notice);
        }

        if flags.dry_run {
            push_output(
                widget,
                vec![String::new(), "Note: This was a dry run. No changes were written.".to_string()],
                HistoryCellType::Notice,
            );
        }
    }

    // Log telemetry event
    tracing::info!(
        target: "stage0.librarian",
        sweep_id = %sweep_id,
        dry_run = flags.dry_run,
        memories_scanned = result.summary.memories_scanned,
        memories_retyped = result.summary.memories_retyped,
        memories_templated = result.summary.memories_templated,
        causal_edges_created = result.summary.causal_edges_created,
        unknown_flagged = result.summary.unknown_flagged,
        duration_ms = result.summary.duration_ms,
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
        "Run /stage0.librarian sweep --dry-run to preview changes.".to_string(),
    ];
    push_output(widget, lines, HistoryCellType::Notice);
}

fn show_help(widget: &mut ChatWidget) {
    let lines = vec![
        "=== /stage0.librarian - Memory Corpus Quality Engine ===".to_string(),
        String::new(),
        "Usage: /stage0.librarian <subcommand> [flags]".to_string(),
        String::new(),
        "Subcommands:".to_string(),
        "  sweep   Run classification, templating, and causal inference".to_string(),
        "  status  Show librarian module status".to_string(),
        "  help    Show this help".to_string(),
        String::new(),
        "Sweep Flags:".to_string(),
        "  --dry-run             Preview changes without writing".to_string(),
        "  --domains=<list>      Filter by domain (comma-separated)".to_string(),
        "  --limit=<N>           Process max N memories (default: 100)".to_string(),
        "  --min-importance=<N>  Only process memories >= importance".to_string(),
        "  --json-report         Output diff as JSON for CI".to_string(),
        String::new(),
        "Examples:".to_string(),
        "  /stage0.librarian sweep --dry-run".to_string(),
        "  /stage0.librarian sweep --domains=spec-kit --limit=50".to_string(),
        "  /stage0.librarian sweep --json-report".to_string(),
        String::new(),
        "SPEC-KIT-103 | P97 Session".to_string(),
    ];
    push_output(widget, lines, HistoryCellType::Notice);
}

/// Sample memories for MVP demonstration
/// In production, these would come from local-memory MCP
fn get_sample_memories() -> Vec<(String, String)> {
    vec![
        (
            "mem-sample-001".to_string(),
            "Decision: Use SQLite for the overlay database because we need embedded storage with ACID guarantees. Trade-off is less scalability but acceptable for our use case.".to_string(),
        ),
        (
            "mem-sample-002".to_string(),
            "Pattern: Always validate spec IDs before processing. Check format matches SPEC-XXX-NNN pattern.".to_string(),
        ),
        (
            "mem-sample-003".to_string(),
            "Bug: Cache invalidation failed when TTL exceeded 24 hours. Fixed by adding explicit timestamp comparison.".to_string(),
        ),
        (
            "mem-sample-004".to_string(),
            "TIL: The Rust borrow checker caught a potential data race in our async code. Lesson: trust the compiler.".to_string(),
        ),
        (
            "mem-sample-005".to_string(),
            "Constitution Exception: SPEC-KIT-102 bypasses the 'no direct SQL' guardrail because we need custom aggregation queries.".to_string(),
        ),
        (
            "mem-sample-006".to_string(),
            "See: https://docs.rust-lang.org/book/ for Rust language reference.".to_string(),
        ),
        (
            "mem-sample-007".to_string(),
            "Random implementation note without clear category.".to_string(),
        ),
        (
            "mem-sample-008".to_string(),
            "The cache bug caused memory leaks which blocked the deployment pipeline.".to_string(),
        ),
        (
            "mem-sample-009".to_string(),
            "Decision: Chose chrono over time crate because of better timezone support.".to_string(),
        ),
        (
            "mem-sample-010".to_string(),
            "Pattern: Use builder pattern for complex configuration structs. Standard approach across codebase.".to_string(),
        ),
    ]
}
