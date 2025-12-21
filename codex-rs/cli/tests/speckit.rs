//! Spec-Kit CLI Integration Tests
//!
//! Tests exit codes per REVIEW-CONTRACT.md and determinism guarantees.
//!
//! ## Exit Codes
//! - 0: Success / proceed
//! - 1: Soft fail (warnings in strict mode)
//! - 2: Hard fail (escalation / missing artifacts in strict mode)
//! - 3: Infrastructure error

use std::fs;
use std::path::Path;

use anyhow::Result;
use serde_json::Value as JsonValue;
use tempfile::TempDir;

/// Create a codex command with isolated CODEX_HOME and custom working directory
fn codex_command(codex_home: &Path, cwd: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::cargo_bin("code")?;
    cmd.env("CODEX_HOME", codex_home);
    cmd.current_dir(cwd);
    Ok(cmd)
}

/// Create evidence directory structure for a SPEC
/// Uses the hardcoded path from executor: docs/SPEC-OPS-004-integrated-coder-hooks/evidence
fn setup_evidence_dir(repo_root: &Path, spec_id: &str) -> std::io::Result<std::path::PathBuf> {
    let evidence_dir = repo_root
        .join("docs")
        .join("SPEC-OPS-004-integrated-coder-hooks")
        .join("evidence")
        .join("consensus")
        .join(spec_id);
    fs::create_dir_all(&evidence_dir)?;
    Ok(evidence_dir)
}

/// Create a consensus JSON file with specified conflicts
fn create_consensus_file(
    dir: &Path,
    stage: &str,
    agent: &str,
    timestamp: &str,
    conflicts: &[&str],
    synthesis_status: &str,
) -> std::io::Result<()> {
    let filename = format!("spec-{stage}_{agent}_{timestamp}.json");
    // Conflicts are plain strings, not objects
    let conflicts_json: Vec<&str> = conflicts.to_vec();

    let content = serde_json::json!({
        "agent": agent,
        "model": "test-model",
        "consensus": {
            "conflicts": conflicts_json,
            "synthesis_status": synthesis_status
        }
    });

    fs::write(dir.join(&filename), serde_json::to_string_pretty(&content)?)?;
    Ok(())
}

// =============================================================================
// EXIT CODE TESTS
// =============================================================================

#[test]
fn review_no_artifacts_exits_0_without_strict() -> Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // No evidence files created - empty directory
    setup_evidence_dir(repo_root.path(), "SPEC-TEST-001")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args(["speckit", "review", "--spec", "SPEC-TEST-001", "--stage", "plan"])
        .output()?;

    // Without --strict-artifacts, missing artifacts should exit 0
    assert!(
        output.status.success(),
        "Expected exit 0 without strict mode, got {:?}",
        output.status.code()
    );
    Ok(())
}

#[test]
fn review_no_artifacts_exits_2_with_strict() -> Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // No evidence files created - empty directory
    setup_evidence_dir(repo_root.path(), "SPEC-TEST-002")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-002",
            "--stage",
            "plan",
            "--strict-artifacts",
        ])
        .output()?;

    // With --strict-artifacts, missing artifacts should exit 2
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit 2 with strict-artifacts, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn review_clean_consensus_exits_0() -> Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    let evidence_dir = setup_evidence_dir(repo_root.path(), "SPEC-TEST-003")?;
    create_consensus_file(
        &evidence_dir,
        "plan",
        "architect",
        "20251220",
        &[], // No conflicts
        "clean",
    )?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args(["speckit", "review", "--spec", "SPEC-TEST-003", "--stage", "plan"])
        .output()?;

    assert!(
        output.status.success(),
        "Expected exit 0 for clean consensus, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn review_conflicts_exits_2() -> Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    let evidence_dir = setup_evidence_dir(repo_root.path(), "SPEC-TEST-004")?;
    create_consensus_file(
        &evidence_dir,
        "plan",
        "architect",
        "20251220",
        &["Risk: API change without migration"], // Has conflicts
        "conflicts_present",
    )?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args(["speckit", "review", "--spec", "SPEC-TEST-004", "--stage", "plan"])
        .output()?;

    // Conflicts should cause exit 2 (hard fail / escalation)
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit 2 for conflicts, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

// =============================================================================
// DETERMINISM TESTS
// =============================================================================

#[test]
fn review_deterministic_output() -> Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    let evidence_dir = setup_evidence_dir(repo_root.path(), "SPEC-TEST-005")?;

    // Create two consensus files with different timestamps
    create_consensus_file(
        &evidence_dir,
        "plan",
        "architect",
        "20251219", // Earlier
        &["Conflict A from architect"],
        "conflicts_present",
    )?;
    create_consensus_file(
        &evidence_dir,
        "plan",
        "implementer",
        "20251220", // Later
        &["Conflict B from implementer"],
        "conflicts_present",
    )?;

    // Run twice and compare JSON outputs
    let mut cmd1 = codex_command(codex_home.path(), repo_root.path())?;
    let output1 = cmd1
        .args([
            "speckit", "review", "--spec", "SPEC-TEST-005", "--stage", "plan", "--json",
        ])
        .output()?;

    let mut cmd2 = codex_command(codex_home.path(), repo_root.path())?;
    let output2 = cmd2
        .args([
            "speckit", "review", "--spec", "SPEC-TEST-005", "--stage", "plan", "--json",
        ])
        .output()?;

    // Same exit codes
    assert_eq!(
        output1.status.code(),
        output2.status.code(),
        "Exit codes should be deterministic"
    );

    // Parse JSON and compare blocking_signals order
    let json1: JsonValue = serde_json::from_slice(&output1.stdout)?;
    let json2: JsonValue = serde_json::from_slice(&output2.stdout)?;

    let signals1 = json1
        .get("blocking_signals")
        .and_then(|v| v.as_array())
        .expect("blocking_signals array");
    let signals2 = json2
        .get("blocking_signals")
        .and_then(|v| v.as_array())
        .expect("blocking_signals array");

    assert_eq!(signals1.len(), signals2.len(), "Signal count should match");

    for (i, (s1, s2)) in signals1.iter().zip(signals2.iter()).enumerate() {
        let msg1 = s1.get("message").and_then(|v| v.as_str());
        let msg2 = s2.get("message").and_then(|v| v.as_str());
        assert_eq!(msg1, msg2, "Signal {i} should be in same position");
    }

    // Verify lexicographic ordering: architect (20251219) before implementer (20251220)
    if signals1.len() >= 2 {
        let first_msg = signals1[0].get("message").and_then(|v| v.as_str()).unwrap_or("");
        let second_msg = signals1[1].get("message").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            first_msg.contains("architect") || first_msg.contains("Conflict A"),
            "First signal should be from architect file (lexicographically first)"
        );
        assert!(
            second_msg.contains("implementer") || second_msg.contains("Conflict B"),
            "Second signal should be from implementer file"
        );
    }

    Ok(())
}

// =============================================================================
// JSON OUTPUT TESTS
// =============================================================================

#[test]
fn review_json_output_structure() -> Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    let evidence_dir = setup_evidence_dir(repo_root.path(), "SPEC-TEST-006")?;
    create_consensus_file(&evidence_dir, "plan", "architect", "20251220", &[], "clean")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit", "review", "--spec", "SPEC-TEST-006", "--stage", "plan", "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Verify expected fields exist
    assert!(json.get("spec_id").is_some(), "Missing spec_id");
    assert!(json.get("stage").is_some(), "Missing stage");
    assert!(json.get("verdict").is_some(), "Missing verdict");
    assert!(json.get("exit_code").is_some(), "Missing exit_code");
    assert!(json.get("blocking_signals").is_some(), "Missing blocking_signals");
    assert!(json.get("advisory_signals").is_some(), "Missing advisory_signals");

    // Verify spec_id matches
    assert_eq!(
        json.get("spec_id").and_then(|v| v.as_str()),
        Some("SPEC-TEST-006")
    );

    Ok(())
}

#[test]
fn review_skipped_json_output() -> Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // No evidence directory at all - completely missing
    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit", "review", "--spec", "SPEC-NONEXISTENT", "--stage", "plan", "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Should have skipped verdict
    let verdict = json.get("verdict").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        verdict.contains("Skipped"),
        "Expected Skipped verdict, got: {verdict}"
    );

    Ok(())
}

// =============================================================================
// STATUS COMMAND TESTS
// =============================================================================

#[test]
fn status_json_output_structure() -> Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create evidence dir (even if empty)
    setup_evidence_dir(repo_root.path(), "SPEC-TEST-007")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args(["speckit", "status", "--spec", "SPEC-TEST-007", "--json"])
        .output()?;

    assert!(
        output.status.success(),
        "Status command should succeed even with no evidence"
    );

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Verify expected fields
    assert!(json.get("spec_id").is_some(), "Missing spec_id");
    assert!(json.get("generated_at").is_some(), "Missing generated_at");
    assert!(json.get("evidence").is_some(), "Missing evidence");

    Ok(())
}

// =============================================================================
// ALL STAGES TESTS
// =============================================================================

#[test]
fn review_accepts_all_valid_stages() -> Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    setup_evidence_dir(repo_root.path(), "SPEC-TEST-STAGES")?;

    let stages = ["specify", "plan", "tasks", "implement", "validate", "audit", "unlock"];

    for stage in stages {
        let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
        let output = cmd
            .args(["speckit", "review", "--spec", "SPEC-TEST-STAGES", "--stage", stage])
            .output()?;

        // Should not exit with infrastructure error (3)
        assert_ne!(
            output.status.code(),
            Some(3),
            "Stage '{stage}' should be recognized, got exit 3"
        );
    }

    Ok(())
}

#[test]
fn review_rejects_invalid_stage() -> Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    setup_evidence_dir(repo_root.path(), "SPEC-TEST-INVALID")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-INVALID",
            "--stage",
            "invalid-stage",
        ])
        .output()?;

    // Invalid stage is a user input error, exits via anyhow (not exit 0)
    // Note: exit 3 is for infrastructure errors (I/O failures, etc.)
    assert!(
        !output.status.success(),
        "Invalid stage should fail, got {:?}",
        output.status.code()
    );

    // Should contain helpful error message
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unknown stage") || stderr.contains("invalid-stage"),
        "Should show error about invalid stage, got: {stderr}"
    );

    Ok(())
}

// =============================================================================
// P0-3: REVIEW EVIDENCE SEMANTICS TESTS
// =============================================================================

#[test]
fn review_spec_docs_only_no_consensus_exits_skipped() -> Result<()> {
    // P0-3: spec docs (plan.md) existing WITHOUT review evidence (consensus files)
    // should result in Skipped, not Passed
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create evidence dir structure (empty - no consensus files)
    setup_evidence_dir(repo_root.path(), "SPEC-TEST-P03")?;

    // Create a spec packet directory with plan.md (spec docs exist)
    let spec_packet_dir = repo_root.path().join("docs").join("SPEC-TEST-P03");
    fs::create_dir_all(&spec_packet_dir)?;
    fs::write(spec_packet_dir.join("plan.md"), "# Plan\n\nThis is a plan.")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit", "review", "--spec", "SPEC-TEST-P03", "--stage", "plan", "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Should be Skipped because no review evidence (consensus files)
    let verdict = json.get("verdict").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        verdict.contains("Skipped"),
        "Expected Skipped verdict when no consensus evidence, got: {verdict}"
    );

    // Should indicate NoArtifactsFound reason
    assert!(
        verdict.contains("NoArtifactsFound"),
        "Expected NoArtifactsFound reason, got: {verdict}"
    );

    Ok(())
}

#[test]
fn review_spec_docs_only_no_consensus_strict_exits_2() -> Result<()> {
    // P0-3: With --strict-artifacts, missing review evidence should exit 2
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create evidence dir structure (empty - no consensus files)
    setup_evidence_dir(repo_root.path(), "SPEC-TEST-P03-STRICT")?;

    // Create a spec packet directory with plan.md (spec docs exist)
    let spec_packet_dir = repo_root.path().join("docs").join("SPEC-TEST-P03-STRICT");
    fs::create_dir_all(&spec_packet_dir)?;
    fs::write(spec_packet_dir.join("plan.md"), "# Plan\n\nThis is a plan.")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-P03-STRICT",
            "--stage",
            "plan",
            "--strict-artifacts",
        ])
        .output()?;

    // With --strict-artifacts, missing review evidence should exit 2
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit 2 with strict-artifacts and no consensus evidence, got {:?}",
        output.status.code()
    );

    Ok(())
}
