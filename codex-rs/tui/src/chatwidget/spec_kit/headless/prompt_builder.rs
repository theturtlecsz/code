//! Headless prompt building for agent execution (SPEC-KIT-900)
//!
//! Provides prompt building for headless mode without ChatWidget dependency.
//! Extracted from `agent_orchestrator.rs:build_individual_agent_prompt()`.
//!
//! D113/D133: Now uses unified prompt-source API for TUI/headless parity.

use std::path::Path;

use super::runner::HeadlessError;
use crate::chatwidget::spec_kit::gate_evaluation::preferred_agent_for_stage;
use crate::spec_prompts::{SpecAgent, SpecStage, get_prompt_with_version, render_prompt_text};

/// Maximum size for individual file content in prompts (~20KB)
const MAX_FILE_SIZE: usize = 20_000;

/// Build a prompt for a specific agent in headless mode
///
/// This is the headless equivalent of `build_individual_agent_prompt()` from
/// `agent_orchestrator.rs`. It doesn't depend on ChatWidget or TUI state.
///
/// D113/D133: Now uses unified prompt-source API for TUI/headless parity.
///
/// # Arguments
/// - `spec_id`: SPEC identifier (e.g., "SPEC-KIT-900")
/// - `stage`: Stage name (e.g., "plan", "tasks")
/// - `agent_name`: Agent canonical name from preferred_agent_for_stage() (e.g., "gemini", "claude")
/// - `cwd`: Working directory (project root)
/// - `stage0_context`: Optional Stage 0 context (Divine Truth + Task Brief)
///
/// # Returns
/// - `Ok(String)`: The complete prompt to send to the agent
/// - `Err(HeadlessError)`: On file read errors or missing config
pub fn build_headless_prompt(
    spec_id: &str,
    stage: &str,
    agent_name: &str,
    cwd: &Path,
    stage0_context: Option<&str>,
) -> Result<String, HeadlessError> {
    // D113/D133: Parse stage to SpecStage enum for parity with TUI
    let spec_stage = SpecStage::from_stage_name(stage)
        .ok_or_else(|| HeadlessError::InfraError(format!("Unknown stage: {}", stage)))?;

    // D113/D133: Parse agent name to SpecAgent enum
    let spec_agent = SpecAgent::from_string(agent_name)
        .ok_or_else(|| HeadlessError::InfraError(format!("Unknown agent: {}", agent_name)))?;

    let stage_key = spec_stage.key();

    // D113/D133: Use unified prompt-source API (project-local with embedded fallback)
    let (prompt_template, prompt_version) =
        get_prompt_with_version(stage_key, spec_agent, Some(cwd)).ok_or_else(|| {
            HeadlessError::InfraError(format!(
                "No prompt found for agent {} in stage {}",
                agent_name, stage_key
            ))
        })?;

    // Find SPEC directory
    let spec_dir = find_spec_directory(cwd, spec_id)?;

    // Read SPEC files
    let spec_md_path = spec_dir.join("spec.md");
    let spec_content = std::fs::read_to_string(&spec_md_path)
        .map_err(|e| HeadlessError::InfraError(format!("Failed to read spec.md: {}", e)))?;

    // Build context
    let mut context = format!("SPEC: {}\n\n", spec_id);

    // Add Stage 0 context (combined Divine Truth + Task Brief)
    if let Some(stage0_ctx) = stage0_context {
        context.push_str("## Stage 0: Shadow Context (Divine Truth + Task Brief)\n\n");
        if stage0_ctx.len() > MAX_FILE_SIZE / 2 {
            context.push_str(
                &stage0_ctx
                    .chars()
                    .take(MAX_FILE_SIZE / 2)
                    .collect::<String>(),
            );
            context.push_str("\n\n[...Stage 0 context truncated...]\n\n");
        } else {
            context.push_str(stage0_ctx);
            context.push_str("\n\n");
        }
        tracing::info!(
            "  Stage 0: Injected {} chars from combined_context_md()",
            stage0_ctx.len()
        );
    } else {
        // Fallback: Read from TASK_BRIEF.md file
        let task_brief_path = spec_dir.join("evidence").join("TASK_BRIEF.md");
        if let Ok(task_brief) = std::fs::read_to_string(&task_brief_path) {
            context.push_str("## Stage 0: Task Context Brief\n\n");
            if task_brief.len() > MAX_FILE_SIZE / 2 {
                context.push_str(
                    &task_brief
                        .chars()
                        .take(MAX_FILE_SIZE / 2)
                        .collect::<String>(),
                );
                context.push_str("\n\n[...Stage 0 context truncated...]\n\n");
            } else {
                context.push_str(&task_brief);
                context.push_str("\n\n");
            }
            tracing::info!(
                "  Stage 0: Injected {} chars from TASK_BRIEF.md (fallback)",
                task_brief.len()
            );
        }
    }

    // Add spec.md content
    context.push_str("## spec.md\n");
    if spec_content.len() > MAX_FILE_SIZE {
        tracing::warn!(
            "  Truncating spec.md: {} -> {} chars",
            spec_content.len(),
            MAX_FILE_SIZE
        );
        context.push_str(&spec_content.chars().take(MAX_FILE_SIZE).collect::<String>());
        context.push_str(&format!(
            "\n\n[...truncated {} chars...]\n\n",
            spec_content.len() - MAX_FILE_SIZE
        ));
    } else {
        context.push_str(&spec_content);
        context.push_str("\n\n");
    }

    // Add plan.md if available (for Tasks, Implement, Validate, etc.)
    if stage != "plan" {
        let plan_md = spec_dir.join("plan.md");
        if let Ok(plan_content) = std::fs::read_to_string(&plan_md) {
            context.push_str("## plan.md (summary)\n");
            let useful_content = extract_useful_content(&plan_content);
            if useful_content.len() > MAX_FILE_SIZE {
                context.push_str(
                    &useful_content
                        .chars()
                        .take(MAX_FILE_SIZE)
                        .collect::<String>(),
                );
                context.push_str("\n\n[...truncated...]\n\n");
            } else {
                context.push_str(&useful_content);
                context.push_str("\n\n");
            }
        }
    }

    // Add tasks.md if available (for Implement, Validate, etc.)
    if matches!(stage, "implement" | "validate" | "audit" | "unlock") {
        let tasks_md = spec_dir.join("tasks.md");
        if let Ok(tasks_content) = std::fs::read_to_string(&tasks_md) {
            context.push_str("## tasks.md (summary)\n");
            let useful_content = extract_useful_content(&tasks_content);
            if useful_content.len() > MAX_FILE_SIZE {
                context.push_str(
                    &useful_content
                        .chars()
                        .take(MAX_FILE_SIZE)
                        .collect::<String>(),
                );
                context.push_str("\n\n[...truncated...]\n\n");
            } else {
                context.push_str(&useful_content);
                context.push_str("\n\n");
            }
        }
    }

    // D113/D133: Use unified render_prompt_text() for all substitutions
    // This ensures ${TEMPLATE:*} expansion, real model metadata, and consistent handling
    let prompt = render_prompt_text(
        &prompt_template,
        &prompt_version,
        &[("SPEC_ID", spec_id), ("CONTEXT", &context)],
        spec_stage,
        spec_agent,
    );

    // D113/D133: Debug assertion - no template tokens should leak
    debug_assert!(
        !prompt.contains("${TEMPLATE:"),
        "Template token leaked in build_headless_prompt: {}",
        prompt.chars().take(200).collect::<String>()
    );

    Ok(prompt)
}

/// Find the SPEC directory using common locations
fn find_spec_directory(cwd: &Path, spec_id: &str) -> Result<std::path::PathBuf, HeadlessError> {
    // Try common patterns
    let candidates = [cwd.join("docs").join(spec_id), cwd.join(spec_id)];

    for candidate in &candidates {
        if candidate.exists() && candidate.is_dir() {
            // Verify it has spec.md
            if candidate.join("spec.md").exists() {
                return Ok(candidate.clone());
            }
        }
    }

    // Try fuzzy matching (find directories starting with spec_id prefix)
    if let Ok(entries) = std::fs::read_dir(cwd.join("docs")) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(spec_id) && entry.path().join("spec.md").exists() {
                return Ok(entry.path());
            }
        }
    }

    Err(HeadlessError::InfraError(format!(
        "Could not find SPEC directory for {} in {}",
        spec_id,
        cwd.display()
    )))
}

/// Extract useful content from stage files (skip debug sections)
fn extract_useful_content(content: &str) -> String {
    let sections_to_skip = [
        "## Debug:",
        "## Raw JSON",
        "## code\n",
        "## Debug: code",
        "Raw JSON from agents",
        "[2025-",
        "[2026-",
    ];

    let cut_pos = sections_to_skip
        .iter()
        .filter_map(|marker| content.find(marker))
        .min()
        .unwrap_or(content.len());

    content[..cut_pos].trim().to_string()
}

/// Get the agent(s) for a given stage.
///
/// D113/D133: Returns single preferred agent matching TUI's `preferred_agent_for_stage()`.
/// This ensures headless execution uses the same agent selection as TUI (GR-001 compliant).
///
/// Note: The `cwd` parameter is kept for API compatibility but is no longer used
/// since agent selection is now based on the canonical stage-to-agent mapping.
pub fn get_agents_for_stage(_cwd: &Path, stage: &str) -> Result<Vec<String>, HeadlessError> {
    // D113/D133: Parse stage to SpecStage enum for parity with TUI
    let spec_stage = SpecStage::from_stage_name(stage)
        .ok_or_else(|| HeadlessError::InfraError(format!("Unknown stage: {}", stage)))?;

    // D113/D133: Use TUI's preferred_agent_for_stage() for single-agent selection
    let preferred = preferred_agent_for_stage(spec_stage);

    // Return single agent as vector (maintains API contract)
    Ok(vec![preferred.canonical_name().to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_spec(temp: &TempDir, spec_id: &str) {
        let spec_dir = temp.path().join("docs").join(spec_id);
        std::fs::create_dir_all(&spec_dir).unwrap();

        // Create minimal spec.md
        std::fs::write(
            spec_dir.join("spec.md"),
            format!(
                "# {}\n\n## Overview\n\nTest spec for headless execution.\n",
                spec_id
            ),
        )
        .unwrap();

        // Create minimal prompts.json
        let prompts_dir = temp.path().join("docs/spec-kit");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        std::fs::write(
            prompts_dir.join("prompts.json"),
            r#"{
                "spec-plan": {
                    "version": "test",
                    "gemini": {
                        "role": "Researcher",
                        "prompt": "Research ${SPEC_ID}.\n\n${CONTEXT}"
                    },
                    "claude": {
                        "role": "Synthesizer",
                        "prompt": "Synthesize ${SPEC_ID}.\n\n${CONTEXT}"
                    }
                },
                "spec-tasks": {
                    "version": "test",
                    "gemini": {
                        "role": "Researcher",
                        "prompt": "Research tasks for ${SPEC_ID}.\n\n${CONTEXT}"
                    }
                }
            }"#,
        )
        .unwrap();
    }

    #[test]
    fn test_find_spec_directory() {
        let temp = TempDir::new().unwrap();
        setup_test_spec(&temp, "TEST-001");

        let result = find_spec_directory(temp.path(), "TEST-001");
        assert!(result.is_ok());
        assert!(result.unwrap().ends_with("TEST-001"));
    }

    #[test]
    fn test_get_agents_for_stage_single_agent() {
        // D113/D133: Headless now returns single preferred agent matching TUI
        let temp = TempDir::new().unwrap();
        setup_test_spec(&temp, "TEST-001");

        // Plan stage should return Gemini (architect role)
        let agents = get_agents_for_stage(temp.path(), "plan").unwrap();
        assert_eq!(agents.len(), 1, "Should return exactly one agent");
        assert_eq!(agents[0], "gemini", "Plan stage should prefer Gemini");

        // Tasks stage should return Claude (implementer role)
        let agents = get_agents_for_stage(temp.path(), "tasks").unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], "claude", "Tasks stage should prefer Claude");

        // Implement stage should return Claude
        let agents = get_agents_for_stage(temp.path(), "implement").unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], "claude", "Implement stage should prefer Claude");
    }

    #[test]
    fn test_build_headless_prompt() {
        let temp = TempDir::new().unwrap();
        setup_test_spec(&temp, "TEST-001");

        let prompt =
            build_headless_prompt("TEST-001", "plan", "gemini", temp.path(), None).unwrap();

        assert!(prompt.contains("TEST-001"));
        assert!(prompt.contains("Test spec for headless execution"));
    }

    #[test]
    fn test_extract_useful_content() {
        let content = "# Plan\n\nUseful content here.\n\n## Debug:\n\nDebug output here.";
        let useful = extract_useful_content(content);
        assert!(useful.contains("Useful content"));
        assert!(!useful.contains("Debug output"));
    }

    #[test]
    fn test_headless_prompt_no_template_leakage() {
        // D113/D133: Verify ${TEMPLATE:*} tokens are expanded
        let temp = TempDir::new().unwrap();

        // Create prompts.json with template token
        let spec_dir = temp.path().join("docs").join("TEST-TPL");
        std::fs::create_dir_all(&spec_dir).unwrap();
        std::fs::write(spec_dir.join("spec.md"), "# TEST-TPL\n\nTest spec.\n").unwrap();

        let prompts_dir = temp.path().join("docs/spec-kit");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        std::fs::write(
            prompts_dir.join("prompts.json"),
            r#"{
                "spec-plan": {
                    "version": "test-v1",
                    "gemini": {
                        "role": "Researcher",
                        "prompt": "Template: ${TEMPLATE:plan}\nSPEC: ${SPEC_ID}\nContext: ${CONTEXT}"
                    }
                }
            }"#,
        )
        .unwrap();

        let prompt =
            build_headless_prompt("TEST-TPL", "plan", "gemini", temp.path(), None).unwrap();

        // Template token should be expanded (not leaked)
        assert!(
            !prompt.contains("${TEMPLATE:"),
            "Template token leaked: {}",
            prompt.chars().take(300).collect::<String>()
        );
        // Should contain expanded template reference
        assert!(
            prompt.contains("[embedded:plan]") || prompt.contains("templates/plan"),
            "Template should be expanded"
        );
    }
}
