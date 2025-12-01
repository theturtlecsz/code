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
        use super::super::pipeline_coordinator::run_constitution_readiness_gate;
        use super::super::project_detector::{detect_project_type, get_project_questions};

        // P91/SPEC-KIT-105: Run constitution readiness gate (warn-only)
        run_constitution_readiness_gate(widget);

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
/// Manage project constitution (view/add/sync)
///
/// P91/SPEC-KIT-105: Constitution management command with subcommands:
/// - view (default): Display current constitution from overlay DB
/// - add: Interactive entry to add constitution items
/// - sync: Regenerate NL_CONSTITUTION.md and memory/constitution.md
/// - ace: Extract and pin bullets to ACE playbook (legacy behavior)
pub struct SpecKitConstitutionCommand;

impl SpecKitCommand for SpecKitConstitutionCommand {
    fn name(&self) -> &'static str {
        "speckit.constitution"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "manage constitution (view/add/sync)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let args = args.trim();
        let (subcommand, rest) = if args.is_empty() {
            ("view", "")
        } else {
            args.split_once(' ').unwrap_or((args, ""))
        };

        match subcommand.to_lowercase().as_str() {
            "view" | "" => execute_constitution_view(widget),
            "add" => execute_constitution_add(widget, rest.trim()),
            "sync" => execute_constitution_sync(widget),
            "ace" => execute_constitution_ace(widget),
            _ => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "Unknown subcommand '{}'. Use: view, add, sync, or ace",
                    subcommand
                )));
                widget.request_redraw();
            }
        }
    }

    fn requires_args(&self) -> bool {
        false
    }
}

/// P91: Display current constitution from overlay DB
fn execute_constitution_view(widget: &mut ChatWidget) {
    // Load Stage0 config and connect to DB
    let config = match codex_stage0::Stage0Config::load() {
        Ok(c) => c,
        Err(e) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Failed to load Stage0 config: {}",
                e
            )));
            widget.request_redraw();
            return;
        }
    };

    let db = match codex_stage0::OverlayDb::connect_and_init(&config) {
        Ok(d) => d,
        Err(e) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Failed to connect to overlay DB: {}",
                e
            )));
            widget.request_redraw();
            return;
        }
    };

    // Get constitution meta
    let (version, hash, updated_at) = match db.get_constitution_meta() {
        Ok(meta) => meta,
        Err(e) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Failed to get constitution meta: {}",
                e
            )));
            widget.request_redraw();
            return;
        }
    };

    if version == 0 {
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![
                ratatui::text::Line::from("📋 Constitution Status"),
                ratatui::text::Line::from(""),
                ratatui::text::Line::from("No constitution defined."),
                ratatui::text::Line::from(""),
                ratatui::text::Line::from("Use /speckit.constitution add to create one."),
            ],
            crate::history_cell::HistoryCellType::Notice,
        ));
        widget.request_redraw();
        return;
    }

    // Get constitution memories
    let memories = match db.get_constitution_memories(50) {
        Ok(m) => m,
        Err(e) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Failed to get constitution memories: {}",
                e
            )));
            widget.request_redraw();
            return;
        }
    };

    // Group by type (priority)
    let guardrails: Vec<_> = memories.iter().filter(|m| m.initial_priority == 10).collect();
    let principles: Vec<_> = memories.iter().filter(|m| m.initial_priority == 9).collect();
    let goals: Vec<_> = memories.iter().filter(|m| m.initial_priority == 8).collect();

    let mut lines: Vec<ratatui::text::Line<'static>> = Vec::new();
    lines.push(ratatui::text::Line::from("📋 Constitution Status"));
    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(format!(
        "Version: {} | Hash: {}",
        version,
        hash.as_deref().unwrap_or("none")
    )));
    if let Some(dt) = updated_at {
        lines.push(ratatui::text::Line::from(format!(
            "Updated: {}",
            dt.format("%Y-%m-%d %H:%M UTC")
        )));
    }
    lines.push(ratatui::text::Line::from(""));

    // Guardrails
    lines.push(ratatui::text::Line::from(format!(
        "🛡️ Guardrails ({})",
        guardrails.len()
    )));
    for m in &guardrails {
        let content = m.content_raw.as_deref().unwrap_or("[no content]");
        let truncated = if content.len() > 60 {
            format!("{}...", &content[..60])
        } else {
            content.to_string()
        };
        lines.push(ratatui::text::Line::from(format!("  • {}", truncated)));
    }

    // Principles
    lines.push(ratatui::text::Line::from(format!(
        "📐 Principles ({})",
        principles.len()
    )));
    for m in &principles {
        let content = m.content_raw.as_deref().unwrap_or("[no content]");
        let truncated = if content.len() > 60 {
            format!("{}...", &content[..60])
        } else {
            content.to_string()
        };
        lines.push(ratatui::text::Line::from(format!("  • {}", truncated)));
    }

    // Goals
    lines.push(ratatui::text::Line::from(format!(
        "🎯 Goals/Non-Goals ({})",
        goals.len()
    )));
    for m in &goals {
        let content = m.content_raw.as_deref().unwrap_or("[no content]");
        let truncated = if content.len() > 60 {
            format!("{}...", &content[..60])
        } else {
            content.to_string()
        };
        lines.push(ratatui::text::Line::from(format!("  • {}", truncated)));
    }

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        lines,
        crate::history_cell::HistoryCellType::Notice,
    ));
    widget.request_redraw();
}

/// P91: Add constitution entry (non-interactive for now, shows usage)
fn execute_constitution_add(widget: &mut ChatWidget, args: &str) {
    if args.is_empty() {
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![
                ratatui::text::Line::from("📋 Constitution Add"),
                ratatui::text::Line::from(""),
                ratatui::text::Line::from("Usage: /speckit.constitution add <type> <content>"),
                ratatui::text::Line::from(""),
                ratatui::text::Line::from("Types:"),
                ratatui::text::Line::from("  guardrail - Hard constraints (priority 10)"),
                ratatui::text::Line::from("  principle - Architectural values (priority 9)"),
                ratatui::text::Line::from("  goal      - Project objectives (priority 8)"),
                ratatui::text::Line::from("  nongoal   - Explicit exclusions (priority 8)"),
                ratatui::text::Line::from(""),
                ratatui::text::Line::from("Example:"),
                ratatui::text::Line::from(
                    "  /speckit.constitution add guardrail Never break backwards compatibility",
                ),
            ],
            crate::history_cell::HistoryCellType::Notice,
        ));
        widget.request_redraw();
        return;
    }

    // Parse type and content
    let (type_str, content) = match args.split_once(' ') {
        Some((t, c)) => (t.trim(), c.trim()),
        None => {
            widget.history_push(crate::history_cell::new_error_event(
                "Missing content. Usage: /speckit.constitution add <type> <content>".to_string(),
            ));
            widget.request_redraw();
            return;
        }
    };

    if content.is_empty() {
        widget.history_push(crate::history_cell::new_error_event(
            "Content cannot be empty".to_string(),
        ));
        widget.request_redraw();
        return;
    }

    let constitution_type = match type_str.to_lowercase().as_str() {
        "guardrail" => codex_stage0::ConstitutionType::Guardrail,
        "principle" => codex_stage0::ConstitutionType::Principle,
        "goal" => codex_stage0::ConstitutionType::Goal,
        "nongoal" | "non-goal" => codex_stage0::ConstitutionType::NonGoal,
        _ => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Unknown type '{}'. Use: guardrail, principle, goal, or nongoal",
                type_str
            )));
            widget.request_redraw();
            return;
        }
    };

    // Connect to DB and add entry
    let config = match codex_stage0::Stage0Config::load() {
        Ok(c) => c,
        Err(e) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Failed to load Stage0 config: {}",
                e
            )));
            widget.request_redraw();
            return;
        }
    };

    let db = match codex_stage0::OverlayDb::connect_and_init(&config) {
        Ok(d) => d,
        Err(e) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Failed to connect to overlay DB: {}",
                e
            )));
            widget.request_redraw();
            return;
        }
    };

    // Generate a unique memory ID
    let memory_id = format!("constitution-{}-{}", type_str, uuid::Uuid::new_v4());

    // Upsert the constitution memory
    if let Err(e) = db.upsert_constitution_memory(&memory_id, constitution_type, content) {
        widget.history_push(crate::history_cell::new_error_event(format!(
            "Failed to add constitution entry: {}",
            e
        )));
        widget.request_redraw();
        return;
    }

    // Compute content hash and increment version
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    let hash = format!("{:016x}", hasher.finish());
    if let Err(e) = db.increment_constitution_version(Some(&hash)) {
        widget.history_push(crate::history_cell::new_error_event(format!(
            "Failed to increment version: {}",
            e
        )));
        widget.request_redraw();
        return;
    }

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![
            ratatui::text::Line::from(format!(
                "✅ Added {} to constitution",
                type_str
            )),
            ratatui::text::Line::from(format!("   Content: {}", content)),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from("   Run /speckit.constitution sync to regenerate files."),
        ],
        crate::history_cell::HistoryCellType::Notice,
    ));
    widget.request_redraw();
}

/// P91: Sync constitution to markdown files
fn execute_constitution_sync(widget: &mut ChatWidget) {
    // Load config and connect to DB
    let config = match codex_stage0::Stage0Config::load() {
        Ok(c) => c,
        Err(e) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Failed to load Stage0 config: {}",
                e
            )));
            widget.request_redraw();
            return;
        }
    };

    let db = match codex_stage0::OverlayDb::connect_and_init(&config) {
        Ok(d) => d,
        Err(e) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Failed to connect to overlay DB: {}",
                e
            )));
            widget.request_redraw();
            return;
        }
    };

    // Get constitution memories
    let memories = match db.get_constitution_memories(100) {
        Ok(m) => m,
        Err(e) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Failed to get constitution memories: {}",
                e
            )));
            widget.request_redraw();
            return;
        }
    };

    if memories.is_empty() {
        widget.history_push(crate::history_cell::new_error_event(
            "No constitution entries to sync. Use /speckit.constitution add first.".to_string(),
        ));
        widget.request_redraw();
        return;
    }

    // Group by type
    let guardrails: Vec<_> = memories
        .iter()
        .filter(|m| m.initial_priority == 10)
        .filter_map(|m| m.content_raw.as_deref())
        .collect();
    let principles: Vec<_> = memories
        .iter()
        .filter(|m| m.initial_priority == 9)
        .filter_map(|m| m.content_raw.as_deref())
        .collect();
    let goals: Vec<_> = memories
        .iter()
        .filter(|m| m.initial_priority == 8)
        .filter_map(|m| m.content_raw.as_deref())
        .collect();

    // Build markdown content
    let mut md = String::new();
    md.push_str("# Project Constitution\n\n");
    md.push_str("_Auto-generated from overlay DB. Do not edit directly._\n\n");

    md.push_str("## Guardrails\n\n");
    md.push_str("Hard constraints that must never be violated.\n\n");
    for g in &guardrails {
        md.push_str(&format!("- {}\n", g));
    }
    md.push('\n');

    md.push_str("## Principles\n\n");
    md.push_str("Architectural values and design principles.\n\n");
    for p in &principles {
        md.push_str(&format!("- {}\n", p));
    }
    md.push('\n');

    md.push_str("## Goals\n\n");
    md.push_str("Project objectives and explicit exclusions.\n\n");
    for g in &goals {
        md.push_str(&format!("- {}\n", g));
    }

    // Write to memory/constitution.md
    let memory_dir = widget.config.cwd.join("memory");
    if let Err(e) = std::fs::create_dir_all(&memory_dir) {
        widget.history_push(crate::history_cell::new_error_event(format!(
            "Failed to create memory directory: {}",
            e
        )));
        widget.request_redraw();
        return;
    }

    let constitution_path = memory_dir.join("constitution.md");
    if let Err(e) = std::fs::write(&constitution_path, &md) {
        widget.history_push(crate::history_cell::new_error_event(format!(
            "Failed to write constitution.md: {}",
            e
        )));
        widget.request_redraw();
        return;
    }

    // Also write to NL_CONSTITUTION.md for NotebookLM seeding
    let nl_path = memory_dir.join("NL_CONSTITUTION.md");
    if let Err(e) = std::fs::write(&nl_path, &md) {
        tracing::warn!("Failed to write NL_CONSTITUTION.md: {}", e);
    }

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![
            ratatui::text::Line::from("✅ Constitution synced"),
            ratatui::text::Line::from(format!(
                "   Guardrails: {} | Principles: {} | Goals: {}",
                guardrails.len(),
                principles.len(),
                goals.len()
            )),
            ratatui::text::Line::from("   Files updated:"),
            ratatui::text::Line::from("   • memory/constitution.md"),
            ratatui::text::Line::from("   • memory/NL_CONSTITUTION.md"),
        ],
        crate::history_cell::HistoryCellType::Notice,
    ));
    widget.request_redraw();
}

/// Legacy ACE bullet extraction and pinning
fn execute_constitution_ace(widget: &mut ChatWidget) {
    tracing::info!("SpecKitConstitution: ace subcommand called");

    // Find constitution.md in the repository
    let constitution_path = widget.config.cwd.join("memory").join("constitution.md");

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
        use super::super::stage0_seeding::{SeedingConfig, run_shadow_seeding};
        use crate::stage0_adapters::{LocalMemoryMcpAdapter, has_local_memory_server};
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
                ratatui::text::Line::from(format!("   Output: {}", output_dir.display())),
                ratatui::text::Line::from(format!(
                    "   Max memories per artifact: {}",
                    max_memories
                )),
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
        use super::super::code_index::CodeUnitExtractor;
        use crate::stage0_adapters::{LocalMemoryMcpAdapter, has_local_memory_server};
        use crate::vector_state::{IndexingStats, VECTOR_STATE};
        use codex_stage0::{
            DocumentKind, DocumentMetadata, Iqo, LocalMemoryClient, LocalMemorySearchParams,
            TfIdfBackend, VectorBackend, VectorDocument,
        };
        use std::sync::Arc;

        // Parse optional arguments
        let mut max_memories = 100usize;
        let mut index_code = true; // P85: Code indexing enabled by default

        for arg in args.split_whitespace() {
            if arg.starts_with("--max=") || arg.starts_with("--max-memories=") {
                if let Ok(n) = arg
                    .trim_start_matches("--max=")
                    .trim_start_matches("--max-memories=")
                    .parse()
                {
                    max_memories = n;
                }
            } else if arg == "--no-code" {
                index_code = false;
            }
        }

        // Show starting message
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![
                ratatui::text::Line::from("🔍 Stage0 Vector Backend Indexing (P85)"),
                ratatui::text::Line::from(format!("   Max memories: {}", max_memories)),
                ratatui::text::Line::from(format!(
                    "   Code indexing: {}",
                    if index_code { "enabled" } else { "disabled" }
                )),
                ratatui::text::Line::from("   Fetching memories from local-memory..."),
            ],
            crate::history_cell::HistoryCellType::Notice,
        ));
        widget.request_redraw();

        let mcp_manager = widget.mcp_manager.clone();
        let cwd = widget.config.cwd.clone();

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
                max_candidates: max_memories,
                ..Default::default()
            };
            let params = LocalMemorySearchParams {
                iqo,
                max_results: max_memories,
            };

            let memories = local_mem
                .search_memories(params)
                .await
                .map_err(|e| format!("Failed to fetch memories: {}", e))?;

            // Convert memories to VectorDocuments
            let mut docs: Vec<VectorDocument> = memories
                .iter()
                .map(|m| {
                    let mut doc =
                        VectorDocument::new(m.id.clone(), DocumentKind::Memory, m.snippet.clone());

                    if let Some(domain) = &m.domain {
                        doc = doc.with_domain(domain.as_str());
                    }

                    for tag in &m.tags {
                        doc = doc.with_tag(tag.as_str());
                    }

                    doc
                })
                .collect();

            let memory_count = docs.len();

            // P85: Extract and index code units
            let code_count = if index_code {
                // Find codex-rs root (walk up from cwd looking for codex-rs/Cargo.toml)
                let codex_rs_root = find_codex_rs_root(&cwd);

                if let Some(root) = codex_rs_root {
                    let extractor = CodeUnitExtractor::new("codex-rs");
                    let (code_units, _extraction_stats) = extractor.extract_from_codex_rs(&root);

                    // Convert code units to VectorDocuments
                    let code_docs: Vec<VectorDocument> = code_units
                        .iter()
                        .map(|cu| {
                            let mut extra = std::collections::HashMap::new();
                            if let Some(sym) = &cu.symbol {
                                extra.insert("symbol".to_string(), serde_json::json!(sym));
                            }
                            extra.insert(
                                "unit_kind".to_string(),
                                serde_json::json!(cu.kind.as_str()),
                            );
                            extra
                                .insert("line_start".to_string(), serde_json::json!(cu.line_start));
                            extra.insert("text".to_string(), serde_json::json!(cu.text.clone()));

                            let metadata = DocumentMetadata {
                                source_path: Some(cu.path.clone()),
                                domain: Some("codex-rs".to_string()),
                                extra,
                                ..Default::default()
                            };

                            VectorDocument::new(cu.id.clone(), DocumentKind::Code, cu.text.clone())
                                .with_metadata(metadata)
                        })
                        .collect();

                    let count = code_docs.len();
                    docs.extend(code_docs);
                    count
                } else {
                    0
                }
            } else {
                0
            };

            let total_docs = docs.len();

            if total_docs == 0 {
                return Ok((0, 0, 0, 0, 0));
            }

            // Create backend and index - V2.5b: Store in shared state
            let backend = TfIdfBackend::new();
            let stats = backend
                .index_documents(docs)
                .await
                .map_err(|e| format!("Indexing failed: {}", e))?;

            // Store in shared VECTOR_STATE for use by run_stage0_blocking
            let indexing_stats = IndexingStats {
                doc_count: total_docs,
                unique_tokens: stats.unique_tokens,
                total_tokens: stats.total_tokens,
                duration_ms: stats.duration_ms,
                indexed_at: chrono::Utc::now(),
            };
            VECTOR_STATE.set_backend(backend, indexing_stats).await;

            Ok((
                memory_count,
                code_count,
                stats.unique_tokens,
                stats.total_tokens,
                stats.duration_ms,
            ))
        });

        match result {
            Ok((memory_count, code_count, unique_tokens, total_tokens, duration_ms)) => {
                let total = memory_count + code_count;
                if total == 0 {
                    widget.history_push(crate::history_cell::PlainHistoryCell::new(
                        vec![ratatui::text::Line::from("⚠ No documents found to index")],
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
                                "   Memories indexed: {}",
                                memory_count
                            )),
                            ratatui::text::Line::from(format!(
                                "   Code units indexed: {}",
                                code_count
                            )),
                            ratatui::text::Line::from(format!(
                                "   Total documents: {}",
                                total
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
/// P86: Extended with --lane and --strict flags for code lane evaluation
pub struct Stage0EvalBackendCommand;

/// P86: Lane filter for evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalLaneFilter {
    /// Both memory and code lanes
    Both,
    /// Memory lane only
    Memory,
    /// Code lane only
    Code,
}

impl SpecKitCommand for Stage0EvalBackendCommand {
    fn name(&self) -> &'static str {
        "stage0.eval-backend"
    }

    fn aliases(&self) -> &[&'static str] {
        &["stage0.eval"]
    }

    fn description(&self) -> &'static str {
        "compare baseline vs hybrid retrieval using eval cases (--lane={memory,code,both} --strict)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        use crate::vector_state::VECTOR_STATE;
        use codex_stage0::{
            EvalLane, TfIdfBackend, VectorBackend, VectorFilters, built_in_eval_cases,
            built_in_test_documents, combined_eval_cases, evaluate_backend,
        };
        use std::path::PathBuf;

        // Parse optional arguments
        let mut top_k = 10usize;
        let mut output_json = false;
        let mut cases_file: Option<PathBuf> = None;
        let mut lane_filter = EvalLaneFilter::Both;
        let mut strict_mode = false;

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
            } else if arg.starts_with("--lane=") {
                let lane_str = arg.trim_start_matches("--lane=");
                lane_filter = match lane_str {
                    "memory" => EvalLaneFilter::Memory,
                    "code" => EvalLaneFilter::Code,
                    "both" => EvalLaneFilter::Both,
                    _ => {
                        widget.history_push(crate::history_cell::new_error_event(format!(
                            "Invalid lane '{}': use memory, code, or both",
                            lane_str
                        )));
                        return;
                    }
                };
            } else if arg == "--strict" {
                strict_mode = true;
            }
        }

        // P86: Convert lane filter to Option<EvalLane>
        let lane_option = match lane_filter {
            EvalLaneFilter::Both => None,
            EvalLaneFilter::Memory => Some(EvalLane::Memory),
            EvalLaneFilter::Code => Some(EvalLane::Code),
        };

        // Show starting message
        let lane_str = match lane_filter {
            EvalLaneFilter::Both => "both",
            EvalLaneFilter::Memory => "memory",
            EvalLaneFilter::Code => "code",
        };
        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![
                ratatui::text::Line::from("📊 Stage0 Baseline vs Hybrid Evaluation"),
                ratatui::text::Line::from(format!("   Top K: {}", top_k)),
                ratatui::text::Line::from(format!("   Lane: {}", lane_str)),
                ratatui::text::Line::from(if strict_mode {
                    "   Mode: strict"
                } else {
                    "   Mode: normal"
                }),
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
            // P86: Load eval cases with lane filtering
            let cases = combined_eval_cases(
                cases_file.is_none(), // use builtins if no file provided
                cases_file.as_deref(),
                lane_option,
            )
            .map_err(|e| format!("Failed to load eval cases: {}", e))?;

            if cases.is_empty() {
                return Err(format!("No eval cases found for lane '{}'", lane_str));
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
                let result = evaluate_backend(hybrid_backend, &cases, &VectorFilters::new(), top_k)
                    .await
                    .map_err(|e| format!("Hybrid evaluation failed: {}", e))?;
                Some(result)
            } else {
                None
            };

            Ok((baseline_result, hybrid_result, cases.len()))
        });

        match result {
            Ok((baseline, hybrid_opt, case_count)) => {
                // P86: Check strict mode - fail if any missing IDs
                if strict_mode && baseline.has_missing_ids() {
                    widget.history_push(crate::history_cell::new_error_event(format!(
                        "Strict mode: {} missing expected IDs in baseline evaluation",
                        baseline.total_missing_ids
                    )));
                    return;
                }
                if strict_mode {
                    if let Some(ref h) = hybrid_opt {
                        if h.has_missing_ids() {
                            widget.history_push(crate::history_cell::new_error_event(format!(
                                "Strict mode: {} missing expected IDs in hybrid evaluation",
                                h.total_missing_ids
                            )));
                            return;
                        }
                    }
                }

                if output_json {
                    // JSON output for CI automation
                    let json_output = serde_json::json!({
                        "top_k": top_k,
                        "lane": lane_str,
                        "strict": strict_mode,
                        "case_count": case_count,
                        "baseline": {
                            "mean_precision": baseline.mean_precision,
                            "mean_recall": baseline.mean_recall,
                            "mrr": baseline.mrr,
                            "cases_passed": baseline.cases_passed,
                            "total_cases": baseline.total_cases,
                            "pass_rate": baseline.pass_rate(),
                            "missing_ids": baseline.total_missing_ids,
                        },
                        "hybrid": hybrid_opt.as_ref().map(|h| serde_json::json!({
                            "mean_precision": h.mean_precision,
                            "mean_recall": h.mean_recall,
                            "mrr": h.mrr,
                            "cases_passed": h.cases_passed,
                            "total_cases": h.total_cases,
                            "pass_rate": h.pass_rate(),
                            "missing_ids": h.total_missing_ids,
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
                        format_delta(
                            baseline.mean_precision,
                            hybrid_opt.as_ref().map(|h| h.mean_precision)
                        )
                    )));

                    lines.push(ratatui::text::Line::from(format!(
                        "{:<20} {:>12.2} {:>12}",
                        "Mean R@k",
                        baseline.mean_recall,
                        format_delta(
                            baseline.mean_recall,
                            hybrid_opt.as_ref().map(|h| h.mean_recall)
                        )
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

/// Command: /stage0.eval-code
/// P86: Sugar for /stage0.eval-backend --lane=code
///
/// Runs evaluation harness specifically for code lane cases.
/// Default k=10.
pub struct Stage0EvalCodeCommand;

impl SpecKitCommand for Stage0EvalCodeCommand {
    fn name(&self) -> &'static str {
        "stage0.eval-code"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "evaluate code lane retrieval quality (P@K, R@K, MRR)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        // P86: Delegate to eval-backend with --lane=code
        let mut new_args = format!("--lane=code {}", args);

        // Set default k=10 if not specified
        if !args.contains("--top-k=") && !args.contains("--k=") {
            new_args.push_str(" --k=10");
        }

        Stage0EvalBackendCommand.execute(widget, new_args);
    }

    fn requires_args(&self) -> bool {
        false
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// P85: Helper Functions for Code Indexing
// ─────────────────────────────────────────────────────────────────────────────

/// Find the codex-rs root directory by walking up from cwd
///
/// Looks for a directory containing stage0/Cargo.toml (our marker for codex-rs workspace)
fn find_codex_rs_root(cwd: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut current = cwd.to_path_buf();

    for _ in 0..10 {
        // Check if this is codex-rs root (has stage0/Cargo.toml)
        if current.join("stage0").join("Cargo.toml").exists() {
            return Some(current);
        }

        // Also check if we're inside codex-rs and need to go up
        if current.join("Cargo.toml").exists() {
            // Check if parent is codex-rs root
            if let Some(parent) = current.parent() {
                if parent.join("stage0").join("Cargo.toml").exists() {
                    return Some(parent.to_path_buf());
                }
            }
        }

        // Go up one level
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    None
}
