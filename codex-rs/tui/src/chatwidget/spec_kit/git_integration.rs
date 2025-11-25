//! Git integration for automated stage artifact commits
//!
//! SPEC-KIT-922: Auto-commit stage artifacts to maintain clean tree throughout pipeline.
//!
//! Problem: /speckit.auto generates stage artifacts (plan.md, tasks.md, etc.) that dirty
//! the git tree, causing guardrail failures at subsequent stages.
//!
//! Solution: Auto-commit stage artifacts after each stage completes, maintaining a clean
//! tree throughout the pipeline while preserving full evidence chain.

use super::error::{Result, SpecKitError};
use crate::spec_prompts::SpecStage;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Auto-commit stage artifacts to maintain clean tree during /speckit.auto pipeline
///
/// Called after each stage's consensus succeeds, before advancing to next stage.
///
/// Commits:
/// - Stage output file (plan.md, tasks.md, etc.)
/// - Consensus artifacts (synthesis + verdict JSON)
/// - Cost summary updates
///
/// Returns Ok(()) even if commit fails (non-fatal - pipeline continues).
pub fn auto_commit_stage_artifacts(
    spec_id: &str,
    stage: SpecStage,
    cwd: &Path,
    auto_commit_enabled: bool,
) -> Result<()> {
    if !auto_commit_enabled {
        tracing::debug!(
            "Auto-commit disabled, skipping for {} stage",
            stage.display_name()
        );
        return Ok(());
    }

    tracing::info!(
        "Auto-committing {} stage artifacts for {}",
        stage.display_name(),
        spec_id
    );

    // 1. Collect paths to commit
    let paths_to_commit = collect_stage_artifact_paths(spec_id, stage, cwd)?;

    if paths_to_commit.is_empty() {
        tracing::debug!(
            "No stage artifacts found to commit for {} stage",
            stage.display_name()
        );
        return Ok(());
    }

    tracing::debug!("Found {} artifact paths to commit", paths_to_commit.len());
    for path in &paths_to_commit {
        tracing::debug!("  - {}", path.display());
    }

    // 2. Stage files
    stage_files(&paths_to_commit, cwd)?;

    // 3. Check if there are changes to commit
    if !has_staged_changes(cwd)? {
        tracing::info!(
            "No changes staged for {} stage (files may already be committed)",
            stage.display_name()
        );
        return Ok(());
    }

    // 4. Commit with descriptive message
    let commit_msg = format_stage_commit_message(spec_id, stage);
    commit_staged_files(&commit_msg, cwd)?;

    tracing::info!(
        "✅ Auto-committed {} stage artifacts for {}",
        stage.display_name(),
        spec_id
    );
    Ok(())
}

/// Collect all artifact paths for a given stage
fn collect_stage_artifact_paths(
    spec_id: &str,
    stage: SpecStage,
    cwd: &Path,
) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    // 1. Stage output file (plan.md, tasks.md, etc.)
    let stage_file = cwd
        .join("docs")
        .join(spec_id)
        .join(format!("{}.md", stage.display_name().to_lowercase()));

    if stage_file.exists() {
        paths.push(stage_file);
    }

    // 2. Consensus artifacts directory
    let consensus_dir = cwd
        .join("docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus")
        .join(spec_id);

    if consensus_dir.exists() {
        // Add synthesis and verdict files for this stage
        let stage_name = stage.command_name(); // "spec-plan", "spec-tasks", etc.

        let synthesis = consensus_dir.join(format!("{}_synthesis.json", stage_name));
        if synthesis.exists() {
            paths.push(synthesis);
        }

        let verdict = consensus_dir.join(format!("{}_verdict.json", stage_name));
        if verdict.exists() {
            paths.push(verdict);
        }
    }

    // 3. Cost summary (updated after each stage)
    let cost_file = cwd
        .join("docs/SPEC-OPS-004-integrated-coder-hooks/evidence/costs")
        .join(format!("{}_cost_summary.json", spec_id));

    if cost_file.exists() {
        paths.push(cost_file);
    }

    Ok(paths)
}

/// Stage files using git add
fn stage_files(paths: &[PathBuf], cwd: &Path) -> Result<()> {
    for path in paths {
        let relative_path = path.strip_prefix(cwd).map_err(|e| {
            SpecKitError::from_string(format!("Invalid path {}: {}", path.display(), e))
        })?;

        let output = Command::new("git")
            .args(["add", "--"])
            .arg(relative_path)
            .current_dir(cwd)
            .output()
            .map_err(|e| SpecKitError::from_string(format!("Git add failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecKitError::from_string(format!(
                "Git add failed for {}: {}",
                relative_path.display(),
                stderr
            )));
        }
    }

    Ok(())
}

/// Check if there are staged changes ready to commit
fn has_staged_changes(cwd: &Path) -> Result<bool> {
    let status = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(cwd)
        .status()
        .map_err(|e| SpecKitError::from_string(format!("Git diff failed: {}", e)))?;

    // Exit code 1 means there are staged changes (diff found differences)
    // Exit code 0 means no staged changes (diff is empty)
    Ok(!status.success())
}

/// Commit staged files with descriptive message
fn commit_staged_files(message: &str, cwd: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(cwd)
        .output()
        .map_err(|e| SpecKitError::from_string(format!("Git commit failed: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SpecKitError::from_string(format!(
            "Git commit failed: {}",
            stderr
        )));
    }

    Ok(())
}

/// Format commit message for stage artifact commit
fn format_stage_commit_message(spec_id: &str, stage: SpecStage) -> String {
    format!(
        "feat({}): complete {} stage

Automated commit from /speckit.auto pipeline

Stage artifacts:
- {}.md
- Consensus synthesis and verdict
- Updated cost tracking

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>",
        spec_id,
        stage.display_name(),
        stage.display_name().to_lowercase()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_stage_commit_message() {
        let msg = format_stage_commit_message("SPEC-KIT-900", SpecStage::Plan);
        assert!(msg.contains("SPEC-KIT-900"));
        assert!(msg.contains("Plan stage"));
        assert!(msg.contains("plan.md"));
        assert!(msg.contains("🤖 Generated with"));
    }

    #[test]
    fn test_format_all_stages() {
        for stage in [
            SpecStage::Plan,
            SpecStage::Tasks,
            SpecStage::Implement,
            SpecStage::Validate,
            SpecStage::Audit,
            SpecStage::Unlock,
        ] {
            let msg = format_stage_commit_message("SPEC-TEST", stage);
            assert!(msg.contains("SPEC-TEST"));
            assert!(msg.contains(stage.display_name()));
        }
    }
}
