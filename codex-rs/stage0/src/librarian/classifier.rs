//! Memory classification for SPEC-KIT-103 Librarian
//!
//! Provides heuristic-based classification of memory content into semantic types.
//! LLM-assisted classification is opt-in for ambiguous cases.

use std::fmt;
use std::str::FromStr;

/// Semantic type for a memory, used to guide retrieval and templating
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryType {
    /// Recurring solutions, established approaches
    Pattern,
    /// Architectural choices with rationale
    Decision,
    /// Issues encountered + resolutions
    Problem,
    /// Observations from execution (learnings)
    Insight,
    /// Constitution exceptions (sanctioned violations)
    Exception,
    /// External docs/links
    Reference,
    /// Unclassifiable (flag for review)
    Unknown,
}

impl MemoryType {
    /// Get the default priority for this memory type
    ///
    /// Based on Stage 0 constitution priority scheme:
    /// - Guardrails: 10
    /// - Principles: 9
    /// - Goals/NonGoals: 8
    /// - Exception: 7
    /// - Everything else: 5-7 based on type
    pub fn default_priority(&self) -> i32 {
        match self {
            Self::Exception => 7, // Constitution exception level
            Self::Decision => 7,  // High-value architectural decisions
            Self::Pattern => 6,   // Established patterns
            Self::Problem => 6,   // Issue tracking
            Self::Insight => 5,   // Learnings (lower priority)
            Self::Reference => 4, // External references
            Self::Unknown => 3,   // Unclassified (lowest)
        }
    }

    /// Tag string for this type (e.g., "type:pattern")
    pub fn as_tag(&self) -> String {
        format!("type:{}", self.as_str())
    }

    /// Lowercase string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pattern => "pattern",
            Self::Decision => "decision",
            Self::Problem => "problem",
            Self::Insight => "insight",
            Self::Exception => "exception",
            Self::Reference => "reference",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for MemoryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for MemoryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pattern" => Ok(Self::Pattern),
            "decision" => Ok(Self::Decision),
            "problem" => Ok(Self::Problem),
            "insight" => Ok(Self::Insight),
            "exception" => Ok(Self::Exception),
            "reference" => Ok(Self::Reference),
            "unknown" => Ok(Self::Unknown),
            _ => Err(format!("Unknown memory type: {s}")),
        }
    }
}

/// Result of classification with confidence score
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    /// The classified memory type
    pub memory_type: MemoryType,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Signals that matched (for debugging/audit)
    pub matched_signals: Vec<String>,
}

impl ClassificationResult {
    /// Create a new classification result
    pub fn new(memory_type: MemoryType, confidence: f32, signals: Vec<String>) -> Self {
        Self {
            memory_type,
            confidence,
            matched_signals: signals,
        }
    }

    /// Check if confidence meets threshold for auto-apply
    pub fn meets_threshold(&self, threshold: f32) -> bool {
        self.confidence >= threshold
    }
}

/// Configuration for the classifier
#[derive(Debug, Clone)]
pub struct ClassifierConfig {
    /// Minimum confidence to auto-apply classification (default: 0.7)
    pub confidence_threshold: f32,
    /// Enable LLM fallback for ambiguous cases (default: false)
    pub llm_fallback_enabled: bool,
}

impl Default for ClassifierConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.7,
            llm_fallback_enabled: false,
        }
    }
}

// Signal definitions for heuristic classification
// Format: (pattern, weight, is_strong_signal)
const PATTERN_SIGNALS: &[(&str, f32, bool)] = &[
    ("pattern:", 0.4, true),
    ("recurring", 0.2, false),
    ("always do", 0.25, false),
    ("standard approach", 0.3, true),
    ("best practice", 0.3, true),
    ("established", 0.15, false),
    ("convention", 0.2, false),
    ("type:pattern", 0.5, true),
];

const DECISION_SIGNALS: &[(&str, f32, bool)] = &[
    ("decision:", 0.4, true),
    ("chose", 0.2, false),
    ("decided", 0.25, false),
    ("because we", 0.2, false),
    ("trade-off", 0.25, false),
    ("tradeoff", 0.25, false),
    ("rationale:", 0.3, true),
    ("architectural decision", 0.4, true),
    ("adr:", 0.5, true),
    ("type:decision", 0.5, true),
];

const PROBLEM_SIGNALS: &[(&str, f32, bool)] = &[
    ("problem:", 0.4, true),
    ("issue:", 0.35, true),
    ("bug:", 0.4, true),
    ("error:", 0.25, false),
    ("failed", 0.15, false),
    ("broke", 0.15, false),
    ("fix:", 0.2, false),
    ("resolved:", 0.25, false),
    ("workaround", 0.2, false),
    ("type:problem", 0.5, true),
];

const INSIGHT_SIGNALS: &[(&str, f32, bool)] = &[
    ("learned:", 0.4, true),
    ("realized:", 0.3, true),
    ("observed:", 0.25, false),
    ("til:", 0.4, true),
    ("note:", 0.15, false),
    ("insight:", 0.4, true),
    ("lesson:", 0.3, true),
    ("takeaway:", 0.3, true),
    ("type:insight", 0.5, true),
];

const EXCEPTION_SIGNALS: &[(&str, f32, bool)] = &[
    ("exception:", 0.5, true),
    ("exemption:", 0.5, true),
    ("override:", 0.3, false),
    ("sanctioned violation", 0.5, true),
    ("constitution exception", 0.6, true),
    ("type:exception", 0.6, true),
];

const REFERENCE_SIGNALS: &[(&str, f32, bool)] = &[
    ("http://", 0.3, false),
    ("https://", 0.3, false),
    ("see:", 0.25, false),
    ("ref:", 0.3, true),
    ("docs:", 0.3, true),
    ("documentation:", 0.25, false),
    ("link:", 0.2, false),
    ("type:reference", 0.5, true),
];

/// Classify memory content using heuristic pattern matching
///
/// Returns the detected type and confidence score (0.0 - 1.0).
/// If multiple types match, returns the one with highest confidence.
pub fn classify_memory(content: &str) -> ClassificationResult {
    let content_lower = content.to_lowercase();

    let mut scores: Vec<(MemoryType, f32, Vec<String>)> = vec![
        score_type(&content_lower, MemoryType::Pattern, PATTERN_SIGNALS),
        score_type(&content_lower, MemoryType::Decision, DECISION_SIGNALS),
        score_type(&content_lower, MemoryType::Problem, PROBLEM_SIGNALS),
        score_type(&content_lower, MemoryType::Insight, INSIGHT_SIGNALS),
        score_type(&content_lower, MemoryType::Exception, EXCEPTION_SIGNALS),
        score_type(&content_lower, MemoryType::Reference, REFERENCE_SIGNALS),
    ];

    // Sort by confidence descending
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Return highest scoring type, or Unknown if no significant signals
    if let Some((mem_type, confidence, signals)) = scores.into_iter().next()
        && confidence > 0.0 {
            return ClassificationResult::new(mem_type, confidence.min(1.0), signals);
        }

    ClassificationResult::new(MemoryType::Unknown, 0.0, vec![])
}

/// Score a specific memory type based on signal matches
fn score_type(
    content: &str,
    mem_type: MemoryType,
    signals: &[(&str, f32, bool)],
) -> (MemoryType, f32, Vec<String>) {
    let mut total_score = 0.0f32;
    let mut matched = Vec::new();
    let mut has_strong_signal = false;

    for (pattern, weight, is_strong) in signals {
        if content.contains(pattern) {
            total_score += weight;
            matched.push(pattern.to_string());
            if *is_strong {
                has_strong_signal = true;
            }
        }
    }

    // Apply bonus for strong signal match
    if has_strong_signal && matched.len() >= 2 {
        total_score *= 1.2;
    }

    // Normalize: Cap at 1.0 but don't artificially inflate low scores
    let confidence = total_score.min(1.0);

    (mem_type, confidence, matched)
}

/// Check if content already has a type tag
pub fn has_type_tag(content: &str) -> Option<MemoryType> {
    let content_lower = content.to_lowercase();

    for mem_type in &[
        MemoryType::Pattern,
        MemoryType::Decision,
        MemoryType::Problem,
        MemoryType::Insight,
        MemoryType::Exception,
        MemoryType::Reference,
    ] {
        let tag = mem_type.as_tag();
        if content_lower.contains(&tag) {
            return Some(*mem_type);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_type_display() {
        assert_eq!(MemoryType::Pattern.to_string(), "pattern");
        assert_eq!(MemoryType::Decision.to_string(), "decision");
        assert_eq!(MemoryType::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_memory_type_from_str() {
        assert_eq!(
            MemoryType::from_str("pattern").unwrap(),
            MemoryType::Pattern
        );
        assert_eq!(
            MemoryType::from_str("DECISION").unwrap(),
            MemoryType::Decision
        );
        assert!(MemoryType::from_str("invalid").is_err());
    }

    #[test]
    fn test_memory_type_tag() {
        assert_eq!(MemoryType::Pattern.as_tag(), "type:pattern");
        assert_eq!(MemoryType::Exception.as_tag(), "type:exception");
    }

    #[test]
    fn test_memory_type_priority() {
        assert_eq!(MemoryType::Exception.default_priority(), 7);
        assert_eq!(MemoryType::Decision.default_priority(), 7);
        assert_eq!(MemoryType::Pattern.default_priority(), 6);
        assert_eq!(MemoryType::Unknown.default_priority(), 3);
    }

    #[test]
    fn test_classify_pattern() {
        let content =
            "Pattern: Always use dependency injection for services. This is our standard approach.";
        let result = classify_memory(content);
        assert_eq!(result.memory_type, MemoryType::Pattern);
        assert!(result.confidence > 0.5);
        assert!(!result.matched_signals.is_empty());
    }

    #[test]
    fn test_classify_decision() {
        let content = "Decision: We chose SQLite over PostgreSQL because we need embedded storage. Trade-off: less scalability.";
        let result = classify_memory(content);
        assert_eq!(result.memory_type, MemoryType::Decision);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_classify_problem() {
        let content = "Bug: The cache invalidation failed when TTL exceeded 24h. Fixed by adding explicit timestamp check.";
        let result = classify_memory(content);
        assert_eq!(result.memory_type, MemoryType::Problem);
        assert!(result.confidence > 0.3);
    }

    #[test]
    fn test_classify_insight() {
        let content = "TIL: The Rust borrow checker actually caught a race condition we would have missed otherwise. Lesson: trust the compiler.";
        let result = classify_memory(content);
        assert_eq!(result.memory_type, MemoryType::Insight);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_classify_exception() {
        let content = "Constitution Exception: This spec bypasses the 'no raw SQL' guardrail because we need custom aggregation.";
        let result = classify_memory(content);
        assert_eq!(result.memory_type, MemoryType::Exception);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_classify_reference() {
        let content = "See: https://docs.rust-lang.org/book/ for the official Rust documentation.";
        let result = classify_memory(content);
        assert_eq!(result.memory_type, MemoryType::Reference);
        assert!(result.confidence > 0.3);
    }

    #[test]
    fn test_classify_unknown() {
        let content =
            "Some random text without any clear signals about what type of memory this is.";
        let result = classify_memory(content);
        assert_eq!(result.memory_type, MemoryType::Unknown);
        assert_eq!(result.confidence, 0.0);
    }

    #[test]
    fn test_classify_with_existing_tag() {
        let content = "Some content with type:pattern tag already present.";
        let result = classify_memory(content);
        assert_eq!(result.memory_type, MemoryType::Pattern);
        assert!(result.confidence >= 0.5);
    }

    #[test]
    fn test_has_type_tag() {
        assert_eq!(
            has_type_tag("type:pattern content"),
            Some(MemoryType::Pattern)
        );
        assert_eq!(
            has_type_tag("type:decision rationale"),
            Some(MemoryType::Decision)
        );
        assert_eq!(has_type_tag("no tag here"), None);
    }

    #[test]
    fn test_classification_result_threshold() {
        let result = ClassificationResult::new(MemoryType::Pattern, 0.8, vec![]);
        assert!(result.meets_threshold(0.7));
        assert!(!result.meets_threshold(0.9));
    }

    #[test]
    fn test_classifier_config_default() {
        let config = ClassifierConfig::default();
        assert_eq!(config.confidence_threshold, 0.7);
        assert!(!config.llm_fallback_enabled);
    }

    #[test]
    fn test_case_insensitive_classification() {
        let content = "PATTERN: This should still match despite uppercase.";
        let result = classify_memory(content);
        assert_eq!(result.memory_type, MemoryType::Pattern);
    }
}
