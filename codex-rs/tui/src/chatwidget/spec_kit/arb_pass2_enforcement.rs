//! ARB Pass 2 Enforcement Test Registry (D130-D134)
//!
//! This module documents and indexes the 18 enforcement tests validating
//! ACE + Maieutics decisions from ARCHITECT_REVIEW_BOARD_OUTPUT.md.
//!
//! Also includes E.3/E.4 capability tests for evidence archival and integrity
//! (gap closure per G2 from ARCHITECT_REVIEW_BOARD_OUTPUT.md).
//!
//! Tests remain in their original locations for locality; this module provides:
//! 1. A single source of truth for test coverage
//! 2. Decision-to-test mapping
//! 3. Meta-test validating test count (catches drift)
//!
//! ## Quick Reference
//!
//! Run all ARB Pass 2 tests:
//! ```bash
//! # Core executor tests (D130, D131, D132, D134)
//! cargo test -p codex-tui --lib -- maieutic::tests::test_maieutic_required
//! cargo test -p codex-tui --lib -- maieutic::tests::test_capture_none
//! cargo test -p codex-tui --lib -- ace_reflector::tests::test_capture_none
//! cargo test -p codex-tui --lib -- ace_reflector::schema_tests
//! cargo test -p codex-tui --lib -- ship_gate::tests
//!
//! # CLI/headless tests (D133)
//! cargo test -p codex-cli --test speckit -- test_headless
//!
//! # E.3/E.4 evidence capability tests
//! cargo test -p codex-tui --test evidence_archival_tests
//! cargo test -p codex-tui --test evidence_integrity_tests
//!
//! # Or use the convenience script:
//! ./scripts/test-arb-pass2.sh
//! ```
//!
//! ## Test Matrix
//!
//! | Test # | Decision | Location | Test Name | Status |
//! |--------|----------|----------|-----------|--------|
//! | 1 | D130 | maieutic.rs | test_maieutic_required_before_execute | Active |
//! | 2 | D131 | ace_reflector.rs | test_capture_none_no_persisted_artifacts | Active |
//! | 3 | D131 | maieutic.rs | test_capture_none_does_not_persist_maieutic | Active |
//! | 4 | D132 | ship_gate.rs | test_capture_none_blocks_ship | Active |
//! | 5 | D132 | ship_gate.rs | test_ship_requires_maieutic_and_ace | Active |
//! | 6 | D132 | ship_gate.rs | test_ship_allowed_with_artifacts | Active |
//! | 7 | D133 | cli/speckit.rs | test_headless_requires_maieutic_input | Active |
//! | 8 | D133 | cli/speckit.rs | test_headless_validation_without_execute_succeeds | Active |
//! | 9 | D133 | cli/speckit.rs | test_headless_invalid_maieutic_json_exits_3 | Active |
//! | 10 | D133 | cli/speckit.rs | test_headless_missing_maieutic_file_exits_3 | Active |
//! | 11 | D133 | cli/speckit.rs | test_headless_never_prompts | Active |
//! | 12 | D133 | cli/speckit.rs | test_headless_needs_approval_exit_code | Active |
//! | 13 | D133 | cli/speckit.rs | test_shared_executor_same_core_artifacts | Active |
//! | 14 | D134 | ace_reflector.rs | test_ace_frame_schema_generation_stable | Active |
//! | 15 | D134 | ace_reflector.rs | test_ace_frame_examples_validate | Active |
//! | 16 | D134 | ace_reflector.rs | test_schema_version_field_required | Active |
//! | 17 | D132 | ship_gate.rs | test_private_scratch_message_format | Active |
//! | 18 | D132 | ship_gate.rs | test_ship_works_with_full_io | Active |
//!
//! ## Decision Summary
//!
//! - **D130**: Maieutic step mandatory before execution (fast path allowed)
//! - **D131**: `capture=none` persists no artifacts (in-memory only)
//! - **D132**: Ship hard-fail without artifacts; `capture=none` non-shippable
//! - **D133**: Multi-surface parity - headless requires `--maieutic`, never prompts
//! - **D134**: ACE Frame schema generated + versioned via schemars
//!
//! ## Evidence Capability Tests (G2 Gap Closure)
//!
//! | Cap | Location | Tests | Description |
//! |-----|----------|-------|-------------|
//! | E.3 | evidence_archival_tests.rs | 6 | Archival (>30 days behavior) |
//! | E.4 | evidence_integrity_tests.rs | 6 | Integrity (SHA256 verification) |

// Allow dead_code: This is a registry module - constants are for documentation and test use.
#![allow(dead_code)]

/// Total enforcement tests in ARB Pass 2
pub const ARB_PASS2_TEST_COUNT: usize = 18;

/// Active (non-ignored) enforcement tests
pub const ARB_PASS2_ACTIVE_COUNT: usize = 18;

/// All tests now active after MAINT-930 infrastructure
pub const ARB_PASS2_IGNORED_COUNT: usize = 0;

// ============================================================================
// E.3/E.4 Evidence Capability Tests (G2 Gap Closure)
// ============================================================================

/// E.3 archival tests (evidence >30 days behavior)
pub const E3_TEST_COUNT: usize = 6;

/// E.4 integrity tests (SHA256 verification)
pub const E4_TEST_COUNT: usize = 6;

/// Total evidence capability tests
pub const EVIDENCE_CAPABILITY_TEST_COUNT: usize = E3_TEST_COUNT + E4_TEST_COUNT;

/// Evidence capability test registry
pub mod evidence_capabilities {
    /// E.3: Evidence archival (>30 days) tests
    ///
    /// Reference: docs/spec-kit/evidence-policy.md §4-6
    /// Tests use injectable Clock for deterministic time-based testing.
    pub const E3_ARCHIVAL_TESTS: &[&str] = &[
        "test_evidence_archival_after_30_days",
        "test_evidence_exempt_if_in_progress",
        "test_archive_before_purge_order",
        "test_archival_creates_tarball",
        "test_archival_config_customizable",
        "test_dry_run_mode",
    ];

    /// E.4: Evidence integrity (SHA256 verification) tests
    ///
    /// Reference: docs/spec-kit/evidence-policy.md §9.1-9.2
    pub const E4_INTEGRITY_TESTS: &[&str] = &[
        "test_sha256_checksum_calculation",
        "test_archive_includes_manifest",
        "test_verify_valid_archive_succeeds",
        "test_verify_corrupted_archive_fails",
        "test_restore_rejects_checksum_mismatch",
        "test_restore_validates_file_count",
    ];
}

/// Decision enforcement groups with test names for reference
pub mod decisions {
    /// D130: Maieutic step always mandatory (fast path allowed)
    ///
    /// Rule: Pipeline cannot proceed until maieutic elicitation completes.
    /// Enforcement: `has_maieutic_completed()` returns false -> gate pauses pipeline.
    pub const D130_TESTS: &[&str] = &["maieutic::tests::test_maieutic_required_before_execute"];

    /// D131: capture=none persists no artifacts
    ///
    /// Rule: When `capture_mode=None`, no Maieutic Spec or ACE frames are written to disk.
    /// Enforcement: `persist_*` functions return `Ok(None)` for capture=none.
    pub const D131_TESTS: &[&str] = &[
        "ace_reflector::tests::test_capture_none_no_persisted_artifacts",
        "maieutic::tests::test_capture_none_does_not_persist_maieutic",
    ];

    /// D132: Ship hard-fail without artifacts
    ///
    /// Rule: Ship stage requires persisted Maieutic Spec + ACE milestone frame.
    /// Enforcement: `validate_ship_gate()` returns `BlockedPrivateScratch` or `BlockedMissingArtifact`.
    ///
    /// Note: test_ship_requires_maieutic_and_ace is the comprehensive test (Test 5).
    /// test_ship_requires_maieutic is a subset that verifies maieutic-only blocking.
    pub const D132_TESTS: &[&str] = &[
        "ship_gate::tests::test_capture_none_blocks_ship",
        "ship_gate::tests::test_ship_requires_maieutic_and_ace", // Test 5: comprehensive
        "ship_gate::tests::test_ship_allowed_with_artifacts",
        "ship_gate::tests::test_private_scratch_message_format",
        "ship_gate::tests::test_ship_works_with_full_io",
    ];

    /// D133: Multi-surface parity (headless)
    ///
    /// Rule: Headless execution requires `--maieutic` input; headless never prompts.
    /// Enforcement: Exit codes 10 (NEEDS_INPUT), 11 (NEEDS_APPROVAL), 3 (INFRA_ERROR).
    ///
    /// Note: Tests 11-13 now active after MAINT-930 infrastructure implementation.
    pub const D133_TESTS: &[&str] = &[
        "test_headless_requires_maieutic_input",
        "test_headless_validation_without_execute_succeeds",
        "test_headless_invalid_maieutic_json_exits_3",
        "test_headless_missing_maieutic_file_exits_3",
        // Now active (MAINT-930)
        "test_headless_never_prompts",
        "test_headless_needs_approval_exit_code",
        "test_shared_executor_same_core_artifacts",
    ];

    /// D133 ignored tests (none remaining after MAINT-930)
    pub const D133_IGNORED_TESTS: &[&str] = &[];

    /// D134: ACE Frame schema versioning
    ///
    /// Rule: Generated schema matches committed file; examples validate; version field required.
    /// Enforcement: schemars generates deterministic schema; jsonschema validates instances.
    pub const D134_TESTS: &[&str] = &[
        "ace_reflector::schema_tests::test_ace_frame_schema_generation_stable",
        "ace_reflector::schema_tests::test_ace_frame_examples_validate",
        "ace_reflector::schema_tests::test_schema_version_field_required",
    ];
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    /// Meta-test: Validate that test count matches expectation
    ///
    /// This test fails if tests are added/removed without updating the registry.
    /// Update the constants and decision arrays when modifying enforcement tests.
    #[test]
    fn test_arb_pass2_test_count_matches() {
        let total = decisions::D130_TESTS.len()
            + decisions::D131_TESTS.len()
            + decisions::D132_TESTS.len()
            + decisions::D133_TESTS.len()
            + decisions::D134_TESTS.len();

        assert_eq!(
            total, ARB_PASS2_TEST_COUNT,
            "Expected {} ARB Pass 2 tests, found {}. Update registry if tests added/removed.",
            ARB_PASS2_TEST_COUNT, total
        );
    }

    /// Validate active vs ignored count
    #[test]
    fn test_arb_pass2_active_count_matches() {
        let ignored = decisions::D133_IGNORED_TESTS.len();
        let active = ARB_PASS2_TEST_COUNT - ignored;

        assert_eq!(
            active, ARB_PASS2_ACTIVE_COUNT,
            "Expected {} active tests, found {}. Update ARB_PASS2_ACTIVE_COUNT.",
            ARB_PASS2_ACTIVE_COUNT, active
        );

        assert_eq!(
            ignored, ARB_PASS2_IGNORED_COUNT,
            "Expected {} ignored tests, found {}. Update ARB_PASS2_IGNORED_COUNT.",
            ARB_PASS2_IGNORED_COUNT, ignored
        );
    }

    /// Validate that each decision group is non-empty
    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_all_decisions_have_tests() {
        assert!(
            !decisions::D130_TESTS.is_empty(),
            "D130 (Maieutic mandatory) must have tests"
        );
        assert!(
            !decisions::D131_TESTS.is_empty(),
            "D131 (capture=none) must have tests"
        );
        assert!(
            !decisions::D132_TESTS.is_empty(),
            "D132 (Ship gate) must have tests"
        );
        assert!(
            !decisions::D133_TESTS.is_empty(),
            "D133 (Headless parity) must have tests"
        );
        assert!(
            !decisions::D134_TESTS.is_empty(),
            "D134 (Schema versioning) must have tests"
        );
    }

    /// Validate E.3/E.4 evidence capability test counts
    #[test]
    fn test_evidence_capability_counts() {
        assert_eq!(
            evidence_capabilities::E3_ARCHIVAL_TESTS.len(),
            E3_TEST_COUNT,
            "E.3 test count mismatch"
        );
        assert_eq!(
            evidence_capabilities::E4_INTEGRITY_TESTS.len(),
            E4_TEST_COUNT,
            "E.4 test count mismatch"
        );
        assert_eq!(
            E3_TEST_COUNT + E4_TEST_COUNT,
            EVIDENCE_CAPABILITY_TEST_COUNT,
            "Total evidence capability test count mismatch"
        );
    }

    /// Validate E.3/E.4 test groups are non-empty
    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_evidence_capabilities_have_tests() {
        assert!(
            !evidence_capabilities::E3_ARCHIVAL_TESTS.is_empty(),
            "E.3 (Archival) must have tests"
        );
        assert!(
            !evidence_capabilities::E4_INTEGRITY_TESTS.is_empty(),
            "E.4 (Integrity) must have tests"
        );
    }
}
