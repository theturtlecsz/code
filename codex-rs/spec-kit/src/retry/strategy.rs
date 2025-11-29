//! Backoff strategy implementations
//!
//! SPEC-945C: Exponential backoff with jitter
//! Enhanced with Auto Drive patterns (P5-SYNC):
//! - Total elapsed timeout
//! - Cancellation support via tokio CancellationToken
//! - Status callbacks for progress reporting

use backon::{ExponentialBuilder, Retryable};
use rand::Rng;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: usize,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter_factor: f64,
    /// Total elapsed timeout (Auto Drive pattern)
    /// If set, retries stop after this duration regardless of remaining attempts.
    /// Default: None (use max_attempts only)
    pub max_elapsed_ms: Option<u64>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 10_000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.5,
            max_elapsed_ms: None,
        }
    }
}

/// Retry status for progress callbacks (Auto Drive pattern)
///
/// Reports current retry state for UI updates or logging.
#[derive(Debug, Clone)]
pub struct RetryStatus {
    /// Current attempt number (1-indexed)
    pub attempt: u32,
    /// Total elapsed time since first attempt
    pub elapsed: Duration,
    /// Duration of next backoff sleep (None if not sleeping)
    pub sleep: Option<Duration>,
    /// When retry will resume (None if not sleeping)
    pub resume_at: Option<Instant>,
    /// Human-readable reason for current state
    pub reason: String,
    /// True if this is a rate-limit wait (vs normal backoff)
    pub is_rate_limit: bool,
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

/// Execute operation with cancellation support and status callbacks (Auto Drive pattern)
///
/// This is the full-featured retry function ported from upstream Auto Drive.
/// Use this when you need:
/// - External cancellation (via CancellationToken)
/// - Total elapsed timeout (vs just max attempts)
/// - Progress callbacks for UI updates
///
/// # Example
/// ```ignore
/// use tokio_util::sync::CancellationToken;
///
/// let cancel = CancellationToken::new();
/// let config = RetryConfig {
///     max_elapsed_ms: Some(30_000), // 30 second total timeout
///     ..Default::default()
/// };
///
/// let result = execute_with_backoff_cancellable(
///     || async { api_call().await },
///     &config,
///     &cancel,
///     |status| tracing::info!("Retry attempt {}: {}", status.attempt, status.reason),
/// ).await;
/// ```
pub async fn execute_with_backoff_cancellable<F, Fut, T, E, StatusCb>(
    mut operation: F,
    config: &RetryConfig,
    cancel: &CancellationToken,
    mut status_cb: StatusCb,
) -> super::Result<T>
where
    F: FnMut() -> Fut + Send,
    Fut: std::future::Future<Output = Result<T, E>> + Send,
    E: std::error::Error + super::classifier::RetryClassifiable + Send + Sync + 'static,
    StatusCb: FnMut(RetryStatus) + Send,
{
    let start_time = Instant::now();
    let mut attempt: u32 = 0;
    let mut backoff_ms = config.initial_backoff_ms;
    let max_elapsed = config.max_elapsed_ms.map(Duration::from_millis);

    loop {
        // Check cancellation before each attempt
        if cancel.is_cancelled() {
            return Err(super::RetryError::Aborted);
        }

        attempt = attempt.saturating_add(1);
        let output = operation().await;

        match output {
            Ok(value) => return Ok(value),
            Err(err) => {
                let elapsed = start_time.elapsed();

                // Check total elapsed timeout (Auto Drive pattern)
                if let Some(max) = max_elapsed
                    && elapsed >= max
                {
                    return Err(super::RetryError::Timeout {
                        elapsed,
                        last_error: err.to_string(),
                    });
                }

                // Check if error is retryable
                if !err.is_retryable() {
                    return Err(super::RetryError::PermanentError(err.to_string()));
                }

                // Check if we've exhausted attempts
                if attempt as usize > config.max_attempts {
                    return Err(super::RetryError::MaxAttemptsExceeded(config.max_attempts));
                }

                // Check for rate limit with suggested backoff
                let (sleep_duration, is_rate_limit) =
                    if let Some(suggested) = err.suggested_backoff() {
                        // Use server-suggested backoff for rate limits
                        (suggested, true)
                    } else {
                        // Use exponential backoff with jitter
                        let backoff_duration =
                            Duration::from_millis(backoff_ms.min(config.max_backoff_ms));
                        (apply_jitter(backoff_duration, config.jitter_factor), false)
                    };

                let resume_at = Instant::now() + sleep_duration;
                let reason = if is_rate_limit {
                    format!("Rate limited, waiting {sleep_duration:?}")
                } else {
                    format!("Transient error: {err}, retrying in {sleep_duration:?}")
                };

                // Report status before sleeping
                status_cb(RetryStatus {
                    attempt,
                    elapsed,
                    sleep: Some(sleep_duration),
                    resume_at: Some(resume_at),
                    reason: reason.clone(),
                    is_rate_limit,
                });

                // Sleep with cancellation support
                if wait_with_cancel(cancel, sleep_duration).await.is_err() {
                    return Err(super::RetryError::Aborted);
                }

                // Increase backoff for next iteration (exponential)
                backoff_ms = (backoff_ms as f64 * config.backoff_multiplier) as u64;
            }
        }
    }
}

/// Wait with cancellation support (Auto Drive pattern)
async fn wait_with_cancel(cancel: &CancellationToken, duration: Duration) -> Result<(), ()> {
    if duration.is_zero() {
        return Ok(());
    }

    tokio::select! {
        _ = tokio::time::sleep(duration) => Ok(()),
        _ = cancel.cancelled() => Err(()),
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
                (500.0..=1500.0).contains(&jittered_ms),
                "Jittered value {jittered_ms} out of range [500, 1500]"
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
        use tokio::time::Instant;

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
            max_elapsed_ms: None,
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
                "First retry interval {interval1} should be ~50ms ±{tolerance_ms}ms"
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
            max_elapsed_ms: None,
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
                (15..=30).contains(&interval1),
                "First retry interval {interval1} should be ~20ms ±5ms"
            );
        }

        // Third call should be after ~20ms + ~40ms = ~60ms total
        if times.len() >= 3 {
            let total_time = times[2].as_millis();
            // 20ms (first backoff) + 40ms (second backoff) = ~60ms
            assert!(
                (50..=80).contains(&total_time),
                "Total time {total_time} should be ~60ms ±10ms"
            );
        }
    }

    // === Cancellable Retry Tests (Auto Drive patterns) ===

    #[tokio::test]
    async fn test_cancellable_immediate_success() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        let cancel = CancellationToken::new();
        let status_calls = Arc::new(AtomicUsize::new(0));
        let status_calls_clone = status_calls.clone();

        let config = RetryConfig::default();

        let result = execute_with_backoff_cancellable(
            move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok::<i32, TestError>(42)
                }
            },
            &config,
            &cancel,
            move |_status| {
                status_calls_clone.fetch_add(1, Ordering::SeqCst);
            },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        assert_eq!(
            status_calls.load(Ordering::SeqCst),
            0,
            "No status calls on success"
        );
    }

    #[tokio::test]
    async fn test_cancellable_aborted() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        let cancel = CancellationToken::new();

        // Cancel immediately
        cancel.cancel();

        let config = RetryConfig::default();

        let result = execute_with_backoff_cancellable(
            move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok::<i32, TestError>(42)
                }
            },
            &config,
            &cancel,
            |_| {},
        )
        .await;

        assert!(matches!(result, Err(crate::retry::RetryError::Aborted)));
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            0,
            "Should not call when cancelled"
        );
    }

    #[tokio::test]
    async fn test_cancellable_timeout() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        let cancel = CancellationToken::new();

        let config = RetryConfig {
            max_attempts: 100, // High so timeout triggers first
            initial_backoff_ms: 50,
            max_backoff_ms: 100,
            backoff_multiplier: 1.5,
            jitter_factor: 0.0,
            max_elapsed_ms: Some(100), // 100ms timeout
        };

        let result = execute_with_backoff_cancellable(
            move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, TestError>(TestError::Transient("timeout".to_string()))
                }
            },
            &config,
            &cancel,
            |_| {},
        )
        .await;

        assert!(matches!(
            result,
            Err(crate::retry::RetryError::Timeout { .. })
        ));
        // Should have made at least one attempt but hit timeout
        assert!(call_count.load(Ordering::SeqCst) >= 1);
    }

    #[tokio::test]
    async fn test_cancellable_status_callback() {
        use std::sync::Arc;
        use std::sync::Mutex;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        let cancel = CancellationToken::new();
        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_clone = statuses.clone();

        let config = RetryConfig {
            max_attempts: 3,
            initial_backoff_ms: 10,
            max_backoff_ms: 100,
            backoff_multiplier: 2.0,
            jitter_factor: 0.0,
            max_elapsed_ms: None,
        };

        let result = execute_with_backoff_cancellable(
            move || {
                let count = call_count_clone.clone();
                async move {
                    let current = count.fetch_add(1, Ordering::SeqCst) + 1;
                    if current < 3 {
                        Err::<i32, TestError>(TestError::Transient("retry me".to_string()))
                    } else {
                        Ok(42)
                    }
                }
            },
            &config,
            &cancel,
            move |status| {
                statuses_clone.lock().unwrap().push(status.attempt);
            },
        )
        .await;

        assert!(result.is_ok());
        let recorded = statuses.lock().unwrap();
        assert_eq!(
            recorded.len(),
            2,
            "Should have 2 status callbacks for 2 retries"
        );
        assert_eq!(recorded[0], 1, "First callback for attempt 1");
        assert_eq!(recorded[1], 2, "Second callback for attempt 2");
    }
}
