//! Headless prompt building for agent execution (SPEC-KIT-900)
//!
//! Provides prompt building for headless mode without ChatWidget dependency.
//! Extracted from `agent_orchestrator.rs:build_individual_agent_prompt()`.
//!
//! D113/D133: Now uses unified prompt-source API for TUI/headless parity.

use std::path::Path;

use super::runner::HeadlessError;
use crate::chatwidget::spec_kit::ace_client::PlaybookBullet;
use crate::chatwidget::spec_kit::gate_evaluation::agent_for_stage;
use crate::chatwidget::spec_kit::maieutic::MaieuticSpec;
use crate::spec_prompts::{SpecAgent, SpecStage, get_prompt_with_version, render_prompt_text};

/// Maximum size for individual file content in prompts (~20KB)
const MAX_FILE_SIZE: usize = 20_000;

/// Build a prompt for a specific agent in headless mode
///
/// This is the headless equivalent of `build_individual_agent_prompt()` from
/// `agent_orchestrator.rs`. It doesn't depend on ChatWidget or TUI state.
///
/// D113/D133: Now uses unified prompt-source API for TUI/headless parity.
/// SPEC-KIT-982: Added maieutic_spec and ace_bullets for contract injection.
///
/// # Arguments
/// - `spec_id`: SPEC identifier (e.g., "SPEC-KIT-900")
/// - `stage`: Stage name (e.g., "plan", "tasks")
/// - `agent_name`: Agent canonical name from preferred_agent_for_stage() (e.g., "gemini", "claude")
/// - `cwd`: Working directory (project root)
/// - `stage0_context`: Optional Stage 0 context (Divine Truth + Task Brief)
/// - `maieutic_spec`: Optional maieutic delegation contract
/// - `ace_bullets`: Optional ACE project heuristics
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
    maieutic_spec: Option<&MaieuticSpec>, // SPEC-KIT-982: Delegation contract
    ace_bullets: Option<&[PlaybookBullet]>, // SPEC-KIT-982: Project heuristics
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

    // SPEC-KIT-982: Use unified prompt context builder for TUI/headless parity
    // This provides deterministic section order, budget enforcement, and ACE/maieutic support
    let prompt_context = crate::chatwidget::spec_kit::prompt_vars::build_prompt_context(
        spec_id,
        spec_stage,
        &spec_dir,
        stage0_context,
        maieutic_spec,
        ace_bullets,
    )
    .map_err(HeadlessError::InfraError)?;

    // Log context stats for debugging
    tracing::debug!(
        "prompt_vars: context {} chars, {} ACE bullets used",
        prompt_context.context.len(),
        prompt_context.ace_bullet_ids_used.len()
    );

    // D113/D133: Use unified render_prompt_text() for all substitutions
    // This ensures ${TEMPLATE:*} expansion, real model metadata, and consistent handling
    let prompt = render_prompt_text(
        &prompt_template,
        &prompt_version,
        &[("SPEC_ID", spec_id), ("CONTEXT", &prompt_context.context)],
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
/// D113/D133: Returns single preferred agent matching TUI's agent selection logic.
/// This ensures headless execution uses the same agent selection as TUI (GR-001 compliant).
///
/// **SPEC-KIT-981**: Now accepts optional config for stage→agent overrides.
/// If config specifies a valid agent for the stage, that agent is used;
/// otherwise falls back to `preferred_agent_for_stage()` defaults.
///
/// Note: The `cwd` parameter is kept for API compatibility but is no longer used
/// since agent selection is now based on the canonical stage-to-agent mapping.
pub fn get_agents_for_stage(
    _cwd: &Path,
    stage: &str,
    stage_agents: Option<&codex_core::config_types::SpecKitStageAgents>,
) -> Result<Vec<String>, HeadlessError> {
    // D113/D133: Parse stage to SpecStage enum for parity with TUI
    let spec_stage = SpecStage::from_stage_name(stage)
        .ok_or_else(|| HeadlessError::InfraError(format!("Unknown stage: {}", stage)))?;

    // SPEC-KIT-981: Use config-aware agent selection for parity with TUI
    let resolved = agent_for_stage(spec_stage, stage_agents);

    tracing::debug!(
        stage = %stage,
        agent = %resolved.canonical_name(),
        config_override = stage_agents.is_some(),
        "Resolved headless stage agent"
    );

    // Return single agent as vector (maintains API contract)
    Ok(vec![resolved.canonical_name().to_string()])
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
        // SPEC-KIT-981: Defaults changed to GPT (gpt_pro for most, gpt_codex for implement)
        let temp = TempDir::new().unwrap();
        setup_test_spec(&temp, "TEST-001");

        // Plan stage should return GptPro (no config override)
        let agents = get_agents_for_stage(temp.path(), "plan", None).unwrap();
        assert_eq!(agents.len(), 1, "Should return exactly one agent");
        assert_eq!(agents[0], "gpt_pro", "Plan stage should prefer GptPro");

        // Tasks stage should return GptPro
        let agents = get_agents_for_stage(temp.path(), "tasks", None).unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], "gpt_pro", "Tasks stage should prefer GptPro");

        // Implement stage should return GptCodex
        let agents = get_agents_for_stage(temp.path(), "implement", None).unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(
            agents[0], "gpt_codex",
            "Implement stage should prefer GptCodex"
        );
    }

    #[test]
    fn test_get_agents_for_stage_with_config_override() {
        // SPEC-KIT-981: Config overrides should change agent selection
        use codex_core::config_types::SpecKitStageAgents;

        let temp = TempDir::new().unwrap();
        setup_test_spec(&temp, "TEST-001");

        let mut config = SpecKitStageAgents::default();
        config.plan = Some("claude".to_string());

        // With config override
        let agents = get_agents_for_stage(temp.path(), "plan", Some(&config)).unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], "claude", "Should use config override");

        // Without config override (None) should use default
        let agents = get_agents_for_stage(temp.path(), "plan", None).unwrap();
        assert_eq!(agents[0], "gpt_pro", "Should use default when no config");
    }

    #[test]
    fn test_headless_parity_with_tui_agent_selection() {
        // D113/D133: Verify same config produces same agent in both paths
        use crate::chatwidget::spec_kit::gate_evaluation::agent_for_stage;
        use crate::spec_prompts::SpecStage;
        use codex_core::config_types::SpecKitStageAgents;

        let temp = TempDir::new().unwrap();
        setup_test_spec(&temp, "TEST-001");

        let mut config = SpecKitStageAgents::default();
        config.tasks = Some("gemini".to_string());

        // TUI path (gate_evaluation)
        let tui_agent = agent_for_stage(SpecStage::Tasks, Some(&config));

        // Headless path (prompt_builder)
        let headless_agents = get_agents_for_stage(temp.path(), "tasks", Some(&config)).unwrap();

        assert_eq!(
            tui_agent.canonical_name(),
            headless_agents[0],
            "TUI and headless must resolve same agent for same config"
        );
    }

    #[test]
    fn test_build_headless_prompt() {
        let temp = TempDir::new().unwrap();
        setup_test_spec(&temp, "TEST-001");

        // SPEC-KIT-982: Pass None for maieutic and ACE (no injection in this test)
        let prompt =
            build_headless_prompt("TEST-001", "plan", "gemini", temp.path(), None, None, None)
                .unwrap();

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

        // SPEC-KIT-982: Pass None for maieutic and ACE
        let prompt =
            build_headless_prompt("TEST-TPL", "plan", "gemini", temp.path(), None, None, None)
                .unwrap();

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

    // =========================================================================
    // D113/D133 Parity Golden Tests
    // =========================================================================

    /// Synchronous helper that replicates TUI logic for testing parity
    fn build_tui_prompt_sync(
        spec_id: &str,
        stage: SpecStage,
        agent_name: &str,
        cwd: &std::path::Path,
        stage0_context: Option<&str>,
        maieutic_spec: Option<&crate::chatwidget::spec_kit::maieutic::MaieuticSpec>,
        ace_bullets: Option<&[PlaybookBullet]>,
    ) -> Result<String, String> {
        use crate::spec_prompts::{SpecAgent, get_prompt_with_version, render_prompt_text};

        // Replicate build_individual_agent_prompt logic
        let spec_agent = SpecAgent::from_string(agent_name)
            .ok_or_else(|| format!("Unknown agent: {}", agent_name))?;

        let stage_key = stage.key();
        let (prompt_template, prompt_version) =
            get_prompt_with_version(stage_key, spec_agent, Some(cwd))
                .ok_or_else(|| format!("No prompt for {} in {}", agent_name, stage_key))?;

        let spec_dir =
            find_spec_directory(cwd, spec_id).map_err(|e| format!("Find spec dir: {}", e))?;

        let prompt_context = crate::chatwidget::spec_kit::prompt_vars::build_prompt_context(
            spec_id,
            stage,
            &spec_dir,
            stage0_context,
            maieutic_spec,
            ace_bullets,
        )?;

        let prompt = render_prompt_text(
            &prompt_template,
            &prompt_version,
            &[("SPEC_ID", spec_id), ("CONTEXT", &prompt_context.context)],
            stage,
            spec_agent,
        );

        Ok(prompt)
    }

    #[test]
    fn test_parity_tui_headless_identical_prompts() {
        // D113/D133: Verify TUI and headless produce byte-identical prompts
        use crate::spec_prompts::SpecStage;

        let temp = TempDir::new().unwrap();
        let spec_id = "PARITY-001";
        setup_test_spec(&temp, spec_id);

        // Build via TUI path (sync helper)
        let tui_prompt = build_tui_prompt_sync(
            spec_id,
            SpecStage::Plan,
            "gemini",
            temp.path(),
            Some("Divine Truth: Test parity."),
            None,
            None,
        )
        .expect("TUI prompt should build");

        // Build via headless path
        let headless_prompt = build_headless_prompt(
            spec_id,
            "plan",
            "gemini",
            temp.path(),
            Some("Divine Truth: Test parity."),
            None,
            None,
        )
        .expect("Headless prompt should build");

        // GOLDEN ASSERTION: Byte-identical
        assert_eq!(
            tui_prompt,
            headless_prompt,
            "D113/D133 PARITY VIOLATION: TUI and headless prompts differ!\n\n\
             TUI length: {} chars\nHeadless length: {} chars",
            tui_prompt.len(),
            headless_prompt.len()
        );

        // Verify no template leakage in either path
        assert!(
            !tui_prompt.contains("${TEMPLATE:"),
            "TUI prompt contains leaked template token"
        );
        assert!(
            !headless_prompt.contains("${TEMPLATE:"),
            "Headless prompt contains leaked template token"
        );
    }

    #[test]
    fn test_parity_with_maieutic_and_ace() {
        // D113/D133: Verify parity with maieutic and ACE bullets
        use crate::chatwidget::spec_kit::maieutic::{
            DelegationBounds, ElicitationMode, MaieuticSpec,
        };
        use crate::spec_prompts::SpecStage;
        use chrono::Utc;

        let temp = TempDir::new().unwrap();
        let spec_id = "PARITY-002";
        setup_test_spec(&temp, spec_id);

        // Create test maieutic spec
        let maieutic = MaieuticSpec {
            spec_id: spec_id.to_string(),
            run_id: "test-run".to_string(),
            timestamp: Utc::now(),
            version: "1.0".to_string(),
            goal: "Test parity with maieutic".to_string(),
            constraints: vec!["No side effects".to_string()],
            acceptance_criteria: vec!["Tests pass".to_string()],
            risks: vec![],
            delegation_bounds: DelegationBounds {
                auto_approve_file_writes: true,
                auto_approve_commands: vec!["cargo test".to_string()],
                require_approval_for: vec![],
                max_iterations_without_check: 3,
            },
            elicitation_mode: ElicitationMode::PreSupplied,
            duration_ms: 100,
        };

        // Create test ACE bullets
        let bullets = vec![PlaybookBullet {
            id: Some(1),
            text: "Always run tests".to_string(),
            helpful: true,
            harmful: false,
            confidence: 0.9,
            source: Some("test".to_string()),
        }];

        // Build via TUI path
        let tui_prompt = build_tui_prompt_sync(
            spec_id,
            SpecStage::Plan,
            "gemini",
            temp.path(),
            None,
            Some(&maieutic),
            Some(&bullets),
        )
        .expect("TUI prompt with maieutic/ACE should build");

        // Build via headless path
        let headless_prompt = build_headless_prompt(
            spec_id,
            "plan",
            "gemini",
            temp.path(),
            None,
            Some(&maieutic),
            Some(&bullets),
        )
        .expect("Headless prompt with maieutic/ACE should build");

        // GOLDEN ASSERTION: Byte-identical
        assert_eq!(
            tui_prompt, headless_prompt,
            "Parity violation with maieutic/ACE"
        );

        // Verify maieutic section present
        assert!(
            tui_prompt.contains("Maieutic Contract"),
            "Maieutic section missing"
        );
        assert!(
            tui_prompt.contains("Test parity with maieutic"),
            "Maieutic goal missing"
        );

        // Verify ACE section present
        assert!(
            tui_prompt.contains("Project Heuristics Learned (ACE)"),
            "ACE section missing"
        );
        assert!(
            tui_prompt.contains("Always run tests"),
            "ACE bullet missing"
        );
    }
}
