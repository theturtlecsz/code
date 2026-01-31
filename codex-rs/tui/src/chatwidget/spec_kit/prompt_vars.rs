//! Unified prompt context builder for TUI and headless parity (SPEC-KIT-982)
//!
//! Provides a single source of truth for building agent prompt context,
//! ensuring identical prompts across TUI and headless execution paths.

use std::path::Path;

use super::ace_client::PlaybookBullet;
use super::maieutic::MaieuticSpec;
use crate::spec_prompts::SpecStage;

/// Budget constants for context sections
const ACE_SECTION_MAX_BYTES: usize = 4 * 1024; // 4KB
const ACE_BULLET_MAX_CHARS: usize = 512;
const MAIEUTIC_SECTION_MAX_BYTES: usize = 4 * 1024; // 4KB
const FILE_SECTION_MAX_BYTES: usize = 20 * 1024; // 20KB per file

/// Result of building prompt context
#[derive(Debug, Clone)]
pub struct PromptContext {
    /// Complete context string ready for ${CONTEXT} substitution
    pub context: String,
    /// IDs of ACE bullets that were included in the context (for attribution)
    pub ace_bullet_ids_used: Vec<i32>,
}

/// Build unified context for agent prompts (SPEC-KIT-982)
///
/// Section order (deterministic):
/// 1. Stage0 context (Divine Truth + Task Brief) if available
/// 2. Maieutic contract (goal, constraints, acceptance, delegation bounds)
/// 3. ACE heuristics section (if enabled and bullets available; dedupe + cap)
/// 4. spec.md + plan/tasks summaries (existing behavior)
///
/// # Arguments
/// - `spec_id`: SPEC identifier (e.g., "SPEC-KIT-982")
/// - `stage`: Current pipeline stage
/// - `spec_dir`: Path to SPEC directory (containing spec.md, plan.md, etc.)
/// - `stage0_context`: Optional Stage 0 context (Divine Truth + Task Brief)
/// - `maieutic_spec`: Optional maieutic contract
/// - `ace_bullets`: Optional ACE playbook bullets
///
/// # Returns
/// - `Ok(PromptContext)`: Context string and ACE bullet IDs used
/// - `Err(String)`: On file read errors
pub fn build_prompt_context(
    spec_id: &str,
    stage: SpecStage,
    spec_dir: &Path,
    stage0_context: Option<&str>,
    maieutic_spec: Option<&MaieuticSpec>,
    ace_bullets: Option<&[PlaybookBullet]>,
) -> Result<PromptContext, String> {
    let mut context = format!("SPEC: {}\n\n", spec_id);
    let mut ace_bullet_ids_used = Vec::new();

    // 1. Stage0 context (Divine Truth + Task Brief)
    if let Some(stage0_ctx) = stage0_context {
        context.push_str("## Stage 0: Shadow Context (Divine Truth + Task Brief)\n\n");
        append_with_budget(&mut context, stage0_ctx, FILE_SECTION_MAX_BYTES / 2);
        context.push_str("\n\n");
        tracing::debug!("prompt_vars: Stage0 context {} chars", stage0_ctx.len());
    }

    // 2. Maieutic contract
    if let Some(maieutic) = maieutic_spec {
        let maieutic_section = format_maieutic_section(maieutic);
        context.push_str("## Maieutic Contract\n\n");
        append_with_budget(&mut context, &maieutic_section, MAIEUTIC_SECTION_MAX_BYTES);
        context.push_str("\n\n");
        tracing::debug!(
            "prompt_vars: Maieutic section {} chars",
            maieutic_section.len()
        );
    }

    // 3. ACE heuristics
    if let Some(bullets) = ace_bullets {
        if !bullets.is_empty() {
            let (ace_section, used_ids) = format_ace_section(bullets);
            if !ace_section.is_empty() {
                context.push_str("## Project Heuristics Learned (ACE)\n\n");
                append_with_budget(&mut context, &ace_section, ACE_SECTION_MAX_BYTES);
                context.push_str("\n\n");
                ace_bullet_ids_used = used_ids;
                tracing::debug!(
                    "prompt_vars: ACE section {} chars, {} bullets",
                    ace_section.len(),
                    ace_bullet_ids_used.len()
                );
            }
        }
    }

    // 4. spec.md content
    let spec_md = spec_dir.join("spec.md");
    if let Ok(spec_content) = std::fs::read_to_string(&spec_md) {
        context.push_str("## spec.md\n\n");
        append_with_budget(&mut context, &spec_content, FILE_SECTION_MAX_BYTES);
        context.push_str("\n\n");
    }

    // 5. plan.md content (for stages after Plan)
    if !matches!(stage, SpecStage::Specify | SpecStage::Plan) {
        let plan_md = spec_dir.join("plan.md");
        if let Ok(plan_content) = std::fs::read_to_string(&plan_md) {
            let useful_content = extract_useful_content(&plan_content);
            context.push_str("## plan.md (summary)\n\n");
            append_with_budget(&mut context, &useful_content, FILE_SECTION_MAX_BYTES);
            context.push_str("\n\n");
        }
    }

    // 6. tasks.md content (for stages after Tasks)
    if matches!(
        stage,
        SpecStage::Implement | SpecStage::Validate | SpecStage::Audit | SpecStage::Unlock
    ) {
        let tasks_md = spec_dir.join("tasks.md");
        if let Ok(tasks_content) = std::fs::read_to_string(&tasks_md) {
            let useful_content = extract_useful_content(&tasks_content);
            context.push_str("## tasks.md (summary)\n\n");
            append_with_budget(&mut context, &useful_content, FILE_SECTION_MAX_BYTES);
            context.push_str("\n\n");
        }
    }

    Ok(PromptContext {
        context,
        ace_bullet_ids_used,
    })
}

/// Format maieutic contract as markdown
fn format_maieutic_section(maieutic: &MaieuticSpec) -> String {
    let mut section = String::new();

    section.push_str(&format!("**Goal:** {}\n\n", maieutic.goal));

    if !maieutic.constraints.is_empty() {
        section.push_str("**Constraints:**\n");
        for c in &maieutic.constraints {
            section.push_str(&format!("- {}\n", c));
        }
        section.push('\n');
    }

    if !maieutic.acceptance_criteria.is_empty() {
        section.push_str("**Acceptance Criteria:**\n");
        for ac in &maieutic.acceptance_criteria {
            section.push_str(&format!("- {}\n", ac));
        }
        section.push('\n');
    }

    if !maieutic.risks.is_empty() {
        section.push_str("**Known Risks:**\n");
        for r in &maieutic.risks {
            section.push_str(&format!("- {}\n", r));
        }
        section.push('\n');
    }

    // Delegation bounds summary
    section.push_str("**Delegation Bounds:**\n");
    section.push_str(&format!(
        "- Auto-approve file writes: {}\n",
        maieutic.delegation_bounds.auto_approve_file_writes
    ));
    if !maieutic.delegation_bounds.auto_approve_commands.is_empty() {
        section.push_str(&format!(
            "- Auto-approve commands: {}\n",
            maieutic.delegation_bounds.auto_approve_commands.join(", ")
        ));
    }
    if !maieutic.delegation_bounds.require_approval_for.is_empty() {
        section.push_str(&format!(
            "- Require approval: {}\n",
            maieutic.delegation_bounds.require_approval_for.join(", ")
        ));
    }

    section
}

/// Format ACE playbook bullets as markdown
///
/// Returns (formatted_section, bullet_ids_used)
fn format_ace_section(bullets: &[PlaybookBullet]) -> (String, Vec<i32>) {
    let mut section = String::new();
    let mut ids_used = Vec::new();
    let mut total_bytes = 0;

    // Deduplicate by text (case-insensitive)
    let mut seen_texts = std::collections::HashSet::new();

    for bullet in bullets {
        // Skip duplicates
        let normalized = bullet.text.to_lowercase();
        if seen_texts.contains(&normalized) {
            continue;
        }
        seen_texts.insert(normalized);

        // Truncate individual bullet if too long
        let text = if bullet.text.len() > ACE_BULLET_MAX_CHARS {
            format!(
                "{}...[truncated]",
                &bullet
                    .text
                    .chars()
                    .take(ACE_BULLET_MAX_CHARS)
                    .collect::<String>()
            )
        } else {
            bullet.text.clone()
        };

        let bullet_line = format!("- {}\n", text);

        // Check budget
        if total_bytes + bullet_line.len() > ACE_SECTION_MAX_BYTES {
            section.push_str("\n[...ACE heuristics truncated due to budget...]\n");
            break;
        }

        section.push_str(&bullet_line);
        total_bytes += bullet_line.len();

        if let Some(id) = bullet.id {
            ids_used.push(id);
        }
    }

    (section, ids_used)
}

/// Append content to buffer with budget enforcement
fn append_with_budget(buffer: &mut String, content: &str, max_bytes: usize) {
    if content.len() <= max_bytes {
        buffer.push_str(content);
    } else {
        // Truncate at char boundary
        let truncated: String = content.chars().take(max_bytes).collect();
        buffer.push_str(&truncated);
        buffer.push_str(&format!(
            "\n\n[...truncated {} chars...]\n",
            content.len() - truncated.len()
        ));
    }
}

/// Extract useful content from stage files (skip debug sections)
fn extract_useful_content(content: &str) -> String {
    content
        .lines()
        .filter(|line| {
            // Skip debug/internal sections
            !line.starts_with("<!-- DEBUG")
                && !line.starts_with("<!-- INTERNAL")
                && !line.starts_with("[DEBUG]")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_test_maieutic() -> MaieuticSpec {
        use super::super::maieutic::{DelegationBounds, ElicitationMode};
        use chrono::Utc;

        MaieuticSpec {
            spec_id: "TEST-001".to_string(),
            run_id: "run-123".to_string(),
            timestamp: Utc::now(),
            version: "1.0".to_string(),
            goal: "Test goal".to_string(),
            constraints: vec!["Constraint 1".to_string()],
            acceptance_criteria: vec!["Criterion 1".to_string()],
            risks: vec!["Risk 1".to_string()],
            delegation_bounds: DelegationBounds {
                auto_approve_file_writes: true,
                auto_approve_commands: vec!["cargo test".to_string()],
                require_approval_for: vec![],
                max_iterations_without_check: 5,
            },
            elicitation_mode: ElicitationMode::Interactive,
            duration_ms: 1000,
        }
    }

    fn make_test_bullets() -> Vec<PlaybookBullet> {
        vec![
            PlaybookBullet {
                id: Some(1),
                text: "Always run tests before commit".to_string(),
                helpful: true,
                harmful: false,
                confidence: 0.9,
                source: Some("learning".to_string()),
            },
            PlaybookBullet {
                id: Some(2),
                text: "Use cargo fmt for formatting".to_string(),
                helpful: true,
                harmful: false,
                confidence: 0.8,
                source: None,
            },
        ]
    }

    #[test]
    fn test_build_prompt_context_basic() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("spec.md"), "# Test Spec\n\nContent here").unwrap();

        let result =
            build_prompt_context("TEST-001", SpecStage::Plan, temp.path(), None, None, None);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert!(ctx.context.contains("SPEC: TEST-001"));
        assert!(ctx.context.contains("## spec.md"));
        assert!(ctx.ace_bullet_ids_used.is_empty());
    }

    #[test]
    fn test_build_prompt_context_with_stage0() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("spec.md"), "# Test").unwrap();

        let result = build_prompt_context(
            "TEST-001",
            SpecStage::Plan,
            temp.path(),
            Some("Divine Truth: This is the truth"),
            None,
            None,
        );

        let ctx = result.unwrap();
        assert!(ctx.context.contains("Stage 0: Shadow Context"));
        assert!(ctx.context.contains("Divine Truth"));
    }

    #[test]
    fn test_build_prompt_context_with_maieutic() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("spec.md"), "# Test").unwrap();

        let maieutic = make_test_maieutic();
        let result = build_prompt_context(
            "TEST-001",
            SpecStage::Plan,
            temp.path(),
            None,
            Some(&maieutic),
            None,
        );

        let ctx = result.unwrap();
        assert!(ctx.context.contains("Maieutic Contract"));
        assert!(ctx.context.contains("Test goal"));
        assert!(ctx.context.contains("Constraint 1"));
    }

    #[test]
    fn test_build_prompt_context_with_ace() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("spec.md"), "# Test").unwrap();

        let bullets = make_test_bullets();
        let result = build_prompt_context(
            "TEST-001",
            SpecStage::Plan,
            temp.path(),
            None,
            None,
            Some(&bullets),
        );

        let ctx = result.unwrap();
        assert!(ctx.context.contains("Project Heuristics Learned (ACE)"));
        assert!(ctx.context.contains("Always run tests"));
        assert_eq!(ctx.ace_bullet_ids_used, vec![1, 2]);
    }

    #[test]
    fn test_ace_section_deduplication() {
        let bullets = vec![
            PlaybookBullet {
                id: Some(1),
                text: "Same text".to_string(),
                helpful: true,
                harmful: false,
                confidence: 0.9,
                source: None,
            },
            PlaybookBullet {
                id: Some(2),
                text: "same text".to_string(), // Case-insensitive duplicate
                helpful: true,
                harmful: false,
                confidence: 0.8,
                source: None,
            },
        ];

        let (section, ids) = format_ace_section(&bullets);
        // Should only include one bullet (first one)
        assert_eq!(ids, vec![1]);
        assert_eq!(section.matches("- ").count(), 1);
    }

    #[test]
    fn test_ace_section_truncation() {
        let long_text = "x".repeat(1000);
        let bullets = vec![PlaybookBullet {
            id: Some(1),
            text: long_text,
            helpful: true,
            harmful: false,
            confidence: 0.9,
            source: None,
        }];

        let (section, _) = format_ace_section(&bullets);
        assert!(section.contains("[truncated]"));
        assert!(section.len() < 600); // Should be truncated
    }

    #[test]
    fn test_section_order_deterministic() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("spec.md"), "# Spec").unwrap();

        let maieutic = make_test_maieutic();
        let bullets = make_test_bullets();

        let result = build_prompt_context(
            "TEST-001",
            SpecStage::Plan,
            temp.path(),
            Some("Stage0 context"),
            Some(&maieutic),
            Some(&bullets),
        );

        let ctx = result.unwrap();

        // Verify deterministic order: Stage0 < Maieutic < ACE < spec.md
        let stage0_pos = ctx.context.find("Stage 0:").unwrap();
        let maieutic_pos = ctx.context.find("Maieutic Contract").unwrap();
        let ace_pos = ctx.context.find("Project Heuristics").unwrap();
        let spec_pos = ctx.context.find("## spec.md").unwrap();

        assert!(stage0_pos < maieutic_pos);
        assert!(maieutic_pos < ace_pos);
        assert!(ace_pos < spec_pos);
    }
}
