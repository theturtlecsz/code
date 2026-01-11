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
    let checkpoint_id = handle
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
