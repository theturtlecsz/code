//! P6-SYNC Phase 3: Fault Injection Framework
//!
//! Enables deterministic testing of error handling via injectable faults.
//! Feature-gated behind `dev-faults` to prevent accidental production use.
//!
//! ## Supported Fault Types
//! - **Disconnect**: Simulates stream disconnection before completion
//! - **RateLimit (429)**: Simulates API rate limiting with optional reset hints
//! - **Timeout**: Simulates operation timeout (extension from Auto Drive)
//!
//! ## Configuration via Environment Variables
//! ```bash
//! CODEX_FAULTS_SCOPE=spec_kit           # Enable faults for spec-kit scope
//! CODEX_FAULTS=disconnect:3,429:1,timeout:2  # Inject 3 disconnects, 1 rate limit, 2 timeouts
//! CODEX_FAULTS_429_RESET=now+30s        # Optional rate limit reset hint
//! ```
//!
//! Pattern source: Auto Drive `faults.rs`

// Note: #[cfg(feature = "dev-faults")] is specified at the module inclusion site (lib.rs)
#![allow(clippy::unwrap_used)] // Fault injection: panicking on poisoned lock is acceptable

use anyhow::anyhow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Scope flag for fault injection - determines which subsystem faults apply to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaultScope {
    /// Spec-Kit multi-agent automation (renamed from AutoDrive)
    SpecKit,
}

/// Represents a fault to inject into the system.
#[derive(Debug, Clone)]
pub enum InjectedFault {
    /// Simulates stream disconnection before completion
    Disconnect,
    /// Simulates 429 rate limiting with optional reset hint
    RateLimit { reset_hint: Option<FaultReset> },
    /// Simulates operation timeout (P6-SYNC extension)
    Timeout { duration_ms: u64 },
}

/// Reset hint for rate limit faults.
#[derive(Debug, Clone)]
pub enum FaultReset {
    /// Reset after N seconds from now
    Seconds(u64),
    /// Reset at specific instant (computed from timestamp)
    Timestamp(Instant),
}

/// Internal fault configuration (per-scope).
#[derive(Debug, Default)]
struct FaultConfig {
    disconnect: AtomicUsize,
    rate_limit: AtomicUsize,
    timeout: AtomicUsize,
    timeout_duration_ms: AtomicUsize,
    rate_limit_reset: Mutex<Option<FaultReset>>,
}

/// Global fault configuration storage.
static CONFIG: OnceLock<HashMap<FaultScope, FaultConfig>> = OnceLock::new();

/// Parse fault scope from environment variable.
fn parse_fault_scope() -> Option<FaultScope> {
    match std::env::var("CODEX_FAULTS_SCOPE").ok().as_deref() {
        Some("spec_kit") | Some("speckit") => Some(FaultScope::SpecKit),
        // Legacy compatibility
        Some("auto_drive") => Some(FaultScope::SpecKit),
        _ => None,
    }
}

/// Parse rate limit reset hint from environment variable.
fn parse_reset_hint() -> Option<FaultReset> {
    let value = std::env::var("CODEX_FAULTS_429_RESET").ok()?;

    // Try parsing as plain seconds
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(FaultReset::Seconds(seconds));
    }

    // Try parsing as "now+Ns" format
    if let Some(stripped) = value.strip_prefix("now+")
        && let Ok(seconds) = stripped.trim_end_matches('s').parse::<u64>()
    {
        return Some(FaultReset::Seconds(seconds));
    }

    // Try parsing as RFC3339 timestamp
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&value) {
        let now = chrono::Utc::now();
        let delta = parsed.signed_duration_since(now);
        let secs = delta.num_seconds().max(0) as u64;
        let instant = Instant::now() + Duration::from_secs(secs);
        return Some(FaultReset::Timestamp(instant));
    }

    None
}

/// Parse timeout duration from environment variable.
fn parse_timeout_duration() -> u64 {
    std::env::var("CODEX_FAULTS_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30_000) // Default 30 second timeout
}

/// Initialize fault configuration from environment variables.
fn init_config() -> HashMap<FaultScope, FaultConfig> {
    let mut map = HashMap::new();

    if let Some(scope) = parse_fault_scope()
        && let Ok(spec) = std::env::var("CODEX_FAULTS")
    {
        let cfg = FaultConfig::default();
        let timeout_ms = parse_timeout_duration();
        cfg.timeout_duration_ms
            .store(timeout_ms as usize, Ordering::Relaxed);

        for entry in spec.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if let Some((label, count)) = entry.split_once(':')
                && let Ok(num) = count.parse::<usize>()
            {
                match label {
                    "disconnect" => cfg.disconnect.store(num, Ordering::Relaxed),
                    "429" | "rate_limit" => cfg.rate_limit.store(num, Ordering::Relaxed),
                    "timeout" => cfg.timeout.store(num, Ordering::Relaxed),
                    _ => {
                        tracing::warn!("[faults] Unknown fault type: {}", label);
                    }
                }
            }
        }

        *cfg.rate_limit_reset.lock().unwrap() = parse_reset_hint();

        // Log before moving into map
        let disconnect_count = cfg.disconnect.load(Ordering::Relaxed);
        let rate_limit_count = cfg.rate_limit.load(Ordering::Relaxed);
        let timeout_count = cfg.timeout.load(Ordering::Relaxed);

        map.insert(scope, cfg);

        tracing::info!(
            "[faults] Initialized for {:?}: disconnect={}, rate_limit={}, timeout={}",
            scope,
            disconnect_count,
            rate_limit_count,
            timeout_count
        );
    }

    map
}

/// Get or initialize the global fault configuration.
fn config() -> &'static HashMap<FaultScope, FaultConfig> {
    CONFIG.get_or_init(init_config)
}

/// Determine whether a fault should fire for the given scope.
///
/// Returns `Some(fault)` if a fault should be injected, consuming one count.
/// Returns `None` if no faults are configured or all counts exhausted.
///
/// # Priority Order
/// 1. Disconnect (most severe)
/// 2. Rate limit
/// 3. Timeout
pub fn next_fault(scope: FaultScope) -> Option<InjectedFault> {
    let cfg = config().get(&scope)?;

    // Check disconnect first (highest priority)
    if cfg.disconnect.load(Ordering::Relaxed) > 0 {
        let remaining = cfg.disconnect.fetch_sub(1, Ordering::Relaxed);
        if remaining > 0 {
            tracing::warn!(
                "[faults] Injecting transient disconnect (remaining: {})",
                remaining - 1
            );
            return Some(InjectedFault::Disconnect);
        }
    }

    // Check rate limit
    if cfg.rate_limit.load(Ordering::Relaxed) > 0 {
        let remaining = cfg.rate_limit.fetch_sub(1, Ordering::Relaxed);
        if remaining > 0 {
            tracing::warn!(
                "[faults] Injecting 429 rate limit (remaining: {})",
                remaining - 1
            );
            return Some(InjectedFault::RateLimit {
                reset_hint: cfg.rate_limit_reset.lock().unwrap().clone(),
            });
        }
    }

    // Check timeout (P6-SYNC extension)
    if cfg.timeout.load(Ordering::Relaxed) > 0 {
        let remaining = cfg.timeout.fetch_sub(1, Ordering::Relaxed);
        if remaining > 0 {
            let duration_ms = cfg.timeout_duration_ms.load(Ordering::Relaxed) as u64;
            tracing::warn!(
                "[faults] Injecting timeout {}ms (remaining: {})",
                duration_ms,
                remaining - 1
            );
            return Some(InjectedFault::Timeout { duration_ms });
        }
    }

    None
}

/// Convert a fault into an `anyhow::Error` matching production failure patterns.
///
/// The generated errors mimic real production failures to test error handling paths.
pub fn fault_to_error(fault: InjectedFault) -> anyhow::Error {
    match fault {
        InjectedFault::Disconnect => {
            anyhow!("model stream error: stream disconnected before completion")
        }
        InjectedFault::RateLimit { reset_hint } => match reset_hint {
            Some(FaultReset::Seconds(secs)) => {
                anyhow!(
                    "rate limit exceeded: retry after {} seconds (fault injected)",
                    secs
                )
            }
            Some(FaultReset::Timestamp(instant)) => {
                let remaining = instant.saturating_duration_since(Instant::now());
                anyhow!(
                    "rate limit exceeded: retry after {:?} (fault injected)",
                    remaining
                )
            }
            None => anyhow!("rate limit exceeded: 429 Too Many Requests (fault injected)"),
        },
        InjectedFault::Timeout { duration_ms } => {
            anyhow!(
                "operation timed out after {}ms (fault injected)",
                duration_ms
            )
        }
    }
}

/// Check if fault injection is enabled for a scope.
pub fn faults_enabled(scope: FaultScope) -> bool {
    config().contains_key(&scope)
}

/// Get remaining fault counts for a scope (for diagnostics).
pub fn remaining_faults(scope: FaultScope) -> Option<(usize, usize, usize)> {
    let cfg = config().get(&scope)?;
    Some((
        cfg.disconnect.load(Ordering::Relaxed),
        cfg.rate_limit.load(Ordering::Relaxed),
        cfg.timeout.load(Ordering::Relaxed),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fault_to_error_disconnect() {
        let err = fault_to_error(InjectedFault::Disconnect);
        let msg = err.to_string();
        assert!(msg.contains("stream disconnected"));
    }

    #[test]
    fn test_fault_to_error_rate_limit_no_hint() {
        let err = fault_to_error(InjectedFault::RateLimit { reset_hint: None });
        let msg = err.to_string();
        assert!(msg.contains("429"));
        assert!(msg.contains("fault injected"));
    }

    #[test]
    fn test_fault_to_error_rate_limit_with_seconds() {
        let err = fault_to_error(InjectedFault::RateLimit {
            reset_hint: Some(FaultReset::Seconds(30)),
        });
        let msg = err.to_string();
        assert!(msg.contains("30 seconds"));
    }

    #[test]
    fn test_fault_to_error_timeout() {
        let err = fault_to_error(InjectedFault::Timeout { duration_ms: 5000 });
        let msg = err.to_string();
        assert!(msg.contains("5000ms"));
        assert!(msg.contains("timed out"));
    }

    #[test]
    fn test_fault_scope_equality() {
        assert_eq!(FaultScope::SpecKit, FaultScope::SpecKit);
    }

    #[test]
    fn test_injected_fault_clone() {
        let fault = InjectedFault::RateLimit {
            reset_hint: Some(FaultReset::Seconds(60)),
        };
        #[allow(clippy::redundant_clone)]
        let cloned = fault.clone();
        match cloned {
            InjectedFault::RateLimit {
                reset_hint: Some(FaultReset::Seconds(s)),
            } => {
                assert_eq!(s, 60);
            }
            _ => panic!("Clone failed"),
        }
    }
}
