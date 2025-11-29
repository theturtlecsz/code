//! Retry logic with exponential backoff
//!
//! SPEC-945C: Error classification, exponential backoff, circuit breakers
//! Enhanced with Auto Drive patterns (P5-SYNC)
//!
//! This module provides:
//! - Error classification (retryable vs permanent)
//! - Exponential backoff with jitter
//! - Max retry limits (3 attempts default)
//! - Total elapsed timeout (Auto Drive pattern)
//! - Cancellation support (Auto Drive pattern)
//! - Status callbacks for progress reporting (Auto Drive pattern)
//! - Circuit breaker patterns (optional)

pub mod circuit_breaker;
pub mod classifier;
pub mod strategy;

pub use classifier::{ErrorClass, RetryClassifiable};
pub use strategy::{
    RetryConfig, RetryStatus, execute_with_backoff, execute_with_backoff_cancellable,
};

use std::time::Duration;

/// Retry module result type
pub type Result<T> = std::result::Result<T, RetryError>;

/// Retry error types
#[derive(Debug, thiserror::Error)]
pub enum RetryError {
    #[error("Max retry attempts exceeded: {0}")]
    MaxAttemptsExceeded(usize),

    #[error("Permanent error (not retryable): {0}")]
    PermanentError(String),

    #[error("Circuit breaker open")]
    CircuitOpen,

    /// Total elapsed time exceeded (Auto Drive pattern)
    #[error("Retry timeout after {elapsed:?}")]
    Timeout {
        elapsed: Duration,
        last_error: String,
    },

    /// External cancellation requested (Auto Drive pattern)
    #[error("Retry aborted by cancellation")]
    Aborted,

    /// Rate limited with explicit wait time (Auto Drive pattern)
    #[error("Rate limited, retry at {retry_after:?}")]
    RateLimited { retry_after: Duration },
}
