// Integration tests for SPEC-945B Write-Path Cutover (Week 2 Day 6)
//
// Tests verify:
// 1. Writes go to NEW schema only (consensus_runs + agent_outputs)
// 2. Old schema is NOT written to (remains empty)
// 3. Reads still work via dual-schema reader
// 4. Zero data loss (all writes accessible)

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

/// Test helper: Count records in old schema
fn count_old_schema_artifacts(db_path: &PathBuf) -> i64 {
    let conn = Connection::open(db_path).unwrap();
    conn.query_row("SELECT COUNT(*) FROM consensus_artifacts", [], |row| {
        row.get(0)
    })
    .unwrap()
}

fn count_old_schema_synthesis(db_path: &PathBuf) -> i64 {
    let conn = Connection::open(db_path).unwrap();
    conn.query_row("SELECT COUNT(*) FROM consensus_synthesis", [], |row| {
        row.get(0)
    })
    .unwrap()
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
    // Test: store_artifact() writes to NEW schema only, NOT old schema
    let (db, _temp, db_path) = create_test_db();

    // Verify old schema is empty before write
    assert_eq!(count_old_schema_artifacts(&db_path), 0);
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

    // Verify NEW schema has data (run + output)
    assert_eq!(count_new_schema_runs(&db_path), 1, "New schema run created");
    assert_eq!(
        count_new_schema_outputs(&db_path),
        1,
        "New schema output created"
    );

    // Verify OLD schema remains EMPTY (write-path cutover)
    assert_eq!(
        count_old_schema_artifacts(&db_path),
        0,
        "Old schema NOT written to"
    );
}

#[test]
fn test_write_path_cutover_synthesis_new_schema_only() {
    // Test: store_synthesis() writes to NEW schema only, NOT old schema
    let (db, _temp, db_path) = create_test_db();

    // Verify old schema is empty before write
    assert_eq!(count_old_schema_synthesis(&db_path), 0);
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

    // Verify NEW schema has data (run with synthesis_json)
    assert_eq!(count_new_schema_runs(&db_path), 1, "New schema run created");

    // Verify OLD schema remains EMPTY (write-path cutover)
    assert_eq!(
        count_old_schema_synthesis(&db_path),
        0,
        "Old schema NOT written to"
    );
}

#[test]
fn test_write_path_cutover_read_fallback_still_works() {
    // Test: After cutover, reads still work for new data
    // (Dual-schema reader should read from new schema now)
    let (db, _temp, _db_path) = create_test_db();

    // Store artifact (goes to new schema)
    let result = db.store_artifact(
        "SPEC-TEST-003",
        SpecStage::Implement,
        "gpt_codex",
        r#"{"code": "fn main() {}"}"#,
        Some("Code implementation"),
        Some("run-789"),
    );
    assert!(result.is_ok());

    // Read artifact (should find it in new schema)
    let artifacts = db.query_artifacts("SPEC-TEST-003", SpecStage::Implement);
    assert!(artifacts.is_ok());
    let artifacts = artifacts.unwrap();
    assert_eq!(artifacts.len(), 1, "Artifact found in new schema");
    assert_eq!(artifacts[0].agent_name, "gpt_codex");
    assert_eq!(artifacts[0].stage, "spec-implement"); // New schema uses "spec-*" prefix
}

#[test]
fn test_write_path_cutover_synthesis_read_fallback() {
    // Test: After cutover, synthesis reads work for new data
    let (db, _temp, _db_path) = create_test_db();

    // Store synthesis (goes to new schema)
    let result = db.store_synthesis(
        "SPEC-TEST-004",
        SpecStage::Validate,
        "# Test Plan\n## Tests\n- Test 1\n- Test 2",
        None,
        "success",
        2,
        Some("3/3 consensus"),
        None,
        false,
        Some("run-999"),
    );
    assert!(result.is_ok());

    // Read synthesis (should find it in new schema)
    let synthesis = db.query_latest_synthesis("SPEC-TEST-004", SpecStage::Validate);
    assert!(synthesis.is_ok());
    let synthesis = synthesis.unwrap();
    assert!(synthesis.is_some(), "Synthesis found in new schema");
    assert!(synthesis.unwrap().contains("Test Plan"));
}

#[test]
fn test_write_path_cutover_multiple_artifacts_new_schema() {
    // Test: Multiple artifacts write to new schema only
    let (db, _temp, db_path) = create_test_db();

    // Store 5 artifacts (all should go to new schema)
    for i in 1..=5 {
        let result = db.store_artifact(
            "SPEC-TEST-005",
            SpecStage::Plan,
            &format!("agent-{}", i),
            &format!(r#"{{"plan": "Plan {}", "version": {}}}"#, i, i),
            Some(&format!("Response {}", i)),
            Some(&format!("run-{}", i)),
        );
        assert!(result.is_ok());
    }

    // Verify NEW schema has all 5 outputs
    // Note: Each store_artifact call creates its own run (different timestamp)
    assert_eq!(count_new_schema_runs(&db_path), 5, "5 runs created");
    assert_eq!(
        count_new_schema_outputs(&db_path),
        5,
        "5 outputs created"
    );

    // Verify OLD schema remains EMPTY
    assert_eq!(
        count_old_schema_artifacts(&db_path),
        0,
        "Old schema empty"
    );

    // Verify all 5 artifacts readable
    let artifacts = db.query_artifacts("SPEC-TEST-005", SpecStage::Plan);
    assert!(artifacts.is_ok());
    let artifacts = artifacts.unwrap();
    assert_eq!(artifacts.len(), 5, "All 5 artifacts readable");
}

#[test]
fn test_write_path_cutover_zero_data_loss() {
    // Test: Zero data loss - all writes immediately accessible
    let (db, _temp, _db_path) = create_test_db();

    // Store artifact + synthesis for same SPEC/stage
    let artifact_id = db
        .store_artifact(
            "SPEC-TEST-006",
            SpecStage::Audit,
            "claude",
            r#"{"security": "PASS", "compliance": "PASS"}"#,
            Some("Security audit passed"),
            Some("run-audit-1"),
        )
        .unwrap();
    assert!(artifact_id > 0, "Artifact stored successfully");

    let synthesis_id = db
        .store_synthesis(
            "SPEC-TEST-006",
            SpecStage::Audit,
            "# Audit Results\n✅ All checks passed",
            None,
            "success",
            1,
            Some("Unanimous approval"),
            None,
            false,
            Some("run-audit-1"),
        )
        .unwrap();
    assert!(synthesis_id > 0, "Synthesis stored successfully");

    // Verify both immediately accessible
    let artifacts = db
        .query_artifacts("SPEC-TEST-006", SpecStage::Audit)
        .unwrap();
    assert_eq!(artifacts.len(), 1, "Artifact accessible");

    let synthesis = db
        .query_latest_synthesis("SPEC-TEST-006", SpecStage::Audit)
        .unwrap();
    assert!(synthesis.is_some(), "Synthesis accessible");
    assert!(synthesis.unwrap().contains("All checks passed"));
}

#[test]
fn test_write_path_cutover_consistency_under_load() {
    // Test: Consistency under load (20 writes)
    let (db, _temp, db_path) = create_test_db();

    // Store 20 artifacts for different specs
    for i in 1..=20 {
        let spec_id = format!("SPEC-TEST-{:03}", i);
        let result = db.store_artifact(
            &spec_id,
            SpecStage::Plan,
            "gemini",
            &format!(r#"{{"iteration": {}}}"#, i),
            Some(&format!("Response {}", i)),
            Some(&format!("run-{}", i)),
        );
        assert!(result.is_ok(), "Write {} succeeded", i);
    }

    // Verify all 20 writes went to new schema
    assert_eq!(count_new_schema_runs(&db_path), 20, "20 runs created");
    assert_eq!(
        count_new_schema_outputs(&db_path),
        20,
        "20 outputs created"
    );

    // Verify old schema remains empty
    assert_eq!(
        count_old_schema_artifacts(&db_path),
        0,
        "Old schema empty"
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

#[test]
fn test_write_path_cutover_stage_specific_isolation() {
    // Test: Stage-specific queries work correctly
    let (db, _temp, _db_path) = create_test_db();

    // Store artifacts for 3 different stages
    db.store_artifact(
        "SPEC-TEST-007",
        SpecStage::Plan,
        "gemini",
        r#"{"plan": "yes"}"#,
        None,
        Some("run-1"),
    )
    .unwrap();

    db.store_artifact(
        "SPEC-TEST-007",
        SpecStage::Tasks,
        "gpt_pro",
        r#"{"tasks": "yes"}"#,
        None,
        Some("run-2"),
    )
    .unwrap();

    db.store_artifact(
        "SPEC-TEST-007",
        SpecStage::Implement,
        "gpt_codex",
        r#"{"code": "yes"}"#,
        None,
        Some("run-3"),
    )
    .unwrap();

    // Verify each stage query returns only its artifact
    let plan = db
        .query_artifacts("SPEC-TEST-007", SpecStage::Plan)
        .unwrap();
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].stage, "spec-plan"); // New schema uses "spec-*" prefix

    let tasks = db
        .query_artifacts("SPEC-TEST-007", SpecStage::Tasks)
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].stage, "spec-tasks"); // New schema uses "spec-*" prefix

    let implement = db
        .query_artifacts("SPEC-TEST-007", SpecStage::Implement)
        .unwrap();
    assert_eq!(implement.len(), 1);
    assert_eq!(implement[0].stage, "spec-implement"); // New schema uses "spec-*" prefix
}
