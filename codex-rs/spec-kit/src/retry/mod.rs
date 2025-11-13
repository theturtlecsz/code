//! Retry logic with exponential backoff
//!
//! SPEC-945C: Error classification, exponential backoff, circuit breakers
//!
//! This module provides:
//! - Error classification (retryable vs permanent)
//! - Exponential backoff with jitter
//! - Max retry limits (3 attempts default)
//! - Circuit breaker patterns (optional)

// TODO: Implementation in Phase 1, Week 2-3

pub mod classifier;
pub mod circuit_breaker;
pub mod strategy;

pub use classifier::{ErrorClass, RetryClassifiable};
pub use strategy::{RetryConfig, execute_with_backoff};

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
}
