//! Policy toggles for gate evaluation configuration.
//!
//! This module provides a single IO boundary for reading policy-related
//! environment variables and config settings. The design separates:
//! - Pure decision functions (unit testable, no env IO)
//! - Thin IO wrapper (single integration test point)
//!
//! ## Usage
//!
//! ```no_run
//! use codex_spec_kit::config::policy_toggles::PolicyToggles;
//!
//! // Load once at startup
//! let toggles = PolicyToggles::from_env_and_config();
//!
//! if toggles.sidecar_critic_enabled {
//!     // Enable non-blocking critic review
//! }
//! ```
//!
//! ## Environment Variables
//!
//! | Canonical | Deprecated | Default |
//! |-----------|------------|---------|
//! | `SPEC_KIT_SIDECAR_CRITIC` | `SPEC_KIT_CRITIC` | `false` |
//! | (none) | `SPEC_KIT_CONSENSUS` | `false` |
//!
//! ## Deprecation Warnings
//!
//! When deprecated env vars are used, a warning is emitted once per process.
//! The `DeprecationWarning` struct captures details for logging.

use std::sync::atomic::{AtomicBool, Ordering};

/// Warning details for deprecated configuration usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeprecationWarning {
    /// The deprecated key/env var that was used
    pub deprecated_key: String,
    /// The canonical replacement
    pub canonical_key: String,
    /// Whether both deprecated and canonical were set
    pub both_present: bool,
    /// Human-readable message
    pub message: String,
}

impl DeprecationWarning {
    /// Create a warning for the SPEC_KIT_CRITIC env var.
    pub fn spec_kit_critic(both_present: bool) -> Self {
        let message = if both_present {
            "Both 'SPEC_KIT_CRITIC' (deprecated) and 'SPEC_KIT_SIDECAR_CRITIC' are set. \
             Using canonical 'SPEC_KIT_SIDECAR_CRITIC'. Remove the deprecated env var."
                .to_string()
        } else {
            "Env var 'SPEC_KIT_CRITIC' is deprecated. Use 'SPEC_KIT_SIDECAR_CRITIC' instead."
                .to_string()
        };

        Self {
            deprecated_key: "SPEC_KIT_CRITIC".to_string(),
            canonical_key: "SPEC_KIT_SIDECAR_CRITIC".to_string(),
            both_present,
            message,
        }
    }

    /// Create a warning for the SPEC_KIT_CONSENSUS env var (REMOVED in PR6).
    pub fn spec_kit_consensus_removed() -> Self {
        Self {
            deprecated_key: "SPEC_KIT_CONSENSUS".to_string(),
            canonical_key: "(removed)".to_string(),
            both_present: false,
            message: "SPEC_KIT_CONSENSUS is deprecated and ignored. \
                      Legacy multi-agent voting has been removed (PR6). \
                      The single-owner pipeline is now the only supported mode. \
                      See: docs/MODEL-POLICY.md"
                .to_string(),
        }
    }

    /// Emit this warning via tracing.
    pub fn emit(&self) {
        tracing::warn!("{}", self.message);
    }
}

/// Policy toggles resolved from environment and configuration.
///
/// Load once at application startup and pass by reference to avoid
/// repeated env lookups.
#[derive(Debug, Clone, Default)]
pub struct PolicyToggles {
    /// Enable non-blocking critic sidecar review.
    pub sidecar_critic_enabled: bool,

    /// Enable legacy multi-agent consensus (DEPRECATED per GR-001).
    pub legacy_voting_enabled: bool,

    /// Deprecation warnings encountered during resolution.
    pub warnings: Vec<DeprecationWarning>,
}

impl PolicyToggles {
    /// Load policy toggles from environment variables and config.
    ///
    /// This is the single IO boundary - call once at startup.
    pub fn from_env_and_config() -> Self {
        let canonical_critic = std::env::var("SPEC_KIT_SIDECAR_CRITIC").ok();
        let deprecated_critic = std::env::var("SPEC_KIT_CRITIC").ok();
        let consensus_var = std::env::var("SPEC_KIT_CONSENSUS").ok();

        let (sidecar_critic_enabled, critic_warning) =
            resolve_sidecar_critic(canonical_critic.as_deref(), deprecated_critic.as_deref());

        let (legacy_voting_enabled, voting_warning) =
            resolve_legacy_voting(consensus_var.as_deref());

        let mut warnings = Vec::new();
        if let Some(w) = critic_warning {
            emit_warning_once_critic(&w);
            warnings.push(w);
        }
        if let Some(w) = voting_warning {
            emit_warning_once_consensus(&w);
            warnings.push(w);
        }

        Self {
            sidecar_critic_enabled,
            legacy_voting_enabled,
            warnings,
        }
    }

    /// Check if any deprecation warnings were generated.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

// ============================================================================
// PURE DECISION FUNCTIONS (unit testable, no env IO)
// ============================================================================

/// Resolve sidecar critic setting from canonical and deprecated env values.
///
/// Precedence:
/// 1. If canonical (`SPEC_KIT_SIDECAR_CRITIC`) is set, use it
/// 2. If only deprecated (`SPEC_KIT_CRITIC`) is set, use it + warn
/// 3. Default: `false`
///
/// # Arguments
/// * `canonical` - Value of `SPEC_KIT_SIDECAR_CRITIC` env var (if set)
/// * `deprecated` - Value of `SPEC_KIT_CRITIC` env var (if set)
///
/// # Returns
/// Tuple of (enabled, optional deprecation warning)
pub fn resolve_sidecar_critic(
    canonical: Option<&str>,
    deprecated: Option<&str>,
) -> (bool, Option<DeprecationWarning>) {
    match (canonical, deprecated) {
        (Some(c), Some(_d)) => {
            // Both set: canonical wins, warn about deprecated
            (parse_bool(c), Some(DeprecationWarning::spec_kit_critic(true)))
        }
        (Some(c), None) => {
            // Only canonical set: use it, no warning
            (parse_bool(c), None)
        }
        (None, Some(d)) => {
            // Only deprecated set: use it, warn
            (
                parse_bool(d),
                Some(DeprecationWarning::spec_kit_critic(false)),
            )
        }
        (None, None) => {
            // Neither set: default false
            (false, None)
        }
    }
}

/// Resolve legacy voting setting from env value.
///
/// **REMOVED in PR6**: Legacy voting is no longer supported. This function
/// always returns `false`. A deprecation warning is emitted **only** when
/// the env var is set to a truthy value (attempting to enable removed feature).
///
/// Warning behavior:
/// - `SPEC_KIT_CONSENSUS` not set → no warning, no spam
/// - `SPEC_KIT_CONSENSUS=false` → no warning (explicit disable is fine)
/// - `SPEC_KIT_CONSENSUS=true` → warn-once: "deprecated and ignored"
///
/// # Arguments
/// * `val` - Value of `SPEC_KIT_CONSENSUS` env var (if set)
///
/// # Returns
/// Tuple of (enabled=false, optional deprecation warning)
pub fn resolve_legacy_voting(val: Option<&str>) -> (bool, Option<DeprecationWarning>) {
    match val {
        // PR6: Only warn if user is trying to ENABLE the removed feature
        Some(v) if parse_bool(v) => {
            (false, Some(DeprecationWarning::spec_kit_consensus_removed()))
        }
        // Not set, empty, or explicitly disabled → no warning, no spam
        _ => (false, None),
    }
}

/// Parse a string value as boolean.
///
/// Accepts: "true", "1", "TRUE", "yes", "on" (case insensitive)
fn parse_bool(val: &str) -> bool {
    matches!(
        val.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "on"
    )
}

// ============================================================================
// WARN-ONCE HELPERS (process-global)
// ============================================================================

static WARNED_CRITIC: AtomicBool = AtomicBool::new(false);
static WARNED_CONSENSUS: AtomicBool = AtomicBool::new(false);

fn emit_warning_once_critic(warning: &DeprecationWarning) {
    if !WARNED_CRITIC.swap(true, Ordering::Relaxed) {
        warning.emit();
    }
}

fn emit_warning_once_consensus(warning: &DeprecationWarning) {
    if !WARNED_CONSENSUS.swap(true, Ordering::Relaxed) {
        warning.emit();
    }
}

// ============================================================================
// UNIT TESTS (pure functions, no env mutation needed)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // resolve_sidecar_critic tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_resolve_critic_both_none() {
        let (enabled, warning) = resolve_sidecar_critic(None, None);
        assert!(!enabled);
        assert!(warning.is_none());
    }

    #[test]
    fn test_resolve_critic_canonical_only_true() {
        let (enabled, warning) = resolve_sidecar_critic(Some("true"), None);
        assert!(enabled);
        assert!(warning.is_none());
    }

    #[test]
    fn test_resolve_critic_canonical_only_false() {
        let (enabled, warning) = resolve_sidecar_critic(Some("false"), None);
        assert!(!enabled);
        assert!(warning.is_none());
    }

    #[test]
    fn test_resolve_critic_deprecated_only_true() {
        let (enabled, warning) = resolve_sidecar_critic(None, Some("true"));
        assert!(enabled);
        assert!(warning.is_some());
        let w = warning.unwrap();
        assert!(!w.both_present);
        assert_eq!(w.deprecated_key, "SPEC_KIT_CRITIC");
    }

    #[test]
    fn test_resolve_critic_deprecated_only_false() {
        let (enabled, warning) = resolve_sidecar_critic(None, Some("false"));
        assert!(!enabled);
        assert!(warning.is_some()); // Still warn about usage
    }

    #[test]
    fn test_resolve_critic_both_set_canonical_wins() {
        // Canonical false, deprecated true -> canonical wins (false)
        let (enabled, warning) = resolve_sidecar_critic(Some("false"), Some("true"));
        assert!(!enabled);
        assert!(warning.is_some());
        let w = warning.unwrap();
        assert!(w.both_present);
    }

    #[test]
    fn test_resolve_critic_both_set_canonical_true() {
        // Canonical true, deprecated false -> canonical wins (true)
        let (enabled, warning) = resolve_sidecar_critic(Some("true"), Some("false"));
        assert!(enabled);
        assert!(warning.is_some());
        assert!(warning.unwrap().both_present);
    }

    #[test]
    fn test_resolve_critic_accepts_various_truthy_values() {
        for val in ["true", "TRUE", "True", "1", "yes", "YES", "on", "ON"] {
            let (enabled, _) = resolve_sidecar_critic(Some(val), None);
            assert!(enabled, "Expected '{val}' to be truthy");
        }
    }

    #[test]
    fn test_resolve_critic_rejects_various_falsy_values() {
        for val in ["false", "FALSE", "0", "no", "off", ""] {
            let (enabled, _) = resolve_sidecar_critic(Some(val), None);
            assert!(!enabled, "Expected '{val}' to be falsy");
        }
    }

    // -------------------------------------------------------------------------
    // resolve_legacy_voting tests (PR6: feature removed, always returns false)
    // -------------------------------------------------------------------------

    #[test]
    fn test_resolve_voting_none_no_warning() {
        // Not set → no warning, no spam
        let (enabled, warning) = resolve_legacy_voting(None);
        assert!(!enabled);
        assert!(warning.is_none(), "No warning when env var not set");
    }

    #[test]
    fn test_resolve_voting_true_warns_and_ignores() {
        // Truthy value → warn (user trying to enable removed feature)
        let (enabled, warning) = resolve_legacy_voting(Some("true"));
        assert!(!enabled, "PR6: voting always disabled");
        assert!(warning.is_some(), "Warn when trying to enable removed feature");
        let w = warning.unwrap();
        assert_eq!(w.deprecated_key, "SPEC_KIT_CONSENSUS");
        assert!(w.message.contains("ignored"));
    }

    #[test]
    fn test_resolve_voting_false_no_warning() {
        // Explicit disable → no warning (not trying to enable)
        let (enabled, warning) = resolve_legacy_voting(Some("false"));
        assert!(!enabled);
        assert!(warning.is_none(), "No warning for explicit disable");
    }

    #[test]
    fn test_resolve_voting_one_warns_and_ignores() {
        // "1" is truthy → warn
        let (enabled, warning) = resolve_legacy_voting(Some("1"));
        assert!(!enabled, "PR6: voting always disabled");
        assert!(warning.is_some(), "Warn for truthy value");
    }

    #[test]
    fn test_resolve_voting_zero_no_warning() {
        // "0" is falsy → no warning
        let (enabled, warning) = resolve_legacy_voting(Some("0"));
        assert!(!enabled);
        assert!(warning.is_none(), "No warning for falsy value");
    }

    #[test]
    fn test_resolve_voting_empty_no_warning() {
        // Empty string → no warning
        let (enabled, warning) = resolve_legacy_voting(Some(""));
        assert!(!enabled);
        assert!(warning.is_none(), "No warning for empty string");
    }

    // -------------------------------------------------------------------------
    // DeprecationWarning tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_deprecation_warning_critic_single() {
        let w = DeprecationWarning::spec_kit_critic(false);
        assert!(!w.both_present);
        assert!(w.message.contains("deprecated"));
        assert!(w.message.contains("SPEC_KIT_SIDECAR_CRITIC"));
    }

    #[test]
    fn test_deprecation_warning_critic_both() {
        let w = DeprecationWarning::spec_kit_critic(true);
        assert!(w.both_present);
        assert!(w.message.contains("Both"));
        assert!(w.message.contains("Remove"));
    }

    #[test]
    fn test_deprecation_warning_consensus_removed() {
        let w = DeprecationWarning::spec_kit_consensus_removed();
        assert!(w.message.contains("ignored"));
        assert!(w.message.contains("removed"));
    }

    // -------------------------------------------------------------------------
    // parse_bool tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_bool_with_whitespace() {
        assert!(parse_bool("  true  "));
        assert!(parse_bool("\t1\n"));
        assert!(!parse_bool("  false  "));
    }
}
