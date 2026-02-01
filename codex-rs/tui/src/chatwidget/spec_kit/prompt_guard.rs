//! Prompt Guard - Prevents UI interactions in headless/test mode (MAINT-930-A)
//!
//! Provides atomic prompt counting and mode-aware guard behavior:
//! - Headless mode: Returns HeadlessError::PromptAttempted (exit 13)
//! - Test mode with CODEX_TEST_PANIC_ON_PROMPT=1: Panics
//! - Normal mode: Allows prompts (no-op guard)
//!
//! This module enforces D133: headless mode MUST NEVER prompt.

use std::sync::atomic::{AtomicU64, Ordering};

/// Global prompt attempt counter for observability
static PROMPT_ATTEMPTS: AtomicU64 = AtomicU64::new(0);

/// Execution mode for prompt guarding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardMode {
    /// Normal TUI mode - prompts allowed
    Normal,
    /// Headless mode - prompts exit with code 13
    Headless,
    /// Test mode with panic - prompts panic immediately
    TestPanic,
}

impl GuardMode {
    fn from_env_values(test_panic_on_prompt: Option<&str>, headless_mode: Option<&str>) -> Self {
        if test_panic_on_prompt == Some("1") {
            Self::TestPanic
        } else if headless_mode == Some("1") {
            Self::Headless
        } else {
            Self::Normal
        }
    }

    /// Detect guard mode from environment variables
    ///
    /// Priority:
    /// 1. CODEX_TEST_PANIC_ON_PROMPT=1 → TestPanic
    /// 2. CODEX_HEADLESS_MODE=1 → Headless
    /// 3. Otherwise → Normal
    pub fn from_env() -> Self {
        let test_panic_on_prompt = std::env::var("CODEX_TEST_PANIC_ON_PROMPT").ok();
        let headless_mode = std::env::var("CODEX_HEADLESS_MODE").ok();
        Self::from_env_values(test_panic_on_prompt.as_deref(), headless_mode.as_deref())
    }

    /// Check if prompts are blocked in this mode
    pub fn blocks_prompts(&self) -> bool {
        matches!(self, Self::Headless | Self::TestPanic)
    }
}

/// Increment prompt counter and return current count
pub fn increment_prompt_attempts() -> u64 {
    PROMPT_ATTEMPTS.fetch_add(1, Ordering::SeqCst) + 1
}

/// Get current prompt attempt count
pub fn get_prompt_attempts() -> u64 {
    PROMPT_ATTEMPTS.load(Ordering::SeqCst)
}

/// Reset prompt counter (for tests only)
#[cfg(test)]
pub fn reset_prompt_attempts() {
    PROMPT_ATTEMPTS.store(0, Ordering::SeqCst);
}

/// Result of a guarded prompt attempt
pub enum PromptGuardResult<T> {
    /// Prompt was allowed and executed
    Allowed(T),
    /// Prompt was blocked in headless mode
    BlockedHeadless,
    /// Prompt was blocked and panicked in test mode (never returned)
    #[allow(dead_code)]
    Panicked,
}

/// Guard a prompt attempt based on current mode
///
/// - Normal mode: Executes prompt_fn and returns Allowed(result)
/// - Headless mode: Increments counter, returns BlockedHeadless
/// - TestPanic mode: Increments counter and panics
///
/// # Panics
///
/// Panics in TestPanic mode to fail tests that trigger prompts.
pub fn guard_prompt<T, F: FnOnce() -> T>(mode: GuardMode, prompt_fn: F) -> PromptGuardResult<T> {
    match mode {
        GuardMode::Normal => {
            increment_prompt_attempts();
            PromptGuardResult::Allowed(prompt_fn())
        }
        GuardMode::Headless => {
            let count = increment_prompt_attempts();
            tracing::warn!(
                attempt_count = count,
                "Prompt blocked in headless mode (D133)"
            );
            PromptGuardResult::BlockedHeadless
        }
        GuardMode::TestPanic => {
            let count = increment_prompt_attempts();
            panic!(
                "CODEX_TEST_PANIC_ON_PROMPT: Prompt attempted in test mode (attempt #{}) - D133 violation",
                count
            );
        }
    }
}

/// Check if prompts are currently blocked
///
/// Returns true if CODEX_HEADLESS_MODE=1 or CODEX_TEST_PANIC_ON_PROMPT=1
pub fn prompts_blocked() -> bool {
    GuardMode::from_env().blocks_prompts()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_mode_normal() {
        let mode = GuardMode::from_env_values(None, None);
        assert_eq!(mode, GuardMode::Normal);
        assert!(!mode.blocks_prompts());
    }

    #[test]
    fn test_guard_mode_headless() {
        let mode = GuardMode::from_env_values(None, Some("1"));
        assert_eq!(mode, GuardMode::Headless);
        assert!(mode.blocks_prompts());
    }

    #[test]
    fn test_guard_mode_test_panic() {
        let mode = GuardMode::from_env_values(Some("1"), None);
        assert_eq!(mode, GuardMode::TestPanic);
        assert!(mode.blocks_prompts());
    }

    #[test]
    fn test_prompt_counter() {
        // Test relative increment behavior (tests run in parallel, can't rely on absolute values)
        let before = get_prompt_attempts();
        let first = increment_prompt_attempts();
        let second = increment_prompt_attempts();
        assert!(first > before, "first increment should increase counter");
        assert_eq!(second, first + 1, "second increment should be first + 1");
    }

    #[test]
    fn test_guard_prompt_normal_mode() {
        let before = get_prompt_attempts();

        let result = guard_prompt(GuardMode::Normal, || 42);
        match result {
            PromptGuardResult::Allowed(v) => assert_eq!(v, 42),
            _ => panic!("Expected Allowed"),
        }
        assert!(get_prompt_attempts() > before, "counter should increment");
    }

    #[test]
    fn test_guard_prompt_headless_mode() {
        let before = get_prompt_attempts();

        let result = guard_prompt(GuardMode::Headless, || 42);
        match result {
            PromptGuardResult::BlockedHeadless => {}
            _ => panic!("Expected BlockedHeadless"),
        }
        assert!(get_prompt_attempts() > before, "counter should increment");
    }

    #[test]
    #[should_panic(expected = "CODEX_TEST_PANIC_ON_PROMPT")]
    fn test_guard_prompt_test_panic_mode() {
        let _ = guard_prompt(GuardMode::TestPanic, || 42);
    }
}
