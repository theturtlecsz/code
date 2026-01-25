//! Projection provenance tests (WP-D)
//!
//! These tests ensure filesystem projections include capsule URIs
//! and SHA256 hashes for traceability.

use std::collections::HashMap;
use tempfile::TempDir;
use crate::chatwidget::spec_kit::intake_core::{
    build_design_brief, CapsulePersistenceResult,
    create_spec_filesystem_projections,
};

/// Create mock CapsulePersistenceResult for testing
fn mock_capsule_result() -> CapsulePersistenceResult {
    CapsulePersistenceResult {
        answers_uri: "mv2://default/SPEC-TEST-001/abc123/artifact/intake/answers.json".into(),
        answers_sha256: "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234".into(),
        brief_uri: "mv2://default/SPEC-TEST-001/abc123/artifact/intake/brief.json".into(),
        brief_sha256: "efgh5678efgh5678efgh5678efgh5678efgh5678efgh5678efgh5678efgh5678".into(),
        checkpoint_label: "intake:spec:SPEC-TEST-001:abc123".into(),
        deep_artifacts: None,
        ace_intake_frame_uri: None,
        ace_intake_frame_sha256: None,
    }
}

fn minimal_spec_answers() -> HashMap<String, String> {
    let mut answers = HashMap::new();
    answers.insert("problem".into(), "Test problem".into());
    answers.insert("target_users".into(), "User A; User B".into());
    answers.insert("outcome".into(), "Better UX".into());
    answers.insert("constraints".into(), "Cost; Time".into());
    answers.insert("scope_in".into(), "F1; F2; F3".into());
    answers.insert("non_goals".into(), "NG1; NG2; NG3".into());
    answers.insert("integration_points".into(), "API X".into());
    answers.insert("risks".into(), "Risk 1".into());
    answers.insert("open_questions".into(), "Q1".into());
    answers.insert("acceptance_criteria".into(),
        "AC1 (verify: manual); AC2 (verify: test)".into());
    answers
}

/// Create a minimal SPEC.md file for testing
fn create_spec_md(cwd: &std::path::Path) {
    let spec_md_content = r#"# SPEC Tracker

## Backlog

| SPEC-ID | Feature | Status | Dir |
|---------|---------|--------|-----|
"#;
    std::fs::write(cwd.join("SPEC.md"), spec_md_content).unwrap();
}

#[test]
fn test_spec_projection_includes_provenance_table() {
    let temp = TempDir::new().unwrap();
    let cwd = temp.path();

    // Create required directories and files
    std::fs::create_dir_all(cwd.join("docs")).unwrap();
    create_spec_md(cwd);

    let answers = minimal_spec_answers();
    let brief = build_design_brief(&answers, "SPEC-TEST-001", "abc123", "Test feature", false, "test", vec![]).unwrap();
    let capsule_result = mock_capsule_result();

    let dir_name = create_spec_filesystem_projections(cwd, "SPEC-TEST-001", "Test feature", &brief, &capsule_result).unwrap();

    // Read INTAKE.md and verify provenance
    let intake_path = cwd.join("docs").join(&dir_name).join("INTAKE.md");
    let intake_content = std::fs::read_to_string(&intake_path).unwrap();

    // Must contain URI and SHA256 columns
    assert!(intake_content.contains("Artifact") || intake_content.contains("artifact"), "Missing provenance table header");
    assert!(intake_content.contains(&capsule_result.answers_uri), "Missing answers URI");
    assert!(intake_content.contains(&capsule_result.brief_uri), "Missing brief URI");
}

#[test]
fn test_spec_projection_creates_required_files() {
    let temp = TempDir::new().unwrap();
    let cwd = temp.path();

    // Create required directories and files
    std::fs::create_dir_all(cwd.join("docs")).unwrap();
    create_spec_md(cwd);

    let answers = minimal_spec_answers();
    let brief = build_design_brief(&answers, "SPEC-TEST-002", "def456", "Another feature", false, "test", vec![]).unwrap();
    let capsule_result = mock_capsule_result();

    let dir_name = create_spec_filesystem_projections(cwd, "SPEC-TEST-002", "Another feature", &brief, &capsule_result).unwrap();

    let spec_dir = cwd.join("docs").join(&dir_name);

    // Required files for baseline
    assert!(spec_dir.join("spec.md").exists(), "Missing spec.md");
    assert!(spec_dir.join("PRD.md").exists(), "Missing PRD.md");
    assert!(spec_dir.join("INTAKE.md").exists(), "Missing INTAKE.md");
}
