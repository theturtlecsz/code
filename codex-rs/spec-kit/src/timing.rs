//! Performance timing infrastructure for SPEC-940
//!
//! Provides macros for measuring operation timing with automatic logging
//! via the tracing infrastructure. Supports both synchronous and asynchronous
//! operations with minimal overhead (<0.5%).
//!
//! # Examples
//!
//! ## Synchronous operations
//!
//! ```rust,no_run
//! use codex_spec_kit::measure_time;
//!
//! let result = measure_time!("config_parse", {
//!     // Heavy config parsing operation
//!     parse_config_file("config.toml")
//! });
//! ```
//!
//! ## Asynchronous operations
//!
//! ```rust,no_run
//! use codex_spec_kit::measure_time_async;
//!
//! let result = measure_time_async!("spawn_agent", async {
//!     spawn_agent_with_retry("gemini").await
//! }).await;
//! ```
//!
//! ## Nested timing
//!
//! ```rust,no_run
//! use codex_spec_kit::{measure_time, measure_time_async};
//!
//! async fn spawn_agent(name: &str) -> Result<AgentHandle> {
//!     measure_time_async!("spawn_agent_total", async {
//!         // Measure session creation
//!         let session_id = measure_time_async!("tmux_create_session", async {
//!             create_tmux_session(name).await
//!         }).await?;
//!
//!         // Measure pane initialization
//!         let pane_id = measure_time_async!("tmux_create_pane", async {
//!             create_tmux_pane(session_id, name).await
//!         }).await?;
//!
//!         Ok(AgentHandle { session_id, pane_id })
//!     }).await
//! }
//! ```

use std::time::Instant;

/// Measure execution time of a synchronous block
///
/// Logs timing information via `tracing::info!` with fields:
/// - `operation`: The operation label
/// - `elapsed_ms`: Elapsed time in milliseconds (u128)
///
/// Returns the result of the block expression.
///
/// # Examples
///
/// ```rust,no_run
/// use codex_spec_kit::measure_time;
///
/// let result = measure_time!("parse_config", {
///     parse_config_file("config.toml")
/// });
/// ```
#[macro_export]
macro_rules! measure_time {
    ($label:expr, $block:block) => {{
        let __start = std::time::Instant::now();
        let __result = $block;
        let __elapsed = __start.elapsed();
        ::tracing::info!(
            operation = $label,
            elapsed_ms = __elapsed.as_millis() as u64,
            "Operation completed"
        );
        __result
    }};
}

/// Measure execution time of an asynchronous block
///
/// Logs timing information via `tracing::info!` with fields:
/// - `operation`: The operation label
/// - `elapsed_ms`: Elapsed time in milliseconds (u128)
///
/// Returns the result of the async block expression.
///
/// # Examples
///
/// ```rust,no_run
/// use codex_spec_kit::measure_time_async;
///
/// let result = measure_time_async!("spawn_agent", async {
///     spawn_agent_with_retry("gemini").await
/// }).await;
/// ```
#[macro_export]
macro_rules! measure_time_async {
    ($label:expr, $block:expr) => {{
        let __start = std::time::Instant::now();
        let __result = $block.await;
        let __elapsed = __start.elapsed();
        ::tracing::info!(
            operation = $label,
            elapsed_ms = __elapsed.as_millis() as u64,
            "Operation completed"
        );
        __result
    }};
}

/// Helper struct for manual timing (when macros aren't suitable)
///
/// Provides explicit start/stop timing with automatic logging on drop.
/// Useful for timing operations across multiple statements or in loops.
///
/// # Examples
///
/// ```rust,no_run
/// use codex_spec_kit::timing::Timer;
///
/// fn process_batch(items: &[Item]) -> Result<()> {
///     let _timer = Timer::new("batch_processing");
///
///     for item in items {
///         process_item(item)?;
///     }
///
///     // Timer logs elapsed time when dropped here
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct Timer {
    operation: String,
    start: Instant,
}

impl Timer {
    /// Create and start a new timer
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            start: Instant::now(),
        }
    }

    /// Get elapsed time without stopping the timer
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    /// Stop the timer and return elapsed duration (logs automatically)
    pub fn stop(self) -> std::time::Duration {
        self.elapsed()
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        tracing::info!(
            operation = %self.operation,
            elapsed_ms = elapsed.as_millis() as u64,
            "Operation completed"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_measure_time_macro() {
        // Test synchronous block timing
        let result = measure_time!("test_operation", {
            std::thread::sleep(Duration::from_millis(10));
            42
        });

        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_measure_time_async_macro() {
        // Test asynchronous block timing
        async fn async_operation() -> i32 {
            tokio::time::sleep(Duration::from_millis(10)).await;
            42
        }

        let result = measure_time_async!("test_async_operation", async_operation());

        assert_eq!(result, 42);
    }

    #[test]
    fn test_measure_time_error_propagation() {
        // Ensure errors propagate correctly through macro
        let result: Result<i32, &str> = measure_time!("test_error", { Err("test error") });

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "test error");
    }

    #[tokio::test]
    async fn test_measure_time_async_error_propagation() {
        // Ensure async errors propagate correctly
        async fn async_error_operation() -> Result<i32, &'static str> {
            Err("test async error")
        }

        let result = measure_time_async!("test_async_error", async_error_operation());

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "test async error");
    }

    #[test]
    fn test_nested_timing() {
        // Test nested timing measurements
        let result = measure_time!("outer_operation", {
            let inner = measure_time!("inner_operation", {
                std::thread::sleep(Duration::from_millis(10));
                21
            });
            inner * 2
        });

        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_nested_async_timing() {
        // Test nested async timing measurements
        async fn inner_operation() -> i32 {
            tokio::time::sleep(Duration::from_millis(10)).await;
            21
        }

        async fn outer_operation() -> i32 {
            let inner = measure_time_async!("inner_async_operation", inner_operation());
            inner * 2
        }

        let result = measure_time_async!("outer_async_operation", outer_operation());

        assert_eq!(result, 42);
    }

    #[test]
    fn test_timer_struct() {
        // Test manual Timer struct
        let timer = Timer::new("manual_timer");
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.elapsed();

        assert!(elapsed.as_millis() >= 10);

        // Timer logs on drop
        drop(timer);
    }

    #[test]
    fn test_timer_stop() {
        // Test explicit timer stop
        let timer = Timer::new("stop_timer");
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.stop();

        assert!(elapsed.as_millis() >= 10);
    }

    #[test]
    fn test_timer_elapsed_multiple_calls() {
        // Ensure elapsed() can be called multiple times
        let timer = Timer::new("multi_elapsed");
        std::thread::sleep(Duration::from_millis(5));
        let elapsed1 = timer.elapsed();

        std::thread::sleep(Duration::from_millis(5));
        let elapsed2 = timer.elapsed();

        assert!(elapsed2 > elapsed1);
        assert!(elapsed2.as_millis() >= 10);
    }
}
