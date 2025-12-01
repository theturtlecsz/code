//! Vision builder event handlers (P93/SPEC-KIT-105)
//!
//! Handles completion and cancellation events from the vision builder modal.
//! Maps vision answers to constitution memories with appropriate types and priorities.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use ratatui::text::Line;

use crate::chatwidget::ChatWidget;
use crate::history_cell::{new_error_event, HistoryCellType, PlainHistoryCell};

/// Called when user completes the vision builder modal with all answers
pub fn on_vision_builder_submitted(widget: &mut ChatWidget, answers: HashMap<String, String>) {
    // Extract answers by category
    let target_users = answers.get("Users").cloned().unwrap_or_default();
    let problem_statement = answers.get("Problem").cloned().unwrap_or_default();
    let goals_raw = answers.get("Goals").cloned().unwrap_or_default();
    let nongoals_raw = answers.get("NonGoals").cloned().unwrap_or_default();
    let principles_raw = answers.get("Principles").cloned().unwrap_or_default();

    // Parse semicolon-separated lists
    let goals: Vec<String> = goals_raw
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let nongoals: Vec<String> = nongoals_raw
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let principles: Vec<String> = principles_raw
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Connect to overlay DB
    let config = match codex_stage0::Stage0Config::load() {
        Ok(c) => c,
        Err(e) => {
            widget.history_push(new_error_event(format!(
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
            widget.history_push(new_error_event(format!(
                "Failed to connect to overlay DB: {}",
                e
            )));
            widget.request_redraw();
            return;
        }
    };

    let mut errors: Vec<String> = Vec::new();

    // Store goals as ConstitutionType::Goal (priority 8)
    for (i, goal) in goals.iter().enumerate() {
        let memory_id = format!("vision-goal-{}", uuid::Uuid::new_v4());
        if let Err(e) =
            db.upsert_constitution_memory(&memory_id, codex_stage0::ConstitutionType::Goal, goal)
        {
            errors.push(format!("Goal {}: {}", i + 1, e));
        }
    }

    // Store non-goals as ConstitutionType::NonGoal (priority 8)
    for (i, nongoal) in nongoals.iter().enumerate() {
        let memory_id = format!("vision-nongoal-{}", uuid::Uuid::new_v4());
        if let Err(e) = db.upsert_constitution_memory(
            &memory_id,
            codex_stage0::ConstitutionType::NonGoal,
            nongoal,
        ) {
            errors.push(format!("Non-goal {}: {}", i + 1, e));
        }
    }

    // Store principles as ConstitutionType::Principle (priority 9)
    for (i, principle) in principles.iter().enumerate() {
        let memory_id = format!("vision-principle-{}", uuid::Uuid::new_v4());
        if let Err(e) = db.upsert_constitution_memory(
            &memory_id,
            codex_stage0::ConstitutionType::Principle,
            principle,
        ) {
            errors.push(format!("Principle {}: {}", i + 1, e));
        }
    }

    // Compute content hash and increment constitution version
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    target_users.hash(&mut hasher);
    problem_statement.hash(&mut hasher);
    goals_raw.hash(&mut hasher);
    nongoals_raw.hash(&mut hasher);
    principles_raw.hash(&mut hasher);
    let hash = format!("{:016x}", hasher.finish());

    let new_version = match db.increment_constitution_version(Some(&hash)) {
        Ok(v) => v,
        Err(e) => {
            errors.push(format!("Version increment: {}", e));
            0
        }
    };

    // P92/SPEC-KIT-105: Invalidate Tier 2 cache
    let cache_invalidated = match db.invalidate_tier2_by_constitution() {
        Ok(count) => count,
        Err(e) => {
            tracing::warn!("Failed to invalidate Tier 2 cache: {}", e);
            0
        }
    };

    // Generate NL_VISION.md
    let nl_vision_result = generate_nl_vision(
        &widget.config.cwd,
        &target_users,
        &problem_statement,
        &goals,
        &nongoals,
        &principles,
    );

    // Build result message
    let mut lines = vec![
        Line::from("Project Vision captured!"),
        Line::from(""),
    ];

    lines.push(Line::from(format!(
        "   Constitution version: {} | Hash: {}",
        new_version,
        &hash[..8]
    )));
    lines.push(Line::from(format!(
        "   Stored: {} goals, {} non-goals, {} principles",
        goals.len(),
        nongoals.len(),
        principles.len()
    )));

    if cache_invalidated > 0 {
        lines.push(Line::from(format!(
            "   Cache: {} Tier 2 entries invalidated (P92)",
            cache_invalidated
        )));
    }

    lines.push(Line::from(""));

    match nl_vision_result {
        Ok(path) => {
            lines.push(Line::from(format!("   NL_VISION.md: {}", path.display())));
        }
        Err(e) => {
            errors.push(format!("NL_VISION.md: {}", e));
        }
    }

    if !errors.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from("Warnings:"));
        for err in &errors {
            lines.push(Line::from(format!("   - {}", err)));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from("Next steps:"));
    lines.push(Line::from("   /speckit.constitution view - Review stored constitution"));
    lines.push(Line::from("   /speckit.constitution sync - Regenerate constitution.md"));

    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
    widget.request_redraw();
}

/// Called when user cancels the vision builder modal
pub fn on_vision_builder_cancelled(widget: &mut ChatWidget) {
    widget.history_push(PlainHistoryCell::new(
        vec![
            Line::from("Vision capture cancelled"),
            Line::from(""),
            Line::from("To try again: /speckit.vision"),
        ],
        HistoryCellType::Notice,
    ));
    widget.request_redraw();
}

/// Generate NL_VISION.md artifact
fn generate_nl_vision(
    cwd: &std::path::Path,
    target_users: &str,
    problem_statement: &str,
    goals: &[String],
    nongoals: &[String],
    principles: &[String],
) -> Result<std::path::PathBuf, String> {
    let memory_dir = cwd.join("memory");
    std::fs::create_dir_all(&memory_dir)
        .map_err(|e| format!("Failed to create memory directory: {}", e))?;

    let mut md = String::new();
    md.push_str("# Project Vision\n\n");
    md.push_str("_Auto-generated by /speckit.vision. Do not edit directly._\n\n");

    md.push_str("## Target Users\n\n");
    md.push_str(target_users);
    md.push_str("\n\n");

    md.push_str("## Problem Statement\n\n");
    md.push_str(problem_statement);
    md.push_str("\n\n");

    md.push_str("## Goals\n\n");
    for goal in goals {
        md.push_str(&format!("- {}\n", goal));
    }
    md.push('\n');

    md.push_str("## Non-Goals\n\n");
    for nongoal in nongoals {
        md.push_str(&format!("- {}\n", nongoal));
    }
    md.push('\n');

    md.push_str("## Principles\n\n");
    for principle in principles {
        md.push_str(&format!("- {}\n", principle));
    }

    let vision_path = memory_dir.join("NL_VISION.md");
    std::fs::write(&vision_path, &md)
        .map_err(|e| format!("Failed to write NL_VISION.md: {}", e))?;

    Ok(vision_path)
}
