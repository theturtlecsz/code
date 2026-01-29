//! Headless prompt building for agent execution (SPEC-KIT-900)
//!
//! Provides prompt building for headless mode without ChatWidget dependency.
//! Extracted from `agent_orchestrator.rs:build_individual_agent_prompt()`.

use std::path::Path;

use super::runner::HeadlessError;

/// Maximum size for individual file content in prompts (~20KB)
const MAX_FILE_SIZE: usize = 20_000;

/// Build a prompt for a specific agent in headless mode
///
/// This is the headless equivalent of `build_individual_agent_prompt()` from
/// `agent_orchestrator.rs`. It doesn't depend on ChatWidget or TUI state.
///
/// # Arguments
/// - `spec_id`: SPEC identifier (e.g., "SPEC-KIT-900")
/// - `stage`: Stage name (e.g., "plan", "tasks")
/// - `agent_name`: Agent name from prompts.json (e.g., "gemini", "claude", "gpt_pro")
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
    // Load prompts.json
    let prompts_path = cwd.join("docs/spec-kit/prompts.json");
    let prompts_content = std::fs::read_to_string(&prompts_path).map_err(|e| {
        HeadlessError::InfraError(format!(
            "Failed to read prompts.json at {}: {}",
            prompts_path.display(),
            e
        ))
    })?;

    let prompts: serde_json::Value = serde_json::from_str(&prompts_content)
        .map_err(|e| HeadlessError::InfraError(format!("Failed to parse prompts.json: {}", e)))?;

    // Map stage name to stage key in prompts.json
    let stage_key = match stage {
        "plan" => "spec-plan",
        "tasks" => "spec-tasks",
        "implement" => "spec-implement",
        "validate" => "spec-validate",
        "audit" => "spec-audit",
        "unlock" => "spec-unlock",
        _ => {
            return Err(HeadlessError::InfraError(format!(
                "Unknown stage: {}",
                stage
            )));
        }
    };

    // Get stage-specific prompts
    let stage_prompts = prompts.get(stage_key).ok_or_else(|| {
        HeadlessError::InfraError(format!("No prompts found for stage {}", stage_key))
    })?;

    // Get THIS agent's prompt template
    let prompt_template = stage_prompts
        .get(agent_name)
        .and_then(|v| v.get("prompt"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
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

    // Get prompt metadata (simplified - use defaults for headless)
    let prompt_version = format!("headless-{}", stage);
    let model_id = "headless";
    let model_release = "unknown";
    let reasoning_mode = "default";

    // Replace all placeholders
    let prompt = prompt_template
        .replace("${SPEC_ID}", spec_id)
        .replace("${CONTEXT}", &context)
        .replace("${PROMPT_VERSION}", &prompt_version)
        .replace("${MODEL_ID}", model_id)
        .replace("${MODEL_RELEASE}", model_release)
        .replace("${REASONING_MODE}", reasoning_mode);

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

/// Get the list of agents for a given stage from prompts.json
pub fn get_agents_for_stage(cwd: &Path, stage: &str) -> Result<Vec<String>, HeadlessError> {
    let prompts_path = cwd.join("docs/spec-kit/prompts.json");
    let prompts_content = std::fs::read_to_string(&prompts_path)
        .map_err(|e| HeadlessError::InfraError(format!("Failed to read prompts.json: {}", e)))?;

    let prompts: serde_json::Value = serde_json::from_str(&prompts_content)
        .map_err(|e| HeadlessError::InfraError(format!("Failed to parse prompts.json: {}", e)))?;

    let stage_key = match stage {
        "plan" => "spec-plan",
        "tasks" => "spec-tasks",
        "implement" => "spec-implement",
        "validate" => "spec-validate",
        "audit" => "spec-audit",
        "unlock" => "spec-unlock",
        _ => {
            return Err(HeadlessError::InfraError(format!(
                "Unknown stage: {}",
                stage
            )));
        }
    };

    let stage_prompts = prompts.get(stage_key).ok_or_else(|| {
        HeadlessError::InfraError(format!("No prompts found for stage {}", stage_key))
    })?;

    // Extract agent names from the stage prompts object
    let mut agents = Vec::new();
    if let Some(obj) = stage_prompts.as_object() {
        for key in obj.keys() {
            // Skip non-agent keys like "version"
            if key != "version" {
                agents.push(key.clone());
            }
        }
    }

    // Sort agents for deterministic order (gemini, claude, gpt_pro)
    agents.sort();

    if agents.is_empty() {
        return Err(HeadlessError::InfraError(format!(
            "No agents found for stage {}",
            stage_key
        )));
    }

    Ok(agents)
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
    fn test_get_agents_for_stage() {
        let temp = TempDir::new().unwrap();
        setup_test_spec(&temp, "TEST-001");

        let agents = get_agents_for_stage(temp.path(), "plan").unwrap();
        assert!(agents.contains(&"gemini".to_string()));
        assert!(agents.contains(&"claude".to_string()));
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
}
