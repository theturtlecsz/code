//! Integration tests for SPEC-945B New Schema Operations
//!
//! Tests verify new schema (consensus_runs + agent_outputs) write operations:
//! 1. Writes succeed and data is persisted
//! 2. Reads return correct data
//! 3. Multiple operations maintain consistency

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
    conn.query_row("SELECT COUNT(*) FROM consensus_runs", [], |row| {
        row.get(0)
    })
    .unwrap()
}

fn count_new_schema_outputs(db_path: &PathBuf) -> i64 {
    let conn = Connection::open(db_path).unwrap();
    conn.query_row("SELECT COUNT(*) FROM agent_outputs", [], |row| {
        row.get(0)
    })
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
    let artifacts = db.query_artifacts("SPEC-TEST-003", SpecStage::Plan).unwrap();
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
            &format!("agent-{}", i),
            &format!(r#"{{"id": {}}}"#, i),
            None,
            None,
        )
        .unwrap();
    }

    // Verify all stored in new schema
    assert_eq!(
        count_new_schema_outputs(&db_path),
        5,
        "5 outputs created"
    );

    // Verify all readable
    let artifacts = db.query_artifacts("SPEC-TEST-006", SpecStage::Plan).unwrap();
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
    assert_eq!(implement_artifacts.len(), 1, "Should find 1 implement artifact");
}

#[test]
fn test_write_path_cutover_consistency_under_load() {
    // Test: Consistency maintained under multiple operations
    let (db, _temp, db_path) = create_test_db();

    // Store 20 artifacts rapidly
    for i in 1..=20 {
        let spec_id = format!("SPEC-TEST-{:03}", i);
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
    assert_eq!(
        count_new_schema_outputs(&db_path),
        20,
        "20 outputs created"
    );

    // Verify all 20 specs readable
    for i in 1..=20 {
        let spec_id = format!("SPEC-TEST-{:03}", i);
        let artifacts = db.query_artifacts(&spec_id, SpecStage::Plan).unwrap();
        assert_eq!(
            artifacts.len(),
            1,
            "Spec {} artifact accessible",
            spec_id
        );
    }
}
