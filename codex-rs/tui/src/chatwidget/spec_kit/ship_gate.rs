//! Ship gate validation for explainability artifacts (D131/D132)
//!
//! Validates required artifacts before allowing ship (Unlock stage completion):
//! - Maieutic Spec (delegation contract)
//! - ACE milestone frames (when implemented)
//!
//! capture=none blocks ship with actionable messaging - this is "private scratch mode"
//! where the user is experimenting without creating audit artifacts.

use crate::memvid_adapter::LLMCaptureMode;
use std::path::Path;

/// Result of ship gate validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShipGateResult {
    /// All requirements met, proceed to ship
    Allowed,
    /// Private scratch mode (capture=none) - cannot ship
    BlockedPrivateScratch,
    /// Missing required artifact
    BlockedMissingArtifact { artifact: String },
}

/// Error message for private scratch mode
pub const PRIVATE_SCRATCH_MESSAGE: &str =
    "Private scratch mode: switch capture mode to prompts_only/full_io to ship";

/// Validate ship gate requirements (D131/D132)
///
/// Returns ShipGateResult indicating whether ship is allowed.
///
/// # Arguments
/// * `spec_id` - SPEC ID
/// * `run_id` - Run ID
/// * `capture_mode` - Capture mode from governance policy
/// * `cwd` - Current working directory
///
/// # Returns
/// * `ShipGateResult::Allowed` - All artifacts present, ship can proceed
/// * `ShipGateResult::BlockedPrivateScratch` - capture=none, cannot ship
/// * `ShipGateResult::BlockedMissingArtifact` - Missing required artifact
pub fn validate_ship_gate(
    spec_id: &str,
    run_id: &str,
    capture_mode: LLMCaptureMode,
    cwd: &Path,
) -> ShipGateResult {
    // D131: capture=none blocks ship immediately
    // This is "private scratch mode" - user is experimenting without audit artifacts
    if capture_mode == LLMCaptureMode::None {
        tracing::info!(
            spec_id = %spec_id,
            "Ship gate: Blocked (capture_mode=none, private scratch mode)"
        );
        return ShipGateResult::BlockedPrivateScratch;
    }

    // D132: Check maieutic artifact exists
    if !super::maieutic::has_maieutic_completed(spec_id, run_id, cwd) {
        tracing::info!(
            spec_id = %spec_id,
            "Ship gate: Blocked (missing maieutic spec)"
        );
        return ShipGateResult::BlockedMissingArtifact {
            artifact: "Maieutic Spec".to_string(),
        };
    }

    // D132: Check ACE milestone frame exists (when implemented)
    // For now, this is a stub that returns true until ACE persistence is wired
    if !has_ace_milestone_frame(spec_id, run_id, cwd) {
        tracing::info!(
            spec_id = %spec_id,
            "Ship gate: Blocked (missing ACE milestone frame)"
        );
        return ShipGateResult::BlockedMissingArtifact {
            artifact: "ACE milestone frame".to_string(),
        };
    }

    tracing::info!(
        spec_id = %spec_id,
        capture_mode = ?capture_mode,
        "Ship gate: Allowed (all artifacts present)"
    );
    ShipGateResult::Allowed
}

/// Check if ACE milestone frame exists for this run
///
/// Checks for ace_milestone_*.json files in the evidence directory,
/// following the pattern of maieutic::has_maieutic_completed.
pub fn has_ace_milestone_frame(spec_id: &str, _run_id: &str, cwd: &Path) -> bool {
    let evidence_dir = super::evidence::evidence_base_for_spec(cwd, spec_id);

    std::fs::read_dir(&evidence_dir)
        .ok()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|n| n.starts_with("ace_milestone_") && n.ends_with(".json"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D131: capture=none blocks ship regardless of artifacts
    #[test]
    fn test_capture_none_blocks_ship() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let spec_id = "SPEC-TEST-SHIP";
        let run_id = "run-001";

        // capture=none should block ship regardless of artifacts
        let result = validate_ship_gate(spec_id, run_id, LLMCaptureMode::None, temp_dir.path());

        assert_eq!(result, ShipGateResult::BlockedPrivateScratch);
    }

    /// D132: Ship requires maieutic spec artifact
    #[test]
    fn test_ship_requires_maieutic() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let spec_id = "SPEC-TEST-SHIP";
        let run_id = "run-001";

        // No maieutic file - should block
        let result = validate_ship_gate(
            spec_id,
            run_id,
            LLMCaptureMode::PromptsOnly,
            temp_dir.path(),
        );

        assert!(matches!(
            result,
            ShipGateResult::BlockedMissingArtifact { artifact } if artifact == "Maieutic Spec"
        ));
    }

    /// Ship allowed when all artifacts present (maieutic + ACE)
    #[test]
    fn test_ship_allowed_with_artifacts() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let spec_id = "SPEC-TEST-SHIP";
        let run_id = "run-001";

        // Create maieutic spec
        let spec = super::super::maieutic::MaieuticSpec::new(
            spec_id.to_string(),
            run_id.to_string(),
            "Test goal".to_string(),
            vec!["Constraint".to_string()],
            vec!["Tests pass".to_string()],
            vec![],
            super::super::maieutic::DelegationBounds::default(),
            super::super::maieutic::ElicitationMode::Interactive,
            1000,
        );

        super::super::maieutic::persist_maieutic_spec(
            spec_id,
            &spec,
            LLMCaptureMode::PromptsOnly,
            temp_dir.path(),
        )
        .unwrap();

        // Create ACE frame
        let reflection = super::super::ace_reflector::ReflectionResult {
            schema_version: super::super::ace_reflector::ACE_FRAME_SCHEMA_VERSION.to_string(),
            patterns: vec![],
            successes: vec!["Test".to_string()],
            failures: vec![],
            recommendations: vec![],
            summary: "Test".to_string(),
        };
        super::super::ace_reflector::persist_ace_frame(
            spec_id,
            &reflection,
            LLMCaptureMode::PromptsOnly,
            temp_dir.path(),
        )
        .unwrap();

        // Now should be allowed
        let result = validate_ship_gate(
            spec_id,
            run_id,
            LLMCaptureMode::PromptsOnly,
            temp_dir.path(),
        );

        assert_eq!(result, ShipGateResult::Allowed);
    }

    /// Verify private scratch message is actionable
    #[test]
    fn test_private_scratch_message_format() {
        // Verify message includes actionable guidance
        assert!(PRIVATE_SCRATCH_MESSAGE.contains("prompts_only"));
        assert!(PRIVATE_SCRATCH_MESSAGE.contains("full_io"));
        assert!(PRIVATE_SCRATCH_MESSAGE.contains("switch"));
    }

    /// capture=full_io should also work (not just prompts_only)
    #[test]
    fn test_ship_works_with_full_io() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let spec_id = "SPEC-TEST-SHIP-FULL";
        let run_id = "run-001";

        // Create maieutic spec with full_io mode
        let spec = super::super::maieutic::MaieuticSpec::new(
            spec_id.to_string(),
            run_id.to_string(),
            "Test goal".to_string(),
            vec!["Constraint".to_string()],
            vec!["Tests pass".to_string()],
            vec![],
            super::super::maieutic::DelegationBounds::default(),
            super::super::maieutic::ElicitationMode::Interactive,
            1000,
        );

        super::super::maieutic::persist_maieutic_spec(
            spec_id,
            &spec,
            LLMCaptureMode::FullIo,
            temp_dir.path(),
        )
        .unwrap();

        // Create ACE frame with full_io mode
        let reflection = super::super::ace_reflector::ReflectionResult {
            schema_version: super::super::ace_reflector::ACE_FRAME_SCHEMA_VERSION.to_string(),
            patterns: vec![],
            successes: vec!["Test".to_string()],
            failures: vec![],
            recommendations: vec![],
            summary: "Test".to_string(),
        };
        super::super::ace_reflector::persist_ace_frame(
            spec_id,
            &reflection,
            LLMCaptureMode::FullIo,
            temp_dir.path(),
        )
        .unwrap();

        // full_io should also be allowed
        let result = validate_ship_gate(spec_id, run_id, LLMCaptureMode::FullIo, temp_dir.path());

        assert_eq!(result, ShipGateResult::Allowed);
    }

    /// D132: Ship requires both maieutic spec AND ACE milestone frame
    #[test]
    fn test_ship_requires_maieutic_and_ace() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let spec_id = "SPEC-TEST-BOTH";
        let run_id = "run-001";

        // No artifacts - blocked on maieutic first
        let result = validate_ship_gate(
            spec_id,
            run_id,
            LLMCaptureMode::PromptsOnly,
            temp_dir.path(),
        );
        assert!(
            matches!(
                result,
                ShipGateResult::BlockedMissingArtifact { ref artifact } if artifact == "Maieutic Spec"
            ),
            "Should block on missing maieutic first"
        );

        // Create maieutic only - blocked on ACE
        let maieutic = super::super::maieutic::MaieuticSpec::new(
            spec_id.to_string(),
            run_id.to_string(),
            "Goal".to_string(),
            vec![],
            vec!["Tests pass".to_string()],
            vec![],
            super::super::maieutic::DelegationBounds::default(),
            super::super::maieutic::ElicitationMode::Interactive,
            1000,
        );
        super::super::maieutic::persist_maieutic_spec(
            spec_id,
            &maieutic,
            LLMCaptureMode::PromptsOnly,
            temp_dir.path(),
        )
        .unwrap();

        let result = validate_ship_gate(
            spec_id,
            run_id,
            LLMCaptureMode::PromptsOnly,
            temp_dir.path(),
        );
        assert!(
            matches!(
                result,
                ShipGateResult::BlockedMissingArtifact { ref artifact } if artifact == "ACE milestone frame"
            ),
            "Should block on missing ACE after maieutic exists"
        );

        // Create ACE frame - now allowed
        let reflection = super::super::ace_reflector::ReflectionResult {
            schema_version: super::super::ace_reflector::ACE_FRAME_SCHEMA_VERSION.to_string(),
            patterns: vec![],
            successes: vec!["Test".to_string()],
            failures: vec![],
            recommendations: vec![],
            summary: "Test".to_string(),
        };
        super::super::ace_reflector::persist_ace_frame(
            spec_id,
            &reflection,
            LLMCaptureMode::PromptsOnly,
            temp_dir.path(),
        )
        .unwrap();

        let result = validate_ship_gate(
            spec_id,
            run_id,
            LLMCaptureMode::PromptsOnly,
            temp_dir.path(),
        );
        assert_eq!(
            result,
            ShipGateResult::Allowed,
            "Should be allowed when both maieutic and ACE exist"
        );
    }
}
