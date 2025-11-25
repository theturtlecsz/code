//! Agent spawn retry logic (SPEC-938)
//!
//! Wraps agent spawn operations with exponential backoff retry to handle
//! transient failures (timeouts, rate limits, service unavailable).
//!
//! Integration:
//! - Uses codex_spec_kit::retry infrastructure (SPEC-945C)
//! - Classifies agent spawn errors as retryable vs permanent
//! - Logs retry attempts with telemetry

use codex_spec_kit::retry::classifier::{
    ErrorClass, PermanentError, RetryClassifiable, RetryableError,
};
use codex_spec_kit::retry::strategy::RetryConfig;
use rand::Rng;
use std::time::Duration;
use thiserror::Error;

/// Agent operation error (implements RetryClassifiable)
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Agent spawn failed: {0}")]
    SpawnFailed(String),

    #[error("Agent timeout after {0}s")]
    Timeout(u64),

    #[error("Agent failed: {0}")]
    ExecutionFailed(String),

    #[error("Agent returned no result")]
    NoResult,
}

impl RetryClassifiable for AgentError {
    fn classify(&self) -> ErrorClass {
        match self {
            // Timeout is retryable
            AgentError::Timeout(secs) => {
                ErrorClass::Retryable(RetryableError::NetworkTimeout(*secs))
            }

            // Execution errors: parse message for retry hints
            AgentError::ExecutionFailed(msg) | AgentError::SpawnFailed(msg) => {
                let msg_lower = msg.to_lowercase();

                // Retryable patterns
                if msg_lower.contains("timeout") || msg_lower.contains("timed out") {
                    ErrorClass::Retryable(RetryableError::NetworkTimeout(30))
                } else if msg_lower.contains("rate limit") || msg_lower.contains("429") {
                    // Extract retry_after if available (default 60s)
                    ErrorClass::Retryable(RetryableError::RateLimitExceeded { retry_after: 60 })
                } else if msg_lower.contains("service unavailable")
                    || msg_lower.contains("503")
                    || msg_lower.contains("502")
                    || msg_lower.contains("overloaded")
                {
                    ErrorClass::Retryable(RetryableError::ServiceUnavailable)
                } else if msg_lower.contains("connection refused")
                    || msg_lower.contains("connection reset")
                {
                    ErrorClass::Retryable(RetryableError::ConnectionRefused)
                }
                // Permanent patterns
                else if msg_lower.contains("invalid api key")
                    || msg_lower.contains("unauthorized")
                    || msg_lower.contains("401")
                {
                    ErrorClass::Permanent(PermanentError::AuthenticationFailed(msg.clone()))
                } else if msg_lower.contains("model not found")
                    || msg_lower.contains("invalid model")
                    || msg_lower.contains("404")
                {
                    ErrorClass::Permanent(PermanentError::ResourceNotFound(msg.clone()))
                } else if msg_lower.contains("quota exceeded")
                    || msg_lower.contains("insufficient quota")
                {
                    ErrorClass::Permanent(PermanentError::InvalidInput {
                        field: "quota".to_string(),
                        reason: msg.clone(),
                    })
                }
                // Default: retry with caution (could be transient)
                else {
                    ErrorClass::Retryable(RetryableError::ServiceUnavailable)
                }
            }

            // No result: permanent (agent logic error)
            AgentError::NoResult => ErrorClass::Permanent(PermanentError::InvalidInput {
                field: "result".to_string(),
                reason: "Agent completed but returned no result".to_string(),
            }),
        }
    }

    fn suggested_backoff(&self) -> Option<Duration> {
        match self.classify() {
            ErrorClass::Retryable(ref err) => match err {
                RetryableError::RateLimitExceeded { retry_after } => {
                    Some(Duration::from_secs(*retry_after))
                }
                RetryableError::NetworkTimeout(_) => Some(Duration::from_secs(5)),
                RetryableError::ServiceUnavailable => Some(Duration::from_secs(10)),
                RetryableError::ConnectionRefused => Some(Duration::from_secs(2)),
                RetryableError::DatabaseLocked => Some(Duration::from_millis(200)),
            },
            ErrorClass::Permanent(_) => None,
            ErrorClass::Degraded(_) => Some(Duration::from_millis(500)),
        }
    }
}

/// Spawn agent with retry logic
///
/// # SPEC-938 Requirements:
/// - Error classification (retryable vs permanent)
/// - Exponential backoff with jitter (100ms → 200ms → 400ms)
/// - Max 3 attempts
/// - Comprehensive telemetry (attempt #, backoff delay, error category)
///
/// # Returns:
/// - Ok((agent_id, result)) on success (possibly after retries)
/// - Err on permanent error or max retries exceeded
pub async fn spawn_agent_with_retry<F, Fut>(
    agent_name: &str,
    mut operation: F,
) -> Result<(String, String), AgentError>
where
    F: FnMut() -> Fut + Send,
    Fut: std::future::Future<Output = Result<(String, String), String>> + Send,
{
    let config = RetryConfig {
        max_attempts: 3,
        initial_backoff_ms: 100,
        max_backoff_ms: 10_000,
        backoff_multiplier: 2.0,
        jitter_factor: 0.5,
    };

    let agent_name = agent_name.to_string();
    let mut attempt = 0;

    // Manual retry loop (execute_with_backoff requires Fn, we have FnMut)
    let mut backoff_ms = config.initial_backoff_ms;

    loop {
        attempt += 1;

        tracing::info!(
            agent = agent_name,
            attempt = attempt,
            max_attempts = config.max_attempts,
            "Attempting agent spawn"
        );

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    tracing::info!(
                        agent = agent_name,
                        attempt = attempt,
                        "Agent spawn succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                // Convert string error to AgentError
                let agent_error = if e.to_lowercase().contains("timeout") {
                    AgentError::Timeout(600)
                } else {
                    AgentError::SpawnFailed(e)
                };

                // Classify error for retry decision
                let error_class = agent_error.classify();
                let is_retryable = agent_error.is_retryable();

                tracing::warn!(
                    agent = agent_name,
                    attempt = attempt,
                    error = %agent_error,
                    error_class = ?error_class,
                    is_retryable = is_retryable,
                    "Agent spawn failed"
                );

                // Check if retryable
                if !is_retryable {
                    tracing::error!(
                        agent = agent_name,
                        error = %agent_error,
                        "Permanent error, not retrying"
                    );
                    return Err(agent_error);
                }

                // Check max attempts
                if attempt >= config.max_attempts {
                    tracing::error!(
                        agent = agent_name,
                        max_attempts = config.max_attempts,
                        "Max retries exceeded"
                    );
                    return Err(AgentError::ExecutionFailed(format!(
                        "Max retries ({}) exceeded",
                        config.max_attempts
                    )));
                }

                // Apply backoff with jitter
                let jitter_range = (backoff_ms as f64 * config.jitter_factor) as u64;
                let jitter = rand::thread_rng().gen_range(0..=jitter_range);
                let actual_backoff = backoff_ms + jitter;

                tracing::info!(
                    agent = agent_name,
                    backoff_ms = actual_backoff,
                    attempt = attempt,
                    "Backing off before retry"
                );

                tokio::time::sleep(Duration::from_millis(actual_backoff)).await;

                // Exponential backoff for next attempt
                backoff_ms = ((backoff_ms as f64 * config.backoff_multiplier) as u64)
                    .min(config.max_backoff_ms);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification_timeout() {
        let err = AgentError::Timeout(600);
        assert!(err.is_retryable());
        matches!(err.classify(), ErrorClass::Retryable(_));
    }

    #[test]
    fn test_error_classification_rate_limit() {
        let err = AgentError::SpawnFailed("Rate limit exceeded".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_error_classification_invalid_api_key() {
        let err = AgentError::SpawnFailed("Invalid API key".to_string());
        assert!(!err.is_retryable());
        matches!(err.classify(), ErrorClass::Permanent(_));
    }

    #[test]
    fn test_error_classification_model_not_found() {
        let err = AgentError::SpawnFailed("Model not found: gpt-17".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_suggested_backoff_rate_limit() {
        let err = AgentError::SpawnFailed("429 Rate Limit".to_string());
        assert!(err.suggested_backoff().is_some());
        assert_eq!(err.suggested_backoff().unwrap(), Duration::from_secs(60));
    }

    #[test]
    fn test_suggested_backoff_permanent() {
        let err = AgentError::NoResult;
        assert!(!err.is_retryable());
        assert!(err.suggested_backoff().is_none());
    }
}
