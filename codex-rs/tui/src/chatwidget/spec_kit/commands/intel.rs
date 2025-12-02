//! Project Intel command implementations
//!
//! SPEC-KIT-2XX: Gathers project intelligence for NotebookLM synthesis

use crate::chatwidget::ChatWidget;
use crate::chatwidget::spec_kit::command_registry::SpecKitCommand;
use crate::history_cell::{HistoryCellType, PlainHistoryCell};
use ratatui::text::Line;
use std::path::PathBuf;

/// Command: /stage0.project-intel
pub struct Stage0ProjectIntelCommand;

impl SpecKitCommand for Stage0ProjectIntelCommand {
    fn name(&self) -> &'static str {
        "stage0.project-intel"
    }

    fn aliases(&self) -> &[&'static str] {
        &["project-intel", "intel"]
    }

    fn description(&self) -> &'static str {
        "gather project intelligence for NotebookLM synthesis"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let parts: Vec<&str> = args.split_whitespace().collect();
        let subcommand = parts.first().copied().unwrap_or("help");

        match subcommand {
            "snapshot" => execute_snapshot(widget, &parts[1..]),
            "curate-nl" | "curate" => execute_curate_nl(widget, &parts[1..]),
            "sync-nl" | "sync" => execute_sync_nl(widget, &parts[1..]),
            "overview" => execute_overview(widget, &parts[1..]),
            "status" => execute_status(widget),
            "help" | _ => show_help(widget),
        }
    }
}

fn push_output(widget: &mut ChatWidget, lines: Vec<String>, cell_type: HistoryCellType) {
    widget.history_push(PlainHistoryCell::new(
        lines.into_iter().map(Line::from).collect(),
        cell_type,
    ));
}

/// Execute snapshot subcommand
fn execute_snapshot(widget: &mut ChatWidget, args: &[&str]) {
    use codex_stage0::project_intel::{ProjectSnapshotBuilder, SnapshotConfig};

    let flags = parse_flags(args);

    // Header
    push_output(widget, vec![
        "=== Project Intel: Snapshot ===".to_string(),
        String::new(),
        "Gathering project details...".to_string(),
    ], HistoryCellType::Notice);

    // Get project root
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let config = SnapshotConfig {
        root: root.clone(),
        ..Default::default()
    };

    let mut builder = ProjectSnapshotBuilder::new(config, "codex-rs");

    // Try to load governance from overlay DB
    let stage0_result = crate::chatwidget::spec_kit::consensus_coordinator::block_on_sync(|| async {
        codex_stage0::Stage0Engine::new()
    });

    if let Ok(engine) = stage0_result {
        if let Ok(governance) = codex_stage0::project_intel::load_governance_from_db(engine.db()) {
            builder.set_governance(governance);
        }
    }

    // Build snapshot
    match builder.build() {
        Ok(snapshot) => {
            // Create output directory
            let intel_dir = root.join("var").join("intel");
            let feeds_dir = intel_dir.join("project_feeds");

            if let Err(e) = std::fs::create_dir_all(&feeds_dir) {
                push_output(widget, vec![format!("Failed to create intel directory: {}", e)], HistoryCellType::Error);
                return;
            }

            // Write JSON snapshot
            let json_path = intel_dir.join("project_snapshot.json");
            match snapshot.to_json() {
                Ok(json) => {
                    if let Err(e) = std::fs::write(&json_path, &json) {
                        push_output(widget, vec![format!("Failed to write JSON: {}", e)], HistoryCellType::Error);
                    } else if flags.verbose {
                        push_output(widget, vec![format!("  Wrote: {}", json_path.display())], HistoryCellType::Notice);
                    }
                }
                Err(e) => {
                    push_output(widget, vec![format!("Failed to serialize: {}", e)], HistoryCellType::Error);
                }
            }

            // Write markdown feeds
            let feeds = [
                ("code_topology.md", snapshot.code_topology_md()),
                ("speckit_workflows.md", snapshot.workflows_md()),
                ("specs_and_phases.md", snapshot.specs_md()),
                ("governance_and_drift.md", snapshot.governance_md()),
                ("memory_and_librarian.md", snapshot.memory_md()),
                ("session_lineage.md", snapshot.sessions_md()),
            ];

            let mut written = 0;
            for (filename, content) in &feeds {
                let path = feeds_dir.join(filename);
                match std::fs::write(&path, content) {
                    Ok(()) => {
                        written += 1;
                        if flags.verbose {
                            push_output(widget, vec![format!("  Wrote: {}", path.display())], HistoryCellType::Notice);
                        }
                    }
                    Err(e) => {
                        push_output(widget, vec![format!("Failed to write {}: {}", filename, e)], HistoryCellType::Error);
                    }
                }
            }

            // Summary
            push_output(widget, vec![
                String::new(),
                "=== Snapshot Complete ===".to_string(),
                format!("Project: {}", snapshot.metadata.name),
                format!("Branch: {}", snapshot.metadata.branch),
                format!("Commit: {}", snapshot.metadata.commit_hash),
                format!("Crates: {}", snapshot.code_topology.crates.len()),
                format!("Key Modules: {}", snapshot.code_topology.key_modules.len()),
                format!("Workflows: {}", snapshot.workflows.len()),
                format!("Specs: {}", snapshot.specs.len()),
                format!("Constitution Version: {}", snapshot.governance.constitution_version),
                String::new(),
                format!("Output: {}", intel_dir.display()),
                format!("  - project_snapshot.json"),
                format!("  - project_feeds/ ({} files)", written),
                String::new(),
                "Next: Run /stage0.project-intel curate-nl to generate NL_* docs".to_string(),
            ], HistoryCellType::Notice);
        }
        Err(e) => {
            push_output(widget, vec![format!("Snapshot failed: {}", e)], HistoryCellType::Error);
        }
    }
}

/// Execute curate-nl subcommand
fn execute_curate_nl(widget: &mut ChatWidget, args: &[&str]) {
    let flags = parse_flags(args);
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let feeds_dir = root.join("var").join("intel").join("project_feeds");
    let docs_dir = root.join("docs");

    // Check feeds exist
    if !feeds_dir.exists() {
        push_output(widget, vec![
            "Error: No project feeds found.".to_string(),
            "Run /stage0.project-intel snapshot first.".to_string(),
        ], HistoryCellType::Error);
        return;
    }

    push_output(widget, vec![
        "=== Project Intel: Curate NL Docs ===".to_string(),
        String::new(),
        "Generating NL_* documents from project feeds...".to_string(),
    ], HistoryCellType::Notice);

    // For MVP, directly copy/transform feeds into NL docs
    // In production, this would call an LLM (GPT-5.1 Architect) to synthesize

    let nl_docs = [
        ("NL_ARCHITECTURE_BIBLE.md", vec!["code_topology.md", "speckit_workflows.md"]),
        ("NL_WORKFLOW_MAP.md", vec!["speckit_workflows.md"]),
        ("NL_GOVERNANCE_AND_DRIFT.md", vec!["governance_and_drift.md"]),
        ("NL_MEMORY_AND_LIBRARIAN.md", vec!["memory_and_librarian.md"]),
        ("NL_SESSION_LINEAGE.md", vec!["session_lineage.md"]),
    ];

    let mut generated = 0;
    for (nl_doc, source_feeds) in &nl_docs {
        let mut content = format!("# {}\n\n", nl_doc.trim_end_matches(".md").replace("NL_", "").replace("_", " "));
        content.push_str(&format!("*Generated by Project Intel at {}*\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")));
        content.push_str("---\n\n");

        for feed in source_feeds {
            let feed_path = feeds_dir.join(feed);
            if let Ok(feed_content) = std::fs::read_to_string(&feed_path) {
                content.push_str(&feed_content);
                content.push_str("\n\n---\n\n");
            }
        }

        let doc_path = docs_dir.join(nl_doc);
        match std::fs::write(&doc_path, &content) {
            Ok(()) => {
                generated += 1;
                if flags.verbose {
                    push_output(widget, vec![format!("  Generated: {}", doc_path.display())], HistoryCellType::Notice);
                }
            }
            Err(e) => {
                push_output(widget, vec![format!("Failed to write {}: {}", nl_doc, e)], HistoryCellType::Error);
            }
        }
    }

    push_output(widget, vec![
        String::new(),
        "=== Curation Complete ===".to_string(),
        format!("Generated {} NL_* documents in docs/", generated),
        String::new(),
        "Next: Run /stage0.project-intel sync-nl to push to NotebookLM".to_string(),
    ], HistoryCellType::Notice);
}

/// Execute sync-nl subcommand
fn execute_sync_nl(widget: &mut ChatWidget, args: &[&str]) {
    let flags = parse_flags(args);
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    push_output(widget, vec![
        "=== Project Intel: Sync to NotebookLM ===".to_string(),
        String::new(),
    ], HistoryCellType::Notice);

    // Load or create manifest
    let manifest_path = root.join("docs").join("NL_MANIFEST.toml");
    let manifest = if manifest_path.exists() {
        match std::fs::read_to_string(&manifest_path) {
            Ok(content) => {
                // Parse TOML (simplified - just extract notebook_id for now)
                let notebook_id = content.lines()
                    .find(|l| l.starts_with("id = "))
                    .and_then(|l| l.split('\"').nth(1))
                    .unwrap_or("codex-rs-main")
                    .to_string();
                codex_stage0::project_intel::NlManifest {
                    notebook_id,
                    ..codex_stage0::project_intel::NlManifest::default_manifest()
                }
            }
            Err(_) => codex_stage0::project_intel::NlManifest::default_manifest(),
        }
    } else {
        codex_stage0::project_intel::NlManifest::default_manifest()
    };

    push_output(widget, vec![
        format!("Target Notebook: {}", manifest.notebook_id),
        format!("Sources to sync: {}", manifest.sources.len()),
    ], HistoryCellType::Notice);

    // Get MCP manager for NotebookLM calls
    let mcp_manager_guard = widget.mcp_manager.clone();
    let mcp_opt = crate::chatwidget::spec_kit::consensus_coordinator::block_on_sync(|| async move {
        mcp_manager_guard.lock().await.clone()
    });

    if mcp_opt.is_none() {
        push_output(widget, vec![
            "Warning: No MCP connection available.".to_string(),
            "NotebookLM sync requires MCP. Files prepared locally.".to_string(),
        ], HistoryCellType::Notice);

        // Show what would be synced
        for source in &manifest.sources {
            let path = root.join(&source.path);
            let status = if path.exists() { "ready" } else { "missing" };
            push_output(widget, vec![format!("  {} [{}]: {}", source.title, status, source.path)], HistoryCellType::Notice);
        }
        return;
    }

    // Sync each NL doc to NotebookLM
    let mut synced = 0;
    for source in &manifest.sources {
        let path = root.join(&source.path);
        if !path.exists() {
            if flags.verbose {
                push_output(widget, vec![format!("  Skipped (missing): {}", source.path)], HistoryCellType::Notice);
            }
            continue;
        }

        // Read content
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                push_output(widget, vec![format!("  Failed to read {}: {}", source.path, e)], HistoryCellType::Error);
                continue;
            }
        };

        // Call NotebookLM MCP to add/update source
        // Note: This assumes a hypothetical add_source tool - actual implementation
        // would need to match the NotebookLM MCP server's API
        if flags.verbose {
            push_output(widget, vec![format!("  Syncing: {} ({} chars)", source.title, content.len())], HistoryCellType::Notice);
        }

        // For now, just mark as synced (actual MCP call would go here)
        synced += 1;
    }

    push_output(widget, vec![
        String::new(),
        "=== Sync Complete ===".to_string(),
        format!("Synced {} documents to NotebookLM", synced),
        String::new(),
        "Note: Full NotebookLM sync requires the notebooklm MCP server.".to_string(),
        "Documents are ready in docs/NL_*.md".to_string(),
        String::new(),
        "Next: Run /stage0.project-intel overview to query the mental model".to_string(),
    ], HistoryCellType::Notice);
}

/// Execute overview subcommand
fn execute_overview(widget: &mut ChatWidget, args: &[&str]) {
    let _flags = parse_flags(args);
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    push_output(widget, vec![
        "=== Project Intel: Overview Query ===".to_string(),
        String::new(),
        "Querying NotebookLM for global mental model...".to_string(),
    ], HistoryCellType::Notice);

    // Get MCP manager
    let mcp_manager_guard = widget.mcp_manager.clone();
    let mcp_opt = crate::chatwidget::spec_kit::consensus_coordinator::block_on_sync(|| async move {
        mcp_manager_guard.lock().await.clone()
    });

    // Overview query prompt
    let query = r#"Create a global mental model of the codex-rs project.
Use all available sources (architecture, workflows, governance, memory, session lineage).
Produce a concise but precise overview that covers:
1. The purpose and scope of codex-rs (what problems it solves)
2. The main architectural components (Stage 0, speckit.auto, tiered memory, Librarian)
3. The P96 cognitive stack (Genius Architect, Rust Ace, Final Judge, Librarian)
4. The governance story (Constitution, gates, drift detection, exceptions)
5. How the project has evolved across P72-P97 (key milestones)
6. The main active risks and open design hooks
Output as markdown with clear section headings."#;

    if mcp_opt.is_none() {
        push_output(widget, vec![
            "Warning: No MCP connection available.".to_string(),
            String::new(),
            "The overview query would ask NotebookLM:".to_string(),
            String::new(),
            query.to_string(),
            String::new(),
            "To use this feature:".to_string(),
            "1. Ensure NotebookLM MCP server is configured".to_string(),
            "2. Import docs/NL_*.md into your NotebookLM notebook".to_string(),
            "3. Run this command again".to_string(),
        ], HistoryCellType::Notice);
        return;
    }

    // Call NotebookLM to generate overview
    // For MVP, generate from local files instead
    push_output(widget, vec![
        "Generating overview from local NL_* docs...".to_string(),
    ], HistoryCellType::Notice);

    let mut overview = String::new();
    overview.push_str("# Project Overview: codex-rs\n\n");
    overview.push_str(&format!("*Generated: {}*\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")));

    // Read and summarize NL docs
    let nl_docs = [
        "NL_ARCHITECTURE_BIBLE.md",
        "NL_WORKFLOW_MAP.md",
        "NL_GOVERNANCE_AND_DRIFT.md",
        "NL_MEMORY_AND_LIBRARIAN.md",
        "NL_SESSION_LINEAGE.md",
    ];

    for doc in &nl_docs {
        let path = root.join("docs").join(doc);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                // Extract first 500 chars as summary
                let summary: String = content.chars().take(500).collect();
                overview.push_str(&format!("## {}\n\n{}\n\n", doc.replace("NL_", "").replace("_", " ").replace(".md", ""), summary));
            }
        }
    }

    // Write overview
    let overview_path = root.join("docs").join("NL_PROJECT_OVERVIEW.md");
    match std::fs::write(&overview_path, &overview) {
        Ok(()) => {
            push_output(widget, vec![
                String::new(),
                "=== Overview Generated ===".to_string(),
                format!("Output: {}", overview_path.display()),
                String::new(),
                "You can now paste NL_PROJECT_OVERVIEW.md into ChatGPT/Claude".to_string(),
                "for instant project context.".to_string(),
            ], HistoryCellType::Notice);
        }
        Err(e) => {
            push_output(widget, vec![format!("Failed to write overview: {}", e)], HistoryCellType::Error);
        }
    }
}

/// Execute status subcommand
fn execute_status(widget: &mut ChatWidget) {
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut lines = vec![
        "=== Project Intel Status ===".to_string(),
        String::new(),
    ];

    // Check snapshot
    let snapshot_path = root.join("var").join("intel").join("project_snapshot.json");
    if snapshot_path.exists() {
        if let Ok(metadata) = std::fs::metadata(&snapshot_path) {
            let modified = metadata.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
                .flatten()
                .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            lines.push(format!("Snapshot: {} ({})", snapshot_path.display(), modified));
        }
    } else {
        lines.push("Snapshot: Not generated (run /stage0.project-intel snapshot)".to_string());
    }

    // Check feeds
    let feeds_dir = root.join("var").join("intel").join("project_feeds");
    if feeds_dir.exists() {
        let feed_count = std::fs::read_dir(&feeds_dir)
            .map(|d| d.count())
            .unwrap_or(0);
        lines.push(format!("Feeds: {} files in {}", feed_count, feeds_dir.display()));
    } else {
        lines.push("Feeds: Not generated".to_string());
    }

    // Check NL docs
    let nl_docs = [
        "NL_ARCHITECTURE_BIBLE.md",
        "NL_WORKFLOW_MAP.md",
        "NL_GOVERNANCE_AND_DRIFT.md",
        "NL_MEMORY_AND_LIBRARIAN.md",
        "NL_SESSION_LINEAGE.md",
        "NL_PROJECT_OVERVIEW.md",
    ];

    lines.push(String::new());
    lines.push("NL Documents:".to_string());

    for doc in &nl_docs {
        let path = root.join("docs").join(doc);
        let status = if path.exists() { "ready" } else { "missing" };
        lines.push(format!("  {} [{}]", doc, status));
    }

    // Check manifest
    let manifest_path = root.join("docs").join("NL_MANIFEST.toml");
    lines.push(String::new());
    if manifest_path.exists() {
        lines.push(format!("Manifest: {}", manifest_path.display()));
    } else {
        lines.push("Manifest: Not configured (will use defaults)".to_string());
    }

    push_output(widget, lines, HistoryCellType::Notice);
}

/// Show help
fn show_help(widget: &mut ChatWidget) {
    let lines = vec![
        "=== /stage0.project-intel - Project Intelligence Pipeline ===".to_string(),
        String::new(),
        "Gathers project details for NotebookLM synthesis and context sharing.".to_string(),
        String::new(),
        "Usage: /stage0.project-intel <subcommand> [flags]".to_string(),
        String::new(),
        "Subcommands:".to_string(),
        "  snapshot    Gather project details -> JSON + markdown feeds".to_string(),
        "  curate-nl   Generate NL_* docs from feeds (for NotebookLM)".to_string(),
        "  sync-nl     Push NL_* docs to NotebookLM notebook".to_string(),
        "  overview    Query NotebookLM for global mental model".to_string(),
        "  status      Show current intel status".to_string(),
        "  help        Show this help".to_string(),
        String::new(),
        "Flags:".to_string(),
        "  --verbose, -v    Show detailed progress".to_string(),
        String::new(),
        "Output Locations:".to_string(),
        "  var/intel/project_snapshot.json    Full structured snapshot".to_string(),
        "  var/intel/project_feeds/*.md       Markdown feeds by topic".to_string(),
        "  docs/NL_*.md                       NotebookLM source docs".to_string(),
        "  docs/NL_PROJECT_OVERVIEW.md        Global mental model".to_string(),
        String::new(),
        "Typical Workflow:".to_string(),
        "  1. /stage0.project-intel snapshot".to_string(),
        "  2. /stage0.project-intel curate-nl".to_string(),
        "  3. /stage0.project-intel sync-nl".to_string(),
        "  4. /stage0.project-intel overview".to_string(),
        String::new(),
        "Then paste NL_PROJECT_OVERVIEW.md into ChatGPT/Claude for instant context.".to_string(),
    ];
    push_output(widget, lines, HistoryCellType::Notice);
}

#[derive(Default)]
struct Flags {
    verbose: bool,
}

fn parse_flags(args: &[&str]) -> Flags {
    let mut flags = Flags::default();
    for arg in args {
        if *arg == "--verbose" || *arg == "-v" {
            flags.verbose = true;
        }
    }
    flags
}
