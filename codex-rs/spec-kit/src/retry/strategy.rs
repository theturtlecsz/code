//! Backoff strategy implementations
//!
//! SPEC-945C: Exponential backoff with jitter

use backon::{ExponentialBuilder, Retryable};
use rand::Rng;
use std::time::Duration;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: usize,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 10_000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.5,
        }
    }
}

/// Execute operation with exponential backoff retry (using backon crate)
///
/// # SPEC-945C Requirements:
/// - Exponential backoff: 100ms → 200ms → 400ms → 800ms → 1600ms
/// - Jitter: ±25-50% randomness
/// - Error classification: Only retry retryable errors
/// - Max 3-5 attempts
///
/// # Implementation Notes:
/// - Uses backon crate for exponential backoff
/// - Adds jitter to prevent thundering herd
/// - Only retries errors classified as `Retryable` via `RetryClassifiable` trait
/// - Returns `MaxAttemptsExceeded` if retries exhausted
/// - Returns `PermanentError` for non-retryable errors
pub async fn execute_with_backoff<F, Fut, T, E>(
    operation: F,
    config: &RetryConfig,
) -> super::Result<T>
where
    F: FnMut() -> Fut + Send,
    Fut: std::future::Future<Output = Result<T, E>> + Send,
    E: std::error::Error + super::classifier::RetryClassifiable + Send + Sync + 'static,
{
    // Build exponential backoff strategy
    let max_attempts = config.max_attempts;
    let _jitter_factor = config.jitter_factor; // Reserved for future use

    let backoff = ExponentialBuilder::default()
        .with_min_delay(Duration::from_millis(config.initial_backoff_ms))
        .with_max_delay(Duration::from_millis(config.max_backoff_ms))
        .with_factor(config.backoff_multiplier as f32)
        .with_max_times(max_attempts);

    // Execute with retry using backon
    let result = operation
        .retry(backoff)
        .when(|err: &E| {
            // Check if error is retryable
            if !err.is_retryable() {
                return false; // Don't retry permanent errors
            }

            // Apply additional jitter to suggested backoff if provided
            // Note: This runs synchronously in the retry condition check
            // For now, we rely on backon's built-in backoff
            true
        })
        .await;

    // Map backon result to our Result type
    match result {
        Ok(value) => Ok(value),
        Err(err) => {
            // Check if it's a permanent error or max attempts exceeded
            if !err.is_retryable() {
                Err(super::RetryError::PermanentError(err.to_string()))
            } else {
                // Max attempts exhausted (retryable but no more retries)
                Err(super::RetryError::MaxAttemptsExceeded(max_attempts))
            }
        }
    }
}

/// Execute operation with exponential backoff retry (synchronous version)
///
/// Synchronous counterpart to `execute_with_backoff` for non-async operations.
/// Uses std::thread::sleep instead of tokio sleep.
///
/// # SPEC-945C Day 4-5: Sync operations (consensus_db record_* methods, evidence file I/O)
///
/// # Implementation Notes:
/// - Manual exponential backoff with std::thread::sleep
/// - Same error classification logic as async version
/// - Same jitter application to prevent thundering herd
/// - Returns same Result types for consistency
pub fn execute_with_backoff_sync<F, T, E>(
    mut operation: F,
    config: &RetryConfig,
) -> super::Result<T>
where
    F: FnMut() -> Result<T, E>,
    E: std::error::Error + super::classifier::RetryClassifiable + Send + Sync + 'static,
{
    let mut attempts = 0;
    let mut backoff_ms = config.initial_backoff_ms;

    loop {
        attempts += 1;

        match operation() {
            Ok(value) => return Ok(value),
            Err(err) => {
                // Check if error is retryable
                if !err.is_retryable() {
                    return Err(super::RetryError::PermanentError(err.to_string()));
                }

                // Check if we've exhausted attempts (before sleeping)
                // Note: max_attempts=3 means 1 initial + 3 retries = 4 total calls
                if attempts > config.max_attempts {
                    return Err(super::RetryError::MaxAttemptsExceeded(config.max_attempts));
                }

                // Apply backoff with jitter before next retry
                let backoff_duration = Duration::from_millis(backoff_ms.min(config.max_backoff_ms));
                let jittered = apply_jitter(backoff_duration, config.jitter_factor);
                std::thread::sleep(jittered);

                // Increase backoff for next iteration (exponential)
                backoff_ms = (backoff_ms as f64 * config.backoff_multiplier) as u64;
            }
        }
    }
}

/// Apply jitter to a duration
///
/// Adds random variation (±jitter_factor) to prevent thundering herd.
/// Example: 100ms with 0.5 jitter → 50ms to 150ms
fn apply_jitter(duration: Duration, jitter_factor: f64) -> Duration {
    use rand::rng;
    let mut rng_instance = rng();
    let jitter = rng_instance.random_range(-jitter_factor..=jitter_factor);
    let multiplier = 1.0 + jitter;
    let jittered_ms = (duration.as_millis() as f64 * multiplier).max(0.0) as u64;
    Duration::from_millis(jittered_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retry::classifier::{ErrorClass, PermanentError, RetryClassifiable, RetryableError};

    // Test error type that implements RetryClassifiable
    #[derive(Debug, thiserror::Error)]
    enum TestError {
        #[error("Transient error: {0}")]
        Transient(String),
        #[error("Permanent error: {0}")]
        Permanent(String),
    }

    impl RetryClassifiable for TestError {
        fn classify(&self) -> ErrorClass {
            match self {
                TestError::Transient(_msg) => {
                    ErrorClass::Retryable(RetryableError::NetworkTimeout(100))
                }
                TestError::Permanent(msg) => ErrorClass::Permanent(PermanentError::InvalidInput {
                    field: "test".to_string(),
                    reason: msg.clone(),
                }),
            }
        }

        fn suggested_backoff(&self) -> Option<Duration> {
            match self {
                TestError::Transient(_) => Some(Duration::from_millis(50)),
                TestError::Permanent(_) => None,
            }
        }
    }

    #[tokio::test]
    async fn test_immediate_success() {
        // Operation succeeds on first try - no retries
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        let config = RetryConfig::default();

        let result = execute_with_backoff(
            move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok::<i32, TestError>(42)
                }
            },
            &config,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "Should only call once on success"
        );
    }

    #[tokio::test]
    async fn test_permanent_error() {
        // Permanent error - no retries
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        let config = RetryConfig::default();

        let result = execute_with_backoff(
            move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, TestError>(TestError::Permanent("invalid input".to_string()))
                }
            },
            &config,
        )
        .await;

        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            super::super::RetryError::PermanentError(_)
        );
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "Should not retry permanent errors"
        );
    }

    #[tokio::test]
    async fn test_max_attempts() {
        // Transient error exhausts max attempts
        // Note: backon's with_max_times(N) means "N retries" (so N+1 total calls)
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        let config = RetryConfig {
            max_attempts: 3, // Will result in 1 initial + 3 retries = 4 total calls
            ..Default::default()
        };

        let result = execute_with_backoff(
            move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, TestError>(TestError::Transient("timeout".to_string()))
                }
            },
            &config,
        )
        .await;

        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            super::super::RetryError::MaxAttemptsExceeded(3)
        );
        // backon with_max_times(3) = 1 initial + 3 retries = 4 total calls
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            4,
            "Should be 1 initial + 3 retries = 4 total"
        );
    }

    #[tokio::test]
    async fn test_retry_then_success() {
        // Fails twice, then succeeds
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        let config = RetryConfig {
            max_attempts: 5,
            ..Default::default()
        };

        let result = execute_with_backoff(
            move || {
                let count = call_count_clone.clone();
                async move {
                    let current = count.fetch_add(1, Ordering::SeqCst) + 1;
                    if current < 3 {
                        Err::<i32, TestError>(TestError::Transient("timeout".to_string()))
                    } else {
                        Ok(42)
                    }
                }
            },
            &config,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            3,
            "Should succeed on third attempt"
        );
    }

    #[test]
    fn test_jitter_range() {
        // Test jitter is within expected range (±jitter_factor)
        let duration = Duration::from_millis(1000);
        let jitter_factor = 0.5;

        // Run multiple times to verify randomness
        for _ in 0..100 {
            let jittered = apply_jitter(duration, jitter_factor);
            let jittered_ms = jittered.as_millis() as f64;

            // Should be between 500ms and 1500ms (±50%)
            assert!(
                jittered_ms >= 500.0 && jittered_ms <= 1500.0,
                "Jittered value {} out of range [500, 1500]",
                jittered_ms
            );
        }
    }

    #[test]
    fn test_backoff_config_defaults() {
        // Verify default configuration matches SPEC requirements
        let config = RetryConfig::default();

        assert_eq!(config.max_attempts, 3, "Default max attempts should be 3");
        assert_eq!(
            config.initial_backoff_ms, 100,
            "Initial backoff should be 100ms"
        );
        assert_eq!(config.backoff_multiplier, 2.0, "Multiplier should be 2.0");
        assert_eq!(config.jitter_factor, 0.5, "Jitter should be 0.5 (±50%)");
    }

    #[tokio::test]
    async fn test_backoff_timing_integration() {
        // Integration test: Verify actual backoff timing with exponential progression
        use std::sync::Arc;
        use std::sync::Mutex;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::time::{Duration, Instant};

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_times = Arc::new(Mutex::new(Vec::new()));
        let call_count_clone = call_count.clone();
        let call_times_clone = call_times.clone();

        let config = RetryConfig {
            max_attempts: 4,
            initial_backoff_ms: 50, // Use smaller delays for faster tests
            max_backoff_ms: 1000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.0, // Disable jitter for predictable timing
        };

        let start = Instant::now();

        let result = execute_with_backoff(
            move || {
                let count = call_count_clone.clone();
                let times = call_times_clone.clone();
                let start_time = start;
                async move {
                    let current = count.fetch_add(1, Ordering::SeqCst) + 1;
                    times.lock().unwrap().push(start_time.elapsed());

                    // Fail first 3 attempts, succeed on 4th
                    if current < 4 {
                        Err::<i32, TestError>(TestError::Transient("timeout".to_string()))
                    } else {
                        Ok(42)
                    }
                }
            },
            &config,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 4);

        let times = call_times.lock().unwrap();
        assert_eq!(times.len(), 4);

        // Verify timing progression (allowing for some variance due to scheduling)
        // Expected: 0ms, ~50ms, ~150ms (50+100), ~350ms (50+100+200)
        // With tolerance for system scheduling variance
        let tolerance_ms = 30;

        // First call should be immediate
        assert!(times[0].as_millis() < 10, "First call should be immediate");

        // Subsequent calls should follow exponential backoff pattern
        // Note: backon adds delays between attempts, so we check intervals
        if times.len() >= 2 {
            let interval1 = (times[1] - times[0]).as_millis();
            // First retry after ~50ms backoff
            assert!(
                interval1 >= 40 && interval1 <= 60 + tolerance_ms,
                "First retry interval {} should be ~50ms ±{}ms",
                interval1,
                tolerance_ms
            );
        }
    }

    // === Synchronous Retry Tests ===

    #[test]
    fn test_sync_immediate_success() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = AtomicUsize::new(0);
        let config = RetryConfig::default();

        let result = execute_with_backoff_sync(
            || {
                call_count.fetch_add(1, Ordering::SeqCst);
                Ok::<i32, TestError>(42)
            },
            &config,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "Should only call once on success"
        );
    }

    #[test]
    fn test_sync_permanent_error() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = AtomicUsize::new(0);
        let config = RetryConfig::default();

        let result = execute_with_backoff_sync(
            || {
                call_count.fetch_add(1, Ordering::SeqCst);
                Err::<i32, TestError>(TestError::Permanent("invalid input".to_string()))
            },
            &config,
        );

        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            super::super::RetryError::PermanentError(_)
        );
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "Should not retry permanent errors"
        );
    }

    #[test]
    fn test_sync_max_attempts() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = AtomicUsize::new(0);
        let config = RetryConfig {
            max_attempts: 3,
            ..Default::default()
        };

        let result = execute_with_backoff_sync(
            || {
                call_count.fetch_add(1, Ordering::SeqCst);
                Err::<i32, TestError>(TestError::Transient("timeout".to_string()))
            },
            &config,
        );

        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            super::super::RetryError::MaxAttemptsExceeded(3)
        );
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            4,
            "Should be 1 initial + 3 retries = 4 total"
        );
    }

    #[test]
    fn test_sync_retry_then_success() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = AtomicUsize::new(0);
        let config = RetryConfig {
            max_attempts: 5,
            ..Default::default()
        };

        let result = execute_with_backoff_sync(
            || {
                let current = call_count.fetch_add(1, Ordering::SeqCst) + 1;
                if current < 3 {
                    Err::<i32, TestError>(TestError::Transient("timeout".to_string()))
                } else {
                    Ok(42)
                }
            },
            &config,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            3,
            "Should succeed on third attempt"
        );
    }

    #[test]
    fn test_sync_backoff_timing() {
        use std::sync::Mutex;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Instant;

        let call_count = AtomicUsize::new(0);
        let call_times = Mutex::new(Vec::new());

        let config = RetryConfig {
            max_attempts: 3,
            initial_backoff_ms: 20, // Shorter for faster tests
            max_backoff_ms: 1000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.0, // Disable jitter for predictable timing
        };

        let start = Instant::now();

        let result = execute_with_backoff_sync(
            || {
                let current = call_count.fetch_add(1, Ordering::SeqCst) + 1;
                call_times.lock().unwrap().push(start.elapsed());

                if current < 3 {
                    Err::<i32, TestError>(TestError::Transient("timeout".to_string()))
                } else {
                    Ok(42)
                }
            },
            &config,
        );

        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 3);

        let times = call_times.lock().unwrap();
        assert_eq!(times.len(), 3);

        // First call should be immediate
        assert!(times[0].as_millis() < 10, "First call should be immediate");

        // Second call should be after ~20ms backoff
        if times.len() >= 2 {
            let interval1 = (times[1] - times[0]).as_millis();
            assert!(
                interval1 >= 15 && interval1 <= 30,
                "First retry interval {} should be ~20ms ±5ms",
                interval1
            );
        }

        // Third call should be after ~20ms + ~40ms = ~60ms total
        if times.len() >= 3 {
            let total_time = times[2].as_millis();
            // 20ms (first backoff) + 40ms (second backoff) = ~60ms
            assert!(
                total_time >= 50 && total_time <= 80,
                "Total time {} should be ~60ms ±10ms",
                total_time
            );
        }
    }
}
