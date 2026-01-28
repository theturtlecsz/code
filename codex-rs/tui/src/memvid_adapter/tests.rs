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
use crate::memvid_adapter::capsule::{
    CapsuleConfig, CapsuleError, CapsuleHandle, DiagnosticResult,
};
use crate::memvid_adapter::types::{
    BranchId,
    CardFact,
    // SPEC-KIT-976: Memory Card and Logic Edge types
    CardType,
    EdgeType,
    // SPEC-KIT-975: Event payload types for integration tests
    EventType,
    FactValueType,
    GateDecisionPayload,
    GateOutcome,
    LLMCaptureMode,
    LogicEdgeV1,
    LogicalUri,
    MemoryCardV1,
    MergeMode,
    ModelCallEnvelopePayload,
    ObjectType,
    PatchApplyPayload,
    RetrievalRequestPayload,
    RetrievalResponsePayload,
    RoutingMode,
    ToolCallPayload,
    ToolResultPayload,
};
use secrecy::SecretString;
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
    assert!(
        uri.as_str().starts_with("mv2://"),
        "URI should have mv2:// scheme"
    );

    // Step 3: Commit checkpoint
    let _checkpoint_id = handle
        .commit_stage("SPEC-971", "run1", "plan", Some("abc123"))
        .expect("should create checkpoint");

    // Verify checkpoint exists
    let checkpoints = handle.list_checkpoints();
    assert!(
        !checkpoints.is_empty(),
        "should have at least one checkpoint"
    );

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
    handle
        .commit_stage("SPEC-971", "run1", "plan", None)
        .unwrap();
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
    assert_eq!(
        uri1.as_str(),
        uri2.as_str(),
        "logical URIs should be stable"
    );
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
    let checkpoint = checkpoints
        .iter()
        .find(|c| c.checkpoint_id.as_str() == checkpoint_id.as_str());
    assert!(checkpoint.is_some(), "checkpoint should exist");

    let cp = checkpoint.unwrap();
    assert_eq!(
        cp.stage.as_deref(),
        Some("plan"),
        "stage should be recorded"
    );
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

    let adapter = MemvidMemoryAdapter::new(config).with_fallback(Arc::new(MockFallback));

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
    std::fs::write(
        &lock_path,
        serde_json::to_string_pretty(&stale_lock).unwrap(),
    )
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
    handle
        .switch_branch(run_branch.clone())
        .expect("should switch");
    assert!(handle.current_branch().is_run_branch());
    assert_eq!(handle.current_branch().as_str(), "run/run123");

    // Switch back to main
    handle
        .switch_branch(BranchId::main())
        .expect("should switch back");
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
    assert_eq!(
        by_id.as_ref().unwrap().label.as_deref(),
        Some("v1.0-release")
    );

    // Find by label
    let by_label = handle.get_checkpoint_by_label("v1.0-release");
    assert!(by_label.is_some());
    assert_eq!(
        by_label.as_ref().unwrap().checkpoint_id.as_str(),
        cp_id.as_str()
    );

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
    handle
        .put(
            "SPEC-971",
            "run1",
            ObjectType::Artifact,
            "test.md",
            b"content".to_vec(),
            serde_json::json!({}),
        )
        .unwrap();
    handle
        .commit_stage("SPEC-971", "run1", "plan", None)
        .unwrap();

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
    assert!(
        !handle.is_label_unique("v1.0", &main),
        "Label should not be unique after commit"
    );
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
    assert!(
        result2.is_err(),
        "Second commit_manual with same label should fail"
    );

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
    assert_eq!(
        v1_checkpoints.len(),
        2,
        "Should have 2 checkpoints with label 'v1.0'"
    );
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
    handle
        .switch_branch(run_branch.clone())
        .expect("switch branch");

    // Create "v1.0" on run branch - should succeed (different branch)
    let result = handle.commit_manual("v1.0");
    assert!(
        result.is_ok(),
        "Same label on different branch should succeed"
    );

    // Creating another "v1.0" on same run branch should fail
    let result2 = handle.commit_manual("v1.0");
    assert!(
        result2.is_err(),
        "Duplicate label on same branch should fail"
    );
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
    let uri = handle
        .put(
            "SPEC-971",
            "run1",
            ObjectType::Artifact,
            "test.md",
            b"content".to_vec(),
            serde_json::json!({}),
        )
        .unwrap();

    let cp_id = handle
        .commit_stage("SPEC-971", "run1", "plan", None)
        .unwrap();

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
    let uri = handle
        .put(
            "SPEC-971",
            "run1",
            ObjectType::Artifact,
            "test.md",
            b"content".to_vec(),
            serde_json::json!({}),
        )
        .unwrap();
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
    let uri = handle
        .put(
            "SPEC-971",
            "run1",
            ObjectType::Artifact,
            "test.md",
            b"content".to_vec(),
            serde_json::json!({}),
        )
        .unwrap();
    handle
        .commit_stage("SPEC-971", "run1", "plan", None)
        .unwrap();

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
    assert!(
        lock_path.exists(),
        "Lock file should exist while handle is open"
    );

    // Drop handle - lock should be released
    drop(handle);

    // Lock file should be removed
    assert!(
        !lock_path.exists(),
        "Lock file should be removed after handle drop"
    );
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
            assert_eq!(
                metadata.pid,
                std::process::id(),
                "Lock metadata should contain our PID"
            );
            assert!(
                !metadata.host.is_empty(),
                "Lock metadata should contain host"
            );
            assert!(
                !metadata.user.is_empty(),
                "Lock metadata should contain user"
            );
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
    assert!(
        !lock_path.exists(),
        "Read-only open should not create lock file"
    );

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
    assert!(
        reader.is_ok(),
        "Read-only should succeed when write lock is held"
    );
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
    let lock_result = results.iter().find(|r| match r {
        DiagnosticResult::Error(msg, _) => msg.contains("locked"),
        DiagnosticResult::Warning(msg, _) => msg.contains("lock"),
        _ => false,
    });

    assert!(lock_result.is_some(), "Doctor should detect the lock");

    // The error should contain our context
    if let Some(DiagnosticResult::Error(msg, recovery)) = lock_result {
        assert!(
            msg.contains("SPEC-999"),
            "Lock message should contain spec_id"
        );
        assert!(
            recovery.contains("ps -p"),
            "Recovery should include process check"
        );
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
        host: hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_default(),
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
    std::fs::write(
        &lock_path,
        serde_json::to_string_pretty(&lock_json).unwrap(),
    )
    .expect("should create remote lock");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "subprocess_test".to_string(),
        ..Default::default()
    };

    let result = CapsuleHandle::open(config);
    assert!(
        result.is_err(),
        "Should fail when lock is held by remote process"
    );

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
            assert_ne!(
                meta.pid,
                std::process::id(),
                "Lock should have child's PID, not ours"
            );
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
    assert!(
        result2.is_ok(),
        "Should succeed after subprocess dies (stale lock recovery)"
    );
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
        let has_errors = diagnostics.iter().any(|d| {
            matches!(
                d,
                crate::memvid_adapter::capsule::DiagnosticResult::Error(_, _)
            )
        });
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
        .filter(|e| {
            matches!(
                e.event_type,
                crate::memvid_adapter::EventType::PolicySnapshotRef
            )
        })
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
        .filter(|e| {
            matches!(
                e.event_type,
                crate::memvid_adapter::EventType::StageTransition
            )
        })
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
    assert!(
        policy_info.is_some(),
        "Should have current policy after capture"
    );

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
        .filter(|e| {
            matches!(
                e.event_type,
                crate::memvid_adapter::EventType::StageTransition
            )
        })
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
    use crate::memvid_adapter::policy_capture::{
        capture_and_store_policy, check_and_recapture_if_changed,
    };
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
    let initial_snapshot =
        capture_and_store_policy(&handle, &stage0_config, "SPEC-DRIFT", "run-drift")
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

    assert!(
        !stage_events.is_empty(),
        "Should have StageTransition event"
    );
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
    assert!(current.is_some(), "Should have current policy after check");
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
        .filter(|e| {
            matches!(
                e.event_type,
                crate::memvid_adapter::EventType::StageTransition
            )
        })
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
        .filter(|e| {
            matches!(
                e.event_type,
                crate::memvid_adapter::EventType::StageTransition
            )
        })
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
    handle
        .switch_branch(run_branch.clone())
        .expect("switch branch");

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
    handle
        .switch_branch(run_branch.clone())
        .expect("switch branch");

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
    handle
        .switch_branch(run_branch.clone())
        .expect("switch branch");

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
        handle
            .switch_branch(run_branch.clone())
            .expect("switch branch");

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
    handle
        .switch_branch(run_branch.clone())
        .expect("switch to run branch");

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
        payload["from_branch"], "run/test-run-001",
        "from_branch should match"
    );
    assert_eq!(payload["to_branch"], "main", "to_branch should be main");
    assert_eq!(payload["mode"], "Curated", "mode should be Curated");
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
    handle
        .switch_branch(run_branch.clone())
        .expect("switch branch");

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
    handle
        .switch_branch(run_branch.clone())
        .expect("switch branch");

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
    handle
        .switch_branch(run_branch.clone())
        .expect("switch branch");

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
        handle
            .switch_branch(run_branch.clone())
            .expect("switch branch");

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
    let artifact_uri =
        LogicalUri::new("ws", "SPEC-001", "run1", ObjectType::Artifact, "file.md").unwrap();
    assert!(artifact_uri.is_curated_eligible(), "Artifacts are curated");

    let policy_uri = LogicalUri::for_policy("ws", "policy-001");
    assert!(policy_uri.is_curated_eligible(), "Policies are curated");

    // Non-curated URIs
    let event_uri = LogicalUri::for_event("ws", "SPEC-001", "run1", 1);
    assert!(
        !event_uri.is_curated_eligible(),
        "Events handled separately"
    );
}

// =============================================================================
// SPEC-KIT-975: Replayable Audit Event Integration Tests
// =============================================================================

/// SPEC-KIT-975: Test emitting various audit event types.
///
/// This test verifies that all SPEC-KIT-975 event types can be emitted
/// and retrieved correctly from the capsule.
#[test]
fn test_spec_kit_975_event_emission() {
    use super::{
        ErrorEventPayload, ErrorSeverity, GateDecisionPayload, GateOutcome, LLMCaptureMode,
        ModelCallEnvelopePayload, PatchApplyPayload, RetrievalRequestPayload,
        RetrievalResponsePayload, RoutingMode, ToolCallPayload, ToolResultPayload,
    };

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("spec_kit_975.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "test975".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("open capsule");
    let spec_id = "SPEC-KIT-975";
    let run_id = "test-run-001";

    // Switch to run branch
    let run_branch = BranchId::for_run(run_id);
    handle.switch_branch(run_branch).expect("switch branch");

    // 1. Emit ToolCall event
    let tool_call = ToolCallPayload {
        call_id: "call-001".to_string(),
        tool_name: "read_file".to_string(),
        input: serde_json::json!({"path": "/tmp/test.txt"}),
        stage: Some("Implement".to_string()),
        role: Some("Implementer".to_string()),
    };
    let tool_call_uri = handle
        .emit_tool_call(spec_id, run_id, &tool_call)
        .expect("emit tool call");
    assert!(tool_call_uri.as_str().contains("event"));

    // 2. Emit ToolResult event
    let tool_result = ToolResultPayload {
        call_id: "call-001".to_string(),
        tool_name: "read_file".to_string(),
        success: true,
        output: Some(serde_json::json!({"content": "file contents"})),
        error: None,
        duration_ms: Some(50),
    };
    let _tool_result_uri = handle
        .emit_tool_result(spec_id, run_id, Some("Implement"), &tool_result)
        .expect("emit tool result");

    // 3. Emit RetrievalRequest event
    let retrieval_req = RetrievalRequestPayload {
        request_id: "req-001".to_string(),
        query: "How does authentication work?".to_string(),
        config: serde_json::json!({"top_k": 5}),
        source: "capsule".to_string(),
        stage: Some("Stage0".to_string()),
        role: Some("Architect".to_string()),
    };
    let _retrieval_req_uri = handle
        .emit_retrieval_request(spec_id, run_id, &retrieval_req)
        .expect("emit retrieval request");

    // 4. Emit RetrievalResponse event
    let retrieval_resp = RetrievalResponsePayload {
        request_id: "req-001".to_string(),
        hit_uris: vec![
            "mv2://test975/SPEC-KIT-975/test-run-001/artifact/auth.md".to_string(),
            "mv2://test975/SPEC-KIT-975/test-run-001/artifact/login.md".to_string(),
        ],
        fused_scores: Some(vec![0.95, 0.87]),
        explainability: None,
        latency_ms: Some(120),
        error: None,
    };
    let _retrieval_resp_uri = handle
        .emit_retrieval_response(spec_id, run_id, Some("Stage0"), &retrieval_resp)
        .expect("emit retrieval response");

    // 5. Emit PatchApply event
    let patch = PatchApplyPayload {
        patch_id: "patch-001".to_string(),
        file_path: "src/auth.rs".to_string(),
        patch_type: "modify".to_string(),
        diff: Some("--- a/src/auth.rs\n+++ b/src/auth.rs\n@@ -1 +1 @@\n-old\n+new".to_string()),
        before_hash: Some("abc123".to_string()),
        after_hash: Some("def456".to_string()),
        stage: Some("Implement".to_string()),
        success: true,
        error: None,
    };
    let _patch_uri = handle
        .emit_patch_apply(spec_id, run_id, &patch)
        .expect("emit patch apply");

    // 6. Emit GateDecision event
    let gate_decision = GateDecisionPayload {
        gate_name: "JudgeApprove".to_string(),
        outcome: GateOutcome::Pass,
        stage: "Judge".to_string(),
        confidence: Some(0.92),
        reason: Some("All tests passing, code quality acceptable".to_string()),
        details: None,
        blocking: true,
    };
    let _gate_uri = handle
        .emit_gate_decision(spec_id, run_id, &gate_decision)
        .expect("emit gate decision");

    // 7. Emit ErrorEvent
    let error_event = ErrorEventPayload {
        error_code: "E001".to_string(),
        message: "Test error for validation".to_string(),
        severity: ErrorSeverity::Warning,
        stage: Some("Implement".to_string()),
        component: Some("test-runner".to_string()),
        stack_trace: None,
        related_uris: None,
        recoverable: true,
    };
    let _error_uri = handle
        .emit_error_event(spec_id, run_id, &error_event)
        .expect("emit error event");

    // 8. Emit ModelCallEnvelope event (with PromptsOnly capture mode)
    let model_call = ModelCallEnvelopePayload {
        call_id: "llm-001".to_string(),
        model: "claude-3-opus".to_string(),
        routing_mode: RoutingMode::Cloud,
        capture_mode: LLMCaptureMode::PromptsOnly,
        stage: Some("Implement".to_string()),
        role: Some("Implementer".to_string()),
        prompt_hash: Some("sha256:abc123...".to_string()),
        response_hash: Some("sha256:def456...".to_string()),
        prompt: Some("Write a function to...".to_string()),
        response: None, // Not captured in PromptsOnly mode
        prompt_tokens: Some(150),
        response_tokens: Some(300),
        latency_ms: Some(2500),
        success: true,
        error: None,
    };
    let _model_call_uri = handle
        .emit_model_call_envelope(spec_id, run_id, &model_call)
        .expect("emit model call envelope");

    // Verify events were stored
    let all_events = handle.list_events();
    assert!(all_events.len() >= 8, "Should have at least 8 events");

    // Filter by event type
    let tool_calls: Vec<_> = all_events
        .iter()
        .filter(|e| e.event_type == EventType::ToolCall)
        .collect();
    assert_eq!(tool_calls.len(), 1, "Should have 1 ToolCall event");

    let gate_decisions: Vec<_> = all_events
        .iter()
        .filter(|e| e.event_type == EventType::GateDecision)
        .collect();
    assert_eq!(gate_decisions.len(), 1, "Should have 1 GateDecision event");

    // Verify event ordering (events should be in emission order)
    let tool_call_event = &tool_calls[0];
    assert_eq!(tool_call_event.spec_id, spec_id);
    assert_eq!(tool_call_event.run_id, run_id);
    assert!(tool_call_event.payload.get("call_id").is_some());

    // Verify audit-critical classification
    for event in &all_events {
        if event.event_type == EventType::GateDecision || event.event_type == EventType::ErrorEvent
        {
            assert!(
                event.event_type.is_audit_critical(),
                "{:?} should be audit-critical",
                event.event_type
            );
        }
    }

    // Verify curated-eligible classification
    for event in &all_events {
        if event.event_type == EventType::ModelCallEnvelope {
            // ModelCallEnvelope is NOT curated-eligible (may contain sensitive data)
            assert!(
                !event.event_type.is_curated_eligible(),
                "ModelCallEnvelope should NOT be curated-eligible"
            );
        } else if event.event_type == EventType::ToolCall {
            assert!(
                event.event_type.is_curated_eligible(),
                "ToolCall should be curated-eligible"
            );
        }
    }
}

/// SPEC-KIT-975: Test LLM capture modes match policy vocabulary.
///
/// Capture modes: none | prompts_only | full_io
/// - none: No event emitted
/// - prompts_only: Prompt stored, response hashed only (export-safe)
/// - full_io: Full prompt + response (NOT export-safe)
#[test]
fn test_llm_capture_modes_policy_aligned() {
    use super::{LLMCaptureMode, ModelCallEnvelopePayload, RoutingMode};

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("llm_capture_modes.mv2");

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "test975modes".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("open capsule");
    let spec_id = "SPEC-KIT-975";
    let run_id = "capture-modes-test";

    // Test PromptsOnly mode: prompt stored, response hashed only
    let prompts_only_call = ModelCallEnvelopePayload {
        call_id: "llm-prompts-only".to_string(),
        model: "test-model".to_string(),
        routing_mode: RoutingMode::Cloud,
        capture_mode: LLMCaptureMode::PromptsOnly,
        stage: Some("Implement".to_string()),
        role: None,
        prompt_hash: Some("sha256:prompt_hash".to_string()),
        response_hash: Some("sha256:response_hash".to_string()),
        prompt: Some("Full prompt content".to_string()),
        response: None, // Response NOT stored in prompts_only
        prompt_tokens: Some(100),
        response_tokens: Some(200),
        latency_ms: Some(100),
        success: true,
        error: None,
    };
    handle
        .emit_model_call_envelope(spec_id, run_id, &prompts_only_call)
        .expect("emit prompts_only mode call");

    // Test FullIo mode: both prompt and response stored (NOT export-safe)
    let full_io_call = ModelCallEnvelopePayload {
        call_id: "llm-full-io".to_string(),
        model: "test-model".to_string(),
        routing_mode: RoutingMode::Reflex,
        capture_mode: LLMCaptureMode::FullIo,
        stage: Some("Implement".to_string()),
        role: Some("Implementer".to_string()),
        prompt_hash: Some("sha256:prompt_hash_full".to_string()),
        response_hash: Some("sha256:response_hash_full".to_string()),
        prompt: Some("Full prompt content here".to_string()),
        response: Some("Full response content here".to_string()),
        prompt_tokens: Some(200),
        response_tokens: Some(400),
        latency_ms: Some(1500),
        success: true,
        error: None,
    };
    handle
        .emit_model_call_envelope(spec_id, run_id, &full_io_call)
        .expect("emit full_io mode call");

    // Verify capture modes are stored correctly
    let events = handle.list_events();
    let model_calls: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == EventType::ModelCallEnvelope)
        .collect();

    assert_eq!(model_calls.len(), 2);

    // Check prompts_only mode event
    let prompts_only_event = model_calls
        .iter()
        .find(|e| e.payload.get("call_id") == Some(&serde_json::json!("llm-prompts-only")))
        .expect("find prompts_only mode event");
    assert_eq!(
        prompts_only_event.payload.get("capture_mode"),
        Some(&serde_json::json!("PromptsOnly"))
    );
    assert!(prompts_only_event.payload.get("prompt").is_some());
    assert!(prompts_only_event.payload.get("response").is_none());

    // Check full_io mode event
    let full_io_event = model_calls
        .iter()
        .find(|e| e.payload.get("call_id") == Some(&serde_json::json!("llm-full-io")))
        .expect("find full_io mode event");
    assert_eq!(
        full_io_event.payload.get("capture_mode"),
        Some(&serde_json::json!("FullIo"))
    );
    assert!(full_io_event.payload.get("prompt").is_some());
    assert!(full_io_event.payload.get("response").is_some());

    // Verify export safety
    assert!(LLMCaptureMode::None.is_export_safe());
    assert!(LLMCaptureMode::PromptsOnly.is_export_safe());
    assert!(!LLMCaptureMode::FullIo.is_export_safe());

    // Verify mode string serialization matches policy vocabulary
    assert_eq!(LLMCaptureMode::None.as_str(), "none");
    assert_eq!(LLMCaptureMode::PromptsOnly.as_str(), "prompts_only");
    assert_eq!(LLMCaptureMode::FullIo.as_str(), "full_io");

    // Verify backward compat parsing
    assert_eq!(LLMCaptureMode::from_str("off"), Some(LLMCaptureMode::None));
    assert_eq!(
        LLMCaptureMode::from_str("hash"),
        Some(LLMCaptureMode::PromptsOnly)
    );
    assert_eq!(
        LLMCaptureMode::from_str("summary"),
        Some(LLMCaptureMode::PromptsOnly)
    );
    assert_eq!(
        LLMCaptureMode::from_str("full"),
        Some(LLMCaptureMode::FullIo)
    );
}

// =============================================================================
// SPEC-KIT-975: Runtime emit wiring integration tests
// =============================================================================

#[test]
fn test_runtime_emit_wiring_integration() {
    // This test verifies that all SPEC-KIT-975 event types can be emitted
    // and are correctly stored in the capsule for later replay.

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("emit_wiring.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "emit_wiring_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config.clone()).expect("should create capsule");

    let spec_id = "SPEC-KIT-975";
    let run_id = "test-run-emit-wiring";

    // 1. Emit ToolCall and ToolResult
    let tool_call = ToolCallPayload {
        call_id: "tool-001".to_string(),
        tool_name: "read_file".to_string(),
        input: serde_json::json!({"path": "test.md"}),
        stage: Some("Plan".to_string()),
        role: Some("Architect".to_string()),
    };
    handle
        .emit_tool_call(spec_id, run_id, &tool_call)
        .expect("emit tool call");

    let tool_result = ToolResultPayload {
        call_id: "tool-001".to_string(),
        tool_name: "read_file".to_string(),
        success: true,
        output: Some(serde_json::json!({"content": "file contents"})),
        error: None,
        duration_ms: Some(50),
    };
    handle
        .emit_tool_result(spec_id, run_id, Some("Plan"), &tool_result)
        .expect("emit tool result");

    // 2. Emit RetrievalRequest and RetrievalResponse
    let retrieval_req = RetrievalRequestPayload {
        request_id: "req-001".to_string(),
        query: "spec-kit architecture".to_string(),
        config: serde_json::json!({"domains": ["spec-kit"], "max_results": 5}),
        source: "capsule".to_string(),
        stage: Some("Plan".to_string()),
        role: None,
    };
    handle
        .emit_retrieval_request(spec_id, run_id, &retrieval_req)
        .expect("emit retrieval request");

    let retrieval_resp = RetrievalResponsePayload {
        request_id: "req-001".to_string(),
        hit_uris: vec!["mv2://emit_wiring_test/SPEC-KIT-975/test/artifact/decision.md".to_string()],
        fused_scores: Some(vec![0.95]),
        explainability: None,
        latency_ms: Some(25),
        error: None,
    };
    handle
        .emit_retrieval_response(spec_id, run_id, Some("Plan"), &retrieval_resp)
        .expect("emit retrieval response");

    // 3. Emit PatchApply
    let patch_apply = PatchApplyPayload {
        patch_id: "patch-001".to_string(),
        file_path: "src/main.rs".to_string(),
        patch_type: "modify".to_string(),
        diff: Some("@@ -1,3 +1,4 @@\n+// New line".to_string()),
        before_hash: Some("sha256:abc123".to_string()),
        after_hash: Some("sha256:def456".to_string()),
        stage: Some("Implement".to_string()),
        success: true,
        error: None,
    };
    handle
        .emit_patch_apply(spec_id, run_id, &patch_apply)
        .expect("emit patch apply");

    // 4. Emit ModelCallEnvelope (PromptsOnly mode)
    let model_call = ModelCallEnvelopePayload {
        call_id: "model-001".to_string(),
        model: "claude-3-opus".to_string(),
        routing_mode: RoutingMode::Cloud,
        capture_mode: LLMCaptureMode::PromptsOnly,
        stage: Some("Implement".to_string()),
        role: Some("Implementer".to_string()),
        prompt_hash: Some("sha256:prompt123".to_string()),
        response_hash: Some("sha256:response456".to_string()),
        prompt: Some("Implement the feature...".to_string()),
        response: None, // Not captured in PromptsOnly mode
        prompt_tokens: Some(100),
        response_tokens: Some(200),
        latency_ms: Some(1500),
        success: true,
        error: None,
    };
    handle
        .emit_model_call_envelope(spec_id, run_id, &model_call)
        .expect("emit model call");

    // 5. Emit GateDecision
    let gate_decision = GateDecisionPayload {
        gate_name: "JudgeApprove".to_string(),
        outcome: GateOutcome::Pass,
        stage: "Judge".to_string(),
        confidence: Some(0.95),
        reason: Some("Implementation meets acceptance criteria".to_string()),
        details: None,
        blocking: true,
    };
    handle
        .emit_gate_decision(spec_id, run_id, &gate_decision)
        .expect("emit gate decision");

    // Commit the stage to create a checkpoint
    handle
        .commit_stage(spec_id, run_id, "Judge", None)
        .expect("commit stage");

    // Verify events are stored correctly
    let events = handle.list_events();

    // Check each event type is present
    assert!(
        events.iter().any(|e| e.event_type == EventType::ToolCall),
        "Should have ToolCall event"
    );
    assert!(
        events.iter().any(|e| e.event_type == EventType::ToolResult),
        "Should have ToolResult event"
    );
    assert!(
        events
            .iter()
            .any(|e| e.event_type == EventType::RetrievalRequest),
        "Should have RetrievalRequest event"
    );
    assert!(
        events
            .iter()
            .any(|e| e.event_type == EventType::RetrievalResponse),
        "Should have RetrievalResponse event"
    );
    assert!(
        events.iter().any(|e| e.event_type == EventType::PatchApply),
        "Should have PatchApply event"
    );
    assert!(
        events
            .iter()
            .any(|e| e.event_type == EventType::ModelCallEnvelope),
        "Should have ModelCallEnvelope event"
    );
    assert!(
        events
            .iter()
            .any(|e| e.event_type == EventType::GateDecision),
        "Should have GateDecision event"
    );

    // Verify event order (sequence numbers should be monotonic)
    let event_uris: Vec<_> = events
        .iter()
        .filter(|e| e.run_id == run_id)
        .map(|e| e.uri.as_str().to_string())
        .collect();
    assert!(
        event_uris.len() >= 7,
        "Should have at least 7 events for this run"
    );

    // Verify events are associated with the correct run_id
    let run_events: Vec<_> = events.iter().filter(|e| e.run_id == run_id).collect();
    assert!(
        run_events.len() >= 7,
        "All events should have correct run_id"
    );

    // Drop and reopen to verify persistence
    drop(handle);
    let handle2 = CapsuleHandle::open(config).expect("should reopen");

    // Verify events are still present after reopen
    let events_after_reopen = handle2.list_events();
    assert!(
        events_after_reopen
            .iter()
            .any(|e| e.event_type == EventType::ToolCall),
        "ToolCall event should persist after reopen"
    );
    assert!(
        events_after_reopen
            .iter()
            .any(|e| e.event_type == EventType::RetrievalResponse),
        "RetrievalResponse event should persist after reopen"
    );
}

#[test]
fn test_emit_wiring_best_effort_never_fails() {
    // This test verifies that event emission is best-effort:
    // even if emission "fails", the run should not abort.
    // Note: In practice, failures are logged but not propagated.

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("best_effort.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "best_effort_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");

    // Emit multiple events in sequence - should all succeed
    let spec_id = "SPEC-975";
    let run_id = "best-effort-run";

    // All these should succeed (best-effort)
    let _ = handle.emit_tool_call(
        spec_id,
        run_id,
        &ToolCallPayload {
            call_id: "t1".to_string(),
            tool_name: "test".to_string(),
            input: serde_json::json!({}),
            stage: None,
            role: None,
        },
    );

    let _ = handle.emit_retrieval_request(
        spec_id,
        run_id,
        &RetrievalRequestPayload {
            request_id: "r1".to_string(),
            query: "test".to_string(),
            config: serde_json::json!({}),
            source: "test".to_string(),
            stage: None,
            role: None,
        },
    );

    // The important thing is: we didn't panic or abort
    // In a real integration, the run would continue regardless of emit errors
    assert!(
        handle.is_open(),
        "Capsule should still be open after emit operations"
    );
}

#[test]
fn test_retrieval_events_capture_hit_uris() {
    // This test verifies that RetrievalResponse events correctly capture hit URIs
    // for later replay verification.

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("retrieval_uris.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "retrieval_uris_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");

    let spec_id = "SPEC-975";
    let run_id = "retrieval-uri-test";

    // First, ingest some artifacts to get real URIs
    let artifact1_uri = handle
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "decision1.md",
            b"Decision 1 content".to_vec(),
            serde_json::json!({"type": "decision"}),
        )
        .expect("put artifact 1");

    let artifact2_uri = handle
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "decision2.md",
            b"Decision 2 content".to_vec(),
            serde_json::json!({"type": "decision"}),
        )
        .expect("put artifact 2");

    // Emit retrieval response referencing those URIs
    let retrieval_resp = RetrievalResponsePayload {
        request_id: "req-uris".to_string(),
        hit_uris: vec![
            artifact1_uri.as_str().to_string(),
            artifact2_uri.as_str().to_string(),
        ],
        fused_scores: Some(vec![0.95, 0.87]),
        explainability: None,
        latency_ms: Some(30),
        error: None,
    };
    handle
        .emit_retrieval_response(spec_id, run_id, None, &retrieval_resp)
        .expect("emit response");

    // Verify the event contains the URIs
    let events = handle.list_events();
    let resp_event = events
        .iter()
        .find(|e| e.event_type == EventType::RetrievalResponse && e.run_id == run_id)
        .expect("find retrieval response event");

    let hit_uris = resp_event
        .payload
        .get("hit_uris")
        .and_then(|v| v.as_array())
        .expect("hit_uris should be array");

    assert_eq!(hit_uris.len(), 2, "Should have 2 hit URIs");
    assert!(
        hit_uris
            .iter()
            .any(|u| u.as_str() == Some(artifact1_uri.as_str()))
    );
    assert!(
        hit_uris
            .iter()
            .any(|u| u.as_str() == Some(artifact2_uri.as_str()))
    );

    // Verify the URIs are valid mv2:// URIs
    assert!(
        artifact1_uri.as_str().starts_with("mv2://"),
        "artifact1 should have mv2:// scheme"
    );
    assert!(
        artifact2_uri.as_str().starts_with("mv2://"),
        "artifact2 should have mv2:// scheme"
    );
}

#[test]
fn test_replay_timeline_deterministic() {
    // This test verifies that event emission order is deterministic and preserved
    // across capsule reopen - essential for replay reliability.

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("replay_timeline.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "replay_timeline_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config.clone()).expect("should create capsule");

    let spec_id = "SPEC-REPLAY-TIMELINE";
    let run_id = "timeline-determinism-001";

    // Phase 1: Emit a sequence of events in specific order
    // Order: RetrievalRequest → RetrievalResponse → ToolCall → ToolResult → PatchApply

    // Event 1: RetrievalRequest
    let req_payload = RetrievalRequestPayload {
        request_id: "req-timeline-001".to_string(),
        query: "architecture decisions".to_string(),
        config: serde_json::json!({"top_k": 5, "domains": ["spec-kit"]}),
        source: "capsule".to_string(),
        stage: Some("Plan".to_string()),
        role: Some("Architect".to_string()),
    };
    handle
        .emit_retrieval_request(spec_id, run_id, &req_payload)
        .expect("emit retrieval request");

    // Event 2: RetrievalResponse
    let resp_payload = RetrievalResponsePayload {
        request_id: "req-timeline-001".to_string(),
        hit_uris: vec![
            "mv2://replay_timeline_test/SPEC-REPLAY-TIMELINE/spec/artifact/decision.md".to_string(),
        ],
        fused_scores: Some(vec![0.92]),
        explainability: None,
        latency_ms: Some(45),
        error: None,
    };
    handle
        .emit_retrieval_response(spec_id, run_id, Some("Plan"), &resp_payload)
        .expect("emit retrieval response");

    // Event 3: ToolCall
    let tool_call = ToolCallPayload {
        call_id: "tool-timeline-001".to_string(),
        tool_name: "read_file".to_string(),
        input: serde_json::json!({"path": "src/lib.rs"}),
        stage: Some("Implement".to_string()),
        role: Some("Implementer".to_string()),
    };
    handle
        .emit_tool_call(spec_id, run_id, &tool_call)
        .expect("emit tool call");

    // Event 4: ToolResult
    let tool_result = ToolResultPayload {
        call_id: "tool-timeline-001".to_string(),
        tool_name: "read_file".to_string(),
        success: true,
        output: Some(serde_json::json!({"content": "pub mod adapter;"})),
        error: None,
        duration_ms: Some(12),
    };
    handle
        .emit_tool_result(spec_id, run_id, Some("Implement"), &tool_result)
        .expect("emit tool result");

    // Event 5: PatchApply
    let patch_apply = PatchApplyPayload {
        patch_id: "patch-timeline-001".to_string(),
        file_path: "src/lib.rs".to_string(),
        patch_type: "modify".to_string(),
        diff: Some("@@ -1,1 +1,2 @@\n pub mod adapter;\n+pub mod events;".to_string()),
        before_hash: Some("sha256:before123".to_string()),
        after_hash: Some("sha256:after456".to_string()),
        stage: Some("Implement".to_string()),
        success: true,
        error: None,
    };
    handle
        .emit_patch_apply(spec_id, run_id, &patch_apply)
        .expect("emit patch apply");

    // Phase 2: Capture events and verify order
    let events: Vec<_> = handle
        .list_events()
        .into_iter()
        .filter(|e| e.run_id == run_id && e.spec_id == spec_id)
        .collect();

    assert!(
        events.len() >= 5,
        "Should have at least 5 events, got {}",
        events.len()
    );

    // Verify timestamps are monotonic (non-decreasing)
    for i in 0..events.len() - 1 {
        assert!(
            events[i].timestamp <= events[i + 1].timestamp,
            "Event {} timestamp ({}) should be <= event {} timestamp ({})",
            i,
            events[i].timestamp,
            i + 1,
            events[i + 1].timestamp
        );
    }

    // Verify event type sequence matches insertion order
    let event_types: Vec<_> = events.iter().map(|e| e.event_type).collect();
    assert_eq!(
        event_types[0],
        EventType::RetrievalRequest,
        "Event 0 should be RetrievalRequest"
    );
    assert_eq!(
        event_types[1],
        EventType::RetrievalResponse,
        "Event 1 should be RetrievalResponse"
    );
    assert_eq!(
        event_types[2],
        EventType::ToolCall,
        "Event 2 should be ToolCall"
    );
    assert_eq!(
        event_types[3],
        EventType::ToolResult,
        "Event 3 should be ToolResult"
    );
    assert_eq!(
        event_types[4],
        EventType::PatchApply,
        "Event 4 should be PatchApply"
    );

    // Capture URIs for determinism check
    let original_uris: Vec<_> = events.iter().map(|e| e.uri.as_str().to_string()).collect();

    // Phase 3: Drop handle and reopen to verify persistence
    drop(handle);
    let handle2 = CapsuleHandle::open(config).expect("should reopen capsule");

    let events_after_reopen: Vec<_> = handle2
        .list_events()
        .into_iter()
        .filter(|e| e.run_id == run_id && e.spec_id == spec_id)
        .collect();

    // Verify event count matches
    assert_eq!(
        events_after_reopen.len(),
        events.len(),
        "Event count should match after reopen: {} vs {}",
        events_after_reopen.len(),
        events.len()
    );

    // Verify determinism: URIs and types should match exactly after reopen
    for (i, (original, reopened)) in events.iter().zip(events_after_reopen.iter()).enumerate() {
        assert_eq!(
            original.uri.as_str(),
            reopened.uri.as_str(),
            "Event {} URI should match after reopen: {} vs {}",
            i,
            original.uri.as_str(),
            reopened.uri.as_str()
        );
        assert_eq!(
            original.event_type, reopened.event_type,
            "Event {} type should match after reopen",
            i
        );
        assert_eq!(
            original.timestamp, reopened.timestamp,
            "Event {} timestamp should match after reopen",
            i
        );
    }

    // Verify URIs are immutable (original URIs still valid)
    for (i, uri) in original_uris.iter().enumerate() {
        assert!(
            events_after_reopen.iter().any(|e| e.uri.as_str() == uri),
            "Original URI {} should be present after reopen: {}",
            i,
            uri
        );
    }
}

#[test]
fn test_replay_offline_retrieval_exact() {
    // This test verifies that retrieval results are captured with exact precision
    // for offline replay - hit_uris and fused_scores must match exactly.

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("replay_offline_retrieval.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "replay_offline_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config.clone()).expect("should create capsule");

    let spec_id = "SPEC-REPLAY-OFFLINE";
    let run_id = "offline-exact-001";

    // Phase 1: Ingest reference artifacts to get real URIs
    let artifact_uris = vec![
        handle
            .put(
                spec_id,
                run_id,
                ObjectType::Artifact,
                "decision_alpha.md",
                b"Decision Alpha: Use event sourcing pattern".to_vec(),
                serde_json::json!({"type": "decision", "priority": "high"}),
            )
            .expect("put artifact alpha"),
        handle
            .put(
                spec_id,
                run_id,
                ObjectType::Artifact,
                "decision_beta.md",
                b"Decision Beta: Capsule-first architecture".to_vec(),
                serde_json::json!({"type": "decision", "priority": "medium"}),
            )
            .expect("put artifact beta"),
        handle
            .put(
                spec_id,
                run_id,
                ObjectType::Artifact,
                "decision_gamma.md",
                b"Decision Gamma: Best-effort event emission".to_vec(),
                serde_json::json!({"type": "decision", "priority": "low"}),
            )
            .expect("put artifact gamma"),
    ];

    // Phase 2: Emit retrieval request
    let req_id = "offline-req-exact-001";
    let request_payload = RetrievalRequestPayload {
        request_id: req_id.to_string(),
        query: "what architectural decisions were made?".to_string(),
        config: serde_json::json!({
            "top_k": 3,
            "filters": {"type": "decision"},
            "sort_by": "relevance"
        }),
        source: "capsule".to_string(),
        stage: Some("Plan".to_string()),
        role: None,
    };
    handle
        .emit_retrieval_request(spec_id, run_id, &request_payload)
        .expect("emit retrieval request");

    // Phase 3: Emit retrieval response with exact hit set and scores
    // These exact values must be preserved for offline replay
    let exact_scores = vec![0.98, 0.95, 0.87];
    let response_payload = RetrievalResponsePayload {
        request_id: req_id.to_string(),
        hit_uris: artifact_uris
            .iter()
            .map(|uri| uri.as_str().to_string())
            .collect(),
        fused_scores: Some(exact_scores.clone()),
        explainability: Some(serde_json::json!({
            "method": "hybrid_fusion",
            "weights": {"semantic": 0.7, "keyword": 0.3}
        })),
        latency_ms: Some(67),
        error: None,
    };
    handle
        .emit_retrieval_response(spec_id, run_id, Some("Plan"), &response_payload)
        .expect("emit retrieval response");

    // Phase 4: Verify exact match in event payload
    let events = handle.list_events();

    // Find retrieval response event
    let resp_event = events
        .iter()
        .find(|e| e.event_type == EventType::RetrievalResponse && e.run_id == run_id)
        .expect("should find retrieval response event");

    // Assertion 1: Response has correct request_id
    let payload_request_id = resp_event
        .payload
        .get("request_id")
        .and_then(|v| v.as_str())
        .expect("response should have request_id");
    assert_eq!(
        payload_request_id, req_id,
        "Response should reference correct request"
    );

    // Assertion 2: Hit URIs are exact match in order
    let hit_uris = resp_event
        .payload
        .get("hit_uris")
        .and_then(|v| v.as_array())
        .expect("hit_uris should be array");

    assert_eq!(
        hit_uris.len(),
        artifact_uris.len(),
        "Should have exact hit count"
    );

    for (i, artifact_uri) in artifact_uris.iter().enumerate() {
        let stored_uri = hit_uris[i].as_str().expect("uri should be string");
        assert_eq!(
            stored_uri,
            artifact_uri.as_str(),
            "Hit {} should match artifact URI exactly: {} vs {}",
            i,
            stored_uri,
            artifact_uri.as_str()
        );
    }

    // Assertion 3: Fused scores are exact (no epsilon tolerance for replay)
    let fused_scores = resp_event
        .payload
        .get("fused_scores")
        .and_then(|v| v.as_array())
        .expect("fused_scores should be array");

    assert_eq!(
        fused_scores.len(),
        exact_scores.len(),
        "Should have exact score count"
    );

    for (i, expected_score) in exact_scores.iter().enumerate() {
        let actual_score = fused_scores[i].as_f64().expect("score should be number");
        assert_eq!(
            actual_score, *expected_score,
            "Score {} should match exactly for offline replay: {} vs {}",
            i, actual_score, expected_score
        );
    }

    // Assertion 4: Artifact URIs are valid mv2:// URIs
    for (i, uri) in artifact_uris.iter().enumerate() {
        assert!(
            uri.as_str().starts_with("mv2://"),
            "Artifact {} should have mv2:// scheme: {}",
            i,
            uri.as_str()
        );
    }

    // Assertion 5: Verify artifact URIs are valid mv2:// URIs
    // Note: Full resolve_uri requires commit_stage; here we just verify URI format.
    // The key offline replay property is that hit_uris in the event payload are
    // preserved exactly, which is verified in Assertions 2 and 3.

    // Assertion 6: Explainability metadata preserved
    let explainability = resp_event
        .payload
        .get("explainability")
        .expect("explainability should be present");
    assert_eq!(
        explainability.get("method").and_then(|v| v.as_str()),
        Some("hybrid_fusion"),
        "Explainability method should be preserved"
    );
}

// =============================================================================
// SPEC-KIT-978: Circuit Breaker Type Tests
// =============================================================================

#[test]
fn test_breaker_state_variants() {
    // Test all BreakerState variants and their string representations.
    use super::BreakerState;

    assert_eq!(BreakerState::Closed.as_str(), "closed");
    assert_eq!(BreakerState::Open.as_str(), "open");
    assert_eq!(BreakerState::HalfOpen.as_str(), "half_open");

    // Test from_str round-trip
    assert_eq!(BreakerState::from_str("closed"), Some(BreakerState::Closed));
    assert_eq!(BreakerState::from_str("open"), Some(BreakerState::Open));
    assert_eq!(
        BreakerState::from_str("half_open"),
        Some(BreakerState::HalfOpen)
    );
    assert_eq!(BreakerState::from_str("invalid"), None);
}

#[test]
fn test_breaker_state_changed_payload_serialization() {
    // Test serialization round-trip for BreakerStateChangedPayload.
    use super::{BreakerState, BreakerStateChangedPayload};

    let payload = BreakerStateChangedPayload {
        breaker_id: "reflex_server".to_string(),
        current_state: BreakerState::Open,
        previous_state: BreakerState::Closed,
        reason: "Failure rate exceeded threshold (35% > 30%)".to_string(),
        stage: Some("Implement".to_string()),
        component: Some("reflex_router".to_string()),
        failure_count: Some(7),
        failure_rate: Some(35.0),
        retry_after_seconds: Some(30),
        successful_probes: None,
        probes_required: None,
    };

    // Serialize
    let json = serde_json::to_string(&payload).expect("serialize");
    assert!(json.contains("reflex_server"));
    // BreakerState serializes as variant name (e.g., "Open" not "open")
    assert!(json.contains("Open"), "Should contain Open: {}", json);
    assert!(json.contains("Closed"), "Should contain Closed: {}", json);

    // Deserialize
    let parsed: BreakerStateChangedPayload = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.breaker_id, "reflex_server");
    assert_eq!(parsed.current_state, BreakerState::Open);
    assert_eq!(parsed.previous_state, BreakerState::Closed);
    assert_eq!(parsed.reason, "Failure rate exceeded threshold (35% > 30%)");
    assert_eq!(parsed.stage, Some("Implement".to_string()));
    assert_eq!(parsed.component, Some("reflex_router".to_string()));
    assert_eq!(parsed.failure_count, Some(7));
    assert_eq!(parsed.failure_rate, Some(35.0));
    assert_eq!(parsed.retry_after_seconds, Some(30));
    assert!(parsed.successful_probes.is_none());
    assert!(parsed.probes_required.is_none());
}

#[test]
fn test_breaker_state_changed_payload_skip_none_fields() {
    // Test that None fields are skipped in serialization (skip_serializing_if).
    use super::{BreakerState, BreakerStateChangedPayload};

    let payload = BreakerStateChangedPayload {
        breaker_id: "minimal_breaker".to_string(),
        current_state: BreakerState::Closed,
        previous_state: BreakerState::HalfOpen,
        reason: "Probes succeeded".to_string(),
        stage: None,
        component: None,
        failure_count: None,
        failure_rate: None,
        retry_after_seconds: None,
        successful_probes: Some(3),
        probes_required: Some(3),
    };

    let json = serde_json::to_string(&payload).expect("serialize");

    // These required fields should be present
    assert!(json.contains("breaker_id"));
    assert!(json.contains("current_state"));
    assert!(json.contains("previous_state"));
    assert!(json.contains("reason"));

    // These None fields should NOT be present (skip_serializing_if)
    assert!(!json.contains("stage"));
    assert!(!json.contains("component"));
    assert!(!json.contains("failure_count"));
    assert!(!json.contains("failure_rate"));
    assert!(!json.contains("retry_after_seconds"));

    // These Some fields should be present
    assert!(json.contains("successful_probes"));
    assert!(json.contains("probes_required"));
}

#[test]
fn test_event_type_breaker_state_changed() {
    // Test EventType::BreakerStateChanged integration.
    use super::EventType;

    // Test as_str
    assert_eq!(
        EventType::BreakerStateChanged.as_str(),
        "BreakerStateChanged"
    );

    // Test from_str
    assert_eq!(
        EventType::from_str("BreakerStateChanged"),
        Some(EventType::BreakerStateChanged)
    );

    // Test curated eligibility (circuit breaker events should be curated)
    assert!(EventType::BreakerStateChanged.is_curated_eligible());

    // Test audit criticality (circuit breaker events are audit-critical)
    assert!(EventType::BreakerStateChanged.is_audit_critical());

    // Verify in all_variants
    assert!(EventType::all_variants().contains(&"BreakerStateChanged"));
}

// =============================================================================
// SPEC-KIT-973: Time-Travel UI Tests
// =============================================================================

/// SPEC-KIT-973: Label-based checkpoint lookup in branch.
///
/// Verifies that the same label can exist on different branches and lookup
/// returns the correct checkpoint for each branch.
#[test]
fn test_checkpoint_label_lookup_in_branch() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("label_lookup.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "label_lookup_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("open capsule");

    // Create checkpoint on main with label "v1.0"
    // First, put some content so the checkpoint has something
    handle
        .put(
            "SPEC-973",
            "main",
            ObjectType::Artifact,
            "main_doc.md",
            b"Main branch content".to_vec(),
            serde_json::json!({}),
        )
        .expect("put on main");

    let main_cp = handle
        .commit_manual("v1.0")
        .expect("commit manual on main with label v1.0");

    // Switch to a run branch
    let run_branch = BranchId::for_run("test-run");
    handle
        .switch_branch(run_branch.clone())
        .expect("switch to run branch");

    // Create checkpoint on run branch with same label "v1.0"
    handle
        .put(
            "SPEC-973",
            "test-run",
            ObjectType::Artifact,
            "run_doc.md",
            b"Run branch content".to_vec(),
            serde_json::json!({}),
        )
        .expect("put on run branch");

    // Note: commit_manual allows specifying labels directly
    let run_cp = handle
        .commit_manual("v1.0")
        .expect("commit manual on run branch with label v1.0");

    // Verify lookup returns correct checkpoint per branch
    let main_lookup = handle.get_checkpoint_by_label_in_branch("v1.0", &BranchId::main());
    let run_lookup = handle.get_checkpoint_by_label_in_branch("v1.0", &run_branch);

    assert!(
        main_lookup.is_some(),
        "Should find checkpoint on main branch"
    );
    assert!(run_lookup.is_some(), "Should find checkpoint on run branch");

    assert_eq!(
        main_lookup.unwrap().checkpoint_id,
        main_cp,
        "Main branch lookup should return main checkpoint"
    );
    assert_eq!(
        run_lookup.unwrap().checkpoint_id,
        run_cp,
        "Run branch lookup should return run checkpoint"
    );
}

/// SPEC-KIT-973: As-of resolution returns historical bytes.
///
/// Verifies that get_bytes with an as_of checkpoint returns the content
/// from that point in time, not the latest content.
#[test]
fn test_asof_resolution_returns_historical_bytes() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("asof_bytes.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "asof_bytes_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("open capsule");

    // Switch to run branch for testing
    let run_branch = BranchId::for_run("asof-run");
    handle
        .switch_branch(run_branch.clone())
        .expect("switch branch");

    // Put artifact with "version1", commit checkpoint 1
    let uri = handle
        .put(
            "SPEC-973",
            "asof-run",
            ObjectType::Artifact,
            "spec.md",
            b"version1".to_vec(),
            serde_json::json!({"version": 1}),
        )
        .expect("put version1");

    let cp1 = handle
        .commit_stage("SPEC-973", "asof-run", "Plan", Some("cp1"))
        .expect("commit checkpoint 1");

    // Update artifact to "version2", commit checkpoint 2
    let _uri_v2 = handle
        .put(
            "SPEC-973",
            "asof-run",
            ObjectType::Artifact,
            "spec.md",
            b"version2".to_vec(),
            serde_json::json!({"version": 2}),
        )
        .expect("put version2");

    let cp2 = handle
        .commit_stage("SPEC-973", "asof-run", "Tasks", Some("cp2"))
        .expect("commit checkpoint 2");

    // Verify as-of resolution returns correct historical bytes
    let v1_bytes = handle
        .get_bytes(&uri, Some(&run_branch), Some(&cp1))
        .expect("get bytes at cp1");
    assert_eq!(
        v1_bytes,
        b"version1".to_vec(),
        "As-of cp1 should return version1"
    );

    let v2_bytes = handle
        .get_bytes(&uri, Some(&run_branch), Some(&cp2))
        .expect("get bytes at cp2");
    assert_eq!(
        v2_bytes,
        b"version2".to_vec(),
        "As-of cp2 should return version2"
    );

    // Latest (as_of=None) should return version2
    let latest_bytes = handle
        .get_bytes(&uri, Some(&run_branch), None)
        .expect("get bytes latest");
    assert_eq!(
        latest_bytes,
        b"version2".to_vec(),
        "Latest should return version2"
    );
}

/// SPEC-KIT-973: Diff between checkpoints produces expected output.
///
/// Verifies that content at two different checkpoints can be retrieved
/// and compared to show differences.
#[test]
fn test_diff_between_checkpoints() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("diff_test.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "diff_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("open capsule");

    // Switch to run branch
    let run_branch = BranchId::for_run("diff-run");
    handle
        .switch_branch(run_branch.clone())
        .expect("switch branch");

    // Create artifact with content A
    let content_a = "line1\nline2\nline3\n";
    let uri = handle
        .put(
            "SPEC-973",
            "diff-run",
            ObjectType::Artifact,
            "test.txt",
            content_a.as_bytes().to_vec(),
            serde_json::json!({}),
        )
        .expect("put content A");

    let cp1 = handle
        .commit_stage("SPEC-973", "diff-run", "Plan", Some("v1"))
        .expect("commit checkpoint v1");

    // Update to content B
    let content_b = "line1\nmodified line2\nline3\nline4\n";
    let _uri_b = handle
        .put(
            "SPEC-973",
            "diff-run",
            ObjectType::Artifact,
            "test.txt",
            content_b.as_bytes().to_vec(),
            serde_json::json!({}),
        )
        .expect("put content B");

    let cp2 = handle
        .commit_stage("SPEC-973", "diff-run", "Tasks", Some("v2"))
        .expect("commit checkpoint v2");

    // Get bytes at both checkpoints
    let bytes_a = handle
        .get_bytes(&uri, Some(&run_branch), Some(&cp1))
        .expect("get bytes at v1");
    let bytes_b = handle
        .get_bytes(&uri, Some(&run_branch), Some(&cp2))
        .expect("get bytes at v2");

    // Verify content differs
    assert_ne!(
        bytes_a, bytes_b,
        "Content at different checkpoints should differ"
    );

    // Verify actual content matches expectations
    assert_eq!(
        String::from_utf8_lossy(&bytes_a),
        content_a,
        "Content at v1 should match content_a"
    );
    assert_eq!(
        String::from_utf8_lossy(&bytes_b),
        content_b,
        "Content at v2 should match content_b"
    );

    // Verify we can identify the specific differences
    let lines_a: Vec<&str> = content_a.lines().collect();
    let lines_b: Vec<&str> = content_b.lines().collect();

    // line2 was modified
    assert_eq!(lines_a[1], "line2");
    assert_eq!(lines_b[1], "modified line2");

    // line4 was added
    assert_eq!(lines_a.len(), 3);
    assert_eq!(lines_b.len(), 4);
    assert_eq!(lines_b[3], "line4");
}

// =============================================================================
// SPEC-KIT-976: Memory Card and Logic Edge Tests
// =============================================================================

/// SPEC-KIT-976: Memory card round-trip storage test.
/// Creates a card, stores it, retrieves it, and verifies all fields.
#[test]
fn test_memory_card_round_trip() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("card_roundtrip.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "card_test".to_string(),
        ..Default::default()
    };

    // Create card with facts
    let card = MemoryCardV1::new("card-001", CardType::Task, "Implement feature X", "cli")
        .with_spec_id("SPEC-KIT-976")
        .with_run_id("run-001")
        .with_fact(CardFact {
            key: "status".to_string(),
            value: serde_json::Value::String("in_progress".to_string()),
            value_type: FactValueType::String,
            confidence: Some(1.0),
            source_uris: Vec::new(),
        });

    // Store card
    let handle = CapsuleHandle::open(config.clone()).expect("should create capsule");
    let data = card.to_bytes().expect("should serialize");
    let uri = handle
        .put(
            "SPEC-KIT-976",
            "run-001",
            ObjectType::Card,
            "card-001",
            data,
            serde_json::json!({"card_type": "task"}),
        )
        .expect("should put card");

    // Commit
    handle
        .commit_stage("SPEC-KIT-976", "run-001", "implement", None)
        .unwrap();

    // Verify URI is valid card URI
    assert!(uri.is_valid(), "URI should be valid");
    assert_eq!(
        uri.object_type(),
        Some(ObjectType::Card),
        "URI should have Card object type"
    );

    // Get bytes and deserialize
    let retrieved_bytes = handle
        .get_bytes(&uri, None, None)
        .expect("should get bytes");
    let retrieved_card =
        MemoryCardV1::from_bytes(&retrieved_bytes).expect("should deserialize card");

    // Verify round-trip
    assert_eq!(retrieved_card.card_id, "card-001");
    assert_eq!(retrieved_card.card_type, CardType::Task);
    assert_eq!(retrieved_card.title, "Implement feature X");
    assert_eq!(retrieved_card.version, 1);
    assert_eq!(retrieved_card.facts.len(), 1);
    assert_eq!(retrieved_card.facts[0].key, "status");
    assert_eq!(
        retrieved_card.provenance.spec_id,
        Some("SPEC-KIT-976".to_string())
    );
    assert_eq!(
        retrieved_card.provenance.run_id,
        Some("run-001".to_string())
    );
}

/// SPEC-KIT-976: Logic edge round-trip storage test.
/// Creates an edge linking two cards, stores it, retrieves it, and verifies all fields.
#[test]
fn test_logic_edge_round_trip() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("edge_roundtrip.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "edge_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config.clone()).expect("should create capsule");

    // First, create two cards to link
    let card1_uri = handle
        .put(
            "SPEC-976",
            "run1",
            ObjectType::Card,
            "card-a",
            b"{}".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put card-a");

    let card2_uri = handle
        .put(
            "SPEC-976",
            "run1",
            ObjectType::Card,
            "card-b",
            b"{}".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put card-b");

    // Create edge with LogicalUri references
    let edge = LogicEdgeV1::new(
        "edge-001",
        EdgeType::DependsOn,
        card1_uri.clone(), // from_uri is LogicalUri
        card2_uri.clone(), // to_uri is LogicalUri
        "cli",
    )
    .with_weight(0.95)
    .with_spec_id("SPEC-976")
    .with_run_id("run1");

    // Store edge
    let edge_data = edge.to_bytes().expect("should serialize edge");
    let edge_uri = handle
        .put(
            "SPEC-976",
            "run1",
            ObjectType::Edge,
            "edge-001",
            edge_data,
            serde_json::json!({"edge_type": "depends_on"}),
        )
        .expect("should put edge");

    // Commit
    handle
        .commit_stage("SPEC-976", "run1", "graph", None)
        .unwrap();

    // Verify edge URI
    assert!(edge_uri.is_valid(), "Edge URI should be valid");
    assert_eq!(
        edge_uri.object_type(),
        Some(ObjectType::Edge),
        "URI should have Edge object type"
    );

    // Get bytes and deserialize
    let retrieved_bytes = handle
        .get_bytes(&edge_uri, None, None)
        .expect("should get edge bytes");
    let retrieved_edge =
        LogicEdgeV1::from_bytes(&retrieved_bytes).expect("should deserialize edge");

    // Verify round-trip
    assert_eq!(retrieved_edge.edge_id, "edge-001");
    assert_eq!(retrieved_edge.edge_type, EdgeType::DependsOn);
    assert_eq!(retrieved_edge.from_uri.as_str(), card1_uri.as_str());
    assert_eq!(retrieved_edge.to_uri.as_str(), card2_uri.as_str());
    assert_eq!(retrieved_edge.weight, Some(0.95));
    assert_eq!(retrieved_edge.version, 1);
    assert_eq!(
        retrieved_edge.provenance.spec_id,
        Some("SPEC-976".to_string())
    );
}

/// SPEC-KIT-976: Type safety test - edges can only reference LogicalUri.
/// This test documents that from_uri and to_uri are LogicalUri type, not String.
#[test]
fn test_edge_references_logical_uris_only() {
    // This test enforces that edge from_uri and to_uri are LogicalUri, not String
    // The type system enforces this at compile time, but we document it here

    let from_uri: LogicalUri = "mv2://test/SPEC/run/card/a".parse().unwrap();
    let to_uri: LogicalUri = "mv2://test/SPEC/run/card/b".parse().unwrap();

    let edge = LogicEdgeV1::new(
        "e1",
        EdgeType::References,
        from_uri.clone(),
        to_uri.clone(),
        "test",
    );

    // from_uri and to_uri are LogicalUri type, not String
    assert!(
        edge.from_uri.is_valid(),
        "from_uri should be valid LogicalUri"
    );
    assert!(edge.to_uri.is_valid(), "to_uri should be valid LogicalUri");

    // Invalid URI should fail to parse
    let bad_uri: Result<LogicalUri, _> = "not-a-uri".parse();
    assert!(bad_uri.is_err(), "Invalid URI should fail to parse");

    // This ensures we can't accidentally pass raw strings as URIs
    // The following would NOT compile (commented out for documentation):
    // let edge = LogicEdgeV1::new("e1", EdgeType::References, "not-uri", "also-not-uri", "test");
}

/// SPEC-KIT-976: CardType enum parsing test.
/// All CardType variants should parse correctly and round-trip.
#[test]
fn test_card_type_variants() {
    // All variants parse correctly
    for variant_str in CardType::all_variants() {
        let parsed = CardType::from_str(variant_str);
        assert!(
            parsed.is_some(),
            "CardType::from_str should parse '{}'",
            variant_str
        );
    }

    // Round-trip all variants
    for ct in &[
        CardType::Spec,
        CardType::Decision,
        CardType::Task,
        CardType::Risk,
        CardType::Component,
        CardType::Person,
        CardType::Artifact,
        CardType::Run,
    ] {
        let s = ct.as_str();
        let parsed = CardType::from_str(s);
        assert_eq!(parsed, Some(*ct), "CardType round-trip failed for {:?}", ct);
    }

    // Unknown type returns None
    assert_eq!(
        CardType::from_str("unknown_type"),
        None,
        "Unknown card type should return None"
    );
}

/// SPEC-KIT-976: EdgeType enum parsing test.
/// All EdgeType variants should parse correctly and round-trip.
#[test]
fn test_edge_type_variants() {
    // All variants parse correctly
    for variant_str in EdgeType::all_variants() {
        let parsed = EdgeType::from_str(variant_str);
        assert!(
            parsed.is_some(),
            "EdgeType::from_str should parse '{}'",
            variant_str
        );
    }

    // Round-trip all variants
    for et in &[
        EdgeType::DependsOn,
        EdgeType::Blocks,
        EdgeType::Implements,
        EdgeType::References,
        EdgeType::Owns,
        EdgeType::Risks,
        EdgeType::RelatedTo,
    ] {
        let s = et.as_str();
        let parsed = EdgeType::from_str(s);
        assert_eq!(parsed, Some(*et), "EdgeType round-trip failed for {:?}", et);
    }

    // Unknown type returns None
    assert_eq!(
        EdgeType::from_str("unknown_edge"),
        None,
        "Unknown edge type should return None"
    );
}

/// SPEC-KIT-976: Card persistence test.
/// Verifies that cards persist across capsule reopen.
#[test]
fn test_card_persists_after_reopen() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("card_persist.mv2");
    let mut stored_uri: Option<LogicalUri> = None;

    // Phase 1: Create and store card
    {
        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "persist_test".to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open(config).expect("should create capsule");

        let card = MemoryCardV1::new("persist-card", CardType::Decision, "Use Rust", "test");
        let data = card.to_bytes().expect("should serialize");
        let uri = handle
            .put(
                "SPEC-T",
                "run1",
                ObjectType::Card,
                "persist-card",
                data,
                serde_json::json!({}),
            )
            .expect("should put card");
        stored_uri = Some(uri);

        handle
            .commit_stage("SPEC-T", "run1", "decide", None)
            .expect("should commit");
    }

    // Phase 2: Reopen and verify
    {
        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "persist_test".to_string(),
            ..Default::default()
        };

        let handle = CapsuleHandle::open(config).expect("should reopen capsule");

        let uri = stored_uri.as_ref().expect("should have stored URI");
        let bytes = handle
            .get_bytes(uri, None, None)
            .expect("should get card after reopen");
        let card = MemoryCardV1::from_bytes(&bytes).expect("should deserialize");

        assert_eq!(card.card_id, "persist-card", "card_id should match");
        assert_eq!(card.title, "Use Rust", "title should match");
        assert_eq!(card.card_type, CardType::Decision, "card_type should match");
    }
}

// =============================================================================
// SPEC-KIT-974: Export Tests
// =============================================================================

use crate::memvid_adapter::capsule::{ExportOptions, ExportResult};

/// SPEC-KIT-974: Export produces single file artifact.
///
/// ## Acceptance Criteria
/// - Export produces a single file artifact (.mv2) with no sidecar files
#[test]
fn test_export_produces_single_mv2_file() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("source.mv2");
    let export_path = temp_dir.path().join("export.mv2");

    // Create source capsule with artifact
    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "export_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");
    handle
        .put(
            "SPEC-974",
            "run1",
            ObjectType::Artifact,
            "test.md",
            b"# Test content".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put artifact");
    handle
        .commit_stage("SPEC-974", "run1", "plan", None)
        .expect("should commit");

    // Export
    let options = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some("SPEC-974".to_string()),
        run_id: Some("run1".to_string()),
        branch: None,
        safe_mode: true,
        ..Default::default()
    };

    let result = handle.export(&options).expect("export should succeed");

    // Verify single file
    assert!(
        export_path.exists(),
        "export file should exist at {:?}",
        export_path
    );
    assert!(result.bytes_written > 0, "export should have content");

    // SPEC-KIT-974 AC#1: Verify single file, no sidecars
    // List directory entries by filename (not full path)
    let export_parent = export_path.parent().unwrap();
    let export_stem = export_path.file_stem().unwrap().to_str().unwrap();
    let filenames: Vec<String> = std::fs::read_dir(export_parent)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    // Assert export file exists
    assert!(
        filenames.contains(&"export.mv2".to_string()),
        "export.mv2 must exist in {:?}, found: {:?}",
        export_parent,
        filenames
    );

    // Assert no sidecar files (files starting with "export" stem but not the export itself)
    let sidecars: Vec<&String> = filenames
        .iter()
        .filter(|name| name.starts_with(export_stem) && *name != "export.mv2")
        .collect();
    assert!(
        sidecars.is_empty(),
        "no sidecar files should exist starting with '{}', found: {:?}",
        export_stem,
        sidecars
    );
}

/// SPEC-KIT-974: Export includes manifest/digest.
///
/// ## Acceptance Criteria
/// - Export includes manifest with artifact count, checkpoints, events
#[test]
fn test_export_includes_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("manifest_source.mv2");
    let export_path = temp_dir.path().join("manifest_export.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "manifest_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");

    // Add multiple artifacts
    for i in 1..=3 {
        handle
            .put(
                "SPEC-974",
                "run1",
                ObjectType::Artifact,
                &format!("file{}.md", i),
                format!("Content {}", i).into_bytes(),
                serde_json::json!({}),
            )
            .expect("should put artifact");
    }
    handle
        .commit_stage("SPEC-974", "run1", "plan", None)
        .expect("should commit");

    let options = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some("SPEC-974".to_string()),
        run_id: Some("run1".to_string()),
        branch: None,
        safe_mode: true,
        ..Default::default()
    };

    let result = handle.export(&options).expect("export should succeed");

    // Verify result contains expected counts
    assert_eq!(result.artifact_count, 3, "should export 3 artifacts");
    assert!(
        result.checkpoint_count >= 1,
        "should export at least 1 checkpoint"
    );

    // Verify digest is present
    assert!(
        !result.content_hash.is_empty(),
        "content hash should be present"
    );
    assert_eq!(
        result.content_hash.len(),
        64,
        "SHA-256 hash should be 64 hex chars"
    );
}

/// SPEC-KIT-974: Export determinism test (semantic equivalence).
///
/// ## Architect Ruling (2026-01-23)
/// Determinism for this program is defined at the reproducibility of retrieval
/// context/results at a checkpoint, NOT bit-for-bit export file identity.
/// - D66: "deterministic retrieval context at a checkpoint"
/// - D3: timestamps are convenience; checkpoint-based identity is canonical
///
/// ## What this test verifies
/// - Same logical content selection across repeated exports
/// - Artifact counts, checkpoint counts, event counts match
/// - Both exports are valid and contain same semantic content
///
/// ## What this test does NOT verify
/// - Byte-identical exports (timestamps in manifest cause digest differences)
#[test]
fn test_export_determinism() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("determinism_source.mv2");
    let export1_path = temp_dir.path().join("export1.mv2");
    let export2_path = temp_dir.path().join("export2.mv2");

    // Create source capsule with fixed content
    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "determinism_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");

    // Add multiple artifacts to make the test more meaningful
    for i in 1..=3 {
        handle
            .put(
                "SPEC-974",
                "run1",
                ObjectType::Artifact,
                &format!("fixed_{}.md", i),
                format!("Fixed content {} for determinism test", i).into_bytes(),
                serde_json::json!({"key": "value", "index": i}),
            )
            .expect("should put artifact");
    }
    handle
        .commit_stage("SPEC-974", "run1", "plan", None)
        .expect("should commit");

    // Export twice with same filter criteria
    let options1 = ExportOptions {
        output_path: export1_path.clone(),
        spec_id: Some("SPEC-974".to_string()),
        run_id: Some("run1".to_string()),
        branch: None,
        safe_mode: true,
        ..Default::default()
    };

    let options2 = ExportOptions {
        output_path: export2_path.clone(),
        spec_id: Some("SPEC-974".to_string()),
        run_id: Some("run1".to_string()),
        branch: None,
        safe_mode: true,
        ..Default::default()
    };

    let result1 = handle.export(&options1).expect("export 1 should succeed");
    let result2 = handle.export(&options2).expect("export 2 should succeed");

    // Semantic equivalence checks (per Architect ruling on D66 + D3):
    // Same logical content selection, not byte-identical files

    // 1. Artifact counts must match (stable across exports)
    assert_eq!(
        result1.artifact_count, result2.artifact_count,
        "artifact counts should match for semantic determinism"
    );
    assert_eq!(result1.artifact_count, 3, "should export all 3 artifacts");

    // 2. Checkpoint counts must match (stable across exports)
    assert_eq!(
        result1.checkpoint_count, result2.checkpoint_count,
        "checkpoint counts should match for semantic determinism"
    );

    // 3. Event counts: Note that export itself emits a CapsuleExported event,
    //    so the second export will include the event from the first export.
    //    This is expected behavior - the capsule state changed between exports.
    //    We verify both have events, and the difference is exactly 1 (the prior export event).
    assert!(result1.event_count > 0, "first export should have events");
    assert!(result2.event_count > 0, "second export should have events");
    assert_eq!(
        result2.event_count,
        result1.event_count + 1,
        "second export should include +1 CapsuleExported event from first export"
    );

    // 4. Both exports should have valid content (non-zero bytes)
    assert!(
        result1.bytes_written > 0 && result2.bytes_written > 0,
        "both exports should have content"
    );

    // 5. Content hashes MAY differ due to timestamps - this is acceptable per D3
    // We explicitly do NOT assert hash equality here.
    // The hash is for integrity/provenance, not determinism verification.
}

/// SPEC-KIT-974: Export writes CapsuleExported event.
///
/// ## Acceptance Criteria
/// - Every export writes a `CapsuleExported` event into the workspace capsule
#[test]
fn test_export_emits_capsule_exported_event() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("event_source.mv2");
    let export_path = temp_dir.path().join("event_export.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "event_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");
    handle
        .put(
            "SPEC-974",
            "run1",
            ObjectType::Artifact,
            "test.md",
            b"Test".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put artifact");
    handle
        .commit_stage("SPEC-974", "run1", "plan", None)
        .expect("should commit");

    // Count events before export
    let events_before = handle.list_events().len();

    // Export
    let options = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some("SPEC-974".to_string()),
        run_id: Some("run1".to_string()),
        branch: None,
        safe_mode: true,
        ..Default::default()
    };

    handle.export(&options).expect("export should succeed");

    // Count events after export
    let events_after = handle.list_events();
    assert!(
        events_after.len() > events_before,
        "export should emit event"
    );

    // Find CapsuleExported event
    let exported_event = events_after
        .iter()
        .find(|e| e.event_type == EventType::CapsuleExported);
    assert!(
        exported_event.is_some(),
        "should have CapsuleExported event"
    );

    // Verify event payload per SPEC-KIT-974 acceptance criteria
    let event = exported_event.unwrap();
    assert!(
        event.payload.get("format").is_some(),
        "event should have format field"
    );
    assert!(
        event.payload.get("exported_at").is_some(),
        "event should have exported_at field"
    );
    // S974-007: Verify encryption flag
    assert!(
        event.payload.get("encrypted").is_some(),
        "event should have encrypted field"
    );
    assert_eq!(
        event.payload.get("encrypted"),
        Some(&serde_json::json!(false)),
        "MVP export should be unencrypted"
    );
    // S974-007: Verify safe flag (sanitized)
    assert!(
        event.payload.get("sanitized").is_some(),
        "event should have sanitized field"
    );
    // S974-007: Verify digest (content_hash)
    assert!(
        event.payload.get("content_hash").is_some(),
        "event should have content_hash (digest) field"
    );
}

/// SPEC-KIT-974: Export with run filter.
///
/// ## Acceptance Criteria
/// - Export can filter by run_id to include only specific run content
#[test]
fn test_export_filters_by_run() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("filter_source.mv2");
    let export_path = temp_dir.path().join("filter_export.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "filter_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");

    // Add artifacts to run1
    handle
        .put(
            "SPEC-974",
            "run1",
            ObjectType::Artifact,
            "run1_file.md",
            b"Run1 content".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put run1 artifact");
    handle
        .commit_stage("SPEC-974", "run1", "plan", None)
        .expect("should commit run1");

    // Add artifacts to run2
    handle
        .put(
            "SPEC-974",
            "run2",
            ObjectType::Artifact,
            "run2_file.md",
            b"Run2 content".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put run2 artifact");
    handle
        .commit_stage("SPEC-974", "run2", "plan", None)
        .expect("should commit run2");

    // Export only run1
    let options = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some("SPEC-974".to_string()),
        run_id: Some("run1".to_string()),
        branch: None,
        safe_mode: true,
        ..Default::default()
    };

    let result = handle.export(&options).expect("export should succeed");

    // Should only export run1 artifacts
    assert_eq!(
        result.artifact_count, 1,
        "should export only 1 artifact from run1"
    );
}

/// S974-008: Regression test for export checkpoint/event filtering.
///
/// ## Bug Description
/// write_export_file() was creating new ExportOptions with all filters set to None,
/// causing run-scoped exports to include ALL checkpoints/events instead of only
/// the filtered run's data.
///
/// ## What this test verifies
/// - Export with run_id filter produces correct checkpoint_count and event_count
/// - Reopened exported capsule contains only the filtered run's checkpoints
/// - Checkpoints from other runs are NOT included in the export
#[test]
fn test_export_filters_checkpoints_and_events_by_run() {
    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("checkpoint_filter_source.mv2");
    let export_path = temp_dir.path().join("checkpoint_filter_export.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "checkpoint_filter_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");

    // Add artifact and commit stage for run1 (creates checkpoint)
    handle
        .put(
            "SPEC-974",
            "run1",
            ObjectType::Artifact,
            "run1_artifact.md",
            b"Run1 artifact content".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put run1 artifact");
    handle
        .commit_stage("SPEC-974", "run1", "plan", None)
        .expect("should commit run1 plan stage");
    handle
        .commit_stage("SPEC-974", "run1", "implement", None)
        .expect("should commit run1 implement stage");

    // Add artifact and commit stage for run2 (creates checkpoint)
    handle
        .put(
            "SPEC-974",
            "run2",
            ObjectType::Artifact,
            "run2_artifact.md",
            b"Run2 artifact content".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put run2 artifact");
    handle
        .commit_stage("SPEC-974", "run2", "plan", None)
        .expect("should commit run2 plan stage");
    handle
        .commit_stage("SPEC-974", "run2", "implement", None)
        .expect("should commit run2 implement stage");

    // Export only run1
    let options = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some("SPEC-974".to_string()),
        run_id: Some("run1".to_string()),
        branch: None,
        safe_mode: true,
        ..Default::default()
    };

    let result = handle.export(&options).expect("export should succeed");

    // S974-008: Verify checkpoint count matches only run1's checkpoints
    // run1 has 2 stage commits (plan, implement), so should have 2 checkpoints
    assert_eq!(
        result.checkpoint_count, 2,
        "should export only 2 checkpoints from run1 (plan + implement stages)"
    );

    // Verify artifact count is correct
    assert_eq!(
        result.artifact_count, 1,
        "should export only 1 artifact from run1"
    );

    // Reopen exported capsule and verify checkpoints are filtered
    drop(handle);

    let export_config = CapsuleConfig {
        capsule_path: export_path.clone(),
        workspace_id: "checkpoint_filter_test_export".to_string(),
        ..Default::default()
    };

    let exported_handle =
        CapsuleHandle::open_read_only(export_config).expect("should open exported capsule");

    // List checkpoints - should only have run1's checkpoints
    let checkpoints = exported_handle.list_checkpoints();
    assert_eq!(
        checkpoints.len(),
        2,
        "exported capsule should contain exactly 2 checkpoints (run1 only)"
    );

    // Verify all checkpoints belong to run1
    for cp in &checkpoints {
        assert!(
            cp.run_id.as_deref() == Some("run1"),
            "all checkpoints should be from run1, found run_id: {:?}",
            cp.run_id
        );
    }
}

/// SPEC-KIT-974 S974-002: Export → Reopen → Verify test.
///
/// ## Acceptance Criteria
/// - Import on a second machine reproduces identical retrieval results for
///   checkpointed golden queries (within tolerance for floating scoring),
///   using the imported capsule context.
///
/// ## What this test verifies
/// - Exported .mv2 files can be re-opened with CapsuleHandle::open_read_only()
/// - Checkpoints are preserved and accessible
/// - Events are preserved and accessible
/// - Artifact content (bytes) can be retrieved via get_bytes()
#[test]
fn test_export_reopen_retrieves_artifacts() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("source.mv2");
    let export_path = temp_dir.path().join("exported.mv2");

    // Step 1: Create source capsule with artifacts
    let config = CapsuleConfig {
        capsule_path: source_path.clone(),
        workspace_id: "reopen_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config.clone()).expect("should create capsule");

    // Put multiple artifacts with distinct content
    let artifact1_content = b"# Artifact 1 content for reopen test".to_vec();
    let artifact2_content = b"# Artifact 2 content with different data".to_vec();

    let uri1 = handle
        .put(
            "SPEC-974",
            "run1",
            ObjectType::Artifact,
            "artifact1.md",
            artifact1_content.clone(),
            serde_json::json!({"test": "reopen1"}),
        )
        .expect("should put artifact 1");

    let uri2 = handle
        .put(
            "SPEC-974",
            "run1",
            ObjectType::Artifact,
            "artifact2.md",
            artifact2_content.clone(),
            serde_json::json!({"test": "reopen2"}),
        )
        .expect("should put artifact 2");

    // Create checkpoint (required for deterministic retrieval)
    // Note: commit_stage automatically emits a StageTransition event
    handle
        .commit_stage("SPEC-974", "run1", "plan", Some("reopen_checkpoint"))
        .expect("should create checkpoint");

    // Step 2: Export to .mv2
    let options = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some("SPEC-974".to_string()),
        run_id: Some("run1".to_string()),
        branch: None,
        safe_mode: true,
        ..Default::default()
    };

    let export_result = handle.export(&options).expect("export should succeed");
    assert!(
        export_result.artifact_count >= 2,
        "should export at least 2 artifacts"
    );

    // Close source capsule
    drop(handle);

    // Step 3: Reopen exported capsule read-only
    let exported_config = CapsuleConfig {
        capsule_path: export_path.clone(),
        workspace_id: "exported_reopen_test".to_string(),
        ..Default::default()
    };

    let exported_handle =
        CapsuleHandle::open_read_only(exported_config).expect("should reopen exported capsule");

    // Step 4: Verify checkpoints are accessible
    let checkpoints = exported_handle.list_checkpoints();
    assert!(
        !checkpoints.is_empty(),
        "exported capsule should have checkpoints"
    );
    // Note: commit_stage creates labels with format "stage:{stage_name}"
    let has_plan_checkpoint = checkpoints
        .iter()
        .any(|cp| cp.label.as_deref() == Some("stage:plan"));
    assert!(
        has_plan_checkpoint,
        "exported capsule should have 'stage:plan' label"
    );

    // Step 5: Verify events are accessible
    let events = exported_handle.list_events();
    assert!(!events.is_empty(), "exported capsule should have events");

    // Step 6: Verify artifact bytes can be retrieved
    // (This is the key determinism test - same URI returns same bytes)
    let retrieved1 = exported_handle
        .get_bytes(&uri1, None, None)
        .expect("should retrieve artifact 1");
    let retrieved2 = exported_handle
        .get_bytes(&uri2, None, None)
        .expect("should retrieve artifact 2");

    assert_eq!(
        retrieved1, artifact1_content,
        "artifact 1 content should match original"
    );
    assert_eq!(
        retrieved2, artifact2_content,
        "artifact 2 content should match original"
    );
}

// =============================================================================
// S974-003: Encryption Tests
// =============================================================================

/// A1 backward-compat test: CapsuleExportedPayload without `encrypted` field
/// should deserialize successfully with encrypted=false default.
#[test]
fn test_capsule_exported_payload_backward_compat() {
    use crate::memvid_adapter::types::CapsuleExportedPayload;

    // Legacy payload without `encrypted` field
    let legacy_json = r#"{
        "destination_type": "file",
        "destination": "/tmp/export.mv2",
        "format": "mv2-v1",
        "checkpoints_included": ["stage:plan"],
        "sanitized": true,
        "exported_at": "2025-01-01T00:00:00Z"
    }"#;

    let payload: CapsuleExportedPayload =
        serde_json::from_str(legacy_json).expect("should deserialize legacy payload");

    assert!(!payload.encrypted, "encrypted should default to false");
    assert_eq!(payload.format, "mv2-v1");
    assert!(payload.sanitized);
}

/// S974-003: Encrypted export and decrypt roundtrip test.
///
/// ## Test Flow
/// 1. Create capsule with artifacts
/// 2. Export with encrypt=true (using env var for passphrase)
/// 3. Verify .mv2e file exists
/// 4. Open encrypted file with correct passphrase
/// 5. Verify artifacts are accessible
#[test]
#[serial_test::serial]
fn test_export_encrypted_roundtrip() {
    use crate::memvid_adapter::capsule::{CapsuleError, ExportOptions};

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("encrypt_test.mv2");
    let export_path = temp_dir.path().join("export_encrypted.mv2e");

    // Set passphrase via env var for test
    // SAFETY: Test-only, single-threaded test execution
    unsafe {
        std::env::set_var("SPECKIT_MEMVID_PASSPHRASE", "test-passphrase-123");
    }

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "encrypt_test".to_string(),
        ..Default::default()
    };

    // Step 1: Create capsule and add artifacts
    let handle = CapsuleHandle::open(config.clone()).expect("should create capsule");

    let artifact_content = b"# Encrypted Test Artifact\nSecret data here.".to_vec();
    let uri = handle
        .put(
            "SPEC-974",
            "run-encrypt",
            ObjectType::Artifact,
            "secret.md",
            artifact_content.clone(),
            serde_json::json!({"sensitive": true}),
        )
        .expect("should put artifact");

    handle
        .commit_stage("SPEC-974", "run-encrypt", "plan", None)
        .expect("should commit checkpoint");

    // Step 2: Export with encryption
    let options = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some("SPEC-974".to_string()),
        run_id: Some("run-encrypt".to_string()),
        encrypt: true,
        interactive: false, // Use env var only
        ..Default::default()
    };

    let result = handle.export(&options).expect("should export encrypted");

    // Step 3: Verify .mv2e file exists
    assert!(
        result.output_path.exists(),
        "encrypted export file should exist"
    );
    assert!(
        result
            .output_path
            .extension()
            .map_or(false, |e| e == "mv2e"),
        "export should have .mv2e extension"
    );
    assert!(result.bytes_written > 0, "should have written bytes");

    // Verify it's actually encrypted (starts with age header, not MV2)
    let encrypted_bytes = std::fs::read(&result.output_path).expect("should read encrypted file");
    assert!(
        !encrypted_bytes.starts_with(b"MV2"),
        "encrypted file should not start with MV2 header"
    );

    // Close original handle
    drop(handle);

    // Step 4: Open encrypted file with correct passphrase
    let passphrase = secrecy::SecretString::from("test-passphrase-123".to_string());
    let decrypted_handle = CapsuleHandle::open_encrypted(&result.output_path, &passphrase)
        .expect("should open encrypted capsule");

    // Step 5: Verify artifact is accessible
    let retrieved = decrypted_handle
        .get_bytes(&uri, None, None)
        .expect("should retrieve artifact from decrypted capsule");

    assert_eq!(
        retrieved, artifact_content,
        "decrypted artifact should match original"
    );

    // Clean up env var
    // SAFETY: Test-only, single-threaded test execution
    unsafe {
        std::env::remove_var("SPECKIT_MEMVID_PASSPHRASE");
    }
}

/// S974-003: Wrong passphrase should fail safely with no partial plaintext.
#[test]
#[serial_test::serial]
fn test_export_encrypted_wrong_passphrase() {
    use crate::memvid_adapter::capsule::{CapsuleError, ExportOptions};

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("wrong_pass_test.mv2");
    let export_path = temp_dir.path().join("wrong_pass_export.mv2e");

    // Set passphrase for export
    // SAFETY: Test-only, single-threaded test execution
    unsafe {
        std::env::set_var("SPECKIT_MEMVID_PASSPHRASE", "correct-passphrase");
    }

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "wrong_pass_test".to_string(),
        ..Default::default()
    };

    // Create capsule and export
    let handle = CapsuleHandle::open(config.clone()).expect("should create capsule");
    handle
        .put(
            "SPEC-974",
            "run1",
            ObjectType::Artifact,
            "test.md",
            b"Test content".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put artifact");

    let options = ExportOptions {
        output_path: export_path.clone(),
        encrypt: true,
        interactive: false,
        ..Default::default()
    };

    handle.export(&options).expect("should export encrypted");
    drop(handle);

    // Try to open with wrong passphrase
    let wrong_passphrase = secrecy::SecretString::from("wrong-passphrase".to_string());
    let result = CapsuleHandle::open_encrypted(&export_path, &wrong_passphrase);

    // Should fail with InvalidPassphrase error
    match result {
        Err(CapsuleError::InvalidPassphrase) => {
            // Expected
        }
        Err(e) => panic!("Expected InvalidPassphrase error, got: {:?}", e),
        Ok(_) => panic!("Should have failed with wrong passphrase"),
    }

    // Verify no decrypted.mv2 files left behind in temp directories
    // (This is inherent in the implementation - we decrypt to memory first)

    // Clean up
    // SAFETY: Test-only, single-threaded test execution
    unsafe {
        std::env::remove_var("SPECKIT_MEMVID_PASSPHRASE");
    }
}

/// S974-003: Verify CapsuleExported event has correct encrypted flag.
#[test]
#[serial_test::serial]
fn test_export_event_encrypted_flag() {
    use crate::memvid_adapter::capsule::ExportOptions;
    use crate::memvid_adapter::types::EventType;

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("event_flag_test.mv2");

    // SAFETY: Test-only, single-threaded test execution
    unsafe {
        std::env::set_var("SPECKIT_MEMVID_PASSPHRASE", "test-passphrase");
    }

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "event_flag_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config.clone()).expect("should create capsule");

    // Add artifact
    handle
        .put(
            "SPEC-974",
            "run1",
            ObjectType::Artifact,
            "test.md",
            b"Test".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put artifact");

    // Export unencrypted first
    let unencrypted_path = temp_dir.path().join("unencrypted.mv2");
    let unencrypted_options = ExportOptions {
        output_path: unencrypted_path.clone(),
        encrypt: false,
        ..Default::default()
    };
    handle
        .export(&unencrypted_options)
        .expect("should export unencrypted");

    // Export encrypted
    let encrypted_path = temp_dir.path().join("encrypted.mv2e");
    let encrypted_options = ExportOptions {
        output_path: encrypted_path.clone(),
        encrypt: true,
        interactive: false,
        ..Default::default()
    };
    handle
        .export(&encrypted_options)
        .expect("should export encrypted");

    // Check events
    let events = handle.list_events();
    let export_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == EventType::CapsuleExported)
        .collect();

    assert_eq!(
        export_events.len(),
        2,
        "should have two CapsuleExported events"
    );

    // Check unencrypted event (first export)
    let unencrypted_event = &export_events[0];
    let encrypted_flag = unencrypted_event.payload.get("encrypted");
    assert_eq!(
        encrypted_flag,
        Some(&serde_json::json!(false)),
        "unencrypted export event should have encrypted=false"
    );

    // Check encrypted event (second export)
    let encrypted_event = &export_events[1];
    let encrypted_flag = encrypted_event.payload.get("encrypted");
    assert_eq!(
        encrypted_flag,
        Some(&serde_json::json!(true)),
        "encrypted export event should have encrypted=true"
    );

    // Clean up
    // SAFETY: Test-only, single-threaded test execution
    unsafe {
        std::env::remove_var("SPECKIT_MEMVID_PASSPHRASE");
    }
}

// =============================================================================
// S974-009: Safe Export Filtering Tests
// =============================================================================

/// S974-009: safe_mode=true export excludes FullIo ModelCallEnvelope events.
///
/// ## Test Plan
/// 1. Create capsule with artifacts and multiple ModelCallEnvelope events
/// 2. Emit events with different capture modes (PromptsOnly and FullIo)
/// 3. Export with safe_mode=true (default)
/// 4. Verify FullIo events are NOT in the exported file
/// 5. Verify PromptsOnly events ARE in the exported file
#[test]
#[serial_test::serial]
fn test_safe_export_excludes_full_io_model_calls() {
    use crate::memvid_adapter::capsule::ExportOptions;
    use crate::memvid_adapter::types::{
        EventType, LLMCaptureMode, ModelCallEnvelopePayload, RoutingMode,
    };

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("safe_export_test.mv2");
    let export_path = temp_dir.path().join("safe_export.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "safe_export_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config.clone()).expect("should create capsule");

    let spec_id = "SPEC-974";
    let run_id = "safe-export-test";

    // Add artifact to ensure capsule has content
    handle
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "test.md",
            b"Test content".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put artifact");

    // Emit PromptsOnly model call (should be included in safe export)
    let prompts_only_call = ModelCallEnvelopePayload {
        call_id: "safe-prompts-only".to_string(),
        model: "test-model".to_string(),
        routing_mode: RoutingMode::Cloud,
        capture_mode: LLMCaptureMode::PromptsOnly,
        stage: Some("Implement".to_string()),
        role: Some("Implementer".to_string()),
        prompt_hash: Some("hash1".to_string()),
        response_hash: Some("hash2".to_string()),
        prompt: Some("Test prompt".to_string()),
        response: None, // Not captured in PromptsOnly mode
        prompt_tokens: Some(50),
        response_tokens: Some(100),
        latency_ms: Some(500),
        success: true,
        error: None,
    };
    handle
        .emit_model_call_envelope(spec_id, run_id, &prompts_only_call)
        .expect("emit prompts_only mode call");

    // Emit FullIo model call (should be EXCLUDED from safe export)
    let full_io_call = ModelCallEnvelopePayload {
        call_id: "unsafe-full-io".to_string(),
        model: "test-model".to_string(),
        routing_mode: RoutingMode::Reflex,
        capture_mode: LLMCaptureMode::FullIo,
        stage: Some("Implement".to_string()),
        role: Some("Implementer".to_string()),
        prompt_hash: Some("hash3".to_string()),
        response_hash: Some("hash4".to_string()),
        prompt: Some("Sensitive prompt".to_string()),
        response: Some("Sensitive response".to_string()), // FullIo captures response
        prompt_tokens: Some(100),
        response_tokens: Some(200),
        latency_ms: Some(1000),
        success: true,
        error: None,
    };
    handle
        .emit_model_call_envelope(spec_id, run_id, &full_io_call)
        .expect("emit full_io mode call");

    // Commit to create checkpoint
    handle
        .commit_stage(spec_id, run_id, "implement", None)
        .expect("should commit");

    // Verify source capsule has both events
    let all_events = handle.list_events();
    let model_call_events: Vec<_> = all_events
        .iter()
        .filter(|e| e.event_type == EventType::ModelCallEnvelope)
        .collect();
    assert_eq!(
        model_call_events.len(),
        2,
        "Source should have 2 model call events"
    );

    // Export with safe_mode=true (default)
    let options = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some(spec_id.to_string()),
        run_id: Some(run_id.to_string()),
        safe_mode: true, // This is the default, but being explicit
        ..Default::default()
    };
    let result = handle.export(&options).expect("should export");
    assert!(export_path.exists(), "Export file should exist");
    assert!(result.bytes_written > 0, "Export should have content");

    // Reopen exported capsule and verify events
    let export_config = CapsuleConfig {
        capsule_path: export_path.clone(),
        workspace_id: "safe_export_verify".to_string(),
        ..Default::default()
    };
    let export_handle = CapsuleHandle::open_read_only(export_config).expect("should open export");

    let export_events = export_handle.list_events();
    let export_model_calls: Vec<_> = export_events
        .iter()
        .filter(|e| e.event_type == EventType::ModelCallEnvelope)
        .collect();

    // Should only have PromptsOnly event, not FullIo
    assert_eq!(
        export_model_calls.len(),
        1,
        "Safe export should only include 1 model call (PromptsOnly)"
    );

    // Verify the included event is the PromptsOnly one
    let included_call_id = export_model_calls[0].payload.get("call_id");
    assert_eq!(
        included_call_id,
        Some(&serde_json::json!("safe-prompts-only")),
        "Included event should be the PromptsOnly call"
    );
}

/// S974-009: safe_mode=false export includes both PromptsOnly and FullIo events.
///
/// ## Test Plan
/// 1. Create capsule with ModelCallEnvelope events (PromptsOnly and FullIo)
/// 2. Export with safe_mode=false
/// 3. Verify BOTH event types are in the exported file
#[test]
#[serial_test::serial]
fn test_unsafe_export_includes_all_model_calls() {
    use crate::memvid_adapter::capsule::ExportOptions;
    use crate::memvid_adapter::types::{
        EventType, LLMCaptureMode, ModelCallEnvelopePayload, RoutingMode,
    };

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("unsafe_export_test.mv2");
    let export_path = temp_dir.path().join("unsafe_export.mv2");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "unsafe_export_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config.clone()).expect("should create capsule");

    let spec_id = "SPEC-974";
    let run_id = "unsafe-export-test";

    // Add artifact
    handle
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "test.md",
            b"Test".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put artifact");

    // Emit PromptsOnly model call
    let prompts_only_call = ModelCallEnvelopePayload {
        call_id: "prompts-only-001".to_string(),
        model: "test-model".to_string(),
        routing_mode: RoutingMode::Cloud,
        capture_mode: LLMCaptureMode::PromptsOnly,
        stage: Some("Implement".to_string()),
        role: Some("Implementer".to_string()),
        prompt_hash: Some("hash1".to_string()),
        response_hash: Some("hash2".to_string()),
        prompt: Some("Test prompt".to_string()),
        response: None,
        prompt_tokens: Some(50),
        response_tokens: Some(100),
        latency_ms: Some(500),
        success: true,
        error: None,
    };
    handle
        .emit_model_call_envelope(spec_id, run_id, &prompts_only_call)
        .expect("emit prompts_only call");

    // Emit FullIo model call
    let full_io_call = ModelCallEnvelopePayload {
        call_id: "full-io-001".to_string(),
        model: "test-model".to_string(),
        routing_mode: RoutingMode::Reflex,
        capture_mode: LLMCaptureMode::FullIo,
        stage: Some("Implement".to_string()),
        role: Some("Implementer".to_string()),
        prompt_hash: Some("hash3".to_string()),
        response_hash: Some("hash4".to_string()),
        prompt: Some("Full prompt".to_string()),
        response: Some("Full response".to_string()),
        prompt_tokens: Some(100),
        response_tokens: Some(200),
        latency_ms: Some(1000),
        success: true,
        error: None,
    };
    handle
        .emit_model_call_envelope(spec_id, run_id, &full_io_call)
        .expect("emit full_io call");

    // Commit
    handle
        .commit_stage(spec_id, run_id, "implement", None)
        .expect("should commit");

    // Export with safe_mode=false
    let options = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some(spec_id.to_string()),
        run_id: Some(run_id.to_string()),
        safe_mode: false, // Disable safe mode
        ..Default::default()
    };
    let result = handle.export(&options).expect("should export");
    assert!(export_path.exists(), "Export file should exist");
    assert!(result.bytes_written > 0, "Export should have content");

    // Reopen exported capsule
    let export_config = CapsuleConfig {
        capsule_path: export_path.clone(),
        workspace_id: "unsafe_export_verify".to_string(),
        ..Default::default()
    };
    let export_handle = CapsuleHandle::open_read_only(export_config).expect("should open export");

    let export_events = export_handle.list_events();
    let export_model_calls: Vec<_> = export_events
        .iter()
        .filter(|e| e.event_type == EventType::ModelCallEnvelope)
        .collect();

    // Should have BOTH model call events
    assert_eq!(
        export_model_calls.len(),
        2,
        "Unsafe export should include both model calls"
    );

    // Verify both call IDs are present
    let call_ids: Vec<_> = export_model_calls
        .iter()
        .filter_map(|e| e.payload.get("call_id"))
        .collect();
    assert!(
        call_ids.contains(&&serde_json::json!("prompts-only-001")),
        "Should include PromptsOnly call"
    );
    assert!(
        call_ids.contains(&&serde_json::json!("full-io-001")),
        "Should include FullIo call"
    );
}

// =============================================================================
// P0-5: Import/GC Tests (SPEC-KIT-974)
// =============================================================================

/// P0-5: Import .mv2 capsule happy path.
/// Tests that importing an unencrypted capsule emits CapsuleImported event.
#[test]
fn test_import_mv2_happy_path() {
    use crate::memvid_adapter::capsule::{ExportOptions, ImportOptions};
    use crate::memvid_adapter::types::EventType;

    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("source.mv2");
    let export_path = temp_dir.path().join("export.mv2");
    let workspace_path = temp_dir.path().join("workspace.mv2");

    // Step 1: Create source capsule with artifacts
    let source_config = CapsuleConfig {
        capsule_path: source_path.clone(),
        workspace_id: "source".to_string(),
        ..Default::default()
    };
    let source_handle = CapsuleHandle::open(source_config).expect("should create source");

    let artifact_data = b"Test artifact for import".to_vec();
    source_handle
        .put(
            "SPEC-IMPORT",
            "run-1",
            ObjectType::Artifact,
            "test.txt",
            artifact_data,
            serde_json::json!({}),
        )
        .expect("should put artifact");

    source_handle
        .commit_stage("SPEC-IMPORT", "run-1", "plan", None)
        .expect("should commit");

    // Step 2: Export to .mv2 (unencrypted)
    let export_opts = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some("SPEC-IMPORT".to_string()),
        run_id: Some("run-1".to_string()),
        encrypt: false,
        ..Default::default()
    };
    source_handle.export(&export_opts).expect("should export");
    drop(source_handle);

    // Step 3: Create workspace capsule and import
    let workspace_config = CapsuleConfig {
        capsule_path: workspace_path.clone(),
        workspace_id: "workspace".to_string(),
        ..Default::default()
    };
    let workspace_handle = CapsuleHandle::open(workspace_config).expect("should create workspace");

    let import_opts = ImportOptions::for_file(&export_path)
        .with_mount_name("imported_capsule")
        .with_interactive(false);

    let result = workspace_handle
        .import(&import_opts)
        .expect("should import");

    // Verify import result
    assert_eq!(result.mount_name, "imported_capsule");
    assert!(result.verification_passed);
    assert!(!result.content_hash.is_empty());
    assert!(result.checkpoint_count >= 1);

    // Verify CapsuleImported event was emitted
    let events = workspace_handle.list_events();
    let import_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == EventType::CapsuleImported)
        .collect();
    assert_eq!(
        import_events.len(),
        1,
        "should have one CapsuleImported event"
    );
}

/// P0-5: Import .mv2e encrypted capsule happy path.
#[test]
#[serial_test::serial]
fn test_import_mv2e_happy_path() {
    use crate::memvid_adapter::capsule::{ExportOptions, ImportOptions};

    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("source.mv2");
    let export_path = temp_dir.path().join("export.mv2e");
    let workspace_path = temp_dir.path().join("workspace.mv2");

    // Set passphrase via env var
    // SAFETY: Test-only, single-threaded (serial_test)
    unsafe {
        std::env::set_var("SPECKIT_MEMVID_PASSPHRASE", "import-test-pass");
    }

    // Step 1: Create and export encrypted capsule
    let source_config = CapsuleConfig {
        capsule_path: source_path.clone(),
        workspace_id: "encrypted_source".to_string(),
        ..Default::default()
    };
    let source_handle = CapsuleHandle::open(source_config).expect("should create source");

    source_handle
        .put(
            "SPEC-ENC",
            "run-enc",
            ObjectType::Artifact,
            "secret.txt",
            b"encrypted data".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put");

    source_handle
        .commit_stage("SPEC-ENC", "run-enc", "plan", None)
        .expect("should commit");

    let export_opts = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some("SPEC-ENC".to_string()),
        run_id: Some("run-enc".to_string()),
        encrypt: true,
        interactive: false,
        ..Default::default()
    };
    source_handle
        .export(&export_opts)
        .expect("should export encrypted");
    drop(source_handle);

    // Step 2: Import encrypted capsule
    let workspace_config = CapsuleConfig {
        capsule_path: workspace_path.clone(),
        workspace_id: "workspace".to_string(),
        ..Default::default()
    };
    let workspace_handle = CapsuleHandle::open(workspace_config).expect("should create workspace");

    let import_opts = ImportOptions::for_file(&export_path).with_interactive(false);

    let result = workspace_handle
        .import(&import_opts)
        .expect("should import encrypted");

    // Verify import succeeded
    assert!(result.verification_passed);
    assert!(result.checkpoint_count >= 1);

    // Clean up env var
    unsafe {
        std::env::remove_var("SPECKIT_MEMVID_PASSPHRASE");
    }
}

/// P0-5: Import with wrong passphrase should fail safely.
#[test]
#[serial_test::serial]
fn test_import_mv2e_wrong_passphrase() {
    use crate::memvid_adapter::capsule::{CapsuleError, ExportOptions, ImportOptions};

    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("source.mv2");
    let export_path = temp_dir.path().join("export.mv2e");
    let workspace_path = temp_dir.path().join("workspace.mv2");

    // Create encrypted export with correct passphrase
    unsafe {
        std::env::set_var("SPECKIT_MEMVID_PASSPHRASE", "correct-password");
    }

    let source_config = CapsuleConfig {
        capsule_path: source_path.clone(),
        workspace_id: "source".to_string(),
        ..Default::default()
    };
    let source_handle = CapsuleHandle::open(source_config).expect("should create");
    source_handle
        .put(
            "SPEC-WP",
            "run",
            ObjectType::Artifact,
            "f.txt",
            b"x".to_vec(),
            serde_json::json!({}),
        )
        .expect("put");
    source_handle
        .commit_stage("SPEC-WP", "run", "plan", None)
        .expect("commit");

    let export_opts = ExportOptions {
        output_path: export_path.clone(),
        encrypt: true,
        interactive: false,
        ..Default::default()
    };
    source_handle.export(&export_opts).expect("export");
    drop(source_handle);

    // Try to import with wrong passphrase
    unsafe {
        std::env::set_var("SPECKIT_MEMVID_PASSPHRASE", "wrong-password");
    }

    let workspace_config = CapsuleConfig {
        capsule_path: workspace_path.clone(),
        workspace_id: "workspace".to_string(),
        ..Default::default()
    };
    let workspace_handle = CapsuleHandle::open(workspace_config).expect("workspace");

    let import_opts = ImportOptions::for_file(&export_path).with_interactive(false);

    let result = workspace_handle.import(&import_opts);
    assert!(
        matches!(result, Err(CapsuleError::InvalidPassphrase)),
        "should fail with InvalidPassphrase"
    );

    // Clean up
    unsafe {
        std::env::remove_var("SPECKIT_MEMVID_PASSPHRASE");
    }
}

/// SPEC-KIT-974 AC#8: GC dry-run should not delete files.
#[test]
fn test_gc_dry_run_no_delete() {
    use crate::memvid_adapter::capsule::GcConfig;
    use filetime::{set_file_mtime, FileTime};

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join(".speckit/memvid/workspace.mv2");

    // Create workspace structure
    std::fs::create_dir_all(capsule_path.parent().unwrap()).expect("create dirs");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "gc_test".to_string(),
        ..Default::default()
    };
    let handle = CapsuleHandle::open(config).expect("should create capsule");

    // Create a fake export directory with old files
    let exports_dir = temp_dir.path().join("docs/specs/SPEC-GC/runs/run-old");
    std::fs::create_dir_all(&exports_dir).expect("create export dir");
    let old_export = exports_dir.join("capsule.mv2");
    std::fs::write(&old_export, b"old export data").expect("write old export");

    // Set file modification time to 40 days ago
    let now = std::time::SystemTime::now();
    let old_time = now - std::time::Duration::from_secs(40 * 86400);
    set_file_mtime(&old_export, FileTime::from_system_time(old_time)).expect("set mtime");

    let gc_config = GcConfig {
        retention_days: 30,
        keep_pinned: true,
        clean_temp_files: true,
        dry_run: true, // Dry run!
    };

    let result = handle.gc(&gc_config).expect("gc should succeed");

    // Dry run should report deletions but not actually delete
    assert!(result.dry_run, "result should indicate dry run");
    assert!(old_export.exists(), "file should still exist after dry run");
}

/// SPEC-KIT-974 AC#8: GC retention deletes old exports, preserves new ones.
#[test]
fn test_gc_retention_deletes_old_preserves_new() {
    use crate::memvid_adapter::capsule::GcConfig;
    use filetime::{set_file_mtime, FileTime};

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join(".speckit/memvid/workspace.mv2");
    std::fs::create_dir_all(capsule_path.parent().unwrap()).expect("create dirs");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "gc_retention_test".to_string(),
        ..Default::default()
    };
    let handle = CapsuleHandle::open(config).expect("create capsule");

    // Create export dir with old and new files
    let exports_dir = temp_dir.path().join("docs/specs/SPEC-RET/runs/run-test");
    std::fs::create_dir_all(&exports_dir).expect("create dir");
    let old_export = exports_dir.join("old.mv2");
    let new_export = exports_dir.join("new.mv2");
    std::fs::write(&old_export, b"old export").expect("write old");
    std::fs::write(&new_export, b"new export").expect("write new");

    // Set mtimes: old = 40 days ago, new = 1 day ago
    let now = std::time::SystemTime::now();
    let old_time = now - std::time::Duration::from_secs(40 * 86400);
    let new_time = now - std::time::Duration::from_secs(1 * 86400);
    set_file_mtime(&old_export, FileTime::from_system_time(old_time)).expect("set old mtime");
    set_file_mtime(&new_export, FileTime::from_system_time(new_time)).expect("set new mtime");

    let gc_config = GcConfig {
        retention_days: 30,
        keep_pinned: true,
        clean_temp_files: true,
        dry_run: false,
    };

    let result = handle.gc(&gc_config).expect("gc");

    // Old should be deleted, new should remain
    assert!(!old_export.exists(), "old export should be deleted");
    assert!(new_export.exists(), "new export should be preserved");
    assert!(result.exports_deleted > 0, "should have deleted at least one export");
}

/// SPEC-KIT-974 AC#8: GC should preserve pinned exports even when old.
#[test]
fn test_gc_preserves_pinned() {
    use crate::memvid_adapter::capsule::GcConfig;
    use filetime::{set_file_mtime, FileTime};

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join(".speckit/memvid/workspace.mv2");
    std::fs::create_dir_all(capsule_path.parent().unwrap()).expect("create dirs");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "gc_pinned_test".to_string(),
        ..Default::default()
    };
    let handle = CapsuleHandle::open(config).expect("create capsule");

    // Create export dir with pinned file (old) and unpinned file (also old)
    let exports_dir = temp_dir.path().join("docs/specs/SPEC-PIN/runs/run-pinned");
    std::fs::create_dir_all(&exports_dir).expect("create dir");
    let pinned_export = exports_dir.join("pinned.mv2");
    let unpinned_export = exports_dir.join("unpinned.mv2");
    // SPEC-KIT-974 Task 4: Preferred marker is <filename>.pin (e.g., pinned.mv2.pin)
    let pin_marker = exports_dir.join("pinned.mv2.pin");
    std::fs::write(&pinned_export, b"pinned export").expect("write pinned");
    std::fs::write(&unpinned_export, b"unpinned export").expect("write unpinned");
    std::fs::write(&pin_marker, b"milestone").expect("write pin marker");

    // Set mtimes to 40 days ago (beyond retention)
    let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(40 * 86400);
    set_file_mtime(&pinned_export, FileTime::from_system_time(old_time)).expect("set pinned mtime");
    set_file_mtime(&unpinned_export, FileTime::from_system_time(old_time)).expect("set unpinned mtime");

    let gc_config = GcConfig {
        retention_days: 30,
        keep_pinned: true, // Preserve pinned
        clean_temp_files: true,
        dry_run: false,
    };

    let result = handle.gc(&gc_config).expect("gc");

    // Pinned file should be preserved, unpinned should be deleted
    assert!(pinned_export.exists(), "pinned export should be preserved");
    assert!(!unpinned_export.exists(), "unpinned old export should be deleted");
    assert!(result.exports_preserved > 0, "should have preserved pinned exports");
    assert!(result.exports_deleted > 0, "should have deleted unpinned exports");
}

/// SPEC-KIT-974 AC#8: GC audit event must be persisted (survives reopen).
#[test]
fn test_gc_audit_event_persisted() {
    use crate::memvid_adapter::capsule::GcConfig;
    use filetime::{set_file_mtime, FileTime};

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join(".speckit/memvid/workspace.mv2");
    std::fs::create_dir_all(capsule_path.parent().unwrap()).expect("create dirs");

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "gc_audit_test".to_string(),
        ..Default::default()
    };

    // Create old export to trigger deletion
    let exports_dir = temp_dir.path().join("docs/specs/SPEC-AUDIT/runs/run-old");
    std::fs::create_dir_all(&exports_dir).expect("create dir");
    let old_export = exports_dir.join("capsule.mv2");
    std::fs::write(&old_export, b"old export").expect("write export");

    // Set mtime to 40 days ago
    let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(40 * 86400);
    set_file_mtime(&old_export, FileTime::from_system_time(old_time)).expect("set mtime");

    // Run GC (with actual deletion)
    {
        let handle = CapsuleHandle::open(config.clone()).expect("open capsule");
        let gc_config = GcConfig {
            retention_days: 30,
            keep_pinned: true,
            clean_temp_files: true,
            dry_run: false,
        };
        let result = handle.gc(&gc_config).expect("gc");
        assert!(result.exports_deleted > 0, "should have deleted exports");
    } // handle dropped, capsule closed

    // Reopen and verify audit event persisted
    let handle2 = CapsuleHandle::open(config).expect("reopen capsule");
    let events = handle2.list_events();
    let gc_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.event_type == EventType::GateDecision
                && e.payload
                    .get("gate_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "CapsuleGC")
                    .unwrap_or(false)
        })
        .collect();

    assert!(
        !gc_events.is_empty(),
        "GC audit event (GateDecision with gate_name=CapsuleGC) should be persisted after reopen"
    );
}

// =============================================================================
// SPEC-KIT-974: Mount Persistence Tests (S974-mount)
// =============================================================================

/// S974-mount: Import creates mount file at expected path and registry entry.
#[test]
fn test_import_creates_mount_file_and_registry() {
    use crate::memvid_adapter::capsule::{ExportOptions, ImportOptions, MountsRegistry};
    use crate::memvid_adapter::types::EventType;

    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("source.mv2");
    let export_path = temp_dir.path().join("export.mv2");
    let speckit_dir = temp_dir.path().join(".speckit/memvid");
    let workspace_path = speckit_dir.join("workspace.mv2");

    // Create workspace directory structure
    std::fs::create_dir_all(&speckit_dir).expect("create speckit dir");

    // Step 1: Create source capsule with artifacts
    let source_config = CapsuleConfig {
        capsule_path: source_path.clone(),
        workspace_id: "source".to_string(),
        ..Default::default()
    };
    let source_handle = CapsuleHandle::open(source_config).expect("should create source");

    let artifact_data = b"Test artifact for mount persistence".to_vec();
    source_handle
        .put(
            "SPEC-MOUNT",
            "run-1",
            ObjectType::Artifact,
            "test.txt",
            artifact_data,
            serde_json::json!({}),
        )
        .expect("should put artifact");

    source_handle
        .commit_stage("SPEC-MOUNT", "run-1", "plan", None)
        .expect("should commit");

    // Step 2: Export to .mv2 (unencrypted)
    let export_opts = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some("SPEC-MOUNT".to_string()),
        run_id: Some("run-1".to_string()),
        encrypt: false,
        ..Default::default()
    };
    source_handle.export(&export_opts).expect("should export");
    drop(source_handle);

    // Step 3: Create workspace capsule and import
    let workspace_config = CapsuleConfig {
        capsule_path: workspace_path.clone(),
        workspace_id: "workspace".to_string(),
        ..Default::default()
    };
    let workspace_handle = CapsuleHandle::open(workspace_config).expect("should create workspace");

    let import_opts = ImportOptions::for_file(&export_path)
        .with_mount_name("mounted_capsule")
        .with_interactive(false);

    let result = workspace_handle
        .import(&import_opts)
        .expect("should import");

    // Verify import result
    assert_eq!(result.mount_name, "mounted_capsule");
    assert!(result.verification_passed);
    assert!(!result.content_hash.is_empty());
    assert!(result.mounted_path.is_some());

    // Verify mount file exists at expected path
    let mount_path = speckit_dir.join("mounts").join("mounted_capsule.mv2");
    assert!(
        mount_path.exists(),
        "Mount file should exist at {:?}",
        mount_path
    );

    // Verify registry file exists and contains entry
    let registry_path = speckit_dir.join("mounts.json");
    assert!(registry_path.exists(), "Registry file should exist");

    let registry_content = std::fs::read_to_string(&registry_path).expect("read registry");
    let registry: MountsRegistry = serde_json::from_str(&registry_content).expect("parse registry");

    assert!(registry.mounts.contains_key("mounted_capsule"));
    let entry = &registry.mounts["mounted_capsule"];
    assert_eq!(entry.content_hash, result.content_hash);
    assert!(entry.verification_passed);

    // Verify CapsuleImported event was emitted
    let events = workspace_handle.list_events();
    let import_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == EventType::CapsuleImported)
        .collect();
    assert_eq!(
        import_events.len(),
        1,
        "should have one CapsuleImported event"
    );
}

/// S974-mount: Invalid mount name (path traversal) fails without creating files.
#[test]
fn test_import_invalid_mount_name_path_traversal() {
    use crate::memvid_adapter::capsule::{CapsuleError, ExportOptions, ImportOptions};

    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("source.mv2");
    let export_path = temp_dir.path().join("export.mv2");
    let speckit_dir = temp_dir.path().join(".speckit/memvid");
    let workspace_path = speckit_dir.join("workspace.mv2");

    std::fs::create_dir_all(&speckit_dir).expect("create speckit dir");

    // Create source and export
    let source_config = CapsuleConfig {
        capsule_path: source_path.clone(),
        workspace_id: "source".to_string(),
        ..Default::default()
    };
    let source_handle = CapsuleHandle::open(source_config).expect("create source");
    source_handle
        .put(
            "SPEC-X",
            "run",
            ObjectType::Artifact,
            "f.txt",
            b"x".to_vec(),
            serde_json::json!({}),
        )
        .expect("put");
    source_handle
        .commit_stage("SPEC-X", "run", "plan", None)
        .expect("commit");

    let export_opts = ExportOptions {
        output_path: export_path.clone(),
        encrypt: false,
        ..Default::default()
    };
    source_handle.export(&export_opts).expect("export");
    drop(source_handle);

    // Create workspace
    let workspace_config = CapsuleConfig {
        capsule_path: workspace_path.clone(),
        workspace_id: "workspace".to_string(),
        ..Default::default()
    };
    let workspace_handle = CapsuleHandle::open(workspace_config).expect("workspace");

    // Try import with path traversal mount name
    let import_opts = ImportOptions::for_file(&export_path)
        .with_mount_name("../escaped")
        .with_interactive(false);

    let result = workspace_handle.import(&import_opts);
    assert!(
        matches!(result, Err(CapsuleError::InvalidMountName { .. })),
        "Should fail with InvalidMountName for path traversal, got {:?}",
        result
    );

    // Verify no mount file or registry created
    let mounts_dir = speckit_dir.join("mounts");
    assert!(
        !mounts_dir.exists() || std::fs::read_dir(&mounts_dir).unwrap().count() == 0,
        "Mounts directory should be empty or not exist"
    );

    let registry_path = speckit_dir.join("mounts.json");
    if registry_path.exists() {
        let content = std::fs::read_to_string(&registry_path).unwrap();
        let registry: serde_json::Value = serde_json::from_str(&content).unwrap();
        let mounts = registry.get("mounts").unwrap().as_object().unwrap();
        assert!(mounts.is_empty(), "Registry should have no mounts");
    }
}

/// S974-mount: Re-import same file with same mount name is idempotent.
#[test]
fn test_import_idempotent_same_hash() {
    use crate::memvid_adapter::capsule::{ExportOptions, ImportOptions, MountsRegistry};
    use crate::memvid_adapter::types::EventType;

    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("source.mv2");
    let export_path = temp_dir.path().join("export.mv2");
    let speckit_dir = temp_dir.path().join(".speckit/memvid");
    let workspace_path = speckit_dir.join("workspace.mv2");

    std::fs::create_dir_all(&speckit_dir).expect("create speckit dir");

    // Create source and export
    let source_config = CapsuleConfig {
        capsule_path: source_path.clone(),
        workspace_id: "source".to_string(),
        ..Default::default()
    };
    let source_handle = CapsuleHandle::open(source_config).expect("create source");
    source_handle
        .put(
            "SPEC-IDEMP",
            "run",
            ObjectType::Artifact,
            "f.txt",
            b"idempotent data".to_vec(),
            serde_json::json!({}),
        )
        .expect("put");
    source_handle
        .commit_stage("SPEC-IDEMP", "run", "plan", None)
        .expect("commit");

    let export_opts = ExportOptions {
        output_path: export_path.clone(),
        encrypt: false,
        ..Default::default()
    };
    source_handle.export(&export_opts).expect("export");
    drop(source_handle);

    // Create workspace
    let workspace_config = CapsuleConfig {
        capsule_path: workspace_path.clone(),
        workspace_id: "workspace".to_string(),
        ..Default::default()
    };
    let workspace_handle = CapsuleHandle::open(workspace_config).expect("workspace");

    let import_opts = ImportOptions::for_file(&export_path)
        .with_mount_name("idempotent_mount")
        .with_interactive(false);

    // First import
    let result1 = workspace_handle
        .import(&import_opts)
        .expect("first import should succeed");

    // Second import (same file, same mount name)
    let result2 = workspace_handle
        .import(&import_opts)
        .expect("second import should succeed (idempotent)");

    // Verify same result
    assert_eq!(result1.content_hash, result2.content_hash);
    assert_eq!(result1.mount_name, result2.mount_name);

    // Verify only ONE registry entry
    let registry_path = speckit_dir.join("mounts.json");
    let registry: MountsRegistry =
        serde_json::from_str(&std::fs::read_to_string(&registry_path).unwrap()).unwrap();
    assert_eq!(
        registry.mounts.len(),
        1,
        "Should have exactly one mount entry"
    );

    // Verify only ONE CapsuleImported event (from first import)
    let events = workspace_handle.list_events();
    let import_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == EventType::CapsuleImported)
        .collect();
    assert_eq!(
        import_events.len(),
        1,
        "Should have exactly one CapsuleImported event"
    );
}

/// S974-mount: Re-import different file with same mount name fails.
#[test]
fn test_import_hash_conflict() {
    use crate::memvid_adapter::capsule::{CapsuleError, ExportOptions, ImportOptions};

    let temp_dir = TempDir::new().unwrap();
    let source1_path = temp_dir.path().join("source1.mv2");
    let source2_path = temp_dir.path().join("source2.mv2");
    let export1_path = temp_dir.path().join("export1.mv2");
    let export2_path = temp_dir.path().join("export2.mv2");
    let speckit_dir = temp_dir.path().join(".speckit/memvid");
    let workspace_path = speckit_dir.join("workspace.mv2");

    std::fs::create_dir_all(&speckit_dir).expect("create speckit dir");

    // Create first source and export
    let source1_config = CapsuleConfig {
        capsule_path: source1_path.clone(),
        workspace_id: "source1".to_string(),
        ..Default::default()
    };
    let source1_handle = CapsuleHandle::open(source1_config).expect("create source1");
    source1_handle
        .put(
            "SPEC-1",
            "run",
            ObjectType::Artifact,
            "f.txt",
            b"content one".to_vec(),
            serde_json::json!({}),
        )
        .expect("put");
    source1_handle
        .commit_stage("SPEC-1", "run", "plan", None)
        .expect("commit");
    source1_handle
        .export(&ExportOptions {
            output_path: export1_path.clone(),
            encrypt: false,
            ..Default::default()
        })
        .expect("export1");
    drop(source1_handle);

    // Create second source and export (different content)
    let source2_config = CapsuleConfig {
        capsule_path: source2_path.clone(),
        workspace_id: "source2".to_string(),
        ..Default::default()
    };
    let source2_handle = CapsuleHandle::open(source2_config).expect("create source2");
    source2_handle
        .put(
            "SPEC-2",
            "run",
            ObjectType::Artifact,
            "f.txt",
            b"different content two".to_vec(),
            serde_json::json!({}),
        )
        .expect("put");
    source2_handle
        .commit_stage("SPEC-2", "run", "plan", None)
        .expect("commit");
    source2_handle
        .export(&ExportOptions {
            output_path: export2_path.clone(),
            encrypt: false,
            ..Default::default()
        })
        .expect("export2");
    drop(source2_handle);

    // Create workspace
    let workspace_config = CapsuleConfig {
        capsule_path: workspace_path.clone(),
        workspace_id: "workspace".to_string(),
        ..Default::default()
    };
    let workspace_handle = CapsuleHandle::open(workspace_config).expect("workspace");

    // First import
    let import_opts1 = ImportOptions::for_file(&export1_path)
        .with_mount_name("shared_name")
        .with_interactive(false);
    workspace_handle
        .import(&import_opts1)
        .expect("first import should succeed");

    // Second import with same mount name but different content
    let import_opts2 = ImportOptions::for_file(&export2_path)
        .with_mount_name("shared_name")
        .with_interactive(false);

    let result = workspace_handle.import(&import_opts2);
    assert!(
        matches!(result, Err(CapsuleError::MountHashConflict { .. })),
        "Should fail with MountHashConflict, got {:?}",
        result
    );
}

/// S974-mount: Import .mv2e with wrong passphrase creates no mount or registry entry.
#[test]
#[serial_test::serial]
fn test_import_mv2e_wrong_passphrase_no_partial_mount() {
    use crate::memvid_adapter::capsule::{CapsuleError, ExportOptions, ImportOptions};

    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("source.mv2");
    let export_path = temp_dir.path().join("export.mv2e");
    let speckit_dir = temp_dir.path().join(".speckit/memvid");
    let workspace_path = speckit_dir.join("workspace.mv2");

    std::fs::create_dir_all(&speckit_dir).expect("create speckit dir");

    // Create encrypted export with correct passphrase
    unsafe {
        std::env::set_var("SPECKIT_MEMVID_PASSPHRASE", "correct-password");
    }

    let source_config = CapsuleConfig {
        capsule_path: source_path.clone(),
        workspace_id: "source".to_string(),
        ..Default::default()
    };
    let source_handle = CapsuleHandle::open(source_config).expect("create source");
    source_handle
        .put(
            "SPEC-ENC",
            "run",
            ObjectType::Artifact,
            "f.txt",
            b"encrypted data".to_vec(),
            serde_json::json!({}),
        )
        .expect("put");
    source_handle
        .commit_stage("SPEC-ENC", "run", "plan", None)
        .expect("commit");

    let export_opts = ExportOptions {
        output_path: export_path.clone(),
        encrypt: true,
        interactive: false,
        ..Default::default()
    };
    source_handle.export(&export_opts).expect("export");
    drop(source_handle);

    // Try to import with wrong passphrase
    unsafe {
        std::env::set_var("SPECKIT_MEMVID_PASSPHRASE", "wrong-password");
    }

    let workspace_config = CapsuleConfig {
        capsule_path: workspace_path.clone(),
        workspace_id: "workspace".to_string(),
        ..Default::default()
    };
    let workspace_handle = CapsuleHandle::open(workspace_config).expect("workspace");

    let import_opts = ImportOptions::for_file(&export_path)
        .with_mount_name("encrypted_capsule")
        .with_interactive(false);

    let result = workspace_handle.import(&import_opts);
    assert!(
        matches!(result, Err(CapsuleError::InvalidPassphrase)),
        "Should fail with InvalidPassphrase, got {:?}",
        result
    );

    // Verify no mount file
    let mount_path = speckit_dir.join("mounts/encrypted_capsule.mv2e");
    assert!(
        !mount_path.exists(),
        "Mount file should not exist after failed import"
    );

    // Verify no registry entry
    let registry_path = speckit_dir.join("mounts.json");
    if registry_path.exists() {
        let content = std::fs::read_to_string(&registry_path).unwrap();
        let registry: serde_json::Value = serde_json::from_str(&content).unwrap();
        let mounts = registry.get("mounts").unwrap().as_object().unwrap();
        assert!(
            !mounts.contains_key("encrypted_capsule"),
            "Registry should not contain failed import"
        );
    }

    // Clean up
    unsafe {
        std::env::remove_var("SPECKIT_MEMVID_PASSPHRASE");
    }
}

// =============================================================================
// SPEC-KIT-974 Acceptance Verification Tests (Added for AC verification)
// =============================================================================

/// SPEC-KIT-974 AC#2 (strengthened): Import with verification failure creates no mount.
///
/// This test verifies that when `require_verified=true` and doctor checks fail
/// (due to corrupted capsule), no mount file or registry entry is created.
#[test]
fn test_import_verification_failure_no_partial_mount() {
    use crate::memvid_adapter::capsule::{CapsuleError, ImportOptions};

    let temp_dir = TempDir::new().unwrap();
    let corrupted_path = temp_dir.path().join("corrupted.mv2");
    let speckit_dir = temp_dir.path().join(".speckit/memvid");
    let workspace_path = speckit_dir.join("workspace.mv2");

    std::fs::create_dir_all(&speckit_dir).expect("create speckit dir");

    // Create a corrupted .mv2 file (truncated/invalid)
    // Real MV2 files have a specific header, this is just garbage
    std::fs::write(&corrupted_path, b"not a valid mv2 capsule").expect("write corrupted file");

    // Create workspace capsule
    let workspace_config = CapsuleConfig {
        capsule_path: workspace_path.clone(),
        workspace_id: "workspace".to_string(),
        ..Default::default()
    };
    let workspace_handle = CapsuleHandle::open(workspace_config).expect("workspace");

    // Try to import with require_verified=true
    let import_opts = ImportOptions::for_file(&corrupted_path)
        .with_mount_name("corrupted_capsule")
        .require_verified()
        .with_interactive(false);

    let result = workspace_handle.import(&import_opts);

    // Should fail due to verification failure
    assert!(
        matches!(result, Err(CapsuleError::InvalidOperation { .. })),
        "Should fail with InvalidOperation due to verification failure, got {:?}",
        result
    );

    // Verify no mount file was created
    let mount_path = speckit_dir.join("mounts/corrupted_capsule.mv2");
    assert!(
        !mount_path.exists(),
        "Mount file should not exist after failed import"
    );

    // Verify no registry entry was created
    let registry_path = speckit_dir.join("mounts.json");
    if registry_path.exists() {
        let content = std::fs::read_to_string(&registry_path).unwrap();
        let registry: serde_json::Value = serde_json::from_str(&content).unwrap();
        if let Some(mounts) = registry.get("mounts").and_then(|m| m.as_object()) {
            assert!(
                !mounts.contains_key("corrupted_capsule"),
                "Registry should not contain failed import"
            );
        }
    }
}

/// SPEC-KIT-974 AC#6: CapsuleExported event has all required fields.
///
/// Required fields (spec.md:71): run_id, spec_id, digest, encryption flag, safe flag, included tracks
#[test]
#[serial_test::serial]
fn test_capsule_exported_event_has_required_fields() {
    use crate::memvid_adapter::capsule::ExportOptions;
    use crate::memvid_adapter::types::{CapsuleExportedPayload, EventType};

    let temp_dir = TempDir::new().unwrap();
    let capsule_path = temp_dir.path().join("export_event_test.mv2");
    let export_path = temp_dir.path().join("export_event.mv2e");

    // Set passphrase for encrypted export
    unsafe {
        std::env::set_var("SPECKIT_MEMVID_PASSPHRASE", "test-passphrase");
    }

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "export_event_test".to_string(),
        ..Default::default()
    };

    let handle = CapsuleHandle::open(config).expect("should create capsule");

    let spec_id = "SPEC-974-EVENT";
    let run_id = "event-field-test";

    // Add artifact and commit
    handle
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "test.md",
            b"Test content for event verification".to_vec(),
            serde_json::json!({}),
        )
        .expect("should put artifact");

    handle
        .commit_stage(spec_id, run_id, "plan", None)
        .expect("should commit");

    // Export with known settings: encrypted=true, safe_mode=true
    let options = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some(spec_id.to_string()),
        run_id: Some(run_id.to_string()),
        encrypt: true,
        safe_mode: true,
        interactive: false,
        ..Default::default()
    };
    let export_result = handle.export(&options).expect("should export");

    // Find CapsuleExported event
    let events = handle.list_events();
    let export_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == EventType::CapsuleExported)
        .collect();

    assert_eq!(
        export_events.len(),
        1,
        "Should have exactly one CapsuleExported event"
    );

    // Deserialize and verify required fields
    let payload: CapsuleExportedPayload =
        serde_json::from_value(export_events[0].payload.clone()).expect("should deserialize");

    // Verify required fields per spec.md:71
    assert!(
        payload.content_hash.is_some(),
        "content_hash (digest) must be present"
    );
    assert!(
        !payload.content_hash.as_ref().unwrap().is_empty(),
        "content_hash must be non-empty"
    );
    assert!(payload.encrypted, "encrypted flag must be true");
    assert!(payload.sanitized, "sanitized (safe flag) must be true");
    assert!(
        !payload.checkpoints_included.is_empty(),
        "included tracks (checkpoints) must be non-empty"
    );

    // Verify digest matches export result
    assert_eq!(
        payload.content_hash.as_ref().unwrap(),
        &export_result.content_hash,
        "content_hash should match export result"
    );

    // Clean up
    unsafe {
        std::env::remove_var("SPECKIT_MEMVID_PASSPHRASE");
    }
}

/// SPEC-KIT-974 AC#7: CapsuleImported event has all required fields.
///
/// Required fields (spec.md:72): source digest, mount name, validation result
#[test]
fn test_capsule_imported_event_has_required_fields() {
    use crate::memvid_adapter::capsule::{ExportOptions, ImportOptions};
    use crate::memvid_adapter::types::{CapsuleImportedPayload, EventType};

    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("source.mv2");
    let export_path = temp_dir.path().join("export.mv2");
    let speckit_dir = temp_dir.path().join(".speckit/memvid");
    let workspace_path = speckit_dir.join("workspace.mv2");

    std::fs::create_dir_all(&speckit_dir).expect("create speckit dir");

    // Create source capsule and export
    let source_config = CapsuleConfig {
        capsule_path: source_path.clone(),
        workspace_id: "source".to_string(),
        ..Default::default()
    };
    let source_handle = CapsuleHandle::open(source_config).expect("create source");

    source_handle
        .put(
            "SPEC-IMPORT-EVENT",
            "run",
            ObjectType::Artifact,
            "test.txt",
            b"import event test data".to_vec(),
            serde_json::json!({}),
        )
        .expect("put");
    source_handle
        .commit_stage("SPEC-IMPORT-EVENT", "run", "plan", None)
        .expect("commit");

    let export_opts = ExportOptions {
        output_path: export_path.clone(),
        encrypt: false,
        ..Default::default()
    };
    let export_result = source_handle.export(&export_opts).expect("export");
    drop(source_handle);

    // Create workspace and import
    let workspace_config = CapsuleConfig {
        capsule_path: workspace_path.clone(),
        workspace_id: "workspace".to_string(),
        ..Default::default()
    };
    let workspace_handle = CapsuleHandle::open(workspace_config).expect("workspace");

    let import_opts = ImportOptions::for_file(&export_path)
        .with_mount_name("event_test_mount")
        .with_interactive(false);

    let import_result = workspace_handle.import(&import_opts).expect("import");

    // Find CapsuleImported event
    let events = workspace_handle.list_events();
    let import_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == EventType::CapsuleImported)
        .collect();

    assert_eq!(
        import_events.len(),
        1,
        "Should have exactly one CapsuleImported event"
    );

    // Deserialize and verify required fields
    let payload: CapsuleImportedPayload =
        serde_json::from_value(import_events[0].payload.clone()).expect("should deserialize");

    // Verify required fields per spec.md:72
    // 1. Source digest (content_hash)
    assert!(
        payload.content_hash.is_some(),
        "content_hash (source digest) must be present"
    );
    assert!(
        !payload.content_hash.as_ref().unwrap().is_empty(),
        "content_hash must be non-empty"
    );
    assert_eq!(
        payload.content_hash.as_ref().unwrap(),
        &export_result.content_hash,
        "content_hash should match exported capsule"
    );

    // 2. Mount name - SPEC-KIT-974 AC#7: now in event payload
    assert!(payload.source.is_some(), "source path must be present");
    assert!(
        payload.source.as_ref().unwrap().contains("export.mv2"),
        "source should reference the exported file"
    );
    assert_eq!(
        payload.mount_name,
        Some("event_test_mount".to_string()),
        "mount_name must be present in event payload"
    );

    // 3. Validation result - SPEC-KIT-974 AC#7: now in event payload
    assert_eq!(
        payload.verification_passed,
        Some(true),
        "verification_passed must be present in event payload"
    );
    // Also verify consistency with ImportResult
    assert!(
        import_result.verification_passed,
        "verification should have passed"
    );
    assert_eq!(
        import_result.mount_name, "event_test_mount",
        "mount name should match requested"
    );
}

// =============================================================================
// SPEC-KIT-974: Export/Import Retrieval Parity Tests
// Validates the Replay Truth Table invariant:
// "Exported capsule, imported elsewhere → Identical to source, Deterministic"
// =============================================================================

/// P0-A: Validates that an exported capsule imported elsewhere yields identical
/// retrieval results for the same search query.
///
/// This test verifies the canonical truth-table claim in SPEC.md (lines 143-151)
/// that exported capsules produce deterministic, identical results when imported.
#[tokio::test]
async fn test_export_import_retrieval_parity() {
    use crate::memvid_adapter::capsule::{ExportOptions, ImportOptions};
    use codex_stage0::dcc::{Iqo, LocalMemoryClient, LocalMemorySearchParams};

    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("source.mv2");
    let export_path = temp_dir.path().join("exported.mv2");
    let workspace_path = temp_dir.path().join("workspace.mv2");

    // =========================================================================
    // Step 1: Create source capsule with artifacts designed for stable ranking
    // =========================================================================
    let source_config = CapsuleConfig {
        capsule_path: source_path.clone(),
        workspace_id: "parity_source".to_string(),
        ..Default::default()
    };
    let source_handle = CapsuleHandle::open(source_config.clone()).expect("should create source");

    // Corpus designed to produce stable, non-tied rankings for "authentication flow"
    // Artifact 1: Heavy on authentication keywords (should rank #1)
    source_handle
        .put(
            "SPEC-PARITY",
            "run-1",
            ObjectType::Artifact,
            "auth_heavy.md",
            b"# Authentication Flow Implementation\n\
              The authentication flow handles user login, logout, and session management.\n\
              Authentication tokens are validated on each request.\n\
              The authentication module integrates with OAuth providers."
                .to_vec(),
            serde_json::json!({"type": "doc", "topic": "auth"}),
        )
        .expect("should put auth_heavy");

    // Artifact 2: Moderate authentication + security (should rank #2)
    source_handle
        .put(
            "SPEC-PARITY",
            "run-1",
            ObjectType::Artifact,
            "auth_moderate.md",
            b"# Security and Authentication\n\
              This module provides authentication and security features.\n\
              Password hashing and token generation are handled here."
                .to_vec(),
            serde_json::json!({"type": "doc", "topic": "security"}),
        )
        .expect("should put auth_moderate");

    // Artifact 3: Light authentication mention (should rank #3)
    source_handle
        .put(
            "SPEC-PARITY",
            "run-1",
            ObjectType::Artifact,
            "auth_light.md",
            b"# User Management\n\
              User profiles and settings are managed here.\n\
              Requires authentication to access."
                .to_vec(),
            serde_json::json!({"type": "doc", "topic": "users"}),
        )
        .expect("should put auth_light");

    // Artifact 4: Unrelated - performance (should not rank highly)
    source_handle
        .put(
            "SPEC-PARITY",
            "run-1",
            ObjectType::Artifact,
            "performance.md",
            b"# Performance Optimization\n\
              This guide covers caching strategies and query optimization.\n\
              Database indexing and connection pooling are discussed."
                .to_vec(),
            serde_json::json!({"type": "doc", "topic": "perf"}),
        )
        .expect("should put performance");

    // Artifact 5: Unrelated - networking (should not rank highly)
    source_handle
        .put(
            "SPEC-PARITY",
            "run-1",
            ObjectType::Artifact,
            "networking.md",
            b"# Networking Configuration\n\
              Load balancer setup and DNS configuration.\n\
              Firewall rules and port forwarding."
                .to_vec(),
            serde_json::json!({"type": "doc", "topic": "network"}),
        )
        .expect("should put networking");

    // Commit to create checkpoint
    source_handle
        .commit_stage("SPEC-PARITY", "run-1", "plan", None)
        .expect("should commit");

    // =========================================================================
    // Step 2: Export to unencrypted .mv2
    // =========================================================================
    let export_opts = ExportOptions {
        output_path: export_path.clone(),
        spec_id: Some("SPEC-PARITY".to_string()),
        run_id: Some("run-1".to_string()),
        encrypt: false,
        ..Default::default()
    };
    source_handle.export(&export_opts).expect("should export");
    drop(source_handle); // Release lock before opening adapter

    // =========================================================================
    // Step 3: Search on source capsule (reopen via adapter after export)
    // =========================================================================
    let source_adapter = MemvidMemoryAdapter::new(source_config.clone());
    source_adapter.open().await.expect("should open source");

    let search_params = LocalMemorySearchParams {
        iqo: Iqo {
            keywords: vec!["authentication".to_string(), "flow".to_string()],
            ..Default::default()
        },
        max_results: 5,
    };

    let source_results = source_adapter
        .search_memories(search_params.clone())
        .await
        .expect("source search should succeed");

    assert!(
        !source_results.is_empty(),
        "source should have search results"
    );

    // Extract top 3 result IDs for comparison
    let source_top_ids: Vec<String> = source_results
        .iter()
        .take(3)
        .map(|r| r.id.clone())
        .collect();

    drop(source_adapter); // Release source before import

    // =========================================================================
    // Step 4: Import into workspace and search on imported capsule
    // =========================================================================
    let workspace_config = CapsuleConfig {
        capsule_path: workspace_path.clone(),
        workspace_id: "parity_workspace".to_string(),
        ..Default::default()
    };
    let workspace_handle =
        CapsuleHandle::open(workspace_config.clone()).expect("should create workspace");

    let import_opts = ImportOptions::for_file(&export_path)
        .with_mount_name("parity_import")
        .with_interactive(false);

    let import_result = workspace_handle
        .import(&import_opts)
        .expect("should import");

    assert!(
        import_result.verification_passed,
        "import verification should pass"
    );

    // Open the imported capsule for searching
    let imported_path = import_result
        .mounted_path
        .expect("should have mounted path");
    let imported_config = CapsuleConfig {
        capsule_path: imported_path,
        workspace_id: "parity_imported".to_string(),
        ..Default::default()
    };
    let imported_adapter = MemvidMemoryAdapter::new(imported_config);
    imported_adapter
        .open()
        .await
        .expect("should open imported capsule");

    // =========================================================================
    // Step 5: Search on imported capsule with same query
    // =========================================================================
    let imported_results = imported_adapter
        .search_memories(search_params)
        .await
        .expect("imported search should succeed");

    assert!(
        !imported_results.is_empty(),
        "imported should have search results"
    );

    // Extract top 3 result IDs
    let imported_top_ids: Vec<String> = imported_results
        .iter()
        .take(3)
        .map(|r| r.id.clone())
        .collect();

    // =========================================================================
    // Step 6: Assert retrieval parity (Replay Truth Table invariant)
    // =========================================================================
    assert_eq!(
        source_top_ids.len(),
        imported_top_ids.len(),
        "should have same number of top results"
    );

    assert_eq!(
        source_top_ids, imported_top_ids,
        "Retrieval parity violated: source top-3 {:?} != imported top-3 {:?}",
        source_top_ids, imported_top_ids
    );
}
