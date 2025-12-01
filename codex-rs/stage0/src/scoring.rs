//! Dynamic scoring for Stage0 memories
//!
//! Implements the scoring formula from STAGE0_SCORING_AND_DCC.md:
//!
//! ```text
//! usage_score   = min(1.0, log(1 + U) / log(6))
//! recency_score = exp(-ln(2) * recency_days / 7)
//! priority_score= clamp(P, 1, 10) / 10.0
//! age_penalty   = 1.0 - exp(-ln(2) * age_days / 30)
//!
//! novelty_factor = 1.0 + boost_max * (1 - U / threshold)  if U < threshold
//!                = 1.0                                     otherwise
//!
//! base_score = w_usage * usage_score + w_recency * recency_score
//!            + w_priority * priority_score - w_decay * age_penalty
//!
//! dynamic_score = clamp(base_score * novelty_factor, 0.0, 1.5)
//! ```

use crate::config::ScoringConfig;
use chrono::{DateTime, Utc};

/// Input data required to calculate a dynamic score
#[derive(Debug, Clone)]
pub struct ScoringInput {
    /// Number of times this memory has been used in Stage 0 context
    pub usage_count: u32,
    /// Initial priority (1-10, from importance or explicit assignment)
    pub initial_priority: i32,
    /// Last time Stage 0 used this memory (None = never accessed)
    pub last_accessed_at: Option<DateTime<Utc>>,
    /// When the memory was created
    pub created_at: DateTime<Utc>,
}

impl ScoringInput {
    /// Create a new scoring input
    pub fn new(
        usage_count: u32,
        initial_priority: i32,
        last_accessed_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            usage_count,
            initial_priority,
            last_accessed_at,
            created_at,
        }
    }

    /// Create scoring input for a brand new memory
    pub fn new_memory(initial_priority: i32) -> Self {
        let now = Utc::now();
        Self {
            usage_count: 0,
            initial_priority,
            last_accessed_at: None,
            created_at: now,
        }
    }
}

/// Breakdown of score components (for explainability)
#[derive(Debug, Clone)]
pub struct ScoringComponents {
    /// Contribution from usage frequency: min(1.0, log(1 + U) / log(6))
    pub usage_score: f64,
    /// Contribution from recency: exp(-ln(2) * days / 7)
    pub recency_score: f64,
    /// Contribution from priority: clamp(P, 1, 10) / 10.0
    pub priority_score: f64,
    /// Penalty from age: 1.0 - exp(-ln(2) * days / 30)
    pub age_penalty: f64,
    /// Novelty boost factor (1.0 to 1.0 + boost_max)
    pub novelty_factor: f64,
    /// Base score before novelty: weighted sum of components
    pub base_score: f64,
    /// Final dynamic score: clamp(base * novelty, 0.0, 1.5)
    pub final_score: f64,
}

/// Calculate the dynamic score for a memory
///
/// Returns both the final score and the component breakdown for explainability.
pub fn calculate_dynamic_score(
    input: &ScoringInput,
    config: &ScoringConfig,
    now: DateTime<Utc>,
) -> ScoringComponents {
    let weights = &config.weights;

    // Usage score: min(1.0, log(1 + U) / log(6))
    // log(6) ≈ 1.79, so usage_count=5 gives score≈1.0
    let usage_score = (1.0 + input.usage_count as f64).ln() / 6_f64.ln();
    let usage_score = usage_score.min(1.0);

    // Recency score: exp(-ln(2) * recency_days / 7)
    // Half-life of 7 days
    let last_access = input.last_accessed_at.unwrap_or(input.created_at);
    let recency_days = (now - last_access).num_days().max(0) as f64;
    let recency_score = (-std::f64::consts::LN_2 * recency_days / 7.0).exp();

    // Priority score: clamp(P, 1, 10) / 10.0
    let priority_clamped = input.initial_priority.clamp(1, 10);
    let priority_score = priority_clamped as f64 / 10.0;

    // Age penalty: 1.0 - exp(-ln(2) * age_days / 30)
    // Half-life of 30 days, penalty approaches 1.0 for very old memories
    let age_days = (now - input.created_at).num_days().max(0) as f64;
    let age_penalty = 1.0 - (-std::f64::consts::LN_2 * age_days / 30.0).exp();

    // Novelty boost for underused memories
    let novelty_factor = if input.usage_count < config.novelty_boost_threshold {
        let usage_ratio = input.usage_count as f64 / config.novelty_boost_threshold as f64;
        1.0 + config.novelty_boost_factor_max as f64 * (1.0 - usage_ratio)
    } else {
        1.0
    };

    // Base score: weighted combination
    let base_score = weights.usage as f64 * usage_score
        + weights.recency as f64 * recency_score
        + weights.priority as f64 * priority_score
        - weights.decay as f64 * age_penalty;

    // Final score: apply novelty and clamp
    let final_score = (base_score * novelty_factor).clamp(0.0, 1.5);

    ScoringComponents {
        usage_score,
        recency_score,
        priority_score,
        age_penalty,
        novelty_factor,
        base_score,
        final_score,
    }
}

/// Calculate just the final dynamic score (convenience function)
pub fn calculate_score(input: &ScoringInput, config: &ScoringConfig, now: DateTime<Utc>) -> f64 {
    calculate_dynamic_score(input, config, now).final_score
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn default_config() -> ScoringConfig {
        ScoringConfig::default()
    }

    #[test]
    fn test_new_high_priority_memory() {
        let config = default_config();
        let now = Utc::now();
        let input = ScoringInput {
            usage_count: 0,
            initial_priority: 10,
            last_accessed_at: None,
            created_at: now,
        };

        let components = calculate_dynamic_score(&input, &config, now);

        // New memory: usage=0, no recency penalty, high priority, no age penalty
        assert_eq!(components.usage_score, 0.0);
        assert!((components.recency_score - 1.0).abs() < 0.001); // Just created
        assert!((components.priority_score - 1.0).abs() < 0.001); // Priority 10
        assert!(components.age_penalty < 0.001); // Just created
        assert!(components.novelty_factor > 1.0); // Should have novelty boost
        assert!(components.final_score > 0.0);
    }

    #[test]
    fn test_heavily_used_memory() {
        let config = default_config();
        let now = Utc::now();
        let input = ScoringInput {
            usage_count: 10,
            initial_priority: 5,
            last_accessed_at: Some(now),
            created_at: now - Duration::days(7),
        };

        let components = calculate_dynamic_score(&input, &config, now);

        // Heavily used: high usage score, no novelty boost
        assert!((components.usage_score - 1.0).abs() < 0.01); // Should be capped at 1.0
        assert!((components.novelty_factor - 1.0).abs() < 0.001); // No boost
        assert!((components.recency_score - 1.0).abs() < 0.001); // Just accessed
    }

    #[test]
    fn test_stale_memory() {
        let config = default_config();
        let now = Utc::now();
        let input = ScoringInput {
            usage_count: 2,
            initial_priority: 5,
            last_accessed_at: Some(now - Duration::days(30)),
            created_at: now - Duration::days(60),
        };

        let components = calculate_dynamic_score(&input, &config, now);

        // Stale: low recency score, high age penalty
        assert!(components.recency_score < 0.1); // 30 days = ~4 half-lives
        assert!(components.age_penalty > 0.7); // 60 days = ~2 half-lives
    }

    #[test]
    fn test_usage_score_logarithmic() {
        let config = default_config();
        let now = Utc::now();

        let scores: Vec<f64> = (0..=10)
            .map(|u| {
                let input = ScoringInput {
                    usage_count: u,
                    initial_priority: 5,
                    last_accessed_at: Some(now),
                    created_at: now,
                };
                calculate_dynamic_score(&input, &config, now).usage_score
            })
            .collect();

        // Should be monotonically increasing
        for i in 1..scores.len() {
            assert!(scores[i] >= scores[i - 1]);
        }

        // Usage=5 should be close to 1.0 (by design)
        assert!((scores[5] - 1.0).abs() < 0.05);

        // Should cap at 1.0
        assert!(scores[10] <= 1.0);
    }

    #[test]
    fn test_recency_half_life() {
        let config = default_config();
        let now = Utc::now();

        // 7 days = half-life, should give ~0.5 recency score
        let input = ScoringInput {
            usage_count: 0,
            initial_priority: 5,
            last_accessed_at: Some(now - Duration::days(7)),
            created_at: now - Duration::days(7),
        };

        let components = calculate_dynamic_score(&input, &config, now);
        assert!((components.recency_score - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_novelty_boost_threshold() {
        let config = default_config(); // threshold=5, max_boost=0.5
        let now = Utc::now();

        // usage_count=0: full boost (1.0 + 0.5 = 1.5)
        let input0 = ScoringInput::new(0, 5, Some(now), now);
        let c0 = calculate_dynamic_score(&input0, &config, now);
        assert!((c0.novelty_factor - 1.5).abs() < 0.001);

        // usage_count=5: no boost (1.0)
        let input5 = ScoringInput::new(5, 5, Some(now), now);
        let c5 = calculate_dynamic_score(&input5, &config, now);
        assert!((c5.novelty_factor - 1.0).abs() < 0.001);

        // usage_count=2: partial boost
        let input2 = ScoringInput::new(2, 5, Some(now), now);
        let c2 = calculate_dynamic_score(&input2, &config, now);
        assert!(c2.novelty_factor > 1.0);
        assert!(c2.novelty_factor < 1.5);
    }

    #[test]
    fn test_score_clamping() {
        let config = default_config();
        let now = Utc::now();

        // Maximum possible score: high priority, just accessed, new, unused
        let max_input = ScoringInput {
            usage_count: 0,
            initial_priority: 10,
            last_accessed_at: Some(now),
            created_at: now,
        };
        let max_score = calculate_score(&max_input, &config, now);
        assert!(max_score <= 1.5);
        assert!(max_score > 0.0);

        // Very negative scenario: old, never accessed, low priority, decayed
        let min_input = ScoringInput {
            usage_count: 100, // No novelty
            initial_priority: 1,
            last_accessed_at: Some(now - Duration::days(365)),
            created_at: now - Duration::days(365),
        };
        let min_score = calculate_score(&min_input, &config, now);
        assert!(min_score >= 0.0);
    }

    #[test]
    fn test_priority_scaling() {
        let config = default_config();
        let now = Utc::now();

        for p in 1..=10 {
            let input = ScoringInput::new(5, p, Some(now), now);
            let c = calculate_dynamic_score(&input, &config, now);
            let expected = p as f64 / 10.0;
            assert!((c.priority_score - expected).abs() < 0.001);
        }
    }

    #[test]
    fn test_priority_clamping() {
        let config = default_config();
        let now = Utc::now();

        // Out of range priorities should be clamped
        let low = ScoringInput::new(0, -5, Some(now), now);
        let high = ScoringInput::new(0, 100, Some(now), now);

        let c_low = calculate_dynamic_score(&low, &config, now);
        let c_high = calculate_dynamic_score(&high, &config, now);

        assert!((c_low.priority_score - 0.1).abs() < 0.001); // clamped to 1
        assert!((c_high.priority_score - 1.0).abs() < 0.001); // clamped to 10
    }
}
