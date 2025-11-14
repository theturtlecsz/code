//! Integration tests for new schema (SPEC-945B Phase 1 Complete)
//!
//! Tests new schema behavior through public API:
//! 1. Data not found (empty result case)
//! 2. Write operations succeed and data is persisted
//! 3. Queries return correct data after writes

use codex_tui::{ConsensusDb, SpecStage};
use tempfile::NamedTempFile;

/// Helper: Create test database with migrations applied
fn create_test_db() -> (NamedTempFile, ConsensusDb) {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let db_path = temp_file.path();
    let db = ConsensusDb::init(db_path).expect("Failed to initialize database");
    (temp_file, db)
}

// ============================================================================
// Test 1: Data in NEITHER schema (not found case)
// ============================================================================

#[test]
fn test_query_artifacts_not_found() {
    let (_temp, db) = create_test_db();

    // Query for non-existent spec (tests graceful not-found handling)
    let artifacts = db
        .query_artifacts("SPEC-KIT-NONEXISTENT", SpecStage::Plan)
        .expect("Query should succeed even if no results");

    assert_eq!(artifacts.len(), 0, "Should return empty result");
}

#[test]
fn test_query_synthesis_not_found() {
    let (_temp, db) = create_test_db();

    // Query for non-existent spec (tests graceful not-found handling)
    let synthesis = db
        .query_latest_synthesis("SPEC-KIT-NONEXISTENT", SpecStage::Plan)
        .expect("Query should succeed");

    assert!(synthesis.is_none(), "Should return None when not found");
}

// ============================================================================
// Test 2: Write operations ensure zero data loss
// ============================================================================

#[test]
fn test_dual_write_artifact_zero_data_loss() {
    let (_temp, db) = create_test_db();

    let spec_id = "SPEC-KIT-TEST-001";
    let stage = SpecStage::Plan;

    // Store artifact (writes to new schema)
    let artifact_id = db
        .store_artifact(
            spec_id,
            stage,
            "test-agent",
            r#"{"test": "artifact test"}"#,
            Some("Response text"),
            Some("run-123"),
        )
        .expect("Store artifact failed");

    assert!(artifact_id > 0, "Should return valid ID");

    // Query should return data from new schema
    let artifacts = db.query_artifacts(spec_id, stage).expect("Query failed");

    assert_eq!(artifacts.len(), 1, "Should find artifact after write");
    assert_eq!(artifacts[0].agent_name, "test-agent");
    assert!(artifacts[0].content_json.contains("artifact test"));

    // Verify multiple reads are consistent
    for _i in 0..5 {
        let artifacts_read = db.query_artifacts(spec_id, stage).expect("Query failed");
        assert_eq!(
            artifacts_read.len(),
            1,
            "Should consistently find 1 artifact"
        );
        assert_eq!(artifacts_read[0].agent_name, "test-agent");
    }
}

#[test]
fn test_dual_write_synthesis_zero_data_loss() {
    let (_temp, db) = create_test_db();

    let spec_id = "SPEC-KIT-TEST-002";
    let stage = SpecStage::Validate;

    // Store synthesis (writes to new schema)
    use std::path::Path;
    db.store_synthesis(
        spec_id,
        stage,
        "# Test Synthesis\n\nSynthesis test.",
        Some(Path::new("/tmp/test-output.md")),
        "success",
        3,
        None,
        None,
        false,
        Some("run-456"),
    )
    .expect("Store synthesis failed");

    // Query should return synthesis from new schema
    let synthesis = db
        .query_latest_synthesis(spec_id, stage)
        .expect("Query failed");

    assert!(synthesis.is_some(), "Should find synthesis");
    assert!(synthesis.as_ref().unwrap().contains("Synthesis test"));

    // Verify multiple reads are consistent
    for _i in 0..5 {
        let synthesis_read = db
            .query_latest_synthesis(spec_id, stage)
            .expect("Query failed");
        assert!(
            synthesis_read.is_some(),
            "Should consistently find synthesis"
        );
        assert!(synthesis_read.unwrap().contains("Synthesis test"));
    }
}

// ============================================================================
// Test 3: Multiple artifacts - consistency
// ============================================================================

#[test]
fn test_multiple_artifacts_dual_write() {
    let (_temp, db) = create_test_db();

    let spec_id = "SPEC-KIT-TEST-003";
    let stage = SpecStage::Plan;

    // Store multiple artifacts
    for i in 1..=5 {
        db.store_artifact(
            spec_id,
            stage,
            &format!("agent-{}", i),
            &format!(r#"{{"data": "artifact {i}"}}"#),
            None,
            None,
        )
        .expect("Write failed");
    }

    // Query should return all artifacts
    let artifacts = db.query_artifacts(spec_id, stage).expect("Query failed");

    assert_eq!(artifacts.len(), 5, "Should find all 5 artifacts");

    // Verify all agent names are present
    let agent_names: Vec<String> = artifacts.iter().map(|a| a.agent_name.clone()).collect();
    for i in 1..=5 {
        assert!(
            agent_names.contains(&format!("agent-{}", i)),
            "Should find agent-{}",
            i
        );
    }
}

// ============================================================================
// Test 4: Dual-schema reader handles empty new schema gracefully
// ============================================================================

#[test]
fn test_read_path_migration_gradual_cutover() {
    let (_temp, db) = create_test_db();

    let spec_id_old = "SPEC-KIT-TEST-004-OLD";
    let spec_id_new = "SPEC-KIT-TEST-004-NEW";
    let stage = SpecStage::Plan;

    // Simulate gradual migration:
    // - Some data written before write started (old schema only)
    // - Some data written after write started (both schemas)

    // Write to old schema only (simulating pre-migration data)
    // We can't do this directly without accessing private methods,
    // so instead we just test that write works for new data

    // Write new data (write active)
    db.store_artifact(
        spec_id_new,
        stage,
        "new-agent",
        r#"{"data": "new data"}"#,
        None,
        None,
    )
    .expect("Write failed");

    // Query new data - should work
    let artifacts_new = db
        .query_artifacts(spec_id_new, stage)
        .expect("Query failed");
    assert_eq!(artifacts_new.len(), 1, "Should find new data");

    // Query old data (doesn't exist) - should return empty gracefully
    let artifacts_old = db
        .query_artifacts(spec_id_old, stage)
        .expect("Query should succeed");
    assert_eq!(artifacts_old.len(), 0, "Should handle missing data");
}

// ============================================================================
// Test 5: Dual-schema reader consistency under load
// ============================================================================

#[test]
fn test_dual_schema_reader_consistency() {
    let (_temp, db) = create_test_db();

    let spec_id = "SPEC-KIT-TEST-005";
    let stage = SpecStage::Validate;

    // Write 20 artifacts sequentially
    for i in 1..=20 {
        db.store_artifact(
            spec_id,
            stage,
            &format!("agent-{:02}", i),
            &format!(r#"{{"iteration": {i}}}"#),
            None,
            None,
        )
        .expect("Write failed");
    }

    // Query should return all 20 artifacts
    let artifacts = db.query_artifacts(spec_id, stage).expect("Query failed");

    assert_eq!(artifacts.len(), 20, "Should find all 20 artifacts");

    // Verify all iterations are present
    for i in 1..=20 {
        assert!(
            artifacts
                .iter()
                .any(|a| a.content_json.contains(&format!(r#""iteration": {i}"#))),
            "Should find iteration {}",
            i
        );
    }
}

// ============================================================================
// Test 6: Stage-specific queries
// ============================================================================

#[test]
fn test_stage_specific_queries() {
    let (_temp, db) = create_test_db();

    let spec_id = "SPEC-KIT-TEST-006";

    // Write artifacts to different stages
    db.store_artifact(
        spec_id,
        SpecStage::Plan,
        "plan-agent",
        r#"{"stage": "plan"}"#,
        None,
        None,
    )
    .expect("Plan write failed");

    db.store_artifact(
        spec_id,
        SpecStage::Implement,
        "implement-agent",
        r#"{"stage": "implement"}"#,
        None,
        None,
    )
    .expect("Implement write failed");

    db.store_artifact(
        spec_id,
        SpecStage::Validate,
        "validate-agent",
        r#"{"stage": "validate"}"#,
        None,
        None,
    )
    .expect("Validate write failed");

    // Query each stage - should only return stage-specific artifacts
    let plan_artifacts = db
        .query_artifacts(spec_id, SpecStage::Plan)
        .expect("Plan query failed");
    assert_eq!(plan_artifacts.len(), 1, "Should find 1 plan artifact");
    assert!(
        plan_artifacts[0]
            .content_json
            .contains(r#""stage": "plan""#)
    );

    let implement_artifacts = db
        .query_artifacts(spec_id, SpecStage::Implement)
        .expect("Implement query failed");
    assert_eq!(
        implement_artifacts.len(),
        1,
        "Should find 1 implement artifact"
    );
    assert!(
        implement_artifacts[0]
            .content_json
            .contains(r#""stage": "implement""#)
    );

    let validate_artifacts = db
        .query_artifacts(spec_id, SpecStage::Validate)
        .expect("Validate query failed");
    assert_eq!(
        validate_artifacts.len(),
        1,
        "Should find 1 validate artifact"
    );
    assert!(
        validate_artifacts[0]
            .content_json
            .contains(r#""stage": "validate""#)
    );
}
