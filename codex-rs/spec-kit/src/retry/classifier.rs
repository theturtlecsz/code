//! Error classification for retry decisions
//!
//! SPEC-945C: Error classification hierarchy

use std::time::Duration;

/// Top-level error classification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorClass {
    /// Transient errors that should be retried with backoff
    Retryable(RetryableError),

    /// Permanent errors that should NOT be retried
    Permanent(PermanentError),

    /// Degraded state (partial success, e.g., 2/3 consensus)
    Degraded(DegradedError),
}

/// Transient errors (retry recommended)
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RetryableError {
    #[error("Network timeout after {0}s")]
    NetworkTimeout(u64),

    #[error("Rate limit exceeded, retry after {retry_after}s")]
    RateLimitExceeded { retry_after: u64 },

    #[error("Service unavailable (HTTP 503)")]
    ServiceUnavailable,

    #[error("Database locked (SQLITE_BUSY)")]
    DatabaseLocked,

    #[error("Connection refused")]
    ConnectionRefused,
}

/// Permanent errors (do NOT retry)
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PermanentError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Invalid input in field '{field}': {reason}")]
    InvalidInput { field: String, reason: String },

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
}

/// Degraded state (partial success)
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DegradedError {
    #[error("Degraded consensus: {success}/{total} agents succeeded")]
    DegradedConsensus { success: usize, total: usize },
}

/// Trait for error classification
pub trait RetryClassifiable {
    fn classify(&self) -> ErrorClass;

    fn is_retryable(&self) -> bool {
        matches!(self.classify(), ErrorClass::Retryable(_))
    }

    fn suggested_backoff(&self) -> Option<Duration>;
}

// TODO: Implement RetryClassifiable for codex_spec_kit::error::SpecKitError in Week 2-3, Day 3
