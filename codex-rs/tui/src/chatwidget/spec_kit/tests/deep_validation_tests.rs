//! Deep validation enforcement tests (WP-D)
//!
//! These tests ensure deep mode validation cannot be weakened.
//! CI should fail if any deep-required field check is removed.

use crate::chatwidget::spec_kit::intake_core::{validate_project_answers, validate_spec_answers};
use std::collections::HashMap;

/// Helper: Create minimal valid baseline spec answers
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
    answers.insert(
        "acceptance_criteria".into(),
        "AC1 (verify: manual); AC2 (verify: test)".into(),
    );
    answers
}

/// Helper: Add deep-required fields to spec answers
fn add_deep_spec_fields(answers: &mut HashMap<String, String>) {
    // Need 5+ acceptance criteria for deep
    answers.insert("acceptance_criteria".into(),
        "AC1 (verify: manual); AC2 (verify: test); AC3 (verify: e2e); AC4 (verify: unit); AC5 (verify: integration)".into());
    answers.insert(
        "architecture_components".into(),
        "Component A; Component B".into(),
    );
    answers.insert("architecture_dataflows".into(), "A -> B; B -> C".into());
    answers.insert("integration_mapping".into(), "Mapping 1".into());
    answers.insert("test_plan".into(), "Test plan content".into());
    answers.insert("threat_model".into(), "Threat model content".into());
    answers.insert("rollout_plan".into(), "Rollout plan content".into());
}

#[test]
fn test_baseline_spec_validation_passes() {
    let answers = minimal_spec_answers();
    let result = validate_spec_answers(&answers, false);
    assert!(result.valid, "Baseline should pass: {:?}", result.errors);
}

#[test]
fn test_deep_spec_requires_5_acceptance_criteria() {
    let mut answers = minimal_spec_answers();
    add_deep_spec_fields(&mut answers);
    // Override with only 4 AC
    answers.insert(
        "acceptance_criteria".into(),
        "AC1 (verify: manual); AC2 (verify: test); AC3 (verify: e2e); AC4 (verify: unit)".into(),
    );

    let result = validate_spec_answers(&answers, true);
    assert!(!result.valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.to_lowercase().contains("acceptance") && e.contains("5")),
        "Expected error about 5 acceptance criteria, got: {:?}",
        result.errors
    );
}

#[test]
fn test_deep_spec_requires_architecture_components() {
    let mut answers = minimal_spec_answers();
    add_deep_spec_fields(&mut answers);
    answers.remove("architecture_components");

    let result = validate_spec_answers(&answers, true);
    assert!(!result.valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.to_lowercase().contains("architecture")),
        "Expected error about architecture components, got: {:?}",
        result.errors
    );
}

#[test]
fn test_deep_spec_requires_threat_model() {
    let mut answers = minimal_spec_answers();
    add_deep_spec_fields(&mut answers);
    answers.remove("threat_model");

    let result = validate_spec_answers(&answers, true);
    assert!(!result.valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.to_lowercase().contains("threat")),
        "Expected error about threat model, got: {:?}",
        result.errors
    );
}

#[test]
fn test_deep_spec_requires_rollout_plan() {
    let mut answers = minimal_spec_answers();
    add_deep_spec_fields(&mut answers);
    answers.remove("rollout_plan");

    let result = validate_spec_answers(&answers, true);
    assert!(!result.valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.to_lowercase().contains("rollout")),
        "Expected error about rollout plan, got: {:?}",
        result.errors
    );
}

// --- Project validation tests ---

fn minimal_project_answers() -> HashMap<String, String> {
    let mut answers = HashMap::new();
    answers.insert("users".into(), "Developers".into());
    answers.insert("problem".into(), "Need tooling".into());
    answers.insert("artifact_kind".into(), "CLI tool".into());
    answers.insert("goals".into(), "G1; G2; G3".into());
    answers.insert("non_goals".into(), "NG1".into());
    answers.insert("principles".into(), "P1".into());
    answers.insert("guardrails".into(), "GR1".into());
    answers
}

fn add_deep_project_fields(answers: &mut HashMap<String, String>) {
    answers.insert("primary_components".into(), "C1; C2".into());
    answers.insert("deployment_target".into(), "Linux".into());
    answers.insert("data_classification".into(), "Internal".into());
    answers.insert("nfr_budgets".into(), "Latency < 100ms".into());
    answers.insert("ops_baseline".into(), "Ops baseline content".into());
    answers.insert("security_posture".into(), "Security posture content".into());
    answers.insert("release_rollout".into(), "Rollout content".into());
}

#[test]
fn test_baseline_project_validation_passes() {
    let answers = minimal_project_answers();
    let result = validate_project_answers(&answers, false);
    assert!(result.valid, "Baseline should pass: {:?}", result.errors);
}

#[test]
fn test_deep_project_requires_security_posture() {
    let mut answers = minimal_project_answers();
    add_deep_project_fields(&mut answers);
    answers.remove("security_posture");

    let result = validate_project_answers(&answers, true);
    assert!(!result.valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.to_lowercase().contains("security")),
        "Expected error about security posture, got: {:?}",
        result.errors
    );
}

#[test]
fn test_deep_project_requires_release_rollout() {
    let mut answers = minimal_project_answers();
    add_deep_project_fields(&mut answers);
    answers.remove("release_rollout");

    let result = validate_project_answers(&answers, true);
    assert!(!result.valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.to_lowercase().contains("rollout")),
        "Expected error about release rollout, got: {:?}",
        result.errors
    );
}

#[test]
fn test_deep_project_requires_ops_baseline() {
    let mut answers = minimal_project_answers();
    add_deep_project_fields(&mut answers);
    answers.remove("ops_baseline");

    let result = validate_project_answers(&answers, true);
    assert!(!result.valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.to_lowercase().contains("ops")),
        "Expected error about ops baseline, got: {:?}",
        result.errors
    );
}
