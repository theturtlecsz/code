//! Performance timing infrastructure for SPEC-940
//!
//! Provides macros for measuring operation timing with automatic logging
//! via the tracing infrastructure.

/// Measure execution time of a synchronous block
///
/// Logs timing information via `tracing::info!` with fields:
/// - `operation`: The operation label
/// - `elapsed_ms`: Elapsed time in milliseconds (u64)
///
/// Returns the result of the block expression.
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
/// - `elapsed_ms`: Elapsed time in milliseconds (u64)
///
/// Returns the result of the async block expression.
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
