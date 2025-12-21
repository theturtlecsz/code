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
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-001",
            "--stage",
            "plan",
        ])
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
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-003",
            "--stage",
            "plan",
        ])
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
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-004",
            "--stage",
            "plan",
        ])
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
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-005",
            "--stage",
            "plan",
            "--json",
        ])
        .output()?;

    let mut cmd2 = codex_command(codex_home.path(), repo_root.path())?;
    let output2 = cmd2
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-005",
            "--stage",
            "plan",
            "--json",
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
        let first_msg = signals1[0]
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let second_msg = signals1[1]
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
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
fn review_json_includes_schema_version() -> Result<()> {
    // P3-A: All JSON outputs must include schema_version and tool_version
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    let evidence_dir = setup_evidence_dir(repo_root.path(), "SPEC-TEST-SCHEMA")?;
    create_consensus_file(&evidence_dir, "plan", "architect", "20251220", &[], "clean")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-SCHEMA",
            "--stage",
            "plan",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Verify schema_version is present and is integer 1
    let schema_version = json.get("schema_version");
    assert!(
        schema_version.is_some(),
        "Missing schema_version in review JSON"
    );
    assert_eq!(
        schema_version.and_then(|v| v.as_u64()),
        Some(1),
        "schema_version should be 1"
    );

    // Verify tool_version is present and has correct format (version+sha or just version)
    let tool_version = json.get("tool_version");
    assert!(
        tool_version.is_some(),
        "Missing tool_version in review JSON"
    );
    let version_str = tool_version
        .and_then(|v| v.as_str())
        .expect("tool_version string");
    // Should be semver-ish: "0.0.0" or "0.0.0+abc1234"
    assert!(
        version_str.starts_with("0."),
        "tool_version should start with version number, got: {version_str}"
    );

    Ok(())
}

#[test]
fn status_json_includes_schema_version() -> Result<()> {
    // P3-A: Status JSON must also include schema_version and tool_version
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    setup_evidence_dir(repo_root.path(), "SPEC-TEST-STATUS-SCHEMA")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "status",
            "--spec",
            "SPEC-TEST-STATUS-SCHEMA",
            "--json",
        ])
        .output()?;

    assert!(output.status.success(), "Status command should succeed");

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Verify schema_version is present and is integer 1
    let schema_version = json.get("schema_version");
    assert!(
        schema_version.is_some(),
        "Missing schema_version in status JSON"
    );
    assert_eq!(
        schema_version.and_then(|v| v.as_u64()),
        Some(1),
        "schema_version should be 1"
    );

    // Verify tool_version is present
    let tool_version = json.get("tool_version");
    assert!(
        tool_version.is_some(),
        "Missing tool_version in status JSON"
    );
    let version_str = tool_version
        .and_then(|v| v.as_str())
        .expect("tool_version string");
    assert!(
        version_str.starts_with("0."),
        "tool_version should start with version number, got: {version_str}"
    );

    Ok(())
}

#[test]
fn review_skipped_json_includes_schema_version() -> Result<()> {
    // P3-A: ReviewSkipped JSON must also include version fields
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // No evidence at all - will trigger Skipped
    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-NONEXISTENT-SCHEMA",
            "--stage",
            "plan",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Should have Skipped verdict
    let verdict = json.get("verdict").and_then(|v| v.as_str()).unwrap_or("");
    assert!(verdict.contains("Skipped"), "Expected Skipped verdict");

    // Still should have version fields
    assert!(
        json.get("schema_version").is_some(),
        "Missing schema_version in skipped review JSON"
    );
    assert!(
        json.get("tool_version").is_some(),
        "Missing tool_version in skipped review JSON"
    );

    Ok(())
}

#[test]
fn review_json_output_structure() -> Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    let evidence_dir = setup_evidence_dir(repo_root.path(), "SPEC-TEST-006")?;
    create_consensus_file(&evidence_dir, "plan", "architect", "20251220", &[], "clean")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-006",
            "--stage",
            "plan",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Verify expected fields exist
    assert!(json.get("spec_id").is_some(), "Missing spec_id");
    assert!(json.get("stage").is_some(), "Missing stage");
    assert!(json.get("verdict").is_some(), "Missing verdict");
    assert!(json.get("exit_code").is_some(), "Missing exit_code");
    assert!(
        json.get("blocking_signals").is_some(),
        "Missing blocking_signals"
    );
    assert!(
        json.get("advisory_signals").is_some(),
        "Missing advisory_signals"
    );

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
            "speckit",
            "review",
            "--spec",
            "SPEC-NONEXISTENT",
            "--stage",
            "plan",
            "--json",
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

    // Verify core fields
    assert!(json.get("spec_id").is_some(), "Missing spec_id");
    assert!(json.get("generated_at").is_some(), "Missing generated_at");
    assert!(json.get("stale_hours").is_some(), "Missing stale_hours");

    // Verify packet section
    let packet = json.get("packet");
    assert!(packet.is_some(), "Missing packet section");
    assert!(packet.unwrap().get("docs").is_some(), "Missing packet.docs");

    // Verify stages array
    let stages = json.get("stages");
    assert!(stages.is_some(), "Missing stages array");
    assert!(
        stages
            .unwrap()
            .as_array()
            .map(std::vec::Vec::len)
            .unwrap_or(0)
            > 0,
        "Stages array should not be empty"
    );

    // Verify evidence section (repo-relative paths)
    let evidence = json.get("evidence");
    assert!(evidence.is_some(), "Missing evidence section");
    let evidence = evidence.unwrap();
    assert!(
        evidence.get("commands_bytes").is_some(),
        "Missing evidence.commands_bytes"
    );
    assert!(
        evidence.get("consensus_bytes").is_some(),
        "Missing evidence.consensus_bytes"
    );
    assert!(
        evidence.get("top_entries").is_some(),
        "Missing evidence.top_entries (repo-relative paths)"
    );

    // Verify warnings array
    assert!(json.get("warnings").is_some(), "Missing warnings array");

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

    let stages = [
        "specify",
        "plan",
        "tasks",
        "implement",
        "validate",
        "audit",
        "unlock",
    ];

    for stage in stages {
        let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
        let output = cmd
            .args([
                "speckit",
                "review",
                "--spec",
                "SPEC-TEST-STAGES",
                "--stage",
                stage,
            ])
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
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-P03",
            "--stage",
            "plan",
            "--json",
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

// =============================================================================
// P1-C: --strict-schema TESTS
// =============================================================================

#[test]
fn review_malformed_json_exits_3_with_strict_schema() -> Result<()> {
    // P1-C: With --strict-schema, parse errors should exit 3
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create evidence directory
    let evidence_dir = setup_evidence_dir(repo_root.path(), "SPEC-TEST-STRICT-SCHEMA")?;

    // Create malformed JSON consensus file
    let malformed_file = evidence_dir.join("spec-plan_broken_20251220.json");
    fs::write(&malformed_file, "{ this is not valid json }")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-STRICT-SCHEMA",
            "--stage",
            "plan",
            "--strict-schema",
        ])
        .output()?;

    // With --strict-schema, parse errors should exit 3 (infrastructure error)
    assert_eq!(
        output.status.code(),
        Some(3),
        "Expected exit 3 with --strict-schema and malformed JSON, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    // Error message should mention parse/schema errors
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Parse/schema errors") || stderr.contains("parse"),
        "Expected error message about parse errors, got: {stderr}"
    );

    Ok(())
}

#[test]
fn review_malformed_json_exits_0_without_strict_schema() -> Result<()> {
    // P1-C: Without --strict-schema, parse errors are advisory (exit 0)
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create evidence directory
    let evidence_dir = setup_evidence_dir(repo_root.path(), "SPEC-TEST-NO-STRICT")?;

    // Create malformed JSON consensus file
    let malformed_file = evidence_dir.join("spec-plan_broken_20251220.json");
    fs::write(&malformed_file, "{ this is not valid json }")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-NO-STRICT",
            "--stage",
            "plan",
            "--json",
        ])
        .output()?;

    // Without --strict-schema, should exit 0 (parse error is advisory)
    assert!(
        output.status.success(),
        "Expected exit 0 without strict-schema, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    // Should show advisory signal in output
    let json: JsonValue = serde_json::from_slice(&output.stdout)?;
    let advisory = json.get("advisory_signals").and_then(|v| v.as_array());
    assert!(
        advisory.map(|a| !a.is_empty()).unwrap_or(false),
        "Expected advisory signals for parse error"
    );

    Ok(())
}

// =============================================================================
// P2-C: --explain FLAG TESTS
// =============================================================================

#[test]
fn review_explain_flag_adds_explanation_to_json() -> Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    let evidence_dir = setup_evidence_dir(repo_root.path(), "SPEC-TEST-EXPLAIN")?;
    create_consensus_file(
        &evidence_dir,
        "plan",
        "architect",
        "20251220",
        &["Test conflict for explanation"],
        "conflicts_present",
    )?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-EXPLAIN",
            "--stage",
            "plan",
            "--explain",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Should have explanation section
    let explanation = json.get("explanation");
    assert!(explanation.is_some(), "Missing explanation section");

    let explanation = explanation.unwrap();
    assert!(
        explanation.get("summary").is_some(),
        "Missing explanation.summary"
    );
    assert!(
        explanation.get("reasons").is_some(),
        "Missing explanation.reasons"
    );
    assert!(
        explanation.get("flags_active").is_some(),
        "Missing explanation.flags_active"
    );

    // Verify conflict is in reasons
    let reasons = explanation
        .get("reasons")
        .and_then(|v| v.as_array())
        .expect("reasons array");
    assert!(!reasons.is_empty(), "Expected reasons for conflict case");

    Ok(())
}

#[test]
fn review_explain_flag_works_without_json() -> Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    let evidence_dir = setup_evidence_dir(repo_root.path(), "SPEC-TEST-EXPLAIN-TEXT")?;
    create_consensus_file(&evidence_dir, "plan", "architect", "20251220", &[], "clean")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "review",
            "--spec",
            "SPEC-TEST-EXPLAIN-TEXT",
            "--stage",
            "plan",
            "--explain",
        ])
        .output()?;

    // Should succeed
    assert!(
        output.status.success(),
        "Expected exit 0 for clean case with --explain"
    );

    // Should contain explanation text
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Exit Code Explanation"),
        "Should contain explanation section, got: {stdout}"
    );
    assert!(stdout.contains("Exit code: 0"), "Should show exit code 0");

    Ok(())
}

// =============================================================================
// P3-B: PLAN COMMAND TESTS
// =============================================================================

#[test]
fn plan_validates_spec_and_exits_0() -> Result<()> {
    // P3-B: Plan command validates SPEC and exits 0 when ready
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory with plan.md
    let spec_dir = repo_root.path().join("docs").join("SPEC-TEST-PLAN");
    fs::create_dir_all(&spec_dir)?;
    fs::write(spec_dir.join("plan.md"), "# Plan\nTest plan.")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args(["speckit", "plan", "--spec", "SPEC-TEST-PLAN", "--dry-run"])
        .output()?;

    // Dry-run validation should succeed
    assert!(
        output.status.success(),
        "Expected exit 0 for plan validation, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn plan_json_includes_schema_version() -> Result<()> {
    // P3-B: Plan JSON output includes schema versioning
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create minimal SPEC structure
    let spec_dir = repo_root.path().join("docs").join("SPEC-TEST-PLAN-JSON");
    fs::create_dir_all(&spec_dir)?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "plan",
            "--spec",
            "SPEC-TEST-PLAN-JSON",
            "--dry-run",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Verify schema_version is present
    assert!(
        json.get("schema_version").is_some(),
        "Missing schema_version in plan JSON"
    );
    assert!(
        json.get("tool_version").is_some(),
        "Missing tool_version in plan JSON"
    );

    // Verify expected fields
    assert!(json.get("spec_id").is_some(), "Missing spec_id");
    assert!(json.get("stage").is_some(), "Missing stage");
    assert!(json.get("status").is_some(), "Missing status");
    assert!(json.get("dry_run").is_some(), "Missing dry_run");

    Ok(())
}

#[test]
fn plan_rejects_stage_flag() -> Result<()> {
    // P4-A: Plan command no longer accepts --stage flag
    // Use `speckit tasks` for tasks stage, etc.
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    fs::create_dir_all(repo_root.path().join("docs").join("SPEC-TEST-STAGES"))?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "plan",
            "--spec",
            "SPEC-TEST-STAGES",
            "--stage",
            "tasks",
            "--dry-run",
        ])
        .output()?;

    // Should fail because --stage is no longer accepted
    assert!(
        !output.status.success(),
        "Expected failure for --stage flag, got success"
    );

    // Error message should mention unexpected argument
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected") || stderr.contains("--stage"),
        "Should show error about unexpected --stage, got: {stderr}"
    );

    Ok(())
}

// =============================================================================
// P4-C: TASKS COMMAND TESTS
// =============================================================================

#[test]
fn tasks_validates_spec_and_exits_0() -> Result<()> {
    // P4-C: Tasks command validates SPEC and exits 0 when ready
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory with plan.md (required for tasks stage)
    let spec_dir = repo_root.path().join("docs").join("SPEC-TEST-TASKS");
    fs::create_dir_all(&spec_dir)?;
    fs::write(spec_dir.join("plan.md"), "# Plan\nTest plan.")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args(["speckit", "tasks", "--spec", "SPEC-TEST-TASKS", "--dry-run"])
        .output()?;

    // Dry-run validation should succeed
    assert!(
        output.status.success(),
        "Expected exit 0 for tasks validation, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn tasks_json_includes_schema_version() -> Result<()> {
    // P4-C: Tasks JSON output includes schema versioning
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create minimal SPEC structure with plan.md
    let spec_dir = repo_root.path().join("docs").join("SPEC-TEST-TASKS-JSON");
    fs::create_dir_all(&spec_dir)?;
    fs::write(spec_dir.join("plan.md"), "# Plan\nTest plan.")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "tasks",
            "--spec",
            "SPEC-TEST-TASKS-JSON",
            "--dry-run",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Verify schema_version is present
    assert!(
        json.get("schema_version").is_some(),
        "Missing schema_version in tasks JSON"
    );
    assert!(
        json.get("tool_version").is_some(),
        "Missing tool_version in tasks JSON"
    );

    // Verify expected fields
    assert!(json.get("spec_id").is_some(), "Missing spec_id");
    assert!(json.get("stage").is_some(), "Missing stage");
    assert!(json.get("status").is_some(), "Missing status");
    assert!(json.get("dry_run").is_some(), "Missing dry_run");

    // Verify stage is Tasks
    let stage = json.get("stage").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        stage.contains("Tasks"),
        "Expected Tasks stage, got: {stage}"
    );

    Ok(())
}

#[test]
fn tasks_warns_when_plan_missing() -> Result<()> {
    // P4-C: Tasks command warns when plan.md is missing
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory WITHOUT plan.md
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-TASKS-NO-PLAN");
    fs::create_dir_all(&spec_dir)?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "tasks",
            "--spec",
            "SPEC-TEST-TASKS-NO-PLAN",
            "--dry-run",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Should still succeed (ready) but with warnings
    let status = json.get("status").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(status, "ready", "Expected ready status");

    // Should have warning about missing plan.md
    let warnings = json.get("warnings").and_then(|v| v.as_array());
    assert!(
        warnings.map(|w| !w.is_empty()).unwrap_or(false),
        "Expected warning about missing plan.md"
    );

    Ok(())
}

// =============================================================================
// P5-A: IMPLEMENT COMMAND TESTS
// =============================================================================

#[test]
fn implement_validates_spec_and_exits_0() -> Result<()> {
    // P5-A: Implement command validates SPEC and exits 0 when ready
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory with plan.md (required for implement stage)
    let spec_dir = repo_root.path().join("docs").join("SPEC-TEST-IMPLEMENT");
    fs::create_dir_all(&spec_dir)?;
    fs::write(spec_dir.join("plan.md"), "# Plan\nTest plan.")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "implement",
            "--spec",
            "SPEC-TEST-IMPLEMENT",
            "--dry-run",
        ])
        .output()?;

    // Dry-run validation should succeed
    assert!(
        output.status.success(),
        "Expected exit 0 for implement validation, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn implement_warns_when_plan_missing() -> Result<()> {
    // P5-A: Implement command warns when plan.md is missing
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory WITHOUT plan.md
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-IMPLEMENT-NO-PLAN");
    fs::create_dir_all(&spec_dir)?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "implement",
            "--spec",
            "SPEC-TEST-IMPLEMENT-NO-PLAN",
            "--dry-run",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Should still succeed (ready) but with warnings
    let status = json.get("status").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(status, "ready", "Expected ready status");

    // Should have warning about missing plan.md
    let warnings = json.get("warnings").and_then(|v| v.as_array());
    assert!(
        warnings.map(|w| !w.is_empty()).unwrap_or(false),
        "Expected warning about missing plan.md"
    );

    Ok(())
}

#[test]
fn implement_json_includes_schema_version() -> Result<()> {
    // P5-A: Implement JSON output includes schema versioning
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create minimal SPEC structure with plan.md
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-IMPLEMENT-JSON");
    fs::create_dir_all(&spec_dir)?;
    fs::write(spec_dir.join("plan.md"), "# Plan\nTest plan.")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "implement",
            "--spec",
            "SPEC-TEST-IMPLEMENT-JSON",
            "--dry-run",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Verify schema_version is present
    assert!(
        json.get("schema_version").is_some(),
        "Missing schema_version in implement JSON"
    );
    assert!(
        json.get("tool_version").is_some(),
        "Missing tool_version in implement JSON"
    );

    // Verify expected fields
    assert!(json.get("spec_id").is_some(), "Missing spec_id");
    assert!(json.get("stage").is_some(), "Missing stage");
    assert!(json.get("status").is_some(), "Missing status");
    assert!(json.get("dry_run").is_some(), "Missing dry_run");

    // Verify stage is Implement
    let stage = json.get("stage").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        stage.contains("Implement"),
        "Expected Implement stage, got: {stage}"
    );

    Ok(())
}

// =============================================================================
// P5-B: VALIDATE STAGE COMMAND TESTS
// =============================================================================

#[test]
fn stage_validate_validates_spec_and_exits_0() -> Result<()> {
    // P5-B: Validate command validates SPEC and exits 0 when ready
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory with implementation artifacts
    let spec_dir = repo_root.path().join("docs").join("SPEC-TEST-VALIDATE");
    fs::create_dir_all(&spec_dir)?;
    fs::write(spec_dir.join("tasks.md"), "# Tasks\nTest tasks.")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "validate",
            "--spec",
            "SPEC-TEST-VALIDATE",
            "--dry-run",
        ])
        .output()?;

    // Dry-run validation should succeed
    assert!(
        output.status.success(),
        "Expected exit 0 for validate validation, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn stage_validate_warns_when_impl_missing() -> Result<()> {
    // P5-B: Validate command warns when implementation artifacts are missing
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory WITHOUT implementation artifacts
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-VALIDATE-NO-IMPL");
    fs::create_dir_all(&spec_dir)?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "validate",
            "--spec",
            "SPEC-TEST-VALIDATE-NO-IMPL",
            "--dry-run",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Should still succeed (ready) but with warnings
    let status = json.get("status").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(status, "ready", "Expected ready status");

    // Should have warning about missing implementation artifacts
    let warnings = json.get("warnings").and_then(|v| v.as_array());
    assert!(
        warnings.map(|w| !w.is_empty()).unwrap_or(false),
        "Expected warning about missing implementation artifacts"
    );

    Ok(())
}

#[test]
fn stage_validate_json_includes_schema_version() -> Result<()> {
    // P5-B: Validate JSON output includes schema versioning
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create minimal SPEC structure with implementation artifacts
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-VALIDATE-JSON");
    fs::create_dir_all(&spec_dir)?;
    fs::write(spec_dir.join("tasks.md"), "# Tasks\nTest tasks.")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "validate",
            "--spec",
            "SPEC-TEST-VALIDATE-JSON",
            "--dry-run",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Verify schema_version is present
    assert!(
        json.get("schema_version").is_some(),
        "Missing schema_version in validate JSON"
    );
    assert!(
        json.get("tool_version").is_some(),
        "Missing tool_version in validate JSON"
    );

    // Verify expected fields
    assert!(json.get("spec_id").is_some(), "Missing spec_id");
    assert!(json.get("stage").is_some(), "Missing stage");
    assert!(json.get("status").is_some(), "Missing status");
    assert!(json.get("dry_run").is_some(), "Missing dry_run");

    // Verify stage is Validate
    let stage = json.get("stage").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        stage.contains("Validate"),
        "Expected Validate stage, got: {stage}"
    );

    Ok(())
}

// =============================================================================
// P5-B: AUDIT COMMAND TESTS
// =============================================================================

#[test]
fn audit_validates_spec_and_exits_0() -> Result<()> {
    // P5-B: Audit command validates SPEC and exits 0 when ready
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory with implementation artifacts
    let spec_dir = repo_root.path().join("docs").join("SPEC-TEST-AUDIT");
    fs::create_dir_all(&spec_dir)?;
    fs::write(spec_dir.join("tasks.md"), "# Tasks\nTest tasks.")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args(["speckit", "audit", "--spec", "SPEC-TEST-AUDIT", "--dry-run"])
        .output()?;

    // Dry-run validation should succeed
    assert!(
        output.status.success(),
        "Expected exit 0 for audit validation, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn audit_warns_when_impl_missing() -> Result<()> {
    // P5-B: Audit command warns when implementation artifacts are missing
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory WITHOUT implementation artifacts
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-AUDIT-NO-IMPL");
    fs::create_dir_all(&spec_dir)?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "audit",
            "--spec",
            "SPEC-TEST-AUDIT-NO-IMPL",
            "--dry-run",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Should still succeed (ready) but with warnings
    let status = json.get("status").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(status, "ready", "Expected ready status");

    // Should have warning about missing implementation artifacts
    let warnings = json.get("warnings").and_then(|v| v.as_array());
    assert!(
        warnings.map(|w| !w.is_empty()).unwrap_or(false),
        "Expected warning about missing implementation artifacts"
    );

    Ok(())
}

#[test]
fn audit_json_includes_schema_version() -> Result<()> {
    // P5-B: Audit JSON output includes schema versioning
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create minimal SPEC structure with implementation artifacts
    let spec_dir = repo_root.path().join("docs").join("SPEC-TEST-AUDIT-JSON");
    fs::create_dir_all(&spec_dir)?;
    fs::write(spec_dir.join("tasks.md"), "# Tasks\nTest tasks.")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "audit",
            "--spec",
            "SPEC-TEST-AUDIT-JSON",
            "--dry-run",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Verify schema_version is present
    assert!(
        json.get("schema_version").is_some(),
        "Missing schema_version in audit JSON"
    );
    assert!(
        json.get("tool_version").is_some(),
        "Missing tool_version in audit JSON"
    );

    // Verify expected fields
    assert!(json.get("spec_id").is_some(), "Missing spec_id");
    assert!(json.get("stage").is_some(), "Missing stage");
    assert!(json.get("status").is_some(), "Missing status");
    assert!(json.get("dry_run").is_some(), "Missing dry_run");

    // Verify stage is Audit
    let stage = json.get("stage").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        stage.contains("Audit"),
        "Expected Audit stage, got: {stage}"
    );

    Ok(())
}

// =============================================================================
// P5-B: UNLOCK COMMAND TESTS
// =============================================================================

#[test]
fn unlock_validates_spec_and_exits_0() -> Result<()> {
    // P5-B: Unlock command validates SPEC and exits 0 when ready
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory with implementation artifacts
    let spec_dir = repo_root.path().join("docs").join("SPEC-TEST-UNLOCK");
    fs::create_dir_all(&spec_dir)?;
    fs::write(spec_dir.join("tasks.md"), "# Tasks\nTest tasks.")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "unlock",
            "--spec",
            "SPEC-TEST-UNLOCK",
            "--dry-run",
        ])
        .output()?;

    // Dry-run validation should succeed
    assert!(
        output.status.success(),
        "Expected exit 0 for unlock validation, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn unlock_warns_when_impl_missing() -> Result<()> {
    // P5-B: Unlock command warns when implementation artifacts are missing
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory WITHOUT implementation artifacts
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-UNLOCK-NO-IMPL");
    fs::create_dir_all(&spec_dir)?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "unlock",
            "--spec",
            "SPEC-TEST-UNLOCK-NO-IMPL",
            "--dry-run",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Should still succeed (ready) but with warnings
    let status = json.get("status").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(status, "ready", "Expected ready status");

    // Should have warning about missing implementation artifacts
    let warnings = json.get("warnings").and_then(|v| v.as_array());
    assert!(
        warnings.map(|w| !w.is_empty()).unwrap_or(false),
        "Expected warning about missing implementation artifacts"
    );

    Ok(())
}

#[test]
fn unlock_json_includes_schema_version() -> Result<()> {
    // P5-B: Unlock JSON output includes schema versioning
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create minimal SPEC structure with implementation artifacts
    let spec_dir = repo_root.path().join("docs").join("SPEC-TEST-UNLOCK-JSON");
    fs::create_dir_all(&spec_dir)?;
    fs::write(spec_dir.join("tasks.md"), "# Tasks\nTest tasks.")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "unlock",
            "--spec",
            "SPEC-TEST-UNLOCK-JSON",
            "--dry-run",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Verify schema_version is present
    assert!(
        json.get("schema_version").is_some(),
        "Missing schema_version in unlock JSON"
    );
    assert!(
        json.get("tool_version").is_some(),
        "Missing tool_version in unlock JSON"
    );

    // Verify expected fields
    assert!(json.get("spec_id").is_some(), "Missing spec_id");
    assert!(json.get("stage").is_some(), "Missing stage");
    assert!(json.get("status").is_some(), "Missing status");
    assert!(json.get("dry_run").is_some(), "Missing dry_run");

    // Verify stage is Unlock
    let stage = json.get("stage").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        stage.contains("Unlock"),
        "Expected Unlock stage, got: {stage}"
    );

    Ok(())
}

// =============================================================================
// P6-C: STRICT-PREREQS FLAG TESTS
// =============================================================================

#[test]
fn tasks_strict_prereqs_blocks_when_plan_missing() -> Result<()> {
    // P6-C: With --strict-prereqs, missing plan.md causes exit 2
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory WITHOUT plan.md
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-STRICT-PREREQS");
    fs::create_dir_all(&spec_dir)?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "tasks",
            "--spec",
            "SPEC-TEST-STRICT-PREREQS",
            "--dry-run",
            "--strict-prereqs",
        ])
        .output()?;

    // With --strict-prereqs, missing plan.md should cause exit 2
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit 2 with --strict-prereqs when plan.md missing, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn tasks_strict_prereqs_json_shows_blocked() -> Result<()> {
    // P6-C: JSON output shows blocked status with strict-prereqs
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory WITHOUT plan.md
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-STRICT-JSON");
    fs::create_dir_all(&spec_dir)?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "tasks",
            "--spec",
            "SPEC-TEST-STRICT-JSON",
            "--dry-run",
            "--strict-prereqs",
            "--json",
        ])
        .output()?;

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Status should be blocked
    let status = json.get("status").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(status, "blocked", "Expected blocked status with --strict-prereqs");

    // Errors should contain the strict-prereqs prefix
    let errors = json.get("errors").and_then(|v| v.as_array());
    let has_strict_error = errors
        .map(|e| e.iter().any(|err| {
            err.as_str().map(|s| s.contains("[strict-prereqs]")).unwrap_or(false)
        }))
        .unwrap_or(false);
    assert!(has_strict_error, "Expected [strict-prereqs] prefix in errors");

    Ok(())
}

#[test]
fn validate_strict_prereqs_blocks_when_impl_missing() -> Result<()> {
    // P6-C: Validate stage with --strict-prereqs blocks when implementation missing
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory WITHOUT implementation artifacts
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-VALIDATE-STRICT");
    fs::create_dir_all(&spec_dir)?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "validate",
            "--spec",
            "SPEC-TEST-VALIDATE-STRICT",
            "--dry-run",
            "--strict-prereqs",
        ])
        .output()?;

    // With --strict-prereqs, missing implementation artifacts should cause exit 2
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit 2 with --strict-prereqs when impl missing, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn plan_strict_prereqs_succeeds_when_dir_exists() -> Result<()> {
    // P6-C: Plan stage with --strict-prereqs succeeds when SPEC dir exists
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory (plan stage only requires dir to exist)
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-PLAN-STRICT");
    fs::create_dir_all(&spec_dir)?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "plan",
            "--spec",
            "SPEC-TEST-PLAN-STRICT",
            "--dry-run",
            "--strict-prereqs",
        ])
        .output()?;

    // Plan stage with existing dir should succeed even with --strict-prereqs
    assert!(
        output.status.success(),
        "Expected exit 0 for plan with --strict-prereqs when dir exists, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn tasks_without_strict_prereqs_warns_but_succeeds() -> Result<()> {
    // P6-C: Without --strict-prereqs, missing prereqs show warnings but don't block
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory WITHOUT plan.md
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-NO-STRICT");
    fs::create_dir_all(&spec_dir)?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "tasks",
            "--spec",
            "SPEC-TEST-NO-STRICT",
            "--dry-run",
            "--json",
            // Note: no --strict-prereqs
        ])
        .output()?;

    // Without --strict-prereqs, should succeed with warnings
    assert!(
        output.status.success(),
        "Expected exit 0 without --strict-prereqs, got {:?}",
        output.status.code()
    );

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;
    let status = json.get("status").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(status, "ready", "Expected ready status without --strict-prereqs");

    // Should have warnings
    let warnings = json.get("warnings").and_then(|v| v.as_array());
    assert!(
        warnings.map(|w| !w.is_empty()).unwrap_or(false),
        "Expected warnings about missing plan.md"
    );

    Ok(())
}

#[test]
fn implement_strict_prereqs_blocks_when_plan_missing() -> Result<()> {
    // P6-C: Implement stage with --strict-prereqs blocks when plan.md missing
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory WITHOUT plan.md
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-IMPL-STRICT");
    fs::create_dir_all(&spec_dir)?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "implement",
            "--spec",
            "SPEC-TEST-IMPL-STRICT",
            "--dry-run",
            "--strict-prereqs",
        ])
        .output()?;

    // With --strict-prereqs, missing plan.md should cause exit 2
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit 2 with --strict-prereqs when plan.md missing, got {:?}",
        output.status.code()
    );

    Ok(())
}

// =============================================================================
// P6-A: SPECIFY COMMAND TESTS
// =============================================================================

#[test]
fn specify_dry_run_reports_would_create() -> Result<()> {
    // P6-A: Dry-run specify (default mode) reports what would be created
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create docs directory only (not the SPEC dir)
    fs::create_dir_all(repo_root.path().join("docs"))?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "specify",
            "--spec",
            "SPEC-TEST-SPECIFY-DRY",
            // No --execute means dry-run mode
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Expected success, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("dry-run") && stdout.contains("Would create"),
        "Expected dry-run message about would create, got: {}",
        stdout
    );

    // Verify directory was NOT created
    let spec_dir = repo_root.path().join("docs").join("SPEC-TEST-SPECIFY-DRY");
    assert!(!spec_dir.exists(), "Directory should not exist in dry-run");

    Ok(())
}

#[test]
fn specify_creates_directory_and_prd() -> Result<()> {
    // P6-A: Actual specify creates directory and PRD.md
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create docs directory
    fs::create_dir_all(repo_root.path().join("docs"))?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "specify",
            "--spec",
            "SPEC-TEST-SPECIFY-CREATE",
            "--execute",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Expected success, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify directory was created
    let spec_dir = repo_root
        .path()
        .join("docs")
        .join("SPEC-TEST-SPECIFY-CREATE");
    assert!(spec_dir.exists(), "SPEC directory should exist");

    // Verify PRD.md was created
    let prd_path = spec_dir.join("PRD.md");
    assert!(prd_path.exists(), "PRD.md should exist");

    // Verify PRD.md has expected content
    let prd_content = fs::read_to_string(&prd_path)?;
    assert!(
        prd_content.contains("SPEC-TEST-SPECIFY-CREATE"),
        "PRD.md should contain SPEC ID"
    );
    assert!(
        prd_content.contains("## Overview"),
        "PRD.md should contain Overview section"
    );

    Ok(())
}

#[test]
fn specify_json_output_structure() -> Result<()> {
    // P6-A: JSON output has expected structure
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create docs directory
    fs::create_dir_all(repo_root.path().join("docs"))?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "specify",
            "--spec",
            "SPEC-TEST-SPECIFY-JSON",
            // Default is dry-run, no --execute
            "--json",
        ])
        .output()?;

    assert!(output.status.success());

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Verify schema version
    let schema_version = json.get("schema_version").and_then(|v| v.as_u64());
    assert_eq!(schema_version, Some(1), "Expected schema_version 1");

    // Verify required fields
    assert!(json.get("spec_id").is_some(), "Missing spec_id");
    assert!(json.get("spec_dir").is_some(), "Missing spec_dir");
    assert!(json.get("dry_run").is_some(), "Missing dry_run");
    assert!(
        json.get("already_existed").is_some(),
        "Missing already_existed"
    );
    assert!(json.get("created_files").is_some(), "Missing created_files");

    Ok(())
}

#[test]
fn specify_existing_dir_reports_already_existed() -> Result<()> {
    // P6-A: Specify on existing directory reports already_existed
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;

    // Create SPEC directory with existing PRD.md
    let spec_dir = repo_root.path().join("docs").join("SPEC-TEST-EXISTING");
    fs::create_dir_all(&spec_dir)?;
    fs::write(spec_dir.join("PRD.md"), "# Existing PRD")?;

    let mut cmd = codex_command(codex_home.path(), repo_root.path())?;
    let output = cmd
        .args([
            "speckit",
            "specify",
            "--spec",
            "SPEC-TEST-EXISTING",
            "--execute",
            "--json",
        ])
        .output()?;

    assert!(output.status.success());

    let json: JsonValue = serde_json::from_slice(&output.stdout)?;

    // Should report already_existed
    let already_existed = json.get("already_existed").and_then(|v| v.as_bool());
    assert_eq!(
        already_existed,
        Some(true),
        "Expected already_existed: true"
    );

    // Should not have created any files
    let created_files = json.get("created_files").and_then(|v| v.as_array());
    assert!(
        created_files.map(|f| f.is_empty()).unwrap_or(true),
        "Expected no created_files when PRD already exists"
    );

    Ok(())
}
