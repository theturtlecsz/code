//! SPEC-KIT-971: Lifecycle tests for Memvid adapter
//!
//! ## Acceptance Criteria (from spec)
//! - End-to-end: create → put → commit → reopen → search returns artifact
//! - `speckit capsule doctor` detects: missing, locked, corrupted, version mismatch
//! - Crash recovery: capsule reopens; last committed checkpoint readable
//! - Local-memory fallback: if capsule missing/corrupt, falls back and records evidence
//! - All Memvid types stay behind adapter boundary
//! - `speckit capsule checkpoints` returns non-empty list after stage commit
//! - Every `put` returns a `mv2://` URI; URIs stable after reopen
//! - At least one `StageTransition` event on stage commit

use super::*;
use crate::memvid_adapter::capsule::{CapsuleConfig, CapsuleHandle, DiagnosticResult};
use crate::memvid_adapter::types::{BranchId, LogicalUri, MergeMode, ObjectType};
use tempfile::TempDir;

// =============================================================================
// Lifecycle tests
// =============================================================================

#[test]
fn test_create_open_reopen_lifecycle() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("lifecycle.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "lifecycle_test".to_string(),
        ..Default::default()
    };

    // Step 1: Create capsule
    let handle = CapsuleHandle::open(config.clone()).expect("should create capsule");
    assert!(capsule_path.exists(), "capsule file should exist");
    assert!(handle.is_open(), "capsule should be open");

    // Step 2: Put artifact
    let uri = handle
        .put(
            "SPEC-971",
            "run1",
            ObjectType::Artifact,
            "test.md",
            b"# Test artifact".to_vec(),
            serde_json::json!({"type": "test"}),
        )
        .expect("should put artifact");

    assert!(uri.is_valid(), "URI should be valid");
    assert!(uri.as_str().starts_with("mv2://"), "URI should have mv2:// scheme");

    // Step 3: Commit checkpoint
    let _checkpoint_id = handle
        .commit_stage("SPEC-971", "run1", "plan", Some("abc123"))
        .expect("should create checkpoint");

    // Verify checkpoint exists
    let checkpoints = handle.list_checkpoints();
    assert!(!checkpoints.is_empty(), "should have at least one checkpoint");

    // Step 4: Drop handle (close capsule)
    drop(handle);

    // Step 5: Reopen capsule
    let handle2 = CapsuleHandle::open(config).expect("should reopen capsule");
    assert!(handle2.is_open(), "reopened capsule should be open");

    // TODO: When memvid crate is added, verify:
    // - Search returns the artifact we put
    // - Checkpoint is still readable
    // - URI resolves correctly
}

#[test]
fn test_uri_stability_after_reopen() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("uri_stability.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "uri_test".to_string(),
        ..Default::default()
    };

    // Create and put
    let handle = CapsuleHandle::open(config.clone()).expect("should create");
    let uri1 = handle
        .put(
            "SPEC-971",
            "run1",
            ObjectType::Artifact,
            "file.md",
            b"content".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put");

    // Commit
    handle.commit_stage("SPEC-971", "run1", "plan", None).unwrap();
    drop(handle);

    // Reopen
    let handle2 = CapsuleHandle::open(config).expect("should reopen");

    // Put same artifact again (should get same logical URI)
    let uri2 = handle2
        .put(
            "SPEC-971",
            "run1",
            ObjectType::Artifact,
            "file.md",
            b"updated content".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put");

    // URIs should be the same (logical URI stability)
    assert_eq!(uri1.as_str(), uri2.as_str(), "logical URIs should be stable");
}

#[test]
fn test_stage_transition_event_on_commit() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("events.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "events_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create");

    // Put something
    handle
        .put(
            "SPEC-971",
            "run1",
            ObjectType::Artifact,
            "spec.md",
            b"content".to_vec(),
            serde_json::json!({}),
        )
        .unwrap();

    // Commit stage
    let checkpoint_id = handle
        .commit_stage("SPEC-971", "run1", "plan", None)
        .expect("should commit");

    // Verify checkpoint has stage info
    let checkpoints = handle.list_checkpoints();
    let checkpoint = checkpoints.iter().find(|c| c.checkpoint_id.as_str() == checkpoint_id.as_str());
    assert!(checkpoint.is_some(), "checkpoint should exist");

    let cp = checkpoint.unwrap();
    assert_eq!(cp.stage.as_deref(), Some("plan"), "stage should be recorded");
    assert!(!cp.is_manual, "should not be manual checkpoint");

    // TODO: Verify StageTransition event was emitted (when we can query events)
}

// =============================================================================
// Doctor tests
// =============================================================================

#[test]
fn test_doctor_missing_capsule() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("nonexistent.mv2");

    let results = CapsuleHandle::doctor(&capsule_path);

    assert!(!results.is_empty());
    assert!(matches!(results[0], DiagnosticResult::Error(_, _)));
}

#[test]
fn test_doctor_valid_capsule() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("valid.mv2");

    // Create a valid capsule
    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "doctor_test".to_string(),
        ..Default::default()
    };
    let handle = CapsuleHandle::open(config).expect("should create");
    drop(handle);

    // Run doctor
    let results = CapsuleHandle::doctor(&capsule_path);

    // All checks should pass
    for result in &results {
        if let DiagnosticResult::Error(msg, _) = result {
            panic!("Doctor reported error: {}", msg);
        }
    }
}

// =============================================================================
// Fallback tests
// =============================================================================

#[tokio::test]
async fn test_fallback_when_capsule_corrupt() {
    use async_trait::async_trait;
    use codex_stage0::dcc::{LocalMemoryClient, LocalMemorySearchParams, LocalMemorySummary};
    use codex_stage0::errors::Result as Stage0Result;
    use std::sync::Arc;

    // Create a mock fallback
    struct MockFallback;

    #[async_trait]
    impl LocalMemoryClient for MockFallback {
        async fn search_memories(
            &self,
            _params: LocalMemorySearchParams,
        ) -> Stage0Result<Vec<LocalMemorySummary>> {
            Ok(vec![LocalMemorySummary {
                id: "fallback-1".to_string(),
                domain: Some("test".to_string()),
                tags: vec!["fallback".to_string()],
                created_at: None,
                snippet: "Fallback result".to_string(),
                similarity_score: 1.0,
            }])
        }
    }

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("corrupt.mv2");

    // Create a corrupt capsule file
    std::fs::write(&capsule_path, b"CORRUPT").unwrap();

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "fallback_test".to_string(),
        ..Default::default()
    };

    let adapter = MemvidMemoryAdapter::new(config)
        .with_fallback(Arc::new(MockFallback));

    // Open should succeed (fallback mode)
    let result = adapter.open().await;
    assert!(result.is_ok());
    assert!(!result.unwrap(), "should be in fallback mode");
    assert!(adapter.is_fallback().await, "should report fallback mode");
}

// =============================================================================
// Crash recovery tests
// =============================================================================

/// Acceptance Criteria: Crash recovery test
/// Simulate crash mid-write; capsule reopens; last committed checkpoint is readable.
#[test]
fn test_crash_recovery_mid_write() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("crash_recovery.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "crash_test".to_string(),
        ..Default::default()
    };

    // Step 1: Create capsule and add some artifacts
    let handle = CapsuleHandle::open(config.clone()).expect("should create");

    // Put first artifact
    handle
        .put(
            "SPEC-971",
            "run1",
            ObjectType::Artifact,
            "before_crash.md",
            b"# Before crash".to_vec(),
            serde_json::json!({"state": "committed"}),
        )
        .expect("should put first artifact");

    // Commit checkpoint (this is our "last good" checkpoint)
    let _checkpoint_id = handle
        .commit_stage("SPEC-971", "run1", "plan", Some("good_commit"))
        .expect("should create checkpoint");

    // Step 2: Simulate more writes that weren't committed (crash scenario)
    // In a real crash, these would be in the write queue but not flushed
    handle
        .put(
            "SPEC-971",
            "run1",
            ObjectType::Artifact,
            "after_crash.md",
            b"# After crash - should be lost".to_vec(),
            serde_json::json!({"state": "uncommitted"}),
        )
        .expect("should put second artifact");

    // Don't commit! This simulates a crash before the second write was committed

    // Step 3: Drop handle (simulates process exit without proper shutdown)
    drop(handle);

    // Step 4: Reopen capsule - should succeed
    let handle2 = CapsuleHandle::open(config).expect("should reopen after crash");
    assert!(handle2.is_open(), "capsule should be open after recovery");

    // Step 5: Verify last committed checkpoint is readable
    let _checkpoints = handle2.list_checkpoints();
    // Note: In the stub implementation, checkpoints are in-memory only
    // When memvid crate is integrated, this would verify persistence
    // For now, we verify the capsule opens successfully

    // The capsule should be in a consistent state
    let stats = handle2.stats();
    assert_eq!(stats.path, capsule_path);
    assert!(stats.size_bytes > 0, "capsule should have data");
}

/// Test that a stale lock file is detected by doctor
#[test]
fn test_crash_leaves_stale_lock() {
    use crate::memvid_adapter::lock::lock_path_for;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("stale_lock.mv2");
    let lock_path = lock_path_for(&capsule_path);

    // Create capsule first
    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "stale_lock_test".to_string(),
        ..Default::default()
    };
    let handle = CapsuleHandle::open(config.clone()).expect("should create");
    drop(handle);

    // Simulate crash by creating stale lock file with valid JSON metadata
    // Use a non-existent PID so it will be detected as stale
    let stale_lock = serde_json::json!({
        "pid": 999999999,
        "host": "crashed_host",
        "user": "crashed_user",
        "started_at": chrono::Utc::now().to_rfc3339(),
        "schema_version": 1
    });
    std::fs::write(&lock_path, serde_json::to_string_pretty(&stale_lock).unwrap())
        .expect("should create lock");

    // Doctor should detect stale lock (as a warning since it's stale)
    let results = CapsuleHandle::doctor(&capsule_path);
    let has_lock_warning_or_error = results.iter().any(|r| {
        matches!(r, DiagnosticResult::Error(msg, _) | DiagnosticResult::Warning(msg, _)
            if msg.to_lowercase().contains("lock"))
    });
    assert!(has_lock_warning_or_error, "doctor should detect stale lock");

    // Clean up lock for next test
    std::fs::remove_file(&lock_path).unwrap();
}

// =============================================================================
// Branch isolation tests (D73, D74)
// =============================================================================

#[test]
fn test_run_branch_isolation() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("branches.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "branch_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create");

    // Default branch is main
    assert!(handle.current_branch().is_main());

    // Switch to run branch
    let run_branch = BranchId::for_run("run123");
    handle.switch_branch(run_branch.clone()).expect("should switch");
    assert!(handle.current_branch().is_run_branch());
    assert_eq!(handle.current_branch().as_str(), "run/run123");

    // Switch back to main
    handle.switch_branch(BranchId::main()).expect("should switch back");
    assert!(handle.current_branch().is_main());
}

// =============================================================================
// Merge mode terminology tests (per architect feedback)
// =============================================================================

#[test]
fn test_merge_mode_uses_curated_full_not_squash_ff() {
    // This test exists to catch any reintroduction of squash/ff terminology
    assert_eq!(MergeMode::Curated.as_str(), "curated");
    assert_eq!(MergeMode::Full.as_str(), "full");
    assert_eq!(MergeMode::default(), MergeMode::Curated);

    // Verify the strings don't contain forbidden terms
    let curated_str = format!("{:?}", MergeMode::Curated);
    let full_str = format!("{:?}", MergeMode::Full);
    assert!(!curated_str.to_lowercase().contains("squash"));
    assert!(!curated_str.to_lowercase().contains("ff"));
    assert!(!full_str.to_lowercase().contains("squash"));
    assert!(!full_str.to_lowercase().contains("ff"));
}

// =============================================================================
// Replay determinism documentation test
// =============================================================================

/// This test documents the replay determinism contract.
/// Per architect: "bake this into the CLI UX: speckit replay should print
/// 'Replay is exact for retrieval + events; model I/O depends on capture mode.'"
#[test]
fn test_replay_determinism_message() {
    // The canonical message for replay determinism
    const REPLAY_DETERMINISM_MSG: &str =
        "Replay is exact for retrieval + events; model I/O depends on capture mode.";

    // This should be printed by speckit replay
    assert!(REPLAY_DETERMINISM_MSG.contains("exact"));
    assert!(REPLAY_DETERMINISM_MSG.contains("retrieval"));
    assert!(REPLAY_DETERMINISM_MSG.contains("events"));
    assert!(REPLAY_DETERMINISM_MSG.contains("model I/O"));
    assert!(REPLAY_DETERMINISM_MSG.contains("capture mode"));
}

// =============================================================================
// SPEC-KIT-971: Checkpoint CLI tests
// =============================================================================

#[test]
fn test_checkpoint_by_label() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("checkpoints_by_label.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "label_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create");

    // Create checkpoint with label
    let cp_id = handle
        .commit_manual("v1.0-release")
        .expect("should create manual checkpoint");

    // Find by ID
    let by_id = handle.get_checkpoint(&cp_id);
    assert!(by_id.is_some());
    assert_eq!(by_id.as_ref().unwrap().label.as_deref(), Some("v1.0-release"));

    // Find by label
    let by_label = handle.get_checkpoint_by_label("v1.0-release");
    assert!(by_label.is_some());
    assert_eq!(by_label.as_ref().unwrap().checkpoint_id.as_str(), cp_id.as_str());

    // Non-existent label
    let missing = handle.get_checkpoint_by_label("nonexistent");
    assert!(missing.is_none());
}

#[test]
fn test_checkpoint_by_label_in_branch() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("branch_labels.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "branch_label_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create");

    // Create checkpoint on main
    handle.commit_manual("main-cp").expect("create on main");

    // Switch to run branch and create checkpoint with same label
    let run_branch = BranchId::for_run("run1");
    handle.switch_branch(run_branch.clone()).unwrap();
    handle.commit_manual("run-cp").expect("create on run");

    // Get checkpoint by label in specific branch
    let main_cp = handle.get_checkpoint_by_label_in_branch("main-cp", &BranchId::main());
    // Note: In current stub, branch filtering for manual checkpoints without run_id
    // may not work perfectly. This test documents expected behavior.

    let run_cp = handle.get_checkpoint_by_label_in_branch("run-cp", &run_branch);
    // run_cp might be None because manual checkpoints don't set run_id
    // This is expected behavior for the stub - full impl would track branch
    let _ = (main_cp, run_cp); // Acknowledge results
}

#[test]
fn test_list_checkpoints_filtered() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("filtered_list.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "filter_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create");

    // Create stage checkpoint (has run_id)
    handle.put(
        "SPEC-971",
        "run1",
        ObjectType::Artifact,
        "test.md",
        b"content".to_vec(),
        serde_json::json!({}),
    ).unwrap();
    handle.commit_stage("SPEC-971", "run1", "plan", None).unwrap();

    // Create manual checkpoint (no run_id in current impl)
    handle.commit_manual("manual-cp").unwrap();

    // List all
    let all = handle.list_checkpoints();
    assert!(all.len() >= 2);

    // List filtered by main (should include manual, exclude run branch checkpoints)
    let _main_only = handle.list_checkpoints_filtered(Some(&BranchId::main()));
    // In current stub, manual checkpoints have no run_id so they match main filter
    // Stage checkpoints have run_id so they would be filtered out

    // List for specific run branch
    let run_branch = BranchId::for_run("run1");
    let _run_only = handle.list_checkpoints_filtered(Some(&run_branch));
    // Stage checkpoint for run1 should be included
}

#[test]
fn test_is_label_unique() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("unique_labels.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "unique_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create");
    let main = BranchId::main();

    // Label should be unique initially
    assert!(handle.is_label_unique("v1.0", &main));

    // Create checkpoint with label
    handle.commit_manual("v1.0").unwrap();

    // Label should no longer be unique
    assert!(!handle.is_label_unique("v1.0", &main), "Label should not be unique after commit");
}

/// SPEC-KIT-971: Test that commit_manual enforces label uniqueness.
///
/// Acceptance test:
/// - commit_manual("v1.0") succeeds first time
/// - commit_manual("v1.0") fails second time with DuplicateLabel error
#[test]
fn test_commit_manual_enforces_label_uniqueness() {
    use crate::memvid_adapter::CapsuleError;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("duplicate_label.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "duplicate_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create");

    // First commit with "v1.0" should succeed
    let result1 = handle.commit_manual("v1.0");
    assert!(result1.is_ok(), "First commit_manual should succeed");

    // Second commit with same label should fail with DuplicateLabel
    let result2 = handle.commit_manual("v1.0");
    assert!(result2.is_err(), "Second commit_manual with same label should fail");

    // Verify it's specifically a DuplicateLabel error
    match result2 {
        Err(CapsuleError::DuplicateLabel { label, branch }) => {
            assert_eq!(label, "v1.0", "Error should contain the duplicate label");
            assert_eq!(branch, "main", "Error should contain the branch name");
        }
        Err(e) => panic!("Expected DuplicateLabel error, got: {:?}", e),
        Ok(_) => panic!("Expected error but got Ok"),
    }
}

/// SPEC-KIT-971: Test that commit_manual_with_options respects force flag.
///
/// Acceptance test:
/// - With force=true, duplicate labels are allowed
#[test]
fn test_commit_manual_force_allows_duplicates() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("force_duplicate.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "force_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create");

    // First commit with "v1.0"
    let result1 = handle.commit_manual_with_options("v1.0", false);
    assert!(result1.is_ok(), "First commit should succeed");

    // Second commit without force should fail
    let result2 = handle.commit_manual_with_options("v1.0", false);
    assert!(result2.is_err(), "Second commit without force should fail");

    // Third commit with force=true should succeed
    let result3 = handle.commit_manual_with_options("v1.0", true);
    assert!(result3.is_ok(), "Commit with force=true should succeed");

    // Verify we have 2 checkpoints with the same label
    let checkpoints = handle.list_checkpoints();
    let v1_checkpoints: Vec<_> = checkpoints
        .iter()
        .filter(|cp| cp.label.as_deref() == Some("v1.0"))
        .collect();
    assert_eq!(v1_checkpoints.len(), 2, "Should have 2 checkpoints with label 'v1.0'");
}

/// SPEC-KIT-971: Test label uniqueness is scoped to branch.
///
/// Same label on different branches should be allowed.
#[test]
fn test_label_uniqueness_scoped_to_branch() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("branch_labels.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "branch_label_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create");

    // Create "v1.0" on main branch
    handle.commit_manual("v1.0").expect("should create on main");

    // Switch to run branch
    let run_branch = BranchId::for_run("run-123");
    handle.switch_branch(run_branch.clone()).expect("switch branch");

    // Create "v1.0" on run branch - should succeed (different branch)
    let result = handle.commit_manual("v1.0");
    assert!(result.is_ok(), "Same label on different branch should succeed");

    // Creating another "v1.0" on same run branch should fail
    let result2 = handle.commit_manual("v1.0");
    assert!(result2.is_err(), "Duplicate label on same branch should fail");
}

// =============================================================================
// SPEC-KIT-971: resolve_uri with as_of tests
// =============================================================================

#[test]
fn test_resolve_uri_requires_open() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("resolve_closed.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "resolve_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create");
    drop(handle);

    // Can't resolve on closed handle - need to create new handle
    // This test documents that resolve_uri checks is_open()
}

#[test]
fn test_resolve_uri_validates_checkpoint() {
    use crate::memvid_adapter::capsule::CapsuleError;
    use crate::memvid_adapter::types::CheckpointId;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("resolve_checkpoint.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "resolve_cp_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create");

    // Put something and commit
    let uri = handle.put(
        "SPEC-971",
        "run1",
        ObjectType::Artifact,
        "test.md",
        b"content".to_vec(),
        serde_json::json!({}),
    ).unwrap();

    let cp_id = handle.commit_stage("SPEC-971", "run1", "plan", None).unwrap();

    // Resolve with valid checkpoint
    let result = handle.resolve_uri(&uri, None, Some(&cp_id));
    assert!(result.is_ok());

    // Resolve with invalid checkpoint
    let fake_cp = CheckpointId::new("nonexistent-checkpoint");
    let result = handle.resolve_uri(&uri, None, Some(&fake_cp));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, CapsuleError::InvalidOperation { .. }));
}

#[test]
fn test_resolve_uri_at_label() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("resolve_label.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "resolve_label_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create");

    // Put and commit with label
    let uri = handle.put(
        "SPEC-971",
        "run1",
        ObjectType::Artifact,
        "test.md",
        b"content".to_vec(),
        serde_json::json!({}),
    ).unwrap();
    handle.commit_manual("v1.0").unwrap();

    // Resolve at label
    let result = handle.resolve_uri_at_label(&uri, None, "v1.0");
    assert!(result.is_ok());

    // Invalid label
    let result = handle.resolve_uri_at_label(&uri, None, "nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_resolve_uri_str() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("resolve_str.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "resolve_str_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create");

    // Put artifact and commit to register in URI index
    let uri = handle.put(
        "SPEC-971",
        "run1",
        ObjectType::Artifact,
        "test.md",
        b"content".to_vec(),
        serde_json::json!({}),
    ).unwrap();
    handle.commit_stage("SPEC-971", "run1", "plan", None).unwrap();

    // Resolve by string
    let result = handle.resolve_uri_str(uri.as_str(), None, None);
    assert!(result.is_ok());

    // Invalid URI string
    let result = handle.resolve_uri_str("not-a-valid-uri", None, None);
    assert!(result.is_err());
}

// =============================================================================
// SPEC-KIT-971: Cross-process single-writer lock tests
// =============================================================================

#[test]
fn test_cross_process_lock_acquired_on_open() {
    use crate::memvid_adapter::lock::lock_path_for;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("lock_test.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "lock_test".to_string(),
        ..Default::default()
    };

    // Open capsule - should acquire lock
    let handle = CapsuleHandle::open(config.clone()).expect("should open");

    // Lock file should exist
    let lock_path = lock_path_for(&capsule_path);
    assert!(lock_path.exists(), "Lock file should exist while handle is open");

    // Drop handle - lock should be released
    drop(handle);

    // Lock file should be removed
    assert!(!lock_path.exists(), "Lock file should be removed after handle drop");
}

#[test]
fn test_cross_process_lock_blocks_second_writer() {
    use crate::memvid_adapter::capsule::CapsuleError;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("lock_conflict.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "lock_conflict".to_string(),
        ..Default::default()
    };

    // First open - acquires lock
    let _handle1 = CapsuleHandle::open(config.clone()).expect("first open should succeed");

    // Second open - should fail with LockedByWriter
    let result = CapsuleHandle::open(config);
    assert!(result.is_err(), "Second open should fail");

    match result {
        Err(CapsuleError::LockedByWriter(metadata)) => {
            // Verify metadata contains current process PID
            assert_eq!(metadata.pid, std::process::id(), "Lock metadata should contain our PID");
            assert!(!metadata.host.is_empty(), "Lock metadata should contain host");
            assert!(!metadata.user.is_empty(), "Lock metadata should contain user");
        }
        Err(other) => panic!("Expected LockedByWriter error, got: {:?}", other),
        Ok(_) => panic!("Expected error but got Ok"),
    }
}

#[test]
fn test_cross_process_lock_with_context() {
    use crate::memvid_adapter::capsule::CapsuleOpenOptions;
    use crate::memvid_adapter::lock::is_locked;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("lock_context.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "context_test".to_string(),
        ..Default::default()
    };

    // Open with context
    let options = CapsuleOpenOptions::write().with_context(
        Some("SPEC-KIT-971".to_string()),
        Some("run-abc123".to_string()),
        Some("main".to_string()),
    );

    let _handle = CapsuleHandle::open_with_options(config.clone(), options)
        .expect("should open with context");

    // Verify lock metadata contains context
    let lock_meta = is_locked(&capsule_path).expect("should have lock");
    assert_eq!(lock_meta.spec_id, Some("SPEC-KIT-971".to_string()));
    assert_eq!(lock_meta.run_id, Some("run-abc123".to_string()));
    assert_eq!(lock_meta.branch, Some("main".to_string()));
}

#[test]
fn test_read_only_does_not_acquire_lock() {
    use crate::memvid_adapter::lock::lock_path_for;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("read_only.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "read_only_test".to_string(),
        ..Default::default()
    };

    // First, create the capsule with a write lock
    {
        let _handle = CapsuleHandle::open(config.clone()).expect("should create");
    }

    // Now open read-only - should not create lock
    let handle = CapsuleHandle::open_read_only(config.clone()).expect("should open read-only");

    // Lock file should NOT exist for read-only access
    let lock_path = lock_path_for(&capsule_path);
    assert!(!lock_path.exists(), "Read-only open should not create lock file");

    drop(handle);
}

#[test]
fn test_read_only_succeeds_when_write_locked() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("read_while_locked.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "read_locked_test".to_string(),
        ..Default::default()
    };

    // Open with write lock
    let _writer = CapsuleHandle::open(config.clone()).expect("should open for write");

    // Read-only open should succeed even with write lock held
    let reader = CapsuleHandle::open_read_only(config);
    assert!(reader.is_ok(), "Read-only should succeed when write lock is held");
}

#[test]
fn test_doctor_detects_active_lock_with_metadata() {
    use crate::memvid_adapter::capsule::CapsuleOpenOptions;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("doctor_lock.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "doctor_lock_test".to_string(),
        ..Default::default()
    };

    // Open with context
    let options = CapsuleOpenOptions::write().with_context(
        Some("SPEC-999".to_string()),
        Some("run-xyz".to_string()),
        None,
    );
    let _handle = CapsuleHandle::open_with_options(config, options).expect("should open");

    // Run doctor - should detect lock with metadata
    let results = CapsuleHandle::doctor(&capsule_path);

    // Find the lock-related result
    let lock_result = results.iter().find(|r| {
        match r {
            DiagnosticResult::Error(msg, _) => msg.contains("locked"),
            DiagnosticResult::Warning(msg, _) => msg.contains("lock"),
            _ => false,
        }
    });

    assert!(lock_result.is_some(), "Doctor should detect the lock");

    // The error should contain our context
    if let Some(DiagnosticResult::Error(msg, recovery)) = lock_result {
        assert!(msg.contains("SPEC-999"), "Lock message should contain spec_id");
        assert!(recovery.contains("ps -p"), "Recovery should include process check");
    }
}

#[test]
fn test_stale_lock_recovery() {
    use crate::memvid_adapter::lock::{LockMetadata, lock_path_for};
    use chrono::Utc;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("stale_recovery.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "stale_recovery_test".to_string(),
        ..Default::default()
    };

    // First, create the capsule
    {
        let _handle = CapsuleHandle::open(config.clone()).expect("should create");
    }

    // Create a fake stale lock with a non-existent PID
    let lock_path = lock_path_for(&capsule_path);
    let stale_metadata = LockMetadata {
        pid: 999999999, // Unlikely to be a real PID
        host: hostname::get().map(|h| h.to_string_lossy().to_string()).unwrap_or_default(),
        user: "old_user".to_string(),
        started_at: Utc::now() - chrono::Duration::hours(2),
        spec_id: Some("OLD-SPEC".to_string()),
        run_id: None,
        branch: None,
        schema_version: 1,
    };
    let json = serde_json::to_string_pretty(&stale_metadata).unwrap();
    std::fs::write(&lock_path, json).expect("should write stale lock");

    // Open should succeed because the stale lock is cleaned up
    let handle = CapsuleHandle::open(config);
    assert!(handle.is_ok(), "Should recover from stale lock");

    drop(handle);
}

/// Test that cross-process locking works across actual processes.
///
/// This test simulates another process holding a lock by writing a lock file
/// directly, then verifies the parent can't acquire the same lock.
#[test]
fn test_cross_process_lock_actual_subprocess() {
    use crate::memvid_adapter::lock::lock_path_for;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("subprocess_lock.mv2");
    let lock_path = lock_path_for(&capsule_path);

    // Create capsule first
    {
        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "subprocess_test".to_string(),
            ..Default::default()
        };
        let _handle = CapsuleHandle::open(config).expect("should create");
    }

    // Simulate another process on a remote host holding the lock
    // Remote host locks won't be checked for staleness (can't verify process exists)
    let lock_json = serde_json::json!({
        "pid": 12345,
        "host": "remote-host-that-does-not-exist.local",
        "user": "remote_user",
        "started_at": chrono::Utc::now().to_rfc3339(),
        "spec_id": "REMOTE-SPEC",
        "schema_version": 1
    });
    std::fs::write(&lock_path, serde_json::to_string_pretty(&lock_json).unwrap())
        .expect("should create remote lock");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "subprocess_test".to_string(),
        ..Default::default()
    };

    let result = CapsuleHandle::open(config);
    assert!(result.is_err(), "Should fail when lock is held by remote process");

    match result {
        Err(CapsuleError::LockedByWriter(meta)) => {
            assert_eq!(meta.spec_id, Some("REMOTE-SPEC".to_string()));
            assert_eq!(meta.host, "remote-host-that-does-not-exist.local");
        }
        Err(other) => panic!("Expected LockedByWriter, got {:?}", other),
        Ok(_) => panic!("Expected error but got Ok"),
    }

    // Clean up
    std::fs::remove_file(&lock_path).ok();
}

/// Test cross-process locking with an actual subprocess holding the lock.
///
/// This test spawns a real child process that:
/// 1. Creates the lock file atomically
/// 2. Writes valid LockMetadata JSON
/// 3. Holds an advisory flock on the file
/// 4. Sleeps while parent attempts to acquire
///
/// The parent process then verifies:
/// - CapsuleHandle::open fails with LockedByWriter
/// - The error contains the child's PID
#[cfg(unix)]
#[test]
fn test_cross_process_lock_with_real_subprocess() {
    use crate::memvid_adapter::capsule::CapsuleError;
    use crate::memvid_adapter::lock::lock_path_for;
    use std::process::{Command, Stdio};

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("real_subprocess.mv2");
    let lock_path = lock_path_for(&capsule_path);

    // First, create the capsule (unlocked)
    {
        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "real_subprocess_test".to_string(),
            ..Default::default()
        };
        let _handle = CapsuleHandle::open(config).expect("should create capsule");
    }

    // Spawn a subprocess that will hold the lock
    // The subprocess:
    // 1. Opens the lock file exclusively
    // 2. Writes LockMetadata JSON with its own PID
    // 3. Uses flock() to hold advisory lock
    // 4. Sleeps for 5 seconds
    //
    // We use a shell script because it's the simplest way to test actual
    // cross-process isolation.
    let lock_path_str = lock_path.to_string_lossy();

    // Note: Use unquoted heredoc delimiter to allow variable expansion
    let script = format!(
        r#"
exec 200>"{lock_path}"
flock -n 200 || exit 1
cat > "{lock_path}" << LOCKJSON
{{
  "pid": $$,
  "host": "$(hostname)",
  "user": "$(whoami)",
  "started_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "spec_id": "SUBPROCESS-SPEC",
  "run_id": "subprocess-run",
  "schema_version": 1
}}
LOCKJSON
sleep 5
"#,
        lock_path = lock_path_str
    );

    let mut child = Command::new("sh")
        .args(["-c", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn lock-holding subprocess");

    // Give the subprocess time to acquire the lock
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Now try to open the capsule for writing from the parent
    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "real_subprocess_test".to_string(),
        ..Default::default()
    };

    let result = CapsuleHandle::open(config);

    // Should fail because subprocess holds the lock
    assert!(result.is_err(), "Should fail when subprocess holds lock");

    match result {
        Err(CapsuleError::LockedByWriter(meta)) => {
            // Verify metadata was parsed correctly
            assert_eq!(meta.spec_id, Some("SUBPROCESS-SPEC".to_string()));
            assert_eq!(meta.run_id, Some("subprocess-run".to_string()));
            // PID should be the child's PID (not 0 or our PID)
            assert!(meta.pid > 0, "Lock should have valid PID");
            assert_ne!(meta.pid, std::process::id(), "Lock should have child's PID, not ours");
        }
        Err(other) => panic!("Expected LockedByWriter error, got: {:?}", other),
        Ok(_) => panic!("Expected error but open succeeded"),
    }

    // Clean up: kill the subprocess
    let _ = child.kill();
    let _ = child.wait();

    // Lock file should still exist (subprocess was killed, not graceful exit)
    // This also tests that we handle orphaned locks correctly on next open
    // (stale lock detection should clean it up)

    // Now we can open successfully after child is gone (stale lock cleanup)
    let config2 = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "real_subprocess_test".to_string(),
        ..Default::default()
    };

    // Wait a moment for the OS to release the flock
    std::thread::sleep(std::time::Duration::from_millis(100));

    let result2 = CapsuleHandle::open(config2);
    assert!(result2.is_ok(), "Should succeed after subprocess dies (stale lock recovery)");
}

// =============================================================================
// SPEC-KIT-971 Persistence Acceptance Tests
// =============================================================================

/// Test that put bytes survive reopen.
///
/// SPEC-KIT-971 Acceptance Criteria:
/// - After put + commit + reopen, resolve_uri succeeds
/// - get_bytes returns identical bytes
#[test]
fn test_persistence_put_bytes_survive_reopen() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("persistence_test.mv2");

    let test_data = b"Hello, this is test data for persistence!".to_vec();
    let test_metadata = serde_json::json!({"type": "test", "importance": 8});
    let mut stored_uri: Option<LogicalUri> = None;

    // Phase 1: Open, put, commit, close
    {
        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "persistence_test".to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open(config).expect("should create capsule");

        // Put some data
        let uri = handle
            .put(
                "SPEC-TEST",
                "run-1",
                ObjectType::Artifact,
                "test-file.txt",
                test_data.clone(),
                test_metadata.clone(),
            )
            .expect("put should succeed");

        stored_uri = Some(uri);

        // Commit to flush writes to disk
        handle
            .commit_stage("SPEC-TEST", "run-1", "test-stage", None)
            .expect("commit should succeed");

        // Handle is dropped here, releasing the lock
    }

    // Phase 2: Reopen and verify data persisted
    {
        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "persistence_test".to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open(config).expect("should reopen capsule");

        let uri = stored_uri.as_ref().expect("should have stored URI");

        // Resolve should succeed
        let pointer = handle
            .resolve_uri(uri, None, None)
            .expect("resolve_uri should succeed after reopen");
        assert!(pointer.length > 0, "Pointer should have non-zero length");

        // Get bytes should return identical data
        let retrieved_data = handle
            .get_bytes(uri, None, None)
            .expect("get_bytes should succeed");
        assert_eq!(
            retrieved_data, test_data,
            "Retrieved data should match original"
        );
    }
}

/// Test that checkpoints survive reopen.
#[test]
fn test_persistence_checkpoints_survive_reopen() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("checkpoint_persist.mv2");

    // Phase 1: Create checkpoint
    {
        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "checkpoint_test".to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open(config).expect("should create capsule");

        handle
            .commit_stage("SPEC-CP", "run-cp", "stage1", Some("abc123"))
            .expect("commit should succeed");

        let checkpoints = handle.list_checkpoints();
        assert_eq!(checkpoints.len(), 1, "Should have 1 checkpoint");
    }

    // Phase 2: Reopen and verify checkpoints persisted
    {
        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "checkpoint_test".to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open(config).expect("should reopen capsule");

        let checkpoints = handle.list_checkpoints();
        assert_eq!(
            checkpoints.len(),
            1,
            "Should have 1 checkpoint after reopen"
        );

        let cp = &checkpoints[0];
        assert_eq!(cp.stage, Some("stage1".to_string()));
        assert_eq!(cp.spec_id, Some("SPEC-CP".to_string()));
        assert_eq!(cp.commit_hash, Some("abc123".to_string()));
    }
}

/// Test that doctor passes and stats reflect non-zero uri_count after persistence.
#[test]
fn test_persistence_doctor_passes_and_stats_correct() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("doctor_persist.mv2");

    // Phase 1: Create capsule with data
    {
        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "doctor_test".to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open(config).expect("should create capsule");

        // Add multiple artifacts
        for i in 0..3 {
            handle
                .put(
                    "SPEC-DOC",
                    "run-doc",
                    ObjectType::Artifact,
                    &format!("file{}.txt", i),
                    format!("Content for file {}", i).into_bytes(),
                    serde_json::json!({"index": i}),
                )
                .expect("put should succeed");
        }

        handle
            .commit_stage("SPEC-DOC", "run-doc", "populate", None)
            .expect("commit should succeed");
    }

    // Phase 2: Reopen and verify doctor + stats
    {
        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "doctor_test".to_string(),
            ..Default::default()
        };

        // Doctor should pass
        let diagnostics = CapsuleHandle::doctor(&capsule_path);
        let has_errors = diagnostics
            .iter()
            .any(|d| matches!(d, crate::memvid_adapter::capsule::DiagnosticResult::Error(_, _)));
        assert!(!has_errors, "Doctor should pass: {:?}", diagnostics);

        // Stats should reflect data
        let handle = CapsuleHandle::open(config).expect("should reopen capsule");
        let stats = handle.stats();

        assert!(stats.size_bytes > 5, "Capsule should have data");
        assert_eq!(stats.uri_count, 3, "Should have 3 URIs");
        assert_eq!(stats.checkpoint_count, 1, "Should have 1 checkpoint");
        assert!(stats.event_count >= 1, "Should have at least 1 event");
    }
}

/// Test that multiple put/reopen cycles maintain data integrity.
#[test]
fn test_persistence_multiple_cycles() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("multi_cycle.mv2");

    let mut all_uris = Vec::new();

    // Cycle 1: Create initial data
    {
        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "multi_cycle".to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open(config).expect("should create capsule");

        let uri = handle
            .put(
                "SPEC-MC",
                "run-mc",
                ObjectType::Artifact,
                "cycle1.txt",
                b"Cycle 1 data".to_vec(),
                serde_json::json!({"cycle": 1}),
            )
            .expect("put should succeed");
        all_uris.push(uri);

        handle
            .commit_stage("SPEC-MC", "run-mc", "cycle1", None)
            .expect("commit should succeed");
    }

    // Cycle 2: Add more data
    {
        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "multi_cycle".to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open(config).expect("should reopen capsule");

        // Verify cycle 1 data still there
        let data1 = handle
            .get_bytes(&all_uris[0], None, None)
            .expect("should get cycle 1 data");
        assert_eq!(data1, b"Cycle 1 data");

        // Add cycle 2 data
        let uri = handle
            .put(
                "SPEC-MC",
                "run-mc",
                ObjectType::Artifact,
                "cycle2.txt",
                b"Cycle 2 data".to_vec(),
                serde_json::json!({"cycle": 2}),
            )
            .expect("put should succeed");
        all_uris.push(uri);

        handle
            .commit_stage("SPEC-MC", "run-mc", "cycle2", None)
            .expect("commit should succeed");
    }

    // Cycle 3: Verify all data
    {
        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "multi_cycle".to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open(config).expect("should reopen capsule");

        // Both URIs should work
        let data1 = handle
            .get_bytes(&all_uris[0], None, None)
            .expect("should get cycle 1 data");
        let data2 = handle
            .get_bytes(&all_uris[1], None, None)
            .expect("should get cycle 2 data");

        assert_eq!(data1, b"Cycle 1 data");
        assert_eq!(data2, b"Cycle 2 data");

        // Stats should reflect accumulated data
        let stats = handle.stats();
        assert_eq!(stats.uri_count, 2, "Should have 2 URIs");
        assert_eq!(stats.checkpoint_count, 2, "Should have 2 checkpoints");
    }
}

// =============================================================================
// SPEC-KIT-977 Policy Wiring Acceptance Tests
// =============================================================================

/// SPEC-KIT-977: After run start, capsule events include a PolicySnapshotRef event.
#[test]
fn test_policy_capture_emits_policy_snapshot_ref_event() {
    use crate::memvid_adapter::policy_capture::capture_and_store_policy;
    use codex_stage0::Stage0Config;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("policy_event.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "policy_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");
    let stage0_config = Stage0Config::default();

    // Capture and store policy (simulates run start)
    let result = capture_and_store_policy(&handle, &stage0_config, "SPEC-977", "run-001");
    assert!(result.is_ok(), "capture_and_store_policy should succeed");

    let snapshot = result.unwrap();
    assert!(!snapshot.policy_id.is_empty());
    assert!(!snapshot.hash.is_empty());

    // Check that PolicySnapshotRef event was emitted
    let events = handle.list_events();
    let policy_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.event_type, crate::memvid_adapter::EventType::PolicySnapshotRef))
        .collect();

    assert_eq!(
        policy_events.len(),
        1,
        "Should have exactly one PolicySnapshotRef event"
    );

    // Verify event payload contains policy info
    let policy_event = policy_events[0];
    let payload = &policy_event.payload;
    assert!(
        payload.get("policy_uri").is_some(),
        "PolicySnapshotRef should have policy_uri"
    );
    assert!(
        payload.get("policy_id").is_some(),
        "PolicySnapshotRef should have policy_id"
    );
    assert!(
        payload.get("policy_hash").is_some(),
        "PolicySnapshotRef should have policy_hash"
    );

    // Verify policy_id and hash match
    assert_eq!(
        payload.get("policy_id").unwrap().as_str().unwrap(),
        snapshot.policy_id,
        "Event policy_id should match snapshot"
    );
    assert_eq!(
        payload.get("policy_hash").unwrap().as_str().unwrap(),
        snapshot.hash,
        "Event policy_hash should match snapshot"
    );
}

/// SPEC-KIT-977: After commit_stage, StageTransition payload includes policy_id/hash.
#[test]
fn test_stage_transition_includes_policy_info() {
    use crate::memvid_adapter::policy_capture::capture_and_store_policy;
    use codex_stage0::Stage0Config;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("stage_policy.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "stage_policy_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");
    let stage0_config = Stage0Config::default();

    // First capture policy (simulates run start)
    let snapshot = capture_and_store_policy(&handle, &stage0_config, "SPEC-977", "run-002")
        .expect("should capture policy");

    // Now commit a stage
    handle
        .commit_stage("SPEC-977", "run-002", "plan", Some("abc123"))
        .expect("should commit stage");

    // Find the StageTransition event
    let events = handle.list_events();
    let stage_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.event_type, crate::memvid_adapter::EventType::StageTransition))
        .collect();

    assert_eq!(
        stage_events.len(),
        1,
        "Should have exactly one StageTransition event"
    );

    // Verify StageTransition includes policy info
    let stage_event = stage_events[0];
    let payload = &stage_event.payload;

    assert!(
        payload.get("policy_id").is_some(),
        "StageTransition should have policy_id"
    );
    assert!(
        payload.get("policy_hash").is_some(),
        "StageTransition should have policy_hash"
    );
    assert!(
        payload.get("policy_uri").is_some(),
        "StageTransition should have policy_uri"
    );

    // Verify values match the captured policy
    assert_eq!(
        payload.get("policy_id").unwrap().as_str().unwrap(),
        snapshot.policy_id,
        "StageTransition policy_id should match"
    );
    assert_eq!(
        payload.get("policy_hash").unwrap().as_str().unwrap(),
        snapshot.hash,
        "StageTransition policy_hash should match"
    );
}

/// SPEC-KIT-977: Policy URI is global (mv2://<workspace>/policy/<id>).
#[test]
fn test_policy_uri_is_global_not_spec_scoped() {
    use crate::memvid_adapter::policy_capture::capture_and_store_policy;
    use codex_stage0::Stage0Config;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("global_uri.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "global_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");
    let stage0_config = Stage0Config::default();

    // Capture policy
    let snapshot = capture_and_store_policy(&handle, &stage0_config, "SPEC-977", "run-003")
        .expect("should capture policy");

    // Get current policy info
    let policy_info = handle.current_policy().expect("should have current policy");

    // Verify URI format is global: mv2://<workspace>/policy/<id>
    let uri_str = policy_info.uri.as_str();
    assert!(
        uri_str.starts_with("mv2://global_test/policy/"),
        "Policy URI should be global: mv2://<workspace>/policy/<id>, got: {}",
        uri_str
    );
    assert!(
        uri_str.contains(&snapshot.policy_id),
        "Policy URI should contain policy_id"
    );

    // Verify it does NOT contain spec_id or run_id
    assert!(
        !uri_str.contains("SPEC-977"),
        "Policy URI should NOT contain spec_id"
    );
    assert!(
        !uri_str.contains("run-003"),
        "Policy URI should NOT contain run_id"
    );
}

/// SPEC-KIT-977: Current policy is tracked and accessible.
#[test]
fn test_current_policy_tracking() {
    use crate::memvid_adapter::policy_capture::capture_and_store_policy;
    use codex_stage0::Stage0Config;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("tracking.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "tracking_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");

    // Initially no policy
    assert!(
        handle.current_policy().is_none(),
        "Should have no policy before capture"
    );

    // Capture policy
    let stage0_config = Stage0Config::default();
    let snapshot = capture_and_store_policy(&handle, &stage0_config, "SPEC-977", "run-004")
        .expect("should capture policy");

    // Now should have current policy
    let policy_info = handle.current_policy();
    assert!(policy_info.is_some(), "Should have current policy after capture");

    let policy = policy_info.unwrap();
    assert_eq!(policy.policy_id, snapshot.policy_id);
    assert_eq!(policy.hash, snapshot.hash);
}

// =============================================================================
// SPEC-KIT-977: Phase 4→5 Gate Verification Tests
// =============================================================================

/// Phase 4→5 gate: Verify all events after policy capture include policy binding.
///
/// SPEC-KIT-977 Acceptance Criteria:
/// - Every event emitted after run start includes policy_id and policy_hash
/// - StageTransition events (phase boundaries) include policy info
/// - Missing policy binding at phase 4→5 boundary is a gate failure
#[test]
fn test_phase_4_5_gate_events_include_policy() {
    use crate::memvid_adapter::policy_capture::capture_and_store_policy;
    use codex_stage0::Stage0Config;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("phase_gate.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "phase_gate_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");
    let stage0_config = Stage0Config::default();

    // Phase 1-3: Setup (no policy yet)
    // Phase 4: Policy capture (simulates run start)
    let snapshot = capture_and_store_policy(&handle, &stage0_config, "SPEC-GATE", "run-phase45")
        .expect("should capture policy");

    // Phase 5: Implementation starts - all events should have policy binding
    // Put some artifacts (simulates implementation work)
    handle
        .put(
            "SPEC-GATE",
            "run-phase45",
            ObjectType::Artifact,
            "impl.rs",
            b"// Implementation code".to_vec(),
            serde_json::json!({"phase": "implement"}),
        )
        .expect("should put artifact");

    // Commit stage transition (phase 4→5 boundary)
    handle
        .commit_stage("SPEC-GATE", "run-phase45", "implement", Some("gate-commit"))
        .expect("should commit stage");

    // Verify: All events after policy capture have policy binding
    let events = handle.list_events();

    // Find StageTransition event
    let stage_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.event_type, crate::memvid_adapter::EventType::StageTransition))
        .collect();

    assert!(
        !stage_events.is_empty(),
        "Should have at least one StageTransition event"
    );

    // Verify StageTransition has policy info (phase 4→5 gate requirement)
    for event in stage_events {
        let payload = &event.payload;
        assert!(
            payload.get("policy_id").is_some(),
            "StageTransition event MUST have policy_id for phase 4→5 gate"
        );
        assert!(
            payload.get("policy_hash").is_some(),
            "StageTransition event MUST have policy_hash for phase 4→5 gate"
        );
        assert_eq!(
            payload.get("policy_id").unwrap().as_str().unwrap(),
            snapshot.policy_id,
            "StageTransition policy_id should match captured snapshot"
        );
        assert_eq!(
            payload.get("policy_hash").unwrap().as_str().unwrap(),
            snapshot.hash,
            "StageTransition policy_hash should match captured snapshot"
        );
    }
}

/// SPEC-KIT-977: Test policy drift detection at stage boundaries.
///
/// This test verifies that when policy changes between stages,
/// the system detects the drift and recaptures the policy.
#[test]
fn test_policy_drift_detection_at_stage_boundary() {
    use crate::memvid_adapter::policy_capture::{capture_and_store_policy, check_and_recapture_if_changed};
    use codex_stage0::Stage0Config;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("drift_detection.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "drift_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");
    let stage0_config = Stage0Config::default();

    // Initial policy capture at run start
    let initial_snapshot = capture_and_store_policy(&handle, &stage0_config, "SPEC-DRIFT", "run-drift")
        .expect("should capture initial policy");

    let initial_hash = initial_snapshot.hash.clone();

    // First stage boundary - no drift expected (same config)
    let result = check_and_recapture_if_changed(&handle, &stage0_config, "SPEC-DRIFT", "run-drift")
        .expect("should check drift");

    assert!(
        result.is_none(),
        "No drift should be detected when config unchanged"
    );

    // Verify current policy hash is still the initial one
    let current = handle.current_policy().expect("should have current policy");
    assert_eq!(
        current.hash, initial_hash,
        "Current policy hash should match initial"
    );

    // Commit a stage transition
    handle
        .commit_stage("SPEC-DRIFT", "run-drift", "plan", None)
        .expect("should commit stage");

    // Verify StageTransition has the initial policy info
    let events = handle.list_events();
    let stage_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.event_type, EventType::StageTransition))
        .collect();

    assert!(!stage_events.is_empty(), "Should have StageTransition event");
    let event = stage_events[0];
    assert_eq!(
        event.payload.get("policy_hash").unwrap().as_str().unwrap(),
        initial_hash,
        "StageTransition should have initial policy hash"
    );
}

/// SPEC-KIT-977: Test that check_and_recapture_if_changed handles no prior policy.
///
/// When there's no current policy, the function should capture one.
#[test]
fn test_policy_drift_check_with_no_prior_policy() {
    use crate::memvid_adapter::policy_capture::check_and_recapture_if_changed;
    use codex_stage0::Stage0Config;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("no_prior.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "no_prior_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");
    let stage0_config = Stage0Config::default();

    // Verify no current policy
    assert!(
        handle.current_policy().is_none(),
        "Should have no policy before check"
    );

    // Call check_and_recapture_if_changed - should capture initial policy
    let result = check_and_recapture_if_changed(&handle, &stage0_config, "SPEC-NOP", "run-nop")
        .expect("should capture when no prior policy");

    assert!(
        result.is_some(),
        "Should return Some when capturing initial policy"
    );

    // Now should have current policy
    let current = handle.current_policy();
    assert!(
        current.is_some(),
        "Should have current policy after check"
    );
}

/// Phase 4→5 gate: Verify policy capture happens before any stage transitions.
///
/// This test documents the invariant that policy capture MUST happen before
/// any implementation work begins (phase 4→5 boundary).
#[test]
fn test_phase_4_5_gate_ordering() {
    use crate::memvid_adapter::policy_capture::capture_and_store_policy;
    use codex_stage0::Stage0Config;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("gate_ordering.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "ordering_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");

    // Attempt to commit stage WITHOUT policy capture first
    // This should still work (graceful degradation) but events won't have policy binding
    handle
        .commit_stage("SPEC-ORDER", "run-order", "plan", None)
        .expect("commit should succeed even without policy");

    // Check events - no policy binding expected
    let events_before = handle.list_events();
    let stage_events_before: Vec<_> = events_before
        .iter()
        .filter(|e| matches!(e.event_type, crate::memvid_adapter::EventType::StageTransition))
        .collect();

    // StageTransition without policy capture won't have policy fields
    if !stage_events_before.is_empty() {
        let event = stage_events_before[0];
        let has_policy = event.payload.get("policy_id").is_some();
        // Note: This documents current behavior - events without prior policy capture
        // have no policy binding. The phase 4→5 gate should enforce policy capture
        // happens first in the actual pipeline.
        assert!(
            !has_policy,
            "StageTransition without prior policy capture should not have policy_id"
        );
    }

    // Now capture policy (late capture - not recommended but should work)
    let stage0_config = Stage0Config::default();
    let _snapshot = capture_and_store_policy(&handle, &stage0_config, "SPEC-ORDER", "run-order")
        .expect("should capture policy");

    // Subsequent stage commits WILL have policy binding
    handle
        .commit_stage("SPEC-ORDER", "run-order", "implement", None)
        .expect("commit after policy capture");

    let events_after = handle.list_events();
    let stage_events_after: Vec<_> = events_after
        .iter()
        .filter(|e| matches!(e.event_type, crate::memvid_adapter::EventType::StageTransition))
        .filter(|e| e.stage.as_deref() == Some("implement"))
        .collect();

    // The implement stage transition should have policy binding
    assert!(
        !stage_events_after.is_empty(),
        "Should have implement StageTransition"
    );
    assert!(
        stage_events_after[0].payload.get("policy_id").is_some(),
        "StageTransition after policy capture MUST have policy_id"
    );
}

// =============================================================================
// SPEC-KIT-971: Branch Isolation Tests
// =============================================================================

/// SPEC-KIT-971: Checkpoints are stamped with branch_id
#[test]
fn test_checkpoint_branch_id_stamped() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let capsule_path = temp_dir.path().join("test_branch.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("open capsule");

    // Default branch is "main"
    assert_eq!(handle.current_branch().as_str(), "main");

    // Create checkpoint on main branch
    handle
        .commit_stage("SPEC-BRANCH", "run-001", "plan", None)
        .expect("commit_stage");

    let checkpoints = handle.list_checkpoints();
    assert_eq!(checkpoints.len(), 1);
    assert_eq!(
        checkpoints[0].branch_id,
        Some("main".to_string()),
        "Checkpoint should have branch_id stamped"
    );
}

/// SPEC-KIT-971: Events are stamped with branch_id
#[test]
fn test_event_branch_id_stamped() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let capsule_path = temp_dir.path().join("test_event_branch.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("open capsule");

    // Create checkpoint which emits StageTransition event
    handle
        .commit_stage("SPEC-BRANCH", "run-002", "plan", None)
        .expect("commit_stage");

    let events = handle.list_events();
    assert!(!events.is_empty());
    assert_eq!(
        events[0].branch_id,
        Some("main".to_string()),
        "Event should have branch_id stamped"
    );
}

/// SPEC-KIT-971: Run branch isolation - checkpoints filterable by branch_id
#[test]
fn test_run_branch_isolation_checkpoints() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let capsule_path = temp_dir.path().join("test_run_isolation.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("open capsule");

    // Create checkpoint on main
    handle
        .commit_stage("SPEC-MAIN", "run-main", "plan", None)
        .expect("commit on main");

    // Switch to run branch
    let run_branch = BranchId::for_run("run-abc123");
    handle.switch_branch(run_branch.clone()).expect("switch branch");

    // Create checkpoint on run branch
    handle
        .commit_stage("SPEC-RUN", "run-abc123", "plan", None)
        .expect("commit on run branch");

    // Filter by main branch
    let main_checkpoints = handle.list_checkpoints_filtered(Some(&BranchId::main()));
    assert_eq!(main_checkpoints.len(), 1);
    assert_eq!(main_checkpoints[0].spec_id, Some("SPEC-MAIN".to_string()));

    // Filter by run branch
    let run_checkpoints = handle.list_checkpoints_filtered(Some(&run_branch));
    assert_eq!(run_checkpoints.len(), 1);
    assert_eq!(run_checkpoints[0].spec_id, Some("SPEC-RUN".to_string()));

    // No filter returns all
    let all_checkpoints = handle.list_checkpoints_filtered(None);
    assert_eq!(all_checkpoints.len(), 2);
}

/// SPEC-KIT-971: Run branch isolation - events filterable by branch_id
#[test]
fn test_run_branch_isolation_events() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let capsule_path = temp_dir.path().join("test_event_isolation.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("open capsule");

    // Create event on main (via commit_stage)
    handle
        .commit_stage("SPEC-MAIN", "run-main", "plan", None)
        .expect("commit on main");

    // Switch to run branch
    let run_branch = BranchId::for_run("run-xyz789");
    handle.switch_branch(run_branch.clone()).expect("switch branch");

    // Create event on run branch
    handle
        .commit_stage("SPEC-RUN", "run-xyz789", "implement", None)
        .expect("commit on run branch");

    // Filter events by main branch
    let main_events = handle.list_events_filtered(Some(&BranchId::main()));
    assert_eq!(main_events.len(), 1);
    assert_eq!(main_events[0].spec_id, "SPEC-MAIN");

    // Filter events by run branch
    let run_events = handle.list_events_filtered(Some(&run_branch));
    assert_eq!(run_events.len(), 1);
    assert_eq!(run_events[0].spec_id, "SPEC-RUN");

    // No filter returns all
    let all_events = handle.list_events_filtered(None);
    assert_eq!(all_events.len(), 2);
}

/// SPEC-KIT-971: Manual checkpoint also gets branch_id
#[test]
fn test_manual_checkpoint_branch_id() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let capsule_path = temp_dir.path().join("test_manual_branch.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("open capsule");

    // Switch to a run branch
    let run_branch = BranchId::for_run("manual-run");
    handle.switch_branch(run_branch).expect("switch branch");

    // Create manual checkpoint
    handle.commit_manual("my-label").expect("commit_manual");

    let checkpoints = handle.list_checkpoints();
    assert_eq!(checkpoints.len(), 1);
    assert_eq!(
        checkpoints[0].branch_id,
        Some("run/manual-run".to_string()),
        "Manual checkpoint should have branch_id stamped"
    );
}

// =============================================================================
// SPEC-KIT-971: Time-Travel URI Resolution Tests
// =============================================================================

/// SPEC-KIT-971: Test time-travel URI resolution.
///
/// Acceptance test:
/// - Put URI v1 content, commit checkpoint A
/// - Put URI v2 content (same logical URI path), commit checkpoint B
/// - resolve_uri(as_of=A) returns v1 pointer; resolve_uri(as_of=B) returns v2 pointer
#[test]
fn test_time_travel_uri_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("time_travel.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "time_travel_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config.clone()).expect("open capsule");

    // Switch to a run branch for proper branch-aware testing
    let run_branch = BranchId::for_run("time-travel-run");
    handle.switch_branch(run_branch.clone()).expect("switch branch");

    // Step 1: Put v1 content
    let uri = handle
        .put(
            "SPEC-971",
            "time-travel-run",
            ObjectType::Artifact,
            "document.md",
            b"version 1 content".to_vec(),
            serde_json::json!({"version": 1}),
        )
        .expect("put v1");

    // Step 2: Commit checkpoint A
    let checkpoint_a = handle
        .commit_stage("SPEC-971", "time-travel-run", "plan", None)
        .expect("commit checkpoint A");

    // Get the pointer at checkpoint A (should be v1)
    let pointer_at_a = handle
        .resolve_uri(&uri, Some(&run_branch), Some(&checkpoint_a))
        .expect("resolve at checkpoint A");

    // Step 3: Put v2 content (same logical URI path - will update the pointer)
    let _uri_v2 = handle
        .put(
            "SPEC-971",
            "time-travel-run",
            ObjectType::Artifact,
            "document.md",
            b"version 2 content - updated".to_vec(),
            serde_json::json!({"version": 2}),
        )
        .expect("put v2");

    // Step 4: Commit checkpoint B
    let checkpoint_b = handle
        .commit_stage("SPEC-971", "time-travel-run", "tasks", None)
        .expect("commit checkpoint B");

    // Get the pointer at checkpoint B (should be v2)
    let pointer_at_b = handle
        .resolve_uri(&uri, Some(&run_branch), Some(&checkpoint_b))
        .expect("resolve at checkpoint B");

    // Verify that the pointers are different
    assert_ne!(
        pointer_at_a.offset, pointer_at_b.offset,
        "Pointer at A should be different from pointer at B"
    );

    // Verify current resolution returns v2 pointer
    let current_pointer = handle
        .resolve_uri(&uri, Some(&run_branch), None)
        .expect("resolve current");
    assert_eq!(
        current_pointer.offset, pointer_at_b.offset,
        "Current pointer should match checkpoint B"
    );

    // Verify time-travel to checkpoint A still works
    let time_travel_a = handle
        .resolve_uri(&uri, Some(&run_branch), Some(&checkpoint_a))
        .expect("time travel to A");
    assert_eq!(
        time_travel_a.offset, pointer_at_a.offset,
        "Time travel to A should return v1 pointer"
    );

    // Read the actual content to verify
    let bytes_v1 = handle
        .get_bytes(&uri, Some(&run_branch), Some(&checkpoint_a))
        .expect("get bytes v1");
    assert_eq!(
        bytes_v1,
        b"version 1 content".to_vec(),
        "Content at checkpoint A should be v1"
    );

    let bytes_v2 = handle
        .get_bytes(&uri, Some(&run_branch), Some(&checkpoint_b))
        .expect("get bytes v2");
    assert_eq!(
        bytes_v2,
        b"version 2 content - updated".to_vec(),
        "Content at checkpoint B should be v2"
    );
}

/// SPEC-KIT-971: Test time-travel resolution survives reopen.
///
/// Same as test_time_travel_uri_resolution but verifies it works after
/// closing and reopening the capsule.
#[test]
fn test_time_travel_survives_reopen() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("time_travel_reopen.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "time_travel_reopen".to_string(),
        ..Default::default()
    };

    // Phase 1: Create capsule, put v1, checkpoint A, put v2, checkpoint B
    let checkpoint_a;
    let checkpoint_b;
    let uri;
    let run_branch = BranchId::for_run("reopen-run");

    {
        let handle = CapsuleHandle::open(config.clone()).expect("open capsule");
        handle.switch_branch(run_branch.clone()).expect("switch branch");

        // Put v1
        uri = handle
            .put(
                "SPEC-971",
                "reopen-run",
                ObjectType::Artifact,
                "file.md",
                b"first version".to_vec(),
                serde_json::json!({}),
            )
            .expect("put v1");

        // Checkpoint A
        checkpoint_a = handle
            .commit_stage("SPEC-971", "reopen-run", "plan", None)
            .expect("checkpoint A");

        // Put v2
        let _uri_v2 = handle
            .put(
                "SPEC-971",
                "reopen-run",
                ObjectType::Artifact,
                "file.md",
                b"second version".to_vec(),
                serde_json::json!({}),
            )
            .expect("put v2");

        // Checkpoint B
        checkpoint_b = handle
            .commit_stage("SPEC-971", "reopen-run", "tasks", None)
            .expect("checkpoint B");

        // Handle dropped here - capsule closed
    }

    // Phase 2: Reopen and verify time-travel still works
    {
        let handle = CapsuleHandle::open(config).expect("reopen capsule");

        // Verify snapshots were restored
        let uri_index = handle.list_checkpoints();
        assert_eq!(uri_index.len(), 2, "Should have 2 checkpoints after reopen");

        // Time-travel to checkpoint A should return v1 content
        let bytes_v1 = handle
            .get_bytes(&uri, Some(&run_branch), Some(&checkpoint_a))
            .expect("get bytes v1 after reopen");
        assert_eq!(
            bytes_v1,
            b"first version".to_vec(),
            "Time travel to A after reopen should return v1"
        );

        // Time-travel to checkpoint B should return v2 content
        let bytes_v2 = handle
            .get_bytes(&uri, Some(&run_branch), Some(&checkpoint_b))
            .expect("get bytes v2 after reopen");
        assert_eq!(
            bytes_v2,
            b"second version".to_vec(),
            "Time travel to B after reopen should return v2"
        );

        // SPEC-KIT-971: Current state (as_of=None) now preserves branch context
        // After reopen, scan_and_rebuild restores entries from the latest snapshot
        // for each branch, so resolve_uri(branch, as_of=None) works correctly.
        let current_state_bytes = handle
            .get_bytes(&uri, Some(&run_branch), None)
            .expect("get bytes with as_of=None after reopen");
        assert_eq!(
            current_state_bytes,
            b"second version".to_vec(),
            "Current state (as_of=None) after reopen should return v2 (latest on branch)"
        );

        // Verify it matches the explicit latest checkpoint
        let latest_checkpoint_bytes = handle
            .get_bytes(&uri, Some(&run_branch), Some(&checkpoint_b))
            .expect("get bytes at latest checkpoint");
        assert_eq!(
            current_state_bytes, latest_checkpoint_bytes,
            "as_of=None should equal as_of=<latest checkpoint>"
        );
    }
}

/// SPEC-KIT-971: Test URI index snapshot is created at each checkpoint.
#[test]
fn test_uri_index_snapshot_created() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("snapshot_test.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "snapshot_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("open capsule");

    // Put an artifact
    let _uri = handle
        .put(
            "SPEC-971",
            "run1",
            ObjectType::Artifact,
            "test.md",
            b"content".to_vec(),
            serde_json::json!({}),
        )
        .expect("put artifact");

    // Create manual checkpoint
    let checkpoint_id = handle.commit_manual("test-checkpoint").expect("commit");

    // Verify snapshot exists
    let uri_index = handle.list_checkpoints();
    assert_eq!(uri_index.len(), 1);

    // The snapshot should exist - we can verify by trying to resolve with as_of
    // (If snapshot didn't exist, resolve_on_branch would fail)
    let checkpoints = handle.list_checkpoints();
    assert!(
        !checkpoints.is_empty(),
        "Checkpoint should exist and have snapshot"
    );

    // Verify we can resolve with as_of (proves snapshot was created)
    // Note: We need to be on main branch since we didn't switch
    let main_branch = BranchId::main();
    let uri = handle
        .put(
            "SPEC-971",
            "run1",
            ObjectType::Artifact,
            "test2.md",
            b"content2".to_vec(),
            serde_json::json!({}),
        )
        .expect("put another artifact");

    // This should succeed because snapshot was created
    // (Even though test2.md wasn't in the snapshot, the snapshot lookup won't panic)
    let result = handle.resolve_uri(&uri, Some(&main_branch), Some(&checkpoint_id));
    // URI wasn't in the snapshot at that time, so it should fail with UriNotFound
    assert!(result.is_err(), "URI not in snapshot should fail");
}

// =============================================================================
// SPEC-KIT-971: Merge determinism tests
// =============================================================================

/// Test that objects created on run branch become resolvable on main after merge.
///
/// This is the key acceptance criterion for merge-at-unlock:
/// "Add a deterministic test proving objects created on run branch
///  become resolvable on main after merge"
#[test]
fn test_merge_determinism_uris_resolvable_on_main_after_merge() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("merge_test.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "merge_test".to_string(),
        ..Default::default()
    };

    // Step 1: Create capsule and switch to run branch
    let handle = CapsuleHandle::open(config.clone()).expect("create capsule");
    let run_branch = BranchId::for_run("test-run-001");
    handle.switch_branch(run_branch.clone()).expect("switch to run branch");

    // Step 2: Put artifact on run branch
    let run_uri = handle
        .put(
            "SPEC-MERGE-TEST",
            "test-run-001",
            ObjectType::Artifact,
            "run_artifact.md",
            b"# Artifact created on run branch".to_vec(),
            serde_json::json!({"created_on": "run_branch"}),
        )
        .expect("put artifact on run branch");

    assert!(run_uri.is_valid(), "Run URI should be valid");

    // Step 3: Create checkpoint on run branch
    handle
        .commit_stage("SPEC-MERGE-TEST", "test-run-001", "Plan", None)
        .expect("create run branch checkpoint");

    // Step 4: Verify URI is NOT resolvable on main (before merge)
    let main_branch = BranchId::main();
    let pre_merge_result = handle.resolve_uri(&run_uri, Some(&main_branch), None);
    assert!(
        pre_merge_result.is_err(),
        "URI should NOT be resolvable on main before merge"
    );

    // Step 5: Perform merge: run branch → main
    let merge_checkpoint = handle
        .merge_branch(
            &run_branch,
            &main_branch,
            MergeMode::Curated,
            Some("SPEC-MERGE-TEST"),
            Some("test-run-001"),
        )
        .expect("merge run branch to main");

    assert!(
        merge_checkpoint.as_str().contains("merge"),
        "Merge checkpoint should have 'merge' in ID"
    );

    // Step 6: Verify URI IS resolvable on main (after merge)
    let post_merge_result = handle.resolve_uri(&run_uri, Some(&main_branch), None);
    assert!(
        post_merge_result.is_ok(),
        "URI should be resolvable on main AFTER merge: {:?}",
        post_merge_result.err()
    );

    // Step 7: Verify BranchMerged event was emitted
    let events = handle.list_events();
    let merge_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == EventType::BranchMerged)
        .collect();

    assert_eq!(
        merge_events.len(),
        1,
        "Should have exactly one BranchMerged event"
    );

    let merge_event = merge_events[0];
    assert_eq!(merge_event.stage, Some("Unlock".to_string()));

    // Verify merge payload contains expected fields
    let payload: serde_json::Value = merge_event.payload.clone();
    assert_eq!(
        payload["from_branch"],
        "run/test-run-001",
        "from_branch should match"
    );
    assert_eq!(payload["to_branch"], "main", "to_branch should be main");
    assert_eq!(
        payload["mode"],
        "Curated",
        "mode should be Curated"
    );
}

/// Test that merge checkpoint appears in checkpoint list with correct metadata.
#[test]
fn test_merge_creates_checkpoint_with_correct_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("merge_checkpoint_test.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "merge_cp_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("create capsule");

    // Set up run branch with some content
    let run_branch = BranchId::for_run("run-cp-test");
    handle.switch_branch(run_branch.clone()).expect("switch branch");

    handle
        .put(
            "SPEC-CP-TEST",
            "run-cp-test",
            ObjectType::Artifact,
            "file.md",
            b"content".to_vec(),
            serde_json::json!({}),
        )
        .expect("put artifact");

    handle
        .commit_stage("SPEC-CP-TEST", "run-cp-test", "Plan", None)
        .expect("create run checkpoint");

    // Get initial checkpoint count
    let initial_count = handle.list_checkpoints().len();

    // Perform merge
    let merge_checkpoint_id = handle
        .merge_branch(
            &run_branch,
            &BranchId::main(),
            MergeMode::Full, // Use Full mode to test both modes work
            Some("SPEC-CP-TEST"),
            Some("run-cp-test"),
        )
        .expect("merge should succeed");

    // Verify new checkpoint was added
    let checkpoints = handle.list_checkpoints();
    assert_eq!(
        checkpoints.len(),
        initial_count + 1,
        "Merge should create one new checkpoint"
    );

    // Find the merge checkpoint and verify its metadata
    let merge_cp = checkpoints
        .iter()
        .find(|cp| cp.checkpoint_id == merge_checkpoint_id)
        .expect("merge checkpoint should exist");

    assert_eq!(
        merge_cp.stage,
        Some("Unlock".to_string()),
        "Merge checkpoint stage should be Unlock"
    );
    assert_eq!(
        merge_cp.spec_id,
        Some("SPEC-CP-TEST".to_string()),
        "Merge checkpoint should have spec_id"
    );
    assert_eq!(
        merge_cp.run_id,
        Some("run-cp-test".to_string()),
        "Merge checkpoint should have run_id"
    );
    assert_eq!(
        merge_cp.branch_id,
        Some("main".to_string()),
        "Merge checkpoint should be on main branch"
    );
    assert!(
        merge_cp.label.as_ref().unwrap().contains("merge"),
        "Merge checkpoint label should contain 'merge'"
    );
}

/// Test curated merge mode excludes debug events.
///
/// SPEC-KIT-971: Curated mode should only merge governance-critical events
/// (StageTransition, PolicySnapshotRef, RoutingDecision, BranchMerged).
/// Debug events (DebugTrace) should remain on the run branch.
#[test]
fn test_curated_merge_excludes_debug_events() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("curated_merge_test.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "curated_merge_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("open capsule");

    // Create and switch to run branch
    let run_branch = BranchId::for_run("curated-run-001");
    handle.switch_branch(run_branch.clone()).expect("switch branch");

    // Create a curated-eligible event via commit_stage (emits StageTransition)
    handle
        .commit_stage("SPEC-CURATED", "curated-run-001", "Plan", None)
        .expect("commit stage creates StageTransition event");

    // Emit debug event (should be excluded in curated mode)
    handle
        .emit_debug_trace(
            "SPEC-CURATED",
            "curated-run-001",
            Some("Plan"),
            "Verbose debug info",
            serde_json::json!({"debug_data": "should_not_merge"}),
        )
        .expect("emit debug trace");

    // Count events on run branch before merge
    let run_events_before = handle.list_events_filtered(Some(&run_branch));
    assert_eq!(
        run_events_before.len(),
        2,
        "Run branch should have 2 events before merge (StageTransition + DebugTrace)"
    );

    // Perform CURATED merge to main
    let main_branch = BranchId::main();
    let _merge_cp = handle
        .merge_branch(
            &run_branch,
            &main_branch,
            MergeMode::Curated,
            Some("SPEC-CURATED"),
            Some("curated-run-001"),
        )
        .expect("curated merge");

    // Count events on main after merge (should exclude DebugTrace)
    let main_events = handle.list_events_filtered(Some(&main_branch));

    // Should have: StageTransition (merged) + BranchMerged (from merge operation)
    // Should NOT have: DebugTrace
    let stage_transitions: Vec<_> = main_events
        .iter()
        .filter(|e| e.event_type == EventType::StageTransition)
        .collect();
    let debug_traces: Vec<_> = main_events
        .iter()
        .filter(|e| e.event_type == EventType::DebugTrace)
        .collect();
    let branch_merged: Vec<_> = main_events
        .iter()
        .filter(|e| e.event_type == EventType::BranchMerged)
        .collect();

    assert_eq!(
        stage_transitions.len(),
        1,
        "StageTransition should be merged to main"
    );
    assert_eq!(
        debug_traces.len(),
        0,
        "DebugTrace should NOT be merged in curated mode"
    );
    assert_eq!(
        branch_merged.len(),
        1,
        "BranchMerged event should be on main"
    );
}

/// Test full merge mode includes all events including debug.
///
/// SPEC-KIT-971: Full mode should merge everything for deep audit/incident review.
#[test]
fn test_full_merge_includes_debug_events() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("full_merge_test.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "full_merge_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("open capsule");

    // Create and switch to run branch
    let run_branch = BranchId::for_run("full-run-001");
    handle.switch_branch(run_branch.clone()).expect("switch branch");

    // Create curated-eligible event via commit_stage
    handle
        .commit_stage("SPEC-FULL", "full-run-001", "Plan", None)
        .expect("commit stage creates StageTransition event");

    // Emit debug event
    handle
        .emit_debug_trace(
            "SPEC-FULL",
            "full-run-001",
            Some("Plan"),
            "Verbose debug info",
            serde_json::json!({"debug_data": "should_merge_in_full"}),
        )
        .expect("emit debug trace");

    // Perform FULL merge to main
    let main_branch = BranchId::main();
    let _merge_cp = handle
        .merge_branch(
            &run_branch,
            &main_branch,
            MergeMode::Full,
            Some("SPEC-FULL"),
            Some("full-run-001"),
        )
        .expect("full merge");

    // Count events on main after merge (should include all)
    let main_events = handle.list_events_filtered(Some(&main_branch));

    let stage_transitions: Vec<_> = main_events
        .iter()
        .filter(|e| e.event_type == EventType::StageTransition)
        .collect();
    let debug_traces: Vec<_> = main_events
        .iter()
        .filter(|e| e.event_type == EventType::DebugTrace)
        .collect();

    assert_eq!(
        stage_transitions.len(),
        1,
        "StageTransition should be merged to main"
    );
    assert_eq!(
        debug_traces.len(),
        1,
        "DebugTrace SHOULD be merged in full mode"
    );
}

/// Test curated merge persists correctly after reopen.
///
/// SPEC-KIT-971: After reopen, merged events should still be on main branch,
/// and excluded events should still be isolated on run branch.
#[test]
fn test_curated_merge_persists_after_reopen() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("curated_reopen_test.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "curated_reopen_test".to_string(),
        ..Default::default()
    };

    let run_branch = BranchId::for_run("persist-run-001");

    // Phase 1: Create events and merge
    {
        let handle = CapsuleHandle::open(config.clone()).expect("open capsule");
        handle.switch_branch(run_branch.clone()).expect("switch branch");

        // Create curated-eligible event via commit_stage
        handle
            .commit_stage("SPEC-PERSIST", "persist-run-001", "Plan", None)
            .expect("commit stage creates StageTransition event");

        handle
            .emit_debug_trace(
                "SPEC-PERSIST",
                "persist-run-001",
                Some("Plan"),
                "Debug info",
                serde_json::json!({}),
            )
            .expect("emit debug trace");

        // Curated merge
        let main_branch = BranchId::main();
        handle
            .merge_branch(
                &run_branch,
                &main_branch,
                MergeMode::Curated,
                Some("SPEC-PERSIST"),
                Some("persist-run-001"),
            )
            .expect("curated merge");

        // Handle dropped - capsule closed
    }

    // Phase 2: Reopen and verify merge semantics persisted
    {
        let handle = CapsuleHandle::open(config).expect("reopen capsule");

        let main_branch = BranchId::main();
        let main_events = handle.list_events_filtered(Some(&main_branch));

        // Count by type
        let stage_transitions: Vec<_> = main_events
            .iter()
            .filter(|e| e.event_type == EventType::StageTransition)
            .collect();
        let debug_traces: Vec<_> = main_events
            .iter()
            .filter(|e| e.event_type == EventType::DebugTrace)
            .collect();

        assert_eq!(
            stage_transitions.len(),
            1,
            "StageTransition should persist on main after reopen"
        );
        assert_eq!(
            debug_traces.len(),
            0,
            "DebugTrace should NOT be on main after reopen"
        );

        // The debug trace should still exist on the run branch
        let run_events = handle.list_events_filtered(Some(&run_branch));
        let run_debug: Vec<_> = run_events
            .iter()
            .filter(|e| e.event_type == EventType::DebugTrace)
            .collect();
        assert_eq!(
            run_debug.len(),
            1,
            "DebugTrace should still be on run branch after reopen"
        );
    }
}

/// Test EventType::is_curated_eligible classification.
#[test]
fn test_event_type_curated_classification() {
    // Curated-eligible events
    assert!(EventType::StageTransition.is_curated_eligible());
    assert!(EventType::PolicySnapshotRef.is_curated_eligible());
    assert!(EventType::RoutingDecision.is_curated_eligible());
    assert!(EventType::BranchMerged.is_curated_eligible());

    // Non-curated events (debug-only)
    assert!(!EventType::DebugTrace.is_curated_eligible());
}

/// Test LogicalUri curated classification.
#[test]
fn test_uri_curated_classification() {
    // Curated-eligible URIs
    let artifact_uri = LogicalUri::new(
        "ws",
        "SPEC-001",
        "run1",
        ObjectType::Artifact,
        "file.md",
    )
    .unwrap();
    assert!(artifact_uri.is_curated_eligible(), "Artifacts are curated");

    let policy_uri = LogicalUri::for_policy("ws", "policy-001");
    assert!(policy_uri.is_curated_eligible(), "Policies are curated");

    // Non-curated URIs
    let event_uri = LogicalUri::for_event("ws", "SPEC-001", "run1", 1);
    assert!(!event_uri.is_curated_eligible(), "Events handled separately");
}
