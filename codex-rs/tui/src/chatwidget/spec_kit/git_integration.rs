//! Git integration for automated stage artifact commits
//!
//! SPEC-KIT-922: Auto-commit stage artifacts to maintain clean tree throughout pipeline.
//! SPEC-KIT-971: Capsule checkpoint integration after git commits.
//! SPEC-KIT-977: Policy drift detection at stage boundaries.
//!
//! Problem: /speckit.auto generates stage artifacts (plan.md, tasks.md, etc.) that dirty
//! the git tree, causing guardrail failures at subsequent stages.
//!
//! Solution: Auto-commit stage artifacts after each stage completes, maintaining a clean
//! tree throughout the pipeline while preserving full evidence chain.

use super::error::{Result, SpecKitError};
use crate::memvid_adapter::policy_capture;
use crate::spec_prompts::SpecStage;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Result of a successful stage commit.
#[derive(Debug, Clone)]
pub struct StageCommitResult {
    /// Short commit hash (7-8 chars)
    pub commit_hash: String,
    /// Stage that was committed
    pub stage: SpecStage,
}

/// Auto-commit stage artifacts to maintain clean tree during /speckit.auto pipeline
///
/// Called after each stage's consensus succeeds, before advancing to next stage.
///
/// Commits:
/// - Stage output file (plan.md, tasks.md, etc.)
/// - Consensus artifacts (synthesis + verdict JSON)
/// - Cost summary updates
///
/// ## Returns
/// - `Ok(Some(StageCommitResult))` if commit succeeded with commit hash
/// - `Ok(None)` if no commit was made (no changes, already committed, or disabled)
/// - `Err` on git command failure
///
/// ## SPEC-KIT-971: Capsule Integration
/// The returned commit_hash can be used to create a capsule checkpoint
/// via `CapsuleHandle::commit_stage(spec_id, run_id, stage, Some(commit_hash))`.
pub fn auto_commit_stage_artifacts(
    spec_id: &str,
    stage: SpecStage,
    cwd: &Path,
    auto_commit_enabled: bool,
) -> Result<Option<StageCommitResult>> {
    if !auto_commit_enabled {
        tracing::debug!(
            "Auto-commit disabled, skipping for {} stage",
            stage.display_name()
        );
        return Ok(None);
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
        return Ok(None);
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
        return Ok(None);
    }

    // 4. Commit with descriptive message
    let commit_msg = format_stage_commit_message(spec_id, stage);
    commit_staged_files(&commit_msg, cwd)?;

    // 5. Get the commit hash (SPEC-KIT-971: for capsule checkpoint)
    let commit_hash = get_head_commit_hash(cwd)?;

    tracing::info!(
        "✅ Auto-committed {} stage artifacts for {} ({})",
        stage.display_name(),
        spec_id,
        commit_hash
    );

    Ok(Some(StageCommitResult { commit_hash, stage }))
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

    // 2. Consensus artifacts directory (in spec's evidence dir)
    let consensus_dir = super::evidence::consensus_dir_for_spec(cwd, spec_id);

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

    // 3. Cost summary (in spec's evidence dir)
    let cost_file = super::evidence::evidence_base_for_spec(cwd, spec_id)
        .join("costs")
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

/// Get the short hash of the HEAD commit.
///
/// Returns the 7-8 character short hash used for capsule checkpoint tracking.
pub fn get_head_commit_hash(cwd: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(cwd)
        .output()
        .map_err(|e| SpecKitError::from_string(format!("Git rev-parse failed: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SpecKitError::from_string(format!(
            "Git rev-parse failed: {}",
            stderr
        )));
    }

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(hash)
}

// =============================================================================
// SPEC-KIT-971: Capsule Checkpoint Integration
// =============================================================================

use crate::memvid_adapter::{
    BranchId, CapsuleConfig, CapsuleHandle, CheckpointId, DEFAULT_WORKSPACE_ID,
    default_capsule_config,
};

/// Create a capsule checkpoint after stage completion.
///
/// This should be called after `auto_commit_stage_artifacts` succeeds.
/// It records a checkpoint in the capsule with the stage metadata and
/// emits a StageTransition event.
///
/// ## Parameters
/// - `spec_id`: The SPEC identifier (e.g., "SPEC-KIT-971")
/// - `run_id`: The pipeline run identifier
/// - `stage`: The completed stage
/// - `commit_hash`: Git commit hash from the stage commit (optional)
/// - `cwd`: Working directory (for capsule path resolution)
///
/// ## Returns
/// - `Ok(CheckpointId)` if checkpoint created successfully
/// - `Err` on capsule errors (logged but typically non-fatal)
pub fn create_capsule_checkpoint(
    spec_id: &str,
    run_id: &str,
    stage: SpecStage,
    commit_hash: Option<&str>,
    cwd: &Path,
) -> Result<CheckpointId> {
    // Use canonical capsule config (SPEC-KIT-971/977 alignment)
    let config = default_capsule_config(cwd);

    // Open capsule with write lock for checkpoint creation
    let handle = CapsuleHandle::open(config)
        .map_err(|e| SpecKitError::from_string(format!("Failed to open capsule: {}", e)))?;

    // SPEC-KIT-971: Switch to run branch before any writes
    // Invariant: every run writes to run/<RUN_ID> branch
    handle
        .switch_branch(BranchId::for_run(run_id))
        .map_err(|e| {
            SpecKitError::from_string(format!("Failed to switch capsule branch: {}", e))
        })?;

    // SPEC-KIT-977: Check for policy drift at stage boundary
    // If policy hash differs from last captured hash, capture new snapshot before checkpoint
    let stage0_config = codex_stage0::Stage0Config::load().unwrap_or_default();
    match policy_capture::check_and_recapture_if_changed(&handle, &stage0_config, spec_id, run_id) {
        Ok(Some(new_policy)) => {
            tracing::info!(
                policy_id = %new_policy.policy_id,
                hash = %new_policy.hash,
                stage = %stage.display_name(),
                "Policy drift detected at stage boundary, recaptured snapshot"
            );
        }
        Ok(None) => {
            // No drift - policy unchanged, proceed with checkpoint
            tracing::debug!(
                stage = %stage.display_name(),
                "Policy unchanged at stage boundary"
            );
        }
        Err(e) => {
            // Log warning but continue - checkpoint creation should proceed
            tracing::warn!(
                error = %e,
                stage = %stage.display_name(),
                "Failed to check policy drift, continuing with checkpoint"
            );
        }
    }

    // Create checkpoint at stage boundary
    let checkpoint_id = handle
        .commit_stage(spec_id, run_id, stage.display_name(), commit_hash)
        .map_err(|e| {
            SpecKitError::from_string(format!("Failed to create capsule checkpoint: {}", e))
        })?;

    tracing::info!(
        spec_id = %spec_id,
        run_id = %run_id,
        stage = %stage.display_name(),
        checkpoint_id = %checkpoint_id,
        commit_hash = ?commit_hash,
        "Created capsule checkpoint for stage"
    );

    Ok(checkpoint_id)
}

/// SPEC-KIT-971: Merge run branch to main at Unlock completion.
///
/// ## Invariant
/// Merge modes are `curated` or `full` only - never squash, ff, or rebase.
/// Objects created on run branch become resolvable on main after merge.
///
/// ## Parameters
/// - `spec_id`: Spec identifier (e.g., "SPEC-KIT-971")
/// - `run_id`: Run identifier for the run branch
/// - `cwd`: Working directory (for capsule path resolution)
///
/// ## Returns
/// - `Ok(CheckpointId)` with the merge checkpoint ID
/// - `Err` on capsule errors
pub fn merge_run_branch_to_main(spec_id: &str, run_id: &str, cwd: &Path) -> Result<CheckpointId> {
    use crate::memvid_adapter::MergeMode;

    // Use canonical capsule config
    let config = default_capsule_config(cwd);

    // Open capsule with write lock for merge
    let handle = CapsuleHandle::open(config).map_err(|e| {
        SpecKitError::from_string(format!("Failed to open capsule for merge: {}", e))
    })?;

    // Perform merge: run/<RUN_ID> → main
    // Using Curated mode by default (per SPEC-KIT-971 invariant)
    let from_branch = BranchId::for_run(run_id);
    let to_branch = BranchId::main();

    let merge_checkpoint_id = handle
        .merge_branch(
            &from_branch,
            &to_branch,
            MergeMode::Curated,
            Some(spec_id),
            Some(run_id),
        )
        .map_err(|e| {
            SpecKitError::from_string(format!("Failed to merge run branch to main: {}", e))
        })?;

    tracing::info!(
        spec_id = %spec_id,
        run_id = %run_id,
        from_branch = %from_branch.as_str(),
        to_branch = %to_branch.as_str(),
        merge_checkpoint_id = %merge_checkpoint_id,
        "Merged run branch to main at Unlock"
    );

    Ok(merge_checkpoint_id)
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

    /// SPEC-KIT-971: Test that create_capsule_checkpoint creates a checkpoint.
    ///
    /// This test simulates calling the stage completion hook and verifies
    /// that list_checkpoints() becomes non-empty after the call.
    #[test]
    fn test_create_capsule_checkpoint_creates_checkpoint() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("tempdir");
        let cwd = temp_dir.path();

        // Create .speckit/memvid directory for capsule
        let capsule_dir = cwd.join(".speckit").join("memvid");
        std::fs::create_dir_all(&capsule_dir).expect("create capsule dir");

        // Create capsule checkpoint
        let result = create_capsule_checkpoint(
            "SPEC-TEST-971",
            "run-checkpoint-test",
            SpecStage::Plan,
            Some("abc1234"),
            cwd,
        );

        assert!(result.is_ok(), "create_capsule_checkpoint should succeed");

        let checkpoint_id = result.unwrap();
        assert!(
            checkpoint_id.as_str().contains("SPEC-TEST-971"),
            "Checkpoint ID should contain spec_id"
        );
        assert!(
            checkpoint_id.as_str().contains("Plan"),
            "Checkpoint ID should contain stage name"
        );

        // Verify checkpoint exists in capsule by reopening and listing
        let capsule_path = capsule_dir.join("workspace.mv2");
        let config = CapsuleConfig {
            capsule_path,
            workspace_id: DEFAULT_WORKSPACE_ID.to_string(),
            ..Default::default()
        };

        // Open read-only to verify checkpoint persisted
        let handle = CapsuleHandle::open_read_only(config).expect("open capsule");
        let checkpoints = handle.list_checkpoints();

        assert!(
            !checkpoints.is_empty(),
            "list_checkpoints() should be non-empty after checkpoint creation"
        );
        assert_eq!(checkpoints.len(), 1, "Should have exactly one checkpoint");

        let cp = &checkpoints[0];
        assert_eq!(cp.spec_id, Some("SPEC-TEST-971".to_string()));
        assert_eq!(cp.stage, Some("Plan".to_string()));
        assert_eq!(cp.commit_hash, Some("abc1234".to_string()));
    }

    /// SPEC-KIT-971: Test checkpoint creation without commit hash.
    #[test]
    fn test_create_capsule_checkpoint_without_commit_hash() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("tempdir");
        let cwd = temp_dir.path();

        // Create .speckit/memvid directory for capsule
        let capsule_dir = cwd.join(".speckit").join("memvid");
        std::fs::create_dir_all(&capsule_dir).expect("create capsule dir");

        // Create capsule checkpoint without commit hash
        let result = create_capsule_checkpoint(
            "SPEC-TEST-971",
            "run-no-hash",
            SpecStage::Tasks,
            None, // No commit hash
            cwd,
        );

        assert!(
            result.is_ok(),
            "create_capsule_checkpoint should succeed without hash"
        );

        // Verify checkpoint has no commit_hash
        let capsule_path = capsule_dir.join("workspace.mv2");
        let config = CapsuleConfig {
            capsule_path,
            workspace_id: DEFAULT_WORKSPACE_ID.to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open_read_only(config).expect("open capsule");
        let checkpoints = handle.list_checkpoints();

        assert_eq!(checkpoints.len(), 1);
        assert_eq!(checkpoints[0].commit_hash, None);
        assert_eq!(checkpoints[0].stage, Some("Tasks".to_string()));
    }

    /// SPEC-KIT-971: Test that StageTransition event is emitted on checkpoint.
    #[test]
    fn test_create_capsule_checkpoint_emits_stage_transition_event() {
        use crate::memvid_adapter::EventType;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("tempdir");
        let cwd = temp_dir.path();

        // Create .speckit/memvid directory
        let capsule_dir = cwd.join(".speckit").join("memvid");
        std::fs::create_dir_all(&capsule_dir).expect("create capsule dir");

        // Create checkpoint
        let _ = create_capsule_checkpoint(
            "SPEC-EVENT-TEST",
            "run-event",
            SpecStage::Implement,
            Some("def5678"),
            cwd,
        )
        .expect("create checkpoint");

        // Open and check events
        let capsule_path = capsule_dir.join("workspace.mv2");
        let config = CapsuleConfig {
            capsule_path,
            workspace_id: DEFAULT_WORKSPACE_ID.to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open_read_only(config).expect("open capsule");
        let events = handle.list_events();

        // Should have a StageTransition event
        let stage_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::StageTransition))
            .collect();

        assert_eq!(
            stage_events.len(),
            1,
            "Should have exactly one StageTransition event"
        );

        let event = stage_events[0];
        assert_eq!(event.spec_id, "SPEC-EVENT-TEST");
        assert_eq!(event.run_id, "run-event");
        assert_eq!(event.stage, Some("Implement".to_string()));

        // Event payload should contain checkpoint info
        let payload = &event.payload;
        assert!(payload.get("stage").is_some());
        assert!(payload.get("checkpoint_id").is_some());
    }

    /// SPEC-KIT-971: Test that branch_id is correctly stamped on events and checkpoints.
    ///
    /// Invariant: Every run writes to run/<RUN_ID> branch.
    /// - StageTransition events should have envelope.branch_id == "run/<RUN_ID>"
    /// - Checkpoint metadata should have branch_id == "run/<RUN_ID>"
    #[test]
    fn test_branch_stamping_invariant() {
        use crate::memvid_adapter::EventType;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("tempdir");
        let cwd = temp_dir.path();

        // Create .speckit/memvid directory
        let capsule_dir = cwd.join(".speckit").join("memvid");
        std::fs::create_dir_all(&capsule_dir).expect("create capsule dir");

        let spec_id = "SPEC-BRANCH-TEST";
        let run_id = "run-branch-001";
        let expected_branch = format!("run/{}", run_id);

        // Create checkpoint (this internally switches to run branch)
        let checkpoint_id =
            create_capsule_checkpoint(spec_id, run_id, SpecStage::Plan, Some("abc1234"), cwd)
                .expect("create checkpoint");

        // Open and verify branch stamping
        let capsule_path = capsule_dir.join("workspace.mv2");
        let config = CapsuleConfig {
            capsule_path,
            workspace_id: DEFAULT_WORKSPACE_ID.to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open_read_only(config).expect("open capsule");

        // Verify checkpoint has correct branch_id
        let checkpoints = handle.list_checkpoints();
        assert_eq!(checkpoints.len(), 1, "Should have exactly one checkpoint");
        let cp = &checkpoints[0];
        assert_eq!(
            cp.branch_id,
            Some(expected_branch.clone()),
            "Checkpoint should have branch_id == run/<RUN_ID>"
        );

        // Verify StageTransition event has correct branch_id
        let events = handle.list_events();
        let stage_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::StageTransition))
            .collect();

        assert_eq!(
            stage_events.len(),
            1,
            "Should have exactly one StageTransition event"
        );
        let event = stage_events[0];
        assert_eq!(
            event.branch_id,
            Some(expected_branch.clone()),
            "StageTransition event should have branch_id == run/<RUN_ID>"
        );

        // Verify events can be filtered by branch
        // Note: There may be multiple events (StageTransition + PolicySnapshotRef from drift check)
        let run_branch = BranchId::for_run(run_id);
        let filtered_events = handle.list_events_filtered(Some(&run_branch));
        assert!(
            !filtered_events.is_empty(),
            "Filtering by run branch should return at least one event"
        );
        // All filtered events should have the correct branch_id
        for event in &filtered_events {
            assert_eq!(
                event.branch_id,
                Some(expected_branch.clone()),
                "All events on run branch should have correct branch_id"
            );
        }

        let filtered_checkpoints = handle.list_checkpoints_filtered(Some(&run_branch));
        assert_eq!(
            filtered_checkpoints.len(),
            1,
            "Filtering by run branch should return the checkpoint"
        );

        // Verify main branch filtering excludes the run events
        let main_branch = BranchId::main();
        let main_events = handle.list_events_filtered(Some(&main_branch));
        assert!(
            main_events.is_empty(),
            "Filtering by main branch should NOT return run events"
        );

        let main_checkpoints = handle.list_checkpoints_filtered(Some(&main_branch));
        assert!(
            main_checkpoints.is_empty(),
            "Filtering by main branch should NOT return run checkpoints"
        );
    }
}
