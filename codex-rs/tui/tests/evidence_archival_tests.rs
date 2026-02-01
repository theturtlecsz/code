//! E.3 Evidence Archival Tests
//!
//! ARB Pass 2 capability test suite for evidence archival (>30 days behavior).
//! Reference: docs/spec-kit/evidence-policy.md §4-6
//!
//! Tests:
//! - E3.1: Evidence >30 days gets marked for archival
//! - E3.2: Files modified <7 days exempt from archival (in-progress)
//! - E3.3: Archive before purge order (30d archive, 180d purge)
//! - E3.4: Archive produces valid tar.gz
//! - E3.5: Config overrides work (custom days)
//! - E3.6: Dry-run reports without modifying files

// SPEC-957: Allow test code flexibility
#![allow(clippy::expect_used, clippy::unwrap_used)]

use chrono::Duration;
use codex_tui::evidence_archival::{
    ArchivalConfig, ArchivalStatus, MockClock, get_archivable_specs, get_spec_archival_info,
    is_spec_in_progress, validate_archive_before_purge,
};
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Test Helpers
// ============================================================================

/// Mock clock's fixed time (2026-01-15 12:00:00 UTC).
/// Must match MockClock::fixed() in evidence_archival.rs
fn mock_clock_fixed_time() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
        .expect("valid fixed time")
        .with_timezone(&chrono::Utc)
}

/// Create evidence structure for a SPEC with controlled file age.
/// Age is relative to the mock clock's fixed time (2026-01-15).
fn create_evidence_with_age(evidence_root: &std::path::Path, spec_id: &str, age_days: i64) {
    let commands_dir = evidence_root.join("commands").join(spec_id);
    let consensus_dir = evidence_root.join("consensus").join(spec_id);

    fs::create_dir_all(&commands_dir).unwrap();
    fs::create_dir_all(&consensus_dir).unwrap();

    // Create test files
    let telemetry = commands_dir.join("telemetry.json");
    fs::write(
        &telemetry,
        r#"{"stage": "plan", "timestamp": "2025-01-01"}"#,
    )
    .unwrap();

    let synthesis = consensus_dir.join("synthesis.json");
    fs::write(&synthesis, r#"{"consensus": true, "agents": 3}"#).unwrap();

    // Set modification time relative to the MOCK CLOCK fixed time, not Utc::now()
    // This ensures deterministic behavior with MockClock::fixed()
    let target_time = mock_clock_fixed_time() - Duration::days(age_days);
    let mtime = filetime::FileTime::from_unix_time(target_time.timestamp(), 0);

    filetime::set_file_mtime(&telemetry, mtime).unwrap();
    filetime::set_file_mtime(&synthesis, mtime).unwrap();
    filetime::set_file_mtime(&commands_dir, mtime).unwrap();
    filetime::set_file_mtime(&consensus_dir, mtime).unwrap();
}

/// Create evidence directories with timestamp set to mock clock's current time.
/// This simulates "just created" evidence that should be in-progress/exempt.
fn create_fresh_evidence(evidence_root: &std::path::Path, spec_id: &str) {
    let commands_dir = evidence_root.join("commands").join(spec_id);
    let consensus_dir = evidence_root.join("consensus").join(spec_id);

    fs::create_dir_all(&commands_dir).unwrap();
    fs::create_dir_all(&consensus_dir).unwrap();

    let telemetry = commands_dir.join("telemetry.json");
    fs::write(&telemetry, r#"{"stage": "plan"}"#).unwrap();

    let synthesis = consensus_dir.join("synthesis.json");
    fs::write(&synthesis, r#"{"consensus": true}"#).unwrap();

    // Set mtime to mock clock's fixed time (fresh = 0 days ago)
    let target_time = mock_clock_fixed_time();
    let mtime = filetime::FileTime::from_unix_time(target_time.timestamp(), 0);

    filetime::set_file_mtime(&telemetry, mtime).unwrap();
    filetime::set_file_mtime(&synthesis, mtime).unwrap();
    filetime::set_file_mtime(&commands_dir, mtime).unwrap();
    filetime::set_file_mtime(&consensus_dir, mtime).unwrap();
}

// ============================================================================
// E3.1: Evidence >30 days archival eligibility
// ============================================================================

/// E3.1: Evidence older than 30 days should be marked as archivable.
#[test]
fn test_evidence_archival_after_30_days() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    // Create evidence 35 days old
    create_evidence_with_age(evidence_root, "SPEC-OLD-001", 35);

    // Create evidence 5 days old (should NOT be archivable)
    create_evidence_with_age(evidence_root, "SPEC-RECENT-001", 5);

    let clock = MockClock::fixed();
    let config = ArchivalConfig::default();

    let archivable = get_archivable_specs(&clock, &config, evidence_root).unwrap();

    // Only the old SPEC should be archivable
    assert_eq!(archivable.len(), 1);
    assert_eq!(archivable[0].spec_id, "SPEC-OLD-001");
    assert_eq!(archivable[0].status, ArchivalStatus::Archivable);
}

/// E3.1b: Multiple SPECs with varying ages.
#[test]
fn test_multiple_specs_archival_eligibility() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    // Create SPECs with different ages
    create_evidence_with_age(evidence_root, "SPEC-A", 10); // Active
    create_evidence_with_age(evidence_root, "SPEC-B", 40); // Archivable
    create_evidence_with_age(evidence_root, "SPEC-C", 60); // Archivable
    create_evidence_with_age(evidence_root, "SPEC-D", 200); // Purgeable

    let clock = MockClock::fixed();
    let config = ArchivalConfig::default(); // 30d archive, 180d purge

    let all_specs = get_spec_archival_info(&clock, &config, evidence_root).unwrap();

    // Verify status assignment
    let spec_a = all_specs.iter().find(|s| s.spec_id == "SPEC-A").unwrap();
    assert_eq!(spec_a.status, ArchivalStatus::Active);

    let spec_b = all_specs.iter().find(|s| s.spec_id == "SPEC-B").unwrap();
    assert_eq!(spec_b.status, ArchivalStatus::Archivable);

    let spec_c = all_specs.iter().find(|s| s.spec_id == "SPEC-C").unwrap();
    assert_eq!(spec_c.status, ArchivalStatus::Archivable);

    let spec_d = all_specs.iter().find(|s| s.spec_id == "SPEC-D").unwrap();
    assert_eq!(spec_d.status, ArchivalStatus::Purgeable);
}

// ============================================================================
// E3.2: In-progress exemption (7-day activity window)
// ============================================================================

/// E3.2: SPECs with files modified within 7 days are exempt from archival.
#[test]
fn test_evidence_exempt_if_in_progress() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    // Create evidence that's 3 days old (within 7-day window)
    create_evidence_with_age(evidence_root, "SPEC-IN-PROGRESS", 3);

    let clock = MockClock::fixed();
    let config = ArchivalConfig::default();

    let is_in_prog =
        is_spec_in_progress(&clock, &config, evidence_root, "SPEC-IN-PROGRESS").unwrap();
    assert!(
        is_in_prog,
        "SPEC with 3-day-old files should be in-progress"
    );

    // Verify it's not in archivable list
    let archivable = get_archivable_specs(&clock, &config, evidence_root).unwrap();
    assert!(
        archivable.is_empty(),
        "In-progress SPEC should not be archivable"
    );
}

/// E3.2b: Verify 7-day boundary behavior.
#[test]
fn test_activity_window_boundary() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    // Create evidence at exactly 7 days (boundary - should be exempt)
    create_evidence_with_age(evidence_root, "SPEC-BOUNDARY-7", 7);

    // Create evidence at 8 days (outside window - not exempt)
    create_evidence_with_age(evidence_root, "SPEC-OUTSIDE-8", 8);

    let clock = MockClock::fixed();
    let config = ArchivalConfig::default();

    let in_prog_7 = is_spec_in_progress(&clock, &config, evidence_root, "SPEC-BOUNDARY-7").unwrap();
    let in_prog_8 = is_spec_in_progress(&clock, &config, evidence_root, "SPEC-OUTSIDE-8").unwrap();

    assert!(in_prog_7, "7-day-old SPEC should be in-progress (<=7 days)");
    assert!(
        !in_prog_8,
        "8-day-old SPEC should NOT be in-progress (>7 days)"
    );
}

/// E3.2c: Fresh evidence (just created) is exempt.
#[test]
fn test_fresh_evidence_exempt() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    // Create evidence with current timestamp
    create_fresh_evidence(evidence_root, "SPEC-FRESH");

    let clock = MockClock::fixed();
    let config = ArchivalConfig::default();

    let is_in_prog = is_spec_in_progress(&clock, &config, evidence_root, "SPEC-FRESH").unwrap();
    assert!(is_in_prog, "Fresh evidence should be in-progress");
}

// ============================================================================
// E3.3: Archive before purge order
// ============================================================================

/// E3.3: Validates that archival (30d) happens before purge (180d).
#[test]
fn test_archive_before_purge_order() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();
    fs::create_dir_all(evidence_root.join("commands")).unwrap();
    fs::create_dir_all(evidence_root.join("archives")).unwrap();

    let clock = MockClock::fixed();
    let config = ArchivalConfig::default();

    // Validate configuration enforces archive < purge
    let result = validate_archive_before_purge(&clock, &config, evidence_root).unwrap();
    assert!(result.is_ok(), "Default config should pass validation");
}

/// E3.3b: Invalid config (purge <= archive) should fail validation.
#[test]
fn test_invalid_purge_before_archive_config() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();
    fs::create_dir_all(evidence_root.join("commands")).unwrap();

    let clock = MockClock::fixed();
    // Invalid: purge before archive
    let config = ArchivalConfig {
        archive_after_days: 90,
        purge_after_days: 30,
        ..Default::default()
    };

    let result = validate_archive_before_purge(&clock, &config, evidence_root).unwrap();
    assert!(
        result.is_err(),
        "Should fail when purge_after_days <= archive_after_days"
    );
}

/// E3.3c: Purgeable SPEC without archive should flag as violation.
#[test]
fn test_purge_without_archive_violation() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    // Create evidence 200 days old (purgeable) WITHOUT an archive
    create_evidence_with_age(evidence_root, "SPEC-NO-ARCHIVE", 200);

    // Ensure archives directory exists but is empty
    fs::create_dir_all(evidence_root.join("archives")).unwrap();

    let clock = MockClock::fixed();
    let config = ArchivalConfig::default();

    let result = validate_archive_before_purge(&clock, &config, evidence_root).unwrap();
    assert!(
        result.is_err(),
        "Should flag purgeable SPEC without archive"
    );

    let violations = result.unwrap_err();
    assert!(violations[0].contains("SPEC-NO-ARCHIVE"));
    assert!(violations[0].contains("no archive exists"));
}

// ============================================================================
// E3.4: Archive creation (tar.gz validation)
// ============================================================================

/// E3.4: Creating an archive produces a valid tar.gz file.
#[test]
fn test_archival_creates_tarball() {
    use codex_tui::evidence_integrity::create_archive_with_checksum;

    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    // Create test evidence
    create_fresh_evidence(evidence_root, "SPEC-ARCHIVE-TEST");

    // Create archive
    let archive_path = temp_dir.path().join("SPEC-ARCHIVE-TEST.tar.gz");
    let manifest =
        create_archive_with_checksum("SPEC-ARCHIVE-TEST", evidence_root, &archive_path).unwrap();

    // Verify archive exists and has content
    assert!(archive_path.exists());
    let archive_size = fs::metadata(&archive_path).unwrap().len();
    assert!(archive_size > 0, "Archive should not be empty");

    // Verify manifest
    assert_eq!(manifest.spec_id, "SPEC-ARCHIVE-TEST");
    assert!(!manifest.archive_checksum.is_empty());
    assert!(manifest.file_count() >= 2); // At least telemetry + synthesis
}

// ============================================================================
// E3.5: Configuration customization
// ============================================================================

/// E3.5: Custom archive/purge thresholds work correctly.
#[test]
fn test_archival_config_customizable() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    // Create evidence at different ages
    create_evidence_with_age(evidence_root, "SPEC-15-DAYS", 15);
    create_evidence_with_age(evidence_root, "SPEC-45-DAYS", 45);

    let clock = MockClock::fixed();

    // Custom config: archive after 10 days (instead of 30)
    let custom_config = ArchivalConfig {
        archive_after_days: 10,
        purge_after_days: 60,
        activity_window_days: 3,
        ..ArchivalConfig::default()
    };

    let archivable = get_archivable_specs(&clock, &custom_config, evidence_root).unwrap();

    // Both SPECs should be archivable with 10-day threshold
    assert_eq!(archivable.len(), 2);
}

/// E3.5b: Custom activity window.
#[test]
fn test_custom_activity_window() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    create_evidence_with_age(evidence_root, "SPEC-5-DAYS", 5);

    let clock = MockClock::fixed();

    // Default (7-day window): should be in-progress
    let default_config = ArchivalConfig::default();
    assert!(is_spec_in_progress(&clock, &default_config, evidence_root, "SPEC-5-DAYS").unwrap());

    // Custom (3-day window): should NOT be in-progress
    let strict_config = ArchivalConfig {
        activity_window_days: 3,
        ..ArchivalConfig::default()
    };
    assert!(!is_spec_in_progress(&clock, &strict_config, evidence_root, "SPEC-5-DAYS").unwrap());
}

// ============================================================================
// E3.6: Dry-run mode
// ============================================================================

/// E3.6: Dry-run mode should report without modifying files.
#[test]
fn test_dry_run_mode() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    create_evidence_with_age(evidence_root, "SPEC-DRY-RUN", 45);

    let clock = MockClock::fixed();
    let config = ArchivalConfig {
        dry_run: true,
        ..ArchivalConfig::default()
    };

    // Get archivable SPECs (dry-run just reads, doesn't modify)
    let archivable = get_archivable_specs(&clock, &config, evidence_root).unwrap();
    assert_eq!(archivable.len(), 1);

    // Verify original files still exist (not archived/deleted)
    let telemetry = evidence_root
        .join("commands")
        .join("SPEC-DRY-RUN")
        .join("telemetry.json");
    assert!(
        telemetry.exists(),
        "Files should not be modified in dry-run"
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Empty evidence directory should return empty results.
#[test]
fn test_empty_evidence_directory() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    // Create empty structure
    fs::create_dir_all(evidence_root.join("commands")).unwrap();
    fs::create_dir_all(evidence_root.join("consensus")).unwrap();

    let clock = MockClock::fixed();
    let config = ArchivalConfig::default();

    let specs = get_spec_archival_info(&clock, &config, evidence_root).unwrap();
    assert!(specs.is_empty());
}

/// SPEC with only commands/ (no consensus/) should be handled.
#[test]
fn test_partial_evidence_commands_only() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    let commands_dir = evidence_root.join("commands").join("SPEC-COMMANDS-ONLY");
    fs::create_dir_all(&commands_dir).unwrap();
    fs::write(commands_dir.join("telemetry.json"), "{}").unwrap();

    let clock = MockClock::fixed();
    let config = ArchivalConfig::default();

    let specs = get_spec_archival_info(&clock, &config, evidence_root).unwrap();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].spec_id, "SPEC-COMMANDS-ONLY");
}

/// SPEC with only consensus/ (no commands/) should be handled.
#[test]
fn test_partial_evidence_consensus_only() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    let consensus_dir = evidence_root.join("consensus").join("SPEC-CONSENSUS-ONLY");
    fs::create_dir_all(&consensus_dir).unwrap();
    fs::write(consensus_dir.join("synthesis.json"), "{}").unwrap();

    let clock = MockClock::fixed();
    let config = ArchivalConfig::default();

    let specs = get_spec_archival_info(&clock, &config, evidence_root).unwrap();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].spec_id, "SPEC-CONSENSUS-ONLY");
}

/// Hidden directories (starting with .) should be ignored.
#[test]
fn test_hidden_directories_ignored() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    // Create .locks directory (should be ignored)
    let locks_dir = evidence_root.join("commands").join(".locks");
    fs::create_dir_all(&locks_dir).unwrap();
    fs::write(locks_dir.join("SPEC-001.lock"), "").unwrap();

    // Create real SPEC
    create_fresh_evidence(evidence_root, "SPEC-REAL");

    let clock = MockClock::fixed();
    let config = ArchivalConfig::default();

    let specs = get_spec_archival_info(&clock, &config, evidence_root).unwrap();

    // Only real SPEC should appear
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].spec_id, "SPEC-REAL");
}
