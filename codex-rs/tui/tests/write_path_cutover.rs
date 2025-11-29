//! Integration tests for SPEC-945B New Schema Operations
//!
//! Tests verify new schema (consensus_runs + agent_outputs) write operations:
//! 1. Writes succeed and data is persisted
//! 2. Reads return correct data
//! 3. Multiple operations maintain consistency

// SPEC-957: Allow test code flexibility
#![allow(dead_code, unused_variables, unused_mut)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui::{ConsensusDb, SpecStage};
use rusqlite::Connection;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test helper: Create ConsensusDb with temp database
fn create_test_db() -> (ConsensusDb, TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_consensus.db");
    let db = ConsensusDb::init(&db_path).unwrap();
    (db, temp_dir, db_path)
}

/// Test helper: Count records in new schema
fn count_new_schema_runs(db_path: &PathBuf) -> i64 {
    let conn = Connection::open(db_path).unwrap();
    conn.query_row("SELECT COUNT(*) FROM consensus_runs", [], |row| row.get(0))
        .unwrap()
}

fn count_new_schema_outputs(db_path: &PathBuf) -> i64 {
    let conn = Connection::open(db_path).unwrap();
    conn.query_row("SELECT COUNT(*) FROM agent_outputs", [], |row| row.get(0))
        .unwrap()
}

#[test]
fn test_write_path_cutover_artifact_new_schema_only() {
    // Test: store_artifact() writes to new schema
    let (db, _temp, db_path) = create_test_db();

    // Verify schema is empty before write
    assert_eq!(count_new_schema_runs(&db_path), 0);
    assert_eq!(count_new_schema_outputs(&db_path), 0);

    // Store artifact
    let result = db.store_artifact(
        "SPEC-TEST-001",
        SpecStage::Plan,
        "gemini",
        r#"{"recommendation": "Implement feature X"}"#,
        Some("Full response text"),
        Some("run-123"),
    );
    assert!(result.is_ok());

    // Verify new schema has data (run + output)
    assert_eq!(count_new_schema_runs(&db_path), 1, "New schema run created");
    assert_eq!(
        count_new_schema_outputs(&db_path),
        1,
        "New schema output created"
    );
}

#[test]
fn test_write_path_cutover_synthesis_new_schema_only() {
    // Test: store_synthesis() writes to new schema
    let (db, _temp, db_path) = create_test_db();

    // Verify schema is empty before write
    assert_eq!(count_new_schema_runs(&db_path), 0);

    // Store synthesis
    let result = db.store_synthesis(
        "SPEC-TEST-002",
        SpecStage::Tasks,
        "# Task Breakdown\n1. Task A\n2. Task B",
        None,
        "success",
        3,
        Some("All agents agreed"),
        None,
        false,
        Some("run-456"),
    );
    assert!(result.is_ok());

    // Verify new schema has data
    assert_eq!(count_new_schema_runs(&db_path), 1, "New schema run created");
}

#[test]
fn test_write_path_cutover_zero_data_loss() {
    // Test: All writes are accessible via reads
    let (db, _temp, _db_path) = create_test_db();

    // Store artifact
    db.store_artifact(
        "SPEC-TEST-003",
        SpecStage::Plan,
        "gemini",
        r#"{"data": "test"}"#,
        None,
        None,
    )
    .unwrap();

    // Read should find the artifact
    let artifacts = db
        .query_artifacts("SPEC-TEST-003", SpecStage::Plan)
        .unwrap();
    assert_eq!(artifacts.len(), 1, "Should find artifact");
    assert_eq!(artifacts[0].agent_name, "gemini");
}

#[test]
fn test_write_path_cutover_read_fallback_still_works() {
    // Test: Reads work correctly from new schema
    let (db, _temp, _db_path) = create_test_db();

    db.store_artifact(
        "SPEC-TEST-004",
        SpecStage::Validate,
        "claude",
        r#"{"validation": "passed"}"#,
        None,
        None,
    )
    .unwrap();

    let artifacts = db
        .query_artifacts("SPEC-TEST-004", SpecStage::Validate)
        .unwrap();
    assert_eq!(artifacts.len(), 1);
}

#[test]
fn test_write_path_cutover_synthesis_read_fallback() {
    // Test: Synthesis reads work correctly from new schema
    let (db, _temp, _db_path) = create_test_db();

    db.store_synthesis(
        "SPEC-TEST-005",
        SpecStage::Audit,
        "# Audit Results\nAll checks passed.",
        None,
        "success",
        5,
        None,
        None,
        false,
        None,
    )
    .unwrap();

    let synthesis = db
        .query_latest_synthesis("SPEC-TEST-005", SpecStage::Audit)
        .unwrap();
    assert!(synthesis.is_some());
}

#[test]
fn test_write_path_cutover_multiple_artifacts_new_schema() {
    // Test: Multiple artifacts stored correctly
    let (db, _temp, db_path) = create_test_db();

    // Store 5 artifacts
    for i in 1..=5 {
        db.store_artifact(
            "SPEC-TEST-006",
            SpecStage::Plan,
            &format!("agent-{i}"),
            &format!(r#"{{"id": {i}}}"#),
            None,
            None,
        )
        .unwrap();
    }

    // Verify all stored in new schema
    assert_eq!(count_new_schema_outputs(&db_path), 5, "5 outputs created");

    // Verify all readable
    let artifacts = db
        .query_artifacts("SPEC-TEST-006", SpecStage::Plan)
        .unwrap();
    assert_eq!(artifacts.len(), 5, "Should find all 5 artifacts");
}

#[test]
fn test_write_path_cutover_stage_specific_isolation() {
    // Test: Different stages are isolated
    let (db, _temp, _db_path) = create_test_db();

    // Store artifacts in different stages
    db.store_artifact(
        "SPEC-TEST-007",
        SpecStage::Plan,
        "plan-agent",
        r#"{"stage": "plan"}"#,
        None,
        None,
    )
    .unwrap();

    db.store_artifact(
        "SPEC-TEST-007",
        SpecStage::Implement,
        "implement-agent",
        r#"{"stage": "implement"}"#,
        None,
        None,
    )
    .unwrap();

    // Query each stage separately
    let plan_artifacts = db
        .query_artifacts("SPEC-TEST-007", SpecStage::Plan)
        .unwrap();
    assert_eq!(plan_artifacts.len(), 1, "Should find 1 plan artifact");

    let implement_artifacts = db
        .query_artifacts("SPEC-TEST-007", SpecStage::Implement)
        .unwrap();
    assert_eq!(
        implement_artifacts.len(),
        1,
        "Should find 1 implement artifact"
    );
}

#[test]
fn test_write_path_cutover_consistency_under_load() {
    // Test: Consistency maintained under multiple operations
    let (db, _temp, db_path) = create_test_db();

    // Store 20 artifacts rapidly
    for i in 1..=20 {
        let spec_id = format!("SPEC-TEST-{i:03}");
        db.store_artifact(
            &spec_id,
            SpecStage::Plan,
            "gemini",
            r#"{"test": "load"}"#,
            None,
            None,
        )
        .unwrap();
    }

    // Verify counts
    assert_eq!(count_new_schema_runs(&db_path), 20, "20 runs created");
    assert_eq!(count_new_schema_outputs(&db_path), 20, "20 outputs created");

    // Verify all 20 specs readable
    for i in 1..=20 {
        let spec_id = format!("SPEC-TEST-{i:03}");
        let artifacts = db.query_artifacts(&spec_id, SpecStage::Plan).unwrap();
        assert_eq!(artifacts.len(), 1, "Spec {spec_id} artifact accessible");
    }
}

// ============================================================================
// P6-SYNC Phase 4: Branch-Aware Resume Filtering Tests
// ============================================================================

#[test]
fn test_branch_id_stored_with_agent_spawn() {
    // Test: record_agent_spawn stores branch_id correctly
    let (db, _temp, db_path) = create_test_db();

    // Record spawn with branch_id
    let result = db.record_agent_spawn(
        "agent-123",
        "SPEC-BRANCH-001",
        SpecStage::Plan,
        "regular_stage",
        "gemini",
        Some("run-abc"),
        Some("SPEC-BRANCH-001-20251129-abc123"),
    );
    assert!(result.is_ok(), "record_agent_spawn should succeed");

    // Verify branch_id is stored in agent_executions table
    let conn = Connection::open(&db_path).unwrap();
    let branch: Option<String> = conn
        .query_row(
            "SELECT branch_id FROM agent_executions WHERE agent_id = ?",
            ["agent-123"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        branch,
        Some("SPEC-BRANCH-001-20251129-abc123".to_string()),
        "branch_id should be stored"
    );
}

#[test]
fn test_branch_id_null_when_not_provided() {
    // Test: branch_id is NULL when not provided
    let (db, _temp, db_path) = create_test_db();

    // Record spawn without branch_id
    db.record_agent_spawn(
        "agent-456",
        "SPEC-BRANCH-002",
        SpecStage::Tasks,
        "quality_gate",
        "claude",
        Some("run-def"),
        None, // No branch_id
    )
    .unwrap();

    // Verify branch_id is NULL
    let conn = Connection::open(&db_path).unwrap();
    let branch: Option<String> = conn
        .query_row(
            "SELECT branch_id FROM agent_executions WHERE agent_id = ?",
            ["agent-456"],
            |row| row.get(0),
        )
        .unwrap();
    assert!(branch.is_none(), "branch_id should be NULL when not provided");
}

#[test]
fn test_get_responses_for_branch_filters_correctly() {
    // Test: get_responses_for_branch returns only matching branch
    let (db, _temp, _db_path) = create_test_db();

    let spec_id = "SPEC-BRANCH-003";
    let stage = SpecStage::Plan;
    let branch_a = "SPEC-BRANCH-003-20251129-aaaa";
    let branch_b = "SPEC-BRANCH-003-20251129-bbbb";

    // Record 2 spawns for branch A
    db.record_agent_spawn(
        "agent-a1",
        spec_id,
        stage,
        "regular",
        "gemini",
        Some("run-a"),
        Some(branch_a),
    )
    .unwrap();
    db.record_agent_spawn(
        "agent-a2",
        spec_id,
        stage,
        "regular",
        "claude",
        Some("run-a"),
        Some(branch_a),
    )
    .unwrap();

    // Record 1 spawn for branch B
    db.record_agent_spawn(
        "agent-b1",
        spec_id,
        stage,
        "regular",
        "gemini",
        Some("run-b"),
        Some(branch_b),
    )
    .unwrap();

    // Query for branch A - should get 2
    let branch_a_responses = db.get_responses_for_branch(spec_id, stage, branch_a).unwrap();
    assert_eq!(
        branch_a_responses.len(),
        2,
        "Should find 2 responses for branch A"
    );

    // Query for branch B - should get 1
    let branch_b_responses = db.get_responses_for_branch(spec_id, stage, branch_b).unwrap();
    assert_eq!(
        branch_b_responses.len(),
        1,
        "Should find 1 response for branch B"
    );

    // Query for non-existent branch - should get 0
    let no_branch = db
        .get_responses_for_branch(spec_id, stage, "non-existent-branch")
        .unwrap();
    assert_eq!(
        no_branch.len(),
        0,
        "Should find 0 responses for non-existent branch"
    );
}

#[test]
fn test_branch_filtering_isolates_parallel_runs() {
    // Test: Two parallel pipeline runs are isolated by branch_id
    let (db, _temp, _db_path) = create_test_db();

    let spec_id = "SPEC-PARALLEL-001";
    let branch_1 = "SPEC-PARALLEL-001-20251129-run1";
    let branch_2 = "SPEC-PARALLEL-001-20251129-run2";

    // Simulate parallel pipeline runs with different branches
    // Run 1: Plan stage
    db.record_agent_spawn(
        "run1-plan-gemini",
        spec_id,
        SpecStage::Plan,
        "regular",
        "gemini",
        Some("run-1"),
        Some(branch_1),
    )
    .unwrap();

    // Run 2: Also Plan stage (parallel)
    db.record_agent_spawn(
        "run2-plan-gemini",
        spec_id,
        SpecStage::Plan,
        "regular",
        "gemini",
        Some("run-2"),
        Some(branch_2),
    )
    .unwrap();

    // Each run should see only its own agents
    let run1_plan = db
        .get_responses_for_branch(spec_id, SpecStage::Plan, branch_1)
        .unwrap();
    assert_eq!(run1_plan.len(), 1, "Run 1 should see only its own agent");
    assert!(
        run1_plan.iter().any(|r| r.agent_id == "run1-plan-gemini"),
        "Run 1 should see run1-plan-gemini"
    );

    let run2_plan = db
        .get_responses_for_branch(spec_id, SpecStage::Plan, branch_2)
        .unwrap();
    assert_eq!(run2_plan.len(), 1, "Run 2 should see only its own agent");
    assert!(
        run2_plan.iter().any(|r| r.agent_id == "run2-plan-gemini"),
        "Run 2 should see run2-plan-gemini"
    );
}
