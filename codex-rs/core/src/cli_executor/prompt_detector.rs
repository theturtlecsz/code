//! Prompt detection for Gemini CLI interactive mode
//!
//! Detects when Gemini CLI has finished responding and returned to idle prompt state.
//! Uses multi-signal heuristic for reliability:
//! 1. Prompt markers ("> " or "gemini> ") - highest confidence
//! 2. Idle timeout (no output for N ms) - medium confidence
//! 3. Response completeness checks - low confidence fallback

use std::time::{Duration, Instant};

/// Confidence level for prompt detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    High,   // Explicit prompt marker detected
    Medium, // Idle timeout + looks complete
    Low,    // Fallback/timeout
}

/// Prompt markers used by Gemini CLI
const PROMPT_MARKERS: &[&str] = &[
    "\n> ",       // Standard prompt
    "\ngemini> ", // Alternative prompt format
    "> ",         // Start of line (fresh session)
];

/// Minimum time before allowing completion detection (prevents false positives)
const MIN_RESPONSE_TIME: Duration = Duration::from_millis(200);

/// Idle threshold - no output for this long suggests completion
const DEFAULT_IDLE_THRESHOLD: Duration = Duration::from_millis(500);

/// Detector for Gemini CLI prompt completion
pub struct PromptDetector {
    /// Last time output was received
    last_output_time: Instant,

    /// Time when current turn started
    turn_start_time: Instant,

    /// Threshold for idle detection
    idle_threshold: Duration,

    /// Current confidence level
    confidence: Confidence,

    /// Whether we're in the middle of a code fence
    in_code_fence: bool,
}

impl PromptDetector {
    /// Create new prompt detector with default settings
    pub fn new() -> Self {
        Self::with_threshold(DEFAULT_IDLE_THRESHOLD)
    }

    /// Create with custom idle threshold
    pub fn with_threshold(idle_threshold: Duration) -> Self {
        let now = Instant::now();
        Self {
            last_output_time: now,
            turn_start_time: now,
            idle_threshold,
            confidence: Confidence::Low,
            in_code_fence: false,
        }
    }

    /// Check if response appears complete based on accumulated output
    pub fn is_complete(&mut self, output: &str) -> bool {
        let now = Instant::now();

        // Don't detect too early (prevents false positives)
        if now.duration_since(self.turn_start_time) < MIN_RESPONSE_TIME {
            return false;
        }

        // Signal 1: Explicit prompt marker (highest confidence)
        for marker in PROMPT_MARKERS {
            if output.ends_with(marker) {
                self.confidence = Confidence::High;
                tracing::debug!(
                    "Prompt detected (HIGH confidence): marker '{}'",
                    marker.escape_debug()
                );
                return true;
            }
        }

        // Signal 2: Idle timeout + looks complete (medium confidence)
        if now.duration_since(self.last_output_time) > self.idle_threshold
            && self.looks_complete(output)
        {
            self.confidence = Confidence::Medium;
            tracing::debug!(
                "Prompt detected (MEDIUM confidence): idle {}ms + looks complete",
                self.idle_threshold.as_millis()
            );
            return true;
        }

        // Signal 3: Still responding
        self.confidence = Confidence::Low;
        false
    }

    /// Update detector with new output data
    pub fn update(&mut self, text: &str) {
        self.last_output_time = Instant::now();

        // Track code fence state
        for line in text.lines() {
            if line.trim().starts_with("```") {
                self.in_code_fence = !self.in_code_fence;
            }
        }
    }

    /// Reset for new turn
    pub fn reset(&mut self) {
        let now = Instant::now();
        self.last_output_time = now;
        self.turn_start_time = now;
        self.confidence = Confidence::Low;
        self.in_code_fence = false;
    }

    /// Check if output looks like a complete response
    fn looks_complete(&self, output: &str) -> bool {
        let trimmed = output.trim_end();

        if trimmed.is_empty() {
            return false;
        }

        // Still in code fence = incomplete
        if self.in_code_fence {
            return false;
        }

        // Get last line
        let last_line = trimmed.lines().last().unwrap_or("");

        // Incomplete markers
        if last_line.ends_with("...")
            || last_line.ends_with(",")
            || last_line.ends_with("and")
            || last_line.ends_with("or")
        {
            return false;
        }

        // Code fence not closed = incomplete
        let fence_count = trimmed.matches("```").count();
        if !fence_count.is_multiple_of(2) {
            return false;
        }

        // Looks complete if ends with:
        // - Period
        // - Closing code fence
        // - Newline (paragraph break)
        // - Question mark / exclamation
        trimmed.ends_with('.')
            || trimmed.ends_with("```")
            || trimmed.ends_with('\n')
            || trimmed.ends_with('?')
            || trimmed.ends_with('!')
    }

    /// Get current confidence level
    pub fn confidence(&self) -> Confidence {
        self.confidence
    }

    /// Get time since last output
    pub fn time_since_last_output(&self) -> Duration {
        Instant::now().duration_since(self.last_output_time)
    }
}

impl Default for PromptDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    // SPEC-957: Allow test code flexibility
    #![allow(clippy::uninlined_format_args)]

    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_detect_explicit_prompt() {
        let mut detector = PromptDetector::new();

        // Simulate turn start (wait past MIN_RESPONSE_TIME)
        sleep(Duration::from_millis(250));

        // Should detect standard prompt
        assert!(detector.is_complete("4\n> "));
        assert_eq!(detector.confidence(), Confidence::High);
    }

    #[test]
    fn test_detect_alternative_prompt() {
        let mut detector = PromptDetector::new();
        sleep(Duration::from_millis(250));

        assert!(detector.is_complete("Response text\ngemini> "));
        assert_eq!(detector.confidence(), Confidence::High);
    }

    #[test]
    fn test_incomplete_sentence() {
        let mut detector = PromptDetector::new();
        sleep(Duration::from_millis(250));
        detector.update("This is incomplete...");

        // Even with idle timeout, incomplete markers prevent completion
        sleep(Duration::from_millis(600));
        assert!(!detector.looks_complete("This is incomplete..."));
    }

    #[test]
    fn test_incomplete_code_fence() {
        let mut detector = PromptDetector::new();
        sleep(Duration::from_millis(250));

        let response = "Here's code:\n```rust\nfn main() {}";
        detector.update(response);

        // Code fence not closed
        assert!(!detector.looks_complete(response));
    }

    #[test]
    fn test_complete_code_fence() {
        let mut detector = PromptDetector::new();
        sleep(Duration::from_millis(250));

        let response = "Here's code:\n```rust\nfn main() {}\n```";
        detector.update(response);

        assert!(detector.looks_complete(response));
    }

    #[test]
    fn test_complete_with_period() {
        let detector = PromptDetector::new();
        sleep(Duration::from_millis(250));

        assert!(detector.looks_complete("This is a complete sentence."));
    }

    #[test]
    fn test_too_early_detection() {
        let mut detector = PromptDetector::new();

        // Immediately after reset, should not detect (< MIN_RESPONSE_TIME)
        assert!(!detector.is_complete("Quick response\n> "));
    }

    #[test]
    fn test_update_resets_idle_timer() {
        let mut detector = PromptDetector::new();

        // Sleep to accumulate time
        sleep(Duration::from_millis(100));
        let time_before_update = detector.time_since_last_output();

        // Update should reset the timer
        detector.update("Some text");
        let time_after_update = detector.time_since_last_output();

        // Before update: should be >= 100ms (we slept that long)
        assert!(
            time_before_update >= Duration::from_millis(90),
            "Expected time_before_update ({:?}) >= 90ms",
            time_before_update
        );

        // After update: timer was reset, should be very small (< 50ms allows for system jitter)
        assert!(
            time_after_update < Duration::from_millis(50),
            "Expected time_after_update ({:?}) < 50ms: timer should reset on update",
            time_after_update
        );
    }

    #[test]
    fn test_reset_clears_state() {
        let mut detector = PromptDetector::new();
        detector.in_code_fence = true;
        detector.confidence = Confidence::High;

        detector.reset();

        assert!(!detector.in_code_fence);
        assert_eq!(detector.confidence, Confidence::Low);
    }
}
