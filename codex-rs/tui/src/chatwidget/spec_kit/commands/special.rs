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
///
/// P95/SPEC-KIT-105: Constitution-Aware Refinement
/// - Runs Stage 0 to detect constitution conflicts
/// - Soft blocks if conflicts detected (requires --force to proceed)
/// - Shows options: Modify spec, Create exception, or --force
pub struct SpecKitSpecifyCommand;

impl SpecKitCommand for SpecKitSpecifyCommand {
    fn name(&self) -> &'static str {
        "speckit.specify"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "refine PRD with constitution-aware conflict detection (Tier 1, ~$0.10)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        execute_constitution_aware_specify(widget, args);
    }

    fn expand_prompt(&self, _args: &str) -> Option<String> {
        None // SPEC-KIT-957: No longer uses orchestrator pattern
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// P95/SPEC-KIT-105: Constitution-aware /speckit.specify execution
///
/// 1. Parse args for SPEC-ID and --force flag
/// 2. Load spec's Constitution-Version vs current
/// 3. Run Stage 0 to detect conflicts
/// 4. If conflicts AND no --force: soft block with options
/// 5. Otherwise proceed with normal specify flow
fn execute_constitution_aware_specify(widget: &mut ChatWidget, args: String) {
    use ratatui::text::Line;
    use crate::history_cell::{HistoryCellType, PlainHistoryCell};

    // Parse args: SPEC-ID [--force]
    let mut force_mode = false;
    let mut spec_id = String::new();

    for arg in args.split_whitespace() {
        if arg == "--force" {
            force_mode = true;
        } else if spec_id.is_empty() {
            spec_id = arg.to_string();
        }
    }

    if spec_id.is_empty() {
        widget.history_push(crate::history_cell::new_error_event(
            "Usage: /speckit.specify SPEC-ID [--force]".to_string(),
        ));
        widget.request_redraw();
        return;
    }

    // Run native guardrail validation first
    let guardrail_result = super::super::native_guardrail::run_native_guardrail(
        &widget.config.cwd,
        &spec_id,
        crate::spec_prompts::SpecStage::Specify,
        false,
    );

    if !guardrail_result.success {
        for error in &guardrail_result.errors {
            widget.history_push(crate::history_cell::new_error_event(error.clone()));
        }
        widget.request_redraw();
        return;
    }

    for warning in &guardrail_result.warnings {
        widget.history_push(crate::history_cell::new_warning_event(warning.clone()));
    }

    // P95: Load spec Constitution-Version and current version
    let spec_path = widget.config.cwd.join(format!("docs/{}/spec.md", spec_id));
    let spec_constitution_version = if spec_path.exists() {
        extract_constitution_version(&spec_path)
    } else {
        None
    };
    let current_version = get_current_constitution_version_for_check();

    // P95: Run Stage 0 to detect constitution conflicts
    let constitution_conflicts = run_stage0_for_constitution_check(widget, &spec_id);

    // P95: Version drift check
    let has_version_drift = match (spec_constitution_version, current_version) {
        (Some(spec_ver), Some(curr_ver)) => spec_ver != curr_ver,
        _ => false,
    };

    // P95: Check if we have conflicts or version drift
    let has_conflicts = constitution_conflicts.is_some() && !constitution_conflicts.as_ref().unwrap().is_empty();

    if (has_conflicts || has_version_drift) && !force_mode {
        // Soft block: show conflict UI
        let mut lines = vec![
            Line::from("⚠ Constitution Conflict Detected (P95)"),
            Line::from(""),
        ];

        // Version drift warning
        if has_version_drift {
            lines.push(Line::from(format!(
                "Version drift: spec created at v{}, current constitution v{}",
                spec_constitution_version.unwrap_or(0),
                current_version.unwrap_or(0)
            )));
            lines.push(Line::from(""));
        }

        // Constitution conflicts
        if let Some(ref conflicts) = constitution_conflicts {
            lines.push(Line::from("Conflicts:"));
            for line in conflicts.lines() {
                lines.push(Line::from(format!("  {}", line)));
            }
            lines.push(Line::from(""));
        }

        lines.push(Line::from("Options:"));
        lines.push(Line::from("  [A] Modify spec to resolve conflicts"));
        lines.push(Line::from("      Edit docs/{}/spec.md and re-run /speckit.specify"));
        lines.push(Line::from("  [B] Create exception for this conflict"));
        lines.push(Line::from(format!(
            "      /speckit.constitution add-exception --spec {} --reason \"<justification>\"",
            spec_id
        )));
        lines.push(Line::from("  [C] Force proceed (acknowledge conflict)"));
        lines.push(Line::from(format!(
            "      /speckit.specify {} --force",
            spec_id
        )));
        lines.push(Line::from(""));
        lines.push(Line::from("Soft block: use --force to proceed without resolution."));

        widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));

        // P95: Log ConstitutionConflictDetected event
        tracing::warn!(
            event_type = "ConstitutionConflictDetected",
            spec_id = %spec_id,
            has_conflicts,
            has_version_drift,
            spec_version = ?spec_constitution_version,
            current_version = ?current_version,
            "Constitution conflict detected at specify time"
        );

        widget.request_redraw();
        return;
    }

    // P95: If --force was used with conflicts, log the override
    if force_mode && (has_conflicts || has_version_drift) {
        tracing::info!(
            event_type = "ConstitutionOverride",
            spec_id = %spec_id,
            has_conflicts,
            has_version_drift,
            "Proceeding with --force despite constitution conflicts"
        );

        widget.history_push(PlainHistoryCell::new(
            vec![
                Line::from("⚡ Proceeding with --force (constitution conflict acknowledged)"),
            ],
            HistoryCellType::Notice,
        ));
    }

    // Proceed with normal specify flow
    super::super::agent_orchestrator::auto_submit_spec_stage_prompt(
        widget,
        crate::spec_prompts::SpecStage::Specify,
        &spec_id,
    );
}

/// P95: Run Stage 0 specifically for constitution conflict detection
///
/// Returns the constitution_conflicts string if any conflicts detected
fn run_stage0_for_constitution_check(widget: &ChatWidget, spec_id: &str) -> Option<String> {
    use super::super::stage0_integration::{Stage0ExecutionConfig, run_stage0_for_spec};

    // Load spec content
    let spec_path = widget.config.cwd.join(format!("docs/{}/spec.md", spec_id));
    let spec_content = match std::fs::read_to_string(&spec_path) {
        Ok(content) => content,
        Err(_) => return None,
    };

    if spec_content.is_empty() {
        return None;
    }

    // Run Stage 0 with explain disabled (we just need conflicts)
    let config = Stage0ExecutionConfig {
        disabled: false,
        explain: false,
    };

    let result = run_stage0_for_spec(
        &widget.mcp_manager,
        spec_id,
        &spec_content,
        &widget.config.cwd,
        &config,
    );

    // Extract constitution_conflicts from Stage0Result
    result.result.and_then(|r| r.constitution_conflicts)
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

    // Group by type (priority) and distinguish goals from non-goals
    let guardrails: Vec<_> = memories.iter().filter(|m| m.initial_priority == 10).collect();
    let principles: Vec<_> = memories.iter().filter(|m| m.initial_priority == 9).collect();
    // P93: Distinguish goals from non-goals by memory_id prefix
    let goals: Vec<_> = memories
        .iter()
        .filter(|m| m.initial_priority == 8 && !m.memory_id.contains("nongoal"))
        .collect();
    let nongoals: Vec<_> = memories
        .iter()
        .filter(|m| m.initial_priority == 8 && m.memory_id.contains("nongoal"))
        .collect();

    // Count vision-created content
    let vision_count = memories
        .iter()
        .filter(|m| m.memory_id.starts_with("vision-"))
        .count();

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
    if vision_count > 0 {
        lines.push(ratatui::text::Line::from(format!(
            "Source: {} from /speckit.vision",
            vision_count
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

    // Goals (P93: now separate from non-goals)
    lines.push(ratatui::text::Line::from(format!("🎯 Goals ({})", goals.len())));
    for m in &goals {
        let content = m.content_raw.as_deref().unwrap_or("[no content]");
        let truncated = if content.len() > 60 {
            format!("{}...", &content[..60])
        } else {
            content.to_string()
        };
        lines.push(ratatui::text::Line::from(format!("  • {}", truncated)));
    }

    // Non-Goals (P93: now shown separately)
    lines.push(ratatui::text::Line::from(format!(
        "🚫 Non-Goals ({})",
        nongoals.len()
    )));
    for m in &nongoals {
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

    // P92/SPEC-KIT-105: Invalidate Tier 2 cache when constitution memory added
    let cache_invalidated = match db.invalidate_tier2_by_constitution() {
        Ok(count) => count,
        Err(e) => {
            tracing::warn!("Failed to invalidate Tier 2 cache: {}", e);
            0
        }
    };

    let mut lines = vec![
        ratatui::text::Line::from(format!("✅ Added {} to constitution", type_str)),
        ratatui::text::Line::from(format!("   Content: {}", content)),
    ];

    if cache_invalidated > 0 {
        lines.push(ratatui::text::Line::from(format!(
            "   Cache: {} Tier 2 entries invalidated (P92)",
            cache_invalidated
        )));
    }

    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(
        "   Run /speckit.constitution sync to regenerate files.",
    ));

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        lines,
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

    // P92/SPEC-KIT-105: Invalidate Tier 2 cache when constitution changes
    // Constitution-dependent cache entries become stale after sync
    let cache_invalidated = match db.invalidate_tier2_by_constitution() {
        Ok(count) => count,
        Err(e) => {
            tracing::warn!("Failed to invalidate Tier 2 cache: {}", e);
            0
        }
    };

    let mut lines = vec![
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
    ];

    if cache_invalidated > 0 {
        lines.push(ratatui::text::Line::from(format!(
            "   Cache: {} Tier 2 entries invalidated (P92)",
            cache_invalidated
        )));
    }

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        lines,
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

// ─────────────────────────────────────────────────────────────────────────────
// P93/SPEC-KIT-105: Vision Front Door Command
// ─────────────────────────────────────────────────────────────────────────────

/// Command: /speckit.vision
/// Guided Q&A wizard for project vision and constitution creation
///
/// P93/SPEC-KIT-105: Captures target users, problem statement, goals, non-goals,
/// and principles through an interactive modal. Answers are stored as constitution
/// memories with appropriate types and priorities.
pub struct SpecKitVisionCommand;

impl SpecKitCommand for SpecKitVisionCommand {
    fn name(&self) -> &'static str {
        "speckit.vision"
    }

    fn aliases(&self) -> &[&'static str] {
        &["vision"]
    }

    fn description(&self) -> &'static str {
        "guided Q&A wizard for project vision and constitution (P93)"
    }

    fn execute(&self, widget: &mut ChatWidget, _args: String) {
        use super::super::pipeline_coordinator::run_constitution_readiness_gate;

        // P91/SPEC-KIT-105: Run constitution readiness gate (warn-only)
        // Vision is how users CREATE constitution, so gate is informational only
        run_constitution_readiness_gate(widget);

        // Show the vision builder modal
        widget.show_vision_builder();
    }

    fn requires_args(&self) -> bool {
        false
    }
}

/// P94/SPEC-KIT-105: Command: /speckit.check-alignment
/// Check drift between specs and current constitution version
/// Compares Constitution-Version at spec creation vs current version
///
/// P95: Extended with --deep mode for content-level drift detection
/// - --deep: Run Stage 0 for each spec to detect actual conflicts (uses Tier-2)
/// - --spec SPEC-ID: Check single spec (reduces Tier-2 usage in --deep mode)
/// - --json: Output in JSON format for CI
pub struct SpecKitCheckAlignmentCommand;

impl SpecKitCommand for SpecKitCheckAlignmentCommand {
    fn name(&self) -> &'static str {
        "speckit.check-alignment"
    }

    fn aliases(&self) -> &[&'static str] {
        &["check-alignment"]
    }

    fn description(&self) -> &'static str {
        "check spec alignment with constitution (--deep for content analysis)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        execute_alignment_check(widget, args);
    }

    fn requires_args(&self) -> bool {
        false
    }
}

/// P95: Execute alignment check with optional --deep mode
fn execute_alignment_check(widget: &mut ChatWidget, args: String) {
    use std::fs;
    use ratatui::text::Line;
    use crate::history_cell::{HistoryCellType, PlainHistoryCell};

    // P95: Parse flags
    let json_mode = args.contains("--json");
    let deep_mode = args.contains("--deep");

    // P95: Parse --spec SPEC-ID flag
    let single_spec: Option<String> = {
        let args_vec: Vec<&str> = args.split_whitespace().collect();
        args_vec.iter().position(|&a| a == "--spec").and_then(|pos| {
            args_vec.get(pos + 1).map(|s| s.to_string())
        })
    };

    let cwd = &widget.config.cwd;
    let docs_dir = cwd.join("docs");

    // Get current constitution version
    let current_version = get_current_constitution_version_for_check();

    // P95: Get exception count from overlay DB
    let exception_count = get_exception_count();

    // Scan for SPEC directories (or single spec)
    let spec_entries: Vec<_> = if let Some(ref spec_id) = single_spec {
        // Single spec mode
        let spec_dir = docs_dir.join(spec_id);
        if spec_dir.exists() && spec_dir.is_dir() {
            vec![spec_dir]
        } else {
            // Try finding by prefix
            if docs_dir.exists() {
                fs::read_dir(&docs_dir)
                    .ok()
                    .map(|entries| {
                        entries
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.file_name()
                                    .to_string_lossy()
                                    .starts_with(spec_id)
                            })
                            .map(|e| e.path())
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    } else {
        // All specs mode
        if docs_dir.exists() {
            fs::read_dir(&docs_dir)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            e.file_name()
                                .to_string_lossy()
                                .starts_with("SPEC-KIT-")
                        })
                        .map(|e| e.path())
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    };

    if spec_entries.is_empty() {
        let msg = if single_spec.is_some() {
            format!("Spec '{}' not found in docs/ directory", single_spec.unwrap())
        } else {
            "No specs found in docs/ directory".to_string()
        };

        if json_mode {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from("[]")],
                HistoryCellType::Notice,
            ));
        } else {
            widget.history_push(PlainHistoryCell::new(
                vec![
                    Line::from(msg),
                    Line::from(""),
                    Line::from("Create a spec with: /speckit.new <description>"),
                ],
                HistoryCellType::Notice,
            ));
        }
        widget.request_redraw();
        return;
    }

    // P95: Track Tier-2 calls for deep mode telemetry
    let mut tier2_calls = 0u32;

    // Collect alignment info for each spec
    let mut results: Vec<DeepAlignmentResult> = Vec::new();

    for spec_path in spec_entries {
        let dir_name = spec_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let spec_id = extract_spec_id(&dir_name);
        let spec_md_path = spec_path.join("spec.md");

        let created_version = if spec_md_path.exists() {
            extract_constitution_version(&spec_md_path)
        } else {
            None
        };

        let version_status = match (created_version, current_version) {
            (Some(created), Some(current)) if created == current => AlignmentStatus::Fresh,
            (Some(_), Some(_)) => AlignmentStatus::Stale,
            _ => AlignmentStatus::Unknown,
        };

        // P95: Deep mode - run Stage 0 for content-level conflict detection
        let (content_conflicts, content_aligned) = if deep_mode {
            tier2_calls += 1;
            let conflicts = run_stage0_for_constitution_check(widget, &spec_id);
            let has_conflicts = conflicts.as_ref().map(|c| !c.is_empty()).unwrap_or(false);
            (conflicts, !has_conflicts)
        } else {
            (None, true) // Assume aligned in non-deep mode
        };

        results.push(DeepAlignmentResult {
            spec_id,
            created_version,
            current_version,
            version_status,
            content_aligned,
            content_conflicts,
        });
    }

    // Count statuses
    let fresh_count = results.iter().filter(|r| matches!(r.version_status, AlignmentStatus::Fresh)).count();
    let stale_count = results.iter().filter(|r| matches!(r.version_status, AlignmentStatus::Stale)).count();
    let unknown_count = results.iter().filter(|r| matches!(r.version_status, AlignmentStatus::Unknown)).count();
    let content_conflict_count = if deep_mode {
        results.iter().filter(|r| !r.content_aligned).count()
    } else {
        0
    };

    // P95: DeepAlignmentCheckRun event for telemetry
    tracing::info!(
        event_type = "DeepAlignmentCheckRun",
        total_specs = results.len(),
        fresh_count,
        stale_count,
        unknown_count,
        content_conflict_count,
        exception_count,
        deep_mode,
        tier2_calls,
        "Deep alignment check completed"
    );

    // Output results
    if json_mode {
        // P95: JSON output with deep mode fields
        let json_entries: Vec<String> = results
            .iter()
            .map(|r| {
                let base = format!(
                    r#"    "spec_id": "{}",
    "constitution_version_at_creation": {},
    "current_constitution_version": {},
    "version_staleness": "{}""#,
                    r.spec_id,
                    r.created_version.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                    r.current_version.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                    r.version_status.as_str()
                );

                if deep_mode {
                    let conflicts_json = r.content_conflicts
                        .as_ref()
                        .map(|c| format!("\"{}\"", c.replace('\"', "\\\"").replace('\n', "\\n")))
                        .unwrap_or_else(|| "null".to_string());

                    format!(
                        "  {{\n{},\n    \"content_aligned\": {},\n    \"content_conflicts\": {}\n  }}",
                        base,
                        r.content_aligned,
                        conflicts_json
                    )
                } else {
                    format!("  {{\n{}\n  }}", base)
                }
            })
            .collect();

        let metadata = format!(
            r#"{{
  "deep_mode": {},
  "tier2_calls": {},
  "exception_count": {},
  "summary": {{
    "total": {},
    "fresh": {},
    "stale": {},
    "unknown": {},
    "content_conflicts": {}
  }},
  "specs": [
{}
  ]
}}"#,
            deep_mode,
            tier2_calls,
            exception_count,
            results.len(),
            fresh_count,
            stale_count,
            unknown_count,
            content_conflict_count,
            json_entries.join(",\n")
        );

        widget.history_push(PlainHistoryCell::new(
            vec![Line::from(metadata)],
            HistoryCellType::Notice,
        ));
    } else {
        // TUI table output
        let mode_indicator = if deep_mode { " (--deep)" } else { "" };
        let mut lines = vec![
            Line::from(format!("Constitution Alignment Check{}", mode_indicator)),
            Line::from(""),
            Line::from(format!(
                "Current constitution version: {} | Exceptions: {}",
                current_version.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string()),
                exception_count
            )),
            Line::from(""),
        ];

        if deep_mode {
            lines.push(Line::from("SPEC ID          | Ver Drift | Content | Conflicts"));
            lines.push(Line::from("-----------------+-----------+---------+----------"));

            for r in &results {
                let ver_drift = match r.version_status {
                    AlignmentStatus::Fresh => "fresh",
                    AlignmentStatus::Stale => "stale",
                    AlignmentStatus::Unknown => "unknown",
                };
                let content = if r.content_aligned { "OK" } else { "CONFLICT" };
                let conflicts = if r.content_conflicts.is_some() && !r.content_aligned {
                    "yes"
                } else {
                    "-"
                };

                lines.push(Line::from(format!(
                    "{:<16} | {:>9} | {:>7} | {}",
                    r.spec_id,
                    ver_drift,
                    content,
                    conflicts
                )));
            }
        } else {
            lines.push(Line::from("SPEC ID          | Created Ver | Current Ver | Status"));
            lines.push(Line::from("-----------------+-------------+-------------+--------"));

            for r in &results {
                let created = r.created_version
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let current = r.current_version
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string());

                lines.push(Line::from(format!(
                    "{:<16} | {:>11} | {:>11} | {}",
                    r.spec_id,
                    created,
                    current,
                    r.version_status.as_str()
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(format!(
            "Summary: {} fresh, {} stale, {} unknown{}",
            fresh_count,
            stale_count,
            unknown_count,
            if deep_mode {
                format!(", {} content conflicts", content_conflict_count)
            } else {
                String::new()
            }
        )));

        if deep_mode {
            lines.push(Line::from(format!("Tier-2 calls: {}", tier2_calls)));
        }

        if stale_count > 0 || content_conflict_count > 0 {
            lines.push(Line::from(""));
            if content_conflict_count > 0 {
                lines.push(Line::from("Use /speckit.specify SPEC-ID --force to proceed with conflicts."));
            } else {
                lines.push(Line::from("Stale specs may benefit from re-specification with updated constitution."));
            }
        }

        if !deep_mode {
            lines.push(Line::from(""));
            lines.push(Line::from("Use --deep for content-level conflict detection (requires Tier-2)."));
        }

        widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
    }

    widget.request_redraw();
}

/// P95: Get count of Exception constitution memories
fn get_exception_count() -> u32 {
    let config = match codex_stage0::Stage0Config::load() {
        Ok(c) => c,
        Err(_) => return 0,
    };

    let db = match codex_stage0::OverlayDb::connect_and_init(&config) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    // Get all constitution memories and filter for exceptions (priority 7)
    let memories = match db.get_constitution_memories(100) {
        Ok(m) => m,
        Err(_) => return 0,
    };

    memories.iter().filter(|m| m.initial_priority == 7).count() as u32
}

/// Alignment status for a spec
#[derive(Debug)]
enum AlignmentStatus {
    Fresh,
    Stale,
    Unknown,
}

impl AlignmentStatus {
    fn as_str(&self) -> &'static str {
        match self {
            AlignmentStatus::Fresh => "fresh",
            AlignmentStatus::Stale => "stale",
            AlignmentStatus::Unknown => "unknown",
        }
    }
}

/// Result of alignment check for a single spec (legacy P94)
#[derive(Debug)]
#[allow(dead_code)]
struct AlignmentResult {
    spec_id: String,
    created_version: Option<u32>,
    current_version: Option<u32>,
    status: AlignmentStatus,
}

/// P95: Extended alignment result with deep mode fields
#[derive(Debug)]
struct DeepAlignmentResult {
    spec_id: String,
    created_version: Option<u32>,
    current_version: Option<u32>,
    version_status: AlignmentStatus,
    /// P95: Whether content passes constitution alignment (deep mode only)
    content_aligned: bool,
    /// P95: Raw conflict text from Stage 0 (deep mode only)
    content_conflicts: Option<String>,
}

/// Extract SPEC-KIT-### from directory name like "SPEC-KIT-105-drift-detection"
fn extract_spec_id(dir_name: &str) -> String {
    // Pattern: SPEC-KIT-### followed by optional suffix
    let parts: Vec<&str> = dir_name.splitn(4, '-').collect();
    if parts.len() >= 3 {
        format!("{}-{}-{}", parts[0], parts[1], parts[2])
    } else {
        dir_name.to_string()
    }
}

/// Extract Constitution-Version from spec.md frontmatter
fn extract_constitution_version(spec_md_path: &std::path::Path) -> Option<u32> {
    let content = std::fs::read_to_string(spec_md_path).ok()?;

    for line in content.lines() {
        if line.starts_with("**Constitution-Version**:") || line.starts_with("**Constitution-Version**: ") {
            let version_str = line
                .trim_start_matches("**Constitution-Version**:")
                .trim_start_matches(" ")
                .trim();
            return version_str.parse().ok();
        }
    }

    None
}

/// Get current constitution version from overlay DB (for check-alignment command)
fn get_current_constitution_version_for_check() -> Option<u32> {
    let config = codex_stage0::Stage0Config::load().ok()?;
    let db = codex_stage0::OverlayDb::connect_and_init(&config).ok()?;
    let version = db.get_constitution_version().ok()?;
    if version == 0 { None } else { Some(version) }
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
// P92/SPEC-KIT-105: Planning Pipeline Command
// ─────────────────────────────────────────────────────────────────────────────

/// Command: /speckit.plan-pipeline
/// Planning-only pipeline: Stage 0 → Specify → Plan → Tasks (NO Implement/Validate/Audit)
///
/// P92/SPEC-KIT-105: Provides parity with `/plan + /tasks` without implementation stages.
/// Respects constitution gate - will abort in Block mode if constitution incomplete.
pub struct SpecKitPlanPipelineCommand;

impl SpecKitCommand for SpecKitPlanPipelineCommand {
    fn name(&self) -> &'static str {
        "speckit.plan-pipeline"
    }

    fn aliases(&self) -> &[&'static str] {
        &["speckit.planning", "plan-pipeline"]
    }

    fn description(&self) -> &'static str {
        "planning-only pipeline: Stage 0 → Specify → Plan → Tasks (P92)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let spec_id = args.split_whitespace().next().unwrap_or("");
        if spec_id.is_empty() {
            widget.history_push(crate::history_cell::new_error_event(
                "Missing SPEC ID. Usage: /speckit.plan-pipeline SPEC-ID".to_string(),
            ));
            widget.request_redraw();
            return;
        }

        // Delegate to pipeline_coordinator
        super::super::pipeline_coordinator::handle_spec_plan(widget, spec_id.to_string());
    }

    fn requires_args(&self) -> bool {
        true
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
