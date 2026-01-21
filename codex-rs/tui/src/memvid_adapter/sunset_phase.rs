//! SPEC-KIT-979: Sunset Phase Resolution and Enforcement
//!
//! Provides phase-based enforcement for the local-memory sunset migration:
//! - Phase 0: No warnings (current default)
//! - Phase 1: Deprecation warning on local-memory usage
//! - Phase 2: Require --force-deprecated flag for local-memory
//! - Phase 3: Hard error (local-memory removed)
//!
//! ## Phase Configuration
//!
//! The effective phase is resolved from:
//! 1. `model_policy.toml` `[gates.local_memory_sunset].current_phase` (default)
//! 2. `CODE_SUNSET_PHASE` environment variable (override)
//!
//! ## Auditability
//!
//! Phase resolution is recorded via `LocalMemorySunsetPhaseResolved` capsule event
//! at run start, capturing both policy and env values for replay/debugging.

use crate::memvid_adapter::types::PhaseResolutionPayload;
use codex_stage0::config::MemoryBackend;
use codex_stage0::policy::GovernancePolicy;

// =============================================================================
// Sunset Phase Enum
// =============================================================================

/// Sunset phases for local-memory deprecation.
///
/// Each phase has different enforcement behavior:
/// - Phase0: No restrictions (current default)
/// - Phase1: Warning messages
/// - Phase2: Blocking without --force-deprecated
/// - Phase3: Hard removal (error)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SunsetPhase {
    /// Phase 0: Current behavior (local-memory default, no warnings)
    #[default]
    Phase0,
    /// Phase 1: Warning on local-memory usage + migration recommendation
    Phase1,
    /// Phase 2: Strong warning + require --force-deprecated flag
    Phase2,
    /// Phase 3: Hard error (local-memory removed)
    Phase3,
}

impl SunsetPhase {
    /// Create from raw u8 value.
    pub fn from_u8(n: u8) -> Self {
        match n {
            0 => SunsetPhase::Phase0,
            1 => SunsetPhase::Phase1,
            2 => SunsetPhase::Phase2,
            _ => SunsetPhase::Phase3, // Phase 3+ treated as Phase3
        }
    }

    /// Convert to u8.
    pub fn as_u8(&self) -> u8 {
        match self {
            SunsetPhase::Phase0 => 0,
            SunsetPhase::Phase1 => 1,
            SunsetPhase::Phase2 => 2,
            SunsetPhase::Phase3 => 3,
        }
    }

    /// Human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            SunsetPhase::Phase0 => "Phase 0: No restrictions",
            SunsetPhase::Phase1 => "Phase 1: Deprecation warning",
            SunsetPhase::Phase2 => "Phase 2: Requires --force-deprecated",
            SunsetPhase::Phase3 => "Phase 3: local-memory removed",
        }
    }
}

// =============================================================================
// Phase Resolution
// =============================================================================

/// Environment variable name for phase override.
pub const SUNSET_PHASE_ENV_VAR: &str = "CODE_SUNSET_PHASE";

/// Resolve effective sunset phase from policy + environment.
///
/// Priority:
/// 1. `CODE_SUNSET_PHASE` env var (if set and valid 0-3)
/// 2. `model_policy.toml` `[gates.local_memory_sunset].current_phase`
/// 3. Default: Phase 0
///
/// Returns a payload suitable for capsule event emission.
pub fn resolve_sunset_phase(policy: Option<&GovernancePolicy>) -> PhaseResolutionPayload {
    let policy_phase = policy
        .map(|p| p.gates.local_memory_sunset.current_phase)
        .unwrap_or(0);

    let env_phase = std::env::var(SUNSET_PHASE_ENV_VAR)
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .filter(|&n| n <= 3);

    let (effective_phase, source) = match env_phase {
        Some(ep) => (ep, format!("env:{}", SUNSET_PHASE_ENV_VAR)),
        None => (policy_phase, "policy:model_policy.toml".to_string()),
    };

    PhaseResolutionPayload {
        policy_phase,
        env_phase,
        effective_phase,
        resolution_source: source,
    }
}

/// Get the effective SunsetPhase from a resolution payload.
pub fn effective_phase(resolution: &PhaseResolutionPayload) -> SunsetPhase {
    SunsetPhase::from_u8(resolution.effective_phase)
}

// =============================================================================
// Phase Enforcement
// =============================================================================

/// Result of phase enforcement check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseEnforcementResult {
    /// Proceed normally (no restrictions)
    Allow,
    /// Proceed with warning message
    AllowWithWarning(String),
    /// Block with error message
    Block(String),
}

/// Check if local-memory usage is allowed in the current phase.
///
/// # Arguments
/// * `backend` - The memory backend being requested
/// * `phase` - The effective sunset phase
/// * `force_deprecated` - Whether --force-deprecated flag is set
///
/// # Returns
/// * `Allow` - Proceed normally
/// * `AllowWithWarning(msg)` - Proceed but print warning
/// * `Block(msg)` - Do not proceed, return error
pub fn check_phase_enforcement(
    backend: MemoryBackend,
    phase: SunsetPhase,
    force_deprecated: bool,
) -> PhaseEnforcementResult {
    // Memvid is always allowed regardless of phase
    if backend == MemoryBackend::Memvid {
        return PhaseEnforcementResult::Allow;
    }

    // Local-memory enforcement depends on phase
    match phase {
        SunsetPhase::Phase0 => {
            // Phase 0: No restrictions
            PhaseEnforcementResult::Allow
        }

        SunsetPhase::Phase1 => {
            // Phase 1: Warning but allow
            let warning = format!(
                "\n\x1b[33m\u{26a0}\u{fe0f}  local-memory backend is deprecated.\x1b[0m\n\
                    Run `lm-import` to migrate to memvid.\n\
                    See: docs/SPEC-KIT-979-local-memory-sunset/MIGRATION.md\n"
            );
            PhaseEnforcementResult::AllowWithWarning(warning)
        }

        SunsetPhase::Phase2 => {
            if force_deprecated {
                // Phase 2 with --force-deprecated: Warning but allow
                let warning = format!(
                    "\n\x1b[33m\u{26a0}\u{fe0f}  local-memory backend is deprecated (--force-deprecated active).\x1b[0m\n\
                        This backend will be removed in Phase 3.\n\
                        Run `lm-import --all --verify` to complete migration.\n"
                );
                PhaseEnforcementResult::AllowWithWarning(warning)
            } else {
                // Phase 2 without --force-deprecated: Block
                let error = format!(
                    "\n\x1b[31m\u{1f6a8} Error: local-memory backend requires --force-deprecated flag in Phase 2.\x1b[0m\n\
                       Add --force-deprecated to proceed, or migrate to memvid:\n\
                       Run: lm-import --all --verify\n"
                );
                PhaseEnforcementResult::Block(error)
            }
        }

        SunsetPhase::Phase3 => {
            // Phase 3: Always block (regardless of --force-deprecated)
            let error = format!(
                "\n\x1b[31m\u{1f6a8} Error: local-memory backend has been removed in Phase 3.\x1b[0m\n\
                   Use memvid (the default) or run lm-import to migrate existing data.\n"
            );
            PhaseEnforcementResult::Block(error)
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_sunset_phase_from_u8() {
        assert_eq!(SunsetPhase::from_u8(0), SunsetPhase::Phase0);
        assert_eq!(SunsetPhase::from_u8(1), SunsetPhase::Phase1);
        assert_eq!(SunsetPhase::from_u8(2), SunsetPhase::Phase2);
        assert_eq!(SunsetPhase::from_u8(3), SunsetPhase::Phase3);
        // Values > 3 treated as Phase3
        assert_eq!(SunsetPhase::from_u8(4), SunsetPhase::Phase3);
        assert_eq!(SunsetPhase::from_u8(255), SunsetPhase::Phase3);
    }

    #[test]
    fn test_sunset_phase_as_u8() {
        assert_eq!(SunsetPhase::Phase0.as_u8(), 0);
        assert_eq!(SunsetPhase::Phase1.as_u8(), 1);
        assert_eq!(SunsetPhase::Phase2.as_u8(), 2);
        assert_eq!(SunsetPhase::Phase3.as_u8(), 3);
    }

    #[test]
    fn test_phase_0_allows_local_memory() {
        let result = check_phase_enforcement(
            MemoryBackend::LocalMemory,
            SunsetPhase::Phase0,
            false,
        );
        assert_eq!(result, PhaseEnforcementResult::Allow);
    }

    #[test]
    fn test_phase_1_warns_local_memory() {
        let result = check_phase_enforcement(
            MemoryBackend::LocalMemory,
            SunsetPhase::Phase1,
            false,
        );
        assert!(matches!(result, PhaseEnforcementResult::AllowWithWarning(_)));
        if let PhaseEnforcementResult::AllowWithWarning(msg) = result {
            assert!(msg.contains("deprecated"));
            assert!(msg.contains("lm-import"));
        }
    }

    #[test]
    fn test_phase_2_blocks_without_force() {
        let result = check_phase_enforcement(
            MemoryBackend::LocalMemory,
            SunsetPhase::Phase2,
            false,
        );
        assert!(matches!(result, PhaseEnforcementResult::Block(_)));
        if let PhaseEnforcementResult::Block(msg) = result {
            assert!(msg.contains("--force-deprecated"));
        }
    }

    #[test]
    fn test_phase_2_allows_with_force() {
        let result = check_phase_enforcement(
            MemoryBackend::LocalMemory,
            SunsetPhase::Phase2,
            true,
        );
        assert!(matches!(result, PhaseEnforcementResult::AllowWithWarning(_)));
        if let PhaseEnforcementResult::AllowWithWarning(msg) = result {
            assert!(msg.contains("--force-deprecated active"));
        }
    }

    #[test]
    fn test_phase_3_always_blocks() {
        // Without force
        let result1 = check_phase_enforcement(
            MemoryBackend::LocalMemory,
            SunsetPhase::Phase3,
            false,
        );
        assert!(matches!(result1, PhaseEnforcementResult::Block(_)));

        // With force (still blocks)
        let result2 = check_phase_enforcement(
            MemoryBackend::LocalMemory,
            SunsetPhase::Phase3,
            true,
        );
        assert!(matches!(result2, PhaseEnforcementResult::Block(_)));
    }

    #[test]
    fn test_memvid_always_allowed() {
        for phase_num in 0..=3 {
            let phase = SunsetPhase::from_u8(phase_num);
            for force in [false, true] {
                let result = check_phase_enforcement(
                    MemoryBackend::Memvid,
                    phase,
                    force,
                );
                assert_eq!(
                    result,
                    PhaseEnforcementResult::Allow,
                    "Memvid should always be allowed (phase={:?}, force={})",
                    phase,
                    force
                );
            }
        }
    }

    #[test]
    #[serial]
    fn test_resolve_without_policy() {
        // Ensure no env var override is set (cleanup from other tests)
        // SAFETY: Cleaning up potential leftover env var
        unsafe {
            std::env::remove_var(SUNSET_PHASE_ENV_VAR);
        }
        // Without policy, defaults to Phase 0
        let resolution = resolve_sunset_phase(None);
        assert_eq!(resolution.policy_phase, 0);
        assert_eq!(resolution.effective_phase, 0);
        assert!(resolution.resolution_source.contains("policy"));
    }

    #[test]
    #[serial]
    fn test_env_override_takes_precedence() {
        // Set env var
        // SAFETY: This test runs in isolation (via #[serial]) and cleans up the env var
        unsafe {
            std::env::set_var(SUNSET_PHASE_ENV_VAR, "2");
        }

        // Create mock policy with phase 0
        let mut policy = GovernancePolicy::default();
        policy.gates.local_memory_sunset.current_phase = 0;

        let resolution = resolve_sunset_phase(Some(&policy));

        assert_eq!(resolution.policy_phase, 0);
        assert_eq!(resolution.env_phase, Some(2));
        assert_eq!(resolution.effective_phase, 2);
        assert!(resolution.resolution_source.contains("env"));

        // Clean up
        // SAFETY: Cleaning up the env var set above
        unsafe {
            std::env::remove_var(SUNSET_PHASE_ENV_VAR);
        }
    }

    #[test]
    #[serial]
    fn test_invalid_env_value_ignored() {
        // Set invalid env var
        // SAFETY: This test runs in isolation (via #[serial]) and cleans up the env var
        unsafe {
            std::env::set_var(SUNSET_PHASE_ENV_VAR, "invalid");
        }

        let resolution = resolve_sunset_phase(None);

        // Should fall back to policy default
        assert_eq!(resolution.env_phase, None);
        assert!(resolution.resolution_source.contains("policy"));

        // Clean up
        // SAFETY: Cleaning up the env var set above
        unsafe {
            std::env::remove_var(SUNSET_PHASE_ENV_VAR);
        }
    }

    #[test]
    #[serial]
    fn test_env_value_out_of_range_ignored() {
        // Set out-of-range env var
        // SAFETY: This test runs in isolation (via #[serial]) and cleans up the env var
        unsafe {
            std::env::set_var(SUNSET_PHASE_ENV_VAR, "5");
        }

        let resolution = resolve_sunset_phase(None);

        // Values > 3 should be ignored (filtered out)
        assert_eq!(resolution.env_phase, None);

        // Clean up
        // SAFETY: Cleaning up the env var set above
        unsafe {
            std::env::remove_var(SUNSET_PHASE_ENV_VAR);
        }
    }
}
