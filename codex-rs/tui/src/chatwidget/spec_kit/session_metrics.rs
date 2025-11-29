// FORK-SPECIFIC (just-every/code): Session metrics for P6-SYNC Phase 2
//!
//! Token usage tracking with sliding window estimation for predictive context management.
//! Ported from Auto Drive's session_metrics.rs pattern.
//!
//! Key features:
//! - Running totals for pipeline-level token accounting
//! - Sliding window (default 3 turns) for next-prompt estimation
//! - Replay and duplicate tracking for cache efficiency metrics
//! - Sync from external sources (e.g., API response headers)

use codex_core::protocol::TokenUsage;
use std::collections::VecDeque;

/// Default estimate when no observations available
const DEFAULT_PROMPT_ESTIMATE: u64 = 4_000;

/// Session-level token metrics with sliding window estimation.
///
/// Tracks cumulative token usage across a pipeline run and provides
/// predictive estimates for context budgeting.
#[derive(Debug, Clone)]
pub struct SessionMetrics {
    /// Cumulative token usage across all turns
    running_total: TokenUsage,
    /// Most recent turn's token usage
    last_turn: TokenUsage,
    /// Number of turns recorded
    turn_count: u32,
    /// Number of replay updates (cache hits)
    replay_updates: u32,
    /// Number of duplicate items detected
    duplicate_items: u32,
    /// Sliding window of recent non-cached input tokens
    recent_prompt_tokens: VecDeque<u64>,
    /// Window size for averaging
    window: usize,
}

impl Default for SessionMetrics {
    fn default() -> Self {
        Self::new(3)
    }
}

impl SessionMetrics {
    /// Create new session metrics with specified window size.
    ///
    /// # Arguments
    /// * `window` - Number of recent turns to average for estimation (minimum 1)
    pub fn new(window: usize) -> Self {
        Self {
            running_total: TokenUsage::default(),
            last_turn: TokenUsage::default(),
            turn_count: 0,
            replay_updates: 0,
            duplicate_items: 0,
            recent_prompt_tokens: VecDeque::with_capacity(window),
            window: window.max(1),
        }
    }

    /// Record a turn's token usage.
    ///
    /// Updates running totals and sliding window for estimation.
    pub fn record_turn(&mut self, usage: &TokenUsage) {
        add_token_usage(&mut self.running_total, usage);
        self.last_turn = usage.clone();
        self.turn_count = self.turn_count.saturating_add(1);
        self.push_prompt_observation(non_cached_input(usage));
    }

    /// Sync from external source (e.g., API response with absolute totals).
    ///
    /// Resets window and counters, useful when resuming from checkpoint.
    pub fn sync_absolute(&mut self, total: TokenUsage, last: TokenUsage, turn_count: u32) {
        self.running_total = total;
        self.last_turn = last.clone();
        self.turn_count = turn_count;
        self.replay_updates = 0;
        self.duplicate_items = 0;
        self.recent_prompt_tokens.clear();
        self.push_prompt_observation(non_cached_input(&last));
    }

    /// Get running total token usage.
    pub fn running_total(&self) -> &TokenUsage {
        &self.running_total
    }

    /// Get last turn's token usage.
    pub fn last_turn(&self) -> &TokenUsage {
        &self.last_turn
    }

    /// Get number of turns recorded.
    pub fn turn_count(&self) -> u32 {
        self.turn_count
    }

    /// Get blended total (input + output tokens).
    pub fn blended_total(&self) -> u64 {
        blended_total(&self.running_total)
    }

    /// Estimate next prompt's token count based on sliding window.
    ///
    /// Uses average of recent non-cached input tokens for prediction.
    /// Falls back to last turn's input or default if no observations.
    pub fn estimated_next_prompt_tokens(&self) -> u64 {
        if !self.recent_prompt_tokens.is_empty() {
            let sum: u64 = self.recent_prompt_tokens.iter().copied().sum();
            return sum / self.recent_prompt_tokens.len() as u64;
        }
        let fallback = non_cached_input(&self.last_turn);
        if fallback > 0 {
            fallback
        } else {
            DEFAULT_PROMPT_ESTIMATE
        }
    }

    /// Reset all metrics for a new pipeline run.
    pub fn reset(&mut self) {
        *self = Self::new(self.window);
    }

    /// Record a replay update (cache hit from prior conversation).
    pub fn record_replay(&mut self) {
        self.replay_updates = self.replay_updates.saturating_add(1);
    }

    /// Get replay update count.
    pub fn replay_updates(&self) -> u32 {
        self.replay_updates
    }

    /// Record duplicate items detected in context.
    pub fn record_duplicate_items(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.duplicate_items = self
            .duplicate_items
            .saturating_add(count.min(u32::MAX as usize) as u32);
    }

    /// Set duplicate items count directly.
    pub fn set_duplicate_items(&mut self, count: u32) {
        self.duplicate_items = count;
    }

    /// Set replay updates count directly.
    pub fn set_replay_updates(&mut self, count: u32) {
        self.replay_updates = count;
    }

    /// Get duplicate items count.
    pub fn duplicate_items(&self) -> u32 {
        self.duplicate_items
    }

    /// Push an observation to the sliding window.
    fn push_prompt_observation(&mut self, tokens: u64) {
        if tokens == 0 {
            return;
        }
        if self.recent_prompt_tokens.len() == self.window {
            self.recent_prompt_tokens.pop_front();
        }
        self.recent_prompt_tokens.push_back(tokens);
    }
}

// ============================================================================
// TokenUsage helper functions (avoiding upstream modifications)
// ============================================================================

/// Add token usage to accumulator (in-place addition).
fn add_token_usage(acc: &mut TokenUsage, other: &TokenUsage) {
    acc.input_tokens = acc.input_tokens.saturating_add(other.input_tokens);
    acc.cached_input_tokens = acc.cached_input_tokens.saturating_add(other.cached_input_tokens);
    acc.output_tokens = acc.output_tokens.saturating_add(other.output_tokens);
    acc.reasoning_output_tokens = acc
        .reasoning_output_tokens
        .saturating_add(other.reasoning_output_tokens);
    acc.total_tokens = acc.total_tokens.saturating_add(other.total_tokens);
}

/// Get non-cached input tokens (input_tokens - cached_input_tokens).
fn non_cached_input(usage: &TokenUsage) -> u64 {
    usage.input_tokens.saturating_sub(usage.cached_input_tokens)
}

/// Get blended total (useful for context budget calculations).
fn blended_total(usage: &TokenUsage) -> u64 {
    usage.input_tokens.saturating_add(usage.output_tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(input: u64, output: u64) -> TokenUsage {
        TokenUsage {
            input_tokens: input,
            cached_input_tokens: 0,
            output_tokens: output,
            reasoning_output_tokens: 0,
            total_tokens: input + output,
        }
    }

    fn usage_with_cache(input: u64, cached: u64, output: u64) -> TokenUsage {
        TokenUsage {
            input_tokens: input,
            cached_input_tokens: cached,
            output_tokens: output,
            reasoning_output_tokens: 0,
            total_tokens: input + output,
        }
    }

    #[test]
    fn record_turn_tracks_totals_and_estimate() {
        let mut metrics = SessionMetrics::default();
        metrics.record_turn(&usage(1_000, 500));
        metrics.record_turn(&usage(4_000, 2_000));

        assert_eq!(metrics.turn_count(), 2);
        assert_eq!(metrics.running_total().input_tokens, 5_000);
        assert_eq!(metrics.running_total().output_tokens, 2_500);

        // Average of observed prompt tokens (non-cached input)
        assert_eq!(metrics.estimated_next_prompt_tokens(), 2_500);
        assert_eq!(metrics.duplicate_items(), 0);
        assert_eq!(metrics.replay_updates(), 0);
    }

    #[test]
    fn sync_absolute_resets_window() {
        let mut metrics = SessionMetrics::default();
        metrics.record_turn(&usage(1_000, 500));
        metrics.sync_absolute(usage(10_000, 4_000), usage(3_000, 1_000), 3);

        assert_eq!(metrics.turn_count(), 3);
        assert_eq!(metrics.running_total().input_tokens, 10_000);
        assert_eq!(metrics.last_turn().input_tokens, 3_000);
        assert_eq!(metrics.estimated_next_prompt_tokens(), 3_000);
        assert_eq!(metrics.duplicate_items(), 0);
        assert_eq!(metrics.replay_updates(), 0);
    }

    #[test]
    fn record_replay_increments_counter() {
        let mut metrics = SessionMetrics::default();
        metrics.record_replay();
        metrics.record_replay();
        assert_eq!(metrics.replay_updates(), 2);
    }

    #[test]
    fn sliding_window_respects_capacity() {
        let mut metrics = SessionMetrics::new(3);

        // Fill window
        metrics.record_turn(&usage(1_000, 100));
        metrics.record_turn(&usage(2_000, 100));
        metrics.record_turn(&usage(3_000, 100));

        // Average of 1k, 2k, 3k = 2k
        assert_eq!(metrics.estimated_next_prompt_tokens(), 2_000);

        // Add another - oldest (1k) should be evicted
        metrics.record_turn(&usage(6_000, 100));

        // Average of 2k, 3k, 6k = 3666
        assert_eq!(metrics.estimated_next_prompt_tokens(), 3_666);
    }

    #[test]
    fn cached_tokens_excluded_from_estimate() {
        let mut metrics = SessionMetrics::default();

        // 5000 input, 3000 cached = 2000 non-cached
        metrics.record_turn(&usage_with_cache(5_000, 3_000, 500));

        assert_eq!(metrics.estimated_next_prompt_tokens(), 2_000);
    }

    #[test]
    fn reset_clears_all_state() {
        let mut metrics = SessionMetrics::new(5);
        metrics.record_turn(&usage(10_000, 5_000));
        metrics.record_replay();
        metrics.record_duplicate_items(10);

        metrics.reset();

        assert_eq!(metrics.turn_count(), 0);
        assert_eq!(metrics.running_total().input_tokens, 0);
        assert_eq!(metrics.replay_updates(), 0);
        assert_eq!(metrics.duplicate_items(), 0);
        // Default estimate when no observations
        assert_eq!(metrics.estimated_next_prompt_tokens(), DEFAULT_PROMPT_ESTIMATE);
    }

    #[test]
    fn blended_total_calculation() {
        let mut metrics = SessionMetrics::default();
        metrics.record_turn(&usage(10_000, 5_000));

        assert_eq!(metrics.blended_total(), 15_000);
    }

    #[test]
    fn duplicate_items_tracking() {
        let mut metrics = SessionMetrics::default();

        metrics.record_duplicate_items(5);
        assert_eq!(metrics.duplicate_items(), 5);

        metrics.record_duplicate_items(3);
        assert_eq!(metrics.duplicate_items(), 8);

        // Zero count is ignored
        metrics.record_duplicate_items(0);
        assert_eq!(metrics.duplicate_items(), 8);

        // Direct set
        metrics.set_duplicate_items(100);
        assert_eq!(metrics.duplicate_items(), 100);
    }
}
