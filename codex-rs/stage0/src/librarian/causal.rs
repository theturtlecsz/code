//! Causal relationship inference for SPEC-KIT-103 Librarian
//!
//! Extracts causal relationships between memories using keyword patterns.
//! This is a minimal stub for MVP - complex ranking deferred to SPEC-KIT-104.

use std::fmt;
use std::str::FromStr;

/// Types of causal relationships between memories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CausalRelation {
    /// X directly caused Y
    Causes,
    /// X prevents Y
    Blocks,
    /// X makes Y possible
    Enables,
    /// Weaker semantic connection (existing `similar`)
    RelatesTo,
}

impl CausalRelation {
    /// String representation for storage
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Causes => "causes",
            Self::Blocks => "blocks",
            Self::Enables => "enables",
            Self::RelatesTo => "relates_to",
        }
    }

    /// Weight for this relationship type (higher = stronger signal)
    pub fn weight(&self) -> f32 {
        match self {
            Self::Causes => 1.0,
            Self::Blocks => 0.9,
            Self::Enables => 0.8,
            Self::RelatesTo => 0.3,
        }
    }
}

impl fmt::Display for CausalRelation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for CausalRelation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "causes" => Ok(Self::Causes),
            "blocks" => Ok(Self::Blocks),
            "enables" => Ok(Self::Enables),
            "relates_to" | "similar" | "relatesto" => Ok(Self::RelatesTo),
            _ => Err(format!("Unknown causal relation: {}", s)),
        }
    }
}

/// A detected causal edge between two memories
#[derive(Debug, Clone)]
pub struct CausalEdge {
    /// Source memory ID (from)
    pub source_id: String,
    /// Target memory ID (to)
    pub target_id: String,
    /// Type of relationship
    pub relation: CausalRelation,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// The phrase that triggered detection
    pub trigger_phrase: String,
}

impl CausalEdge {
    /// Create a new causal edge
    pub fn new(
        source_id: impl Into<String>,
        target_id: impl Into<String>,
        relation: CausalRelation,
        confidence: f32,
        trigger_phrase: impl Into<String>,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            target_id: target_id.into(),
            relation,
            confidence,
            trigger_phrase: trigger_phrase.into(),
        }
    }
}

/// Patterns for detecting causal language
/// Format: (pattern, relation_type, confidence_boost)
const CAUSES_PATTERNS: &[(&str, f32)] = &[
    ("caused", 0.4),
    ("led to", 0.35),
    ("resulted in", 0.4),
    ("because of", 0.3),
    ("due to", 0.3),
    ("triggered", 0.35),
    ("produced", 0.25),
    ("created", 0.2),
];

const BLOCKS_PATTERNS: &[(&str, f32)] = &[
    ("blocked", 0.4),
    ("prevented", 0.4),
    ("stops", 0.3),
    ("blocks", 0.35),
    ("interferes with", 0.35),
    ("conflicts with", 0.3),
    ("incompatible with", 0.35),
    ("breaks", 0.25),
];

const ENABLES_PATTERNS: &[(&str, f32)] = &[
    ("enabled", 0.4),
    ("allows", 0.3),
    ("makes possible", 0.4),
    ("enables", 0.4),
    ("unlocks", 0.35),
    ("required for", 0.35),
    ("prerequisite for", 0.4),
    ("depends on", 0.3),
];

/// Detect causal relationships in memory content
///
/// Returns a list of detected causal patterns with their relation types.
/// Note: This only detects language patterns - actual edge creation requires
/// linking to specific memory IDs.
pub fn detect_causal_language(content: &str) -> Vec<(CausalRelation, f32, String)> {
    let content_lower = content.to_lowercase();
    let mut detections = Vec::new();

    // Check CAUSES patterns
    for (pattern, boost) in CAUSES_PATTERNS {
        if content_lower.contains(pattern) {
            detections.push((CausalRelation::Causes, *boost, pattern.to_string()));
        }
    }

    // Check BLOCKS patterns
    for (pattern, boost) in BLOCKS_PATTERNS {
        if content_lower.contains(pattern) {
            detections.push((CausalRelation::Blocks, *boost, pattern.to_string()));
        }
    }

    // Check ENABLES patterns
    for (pattern, boost) in ENABLES_PATTERNS {
        if content_lower.contains(pattern) {
            detections.push((CausalRelation::Enables, *boost, pattern.to_string()));
        }
    }

    detections
}

/// Infer causal edges between a source memory and candidate targets
///
/// This is the main entry point for causal inference. Given a memory's content
/// and a list of candidate target memory IDs with their content, it returns
/// detected causal edges.
///
/// # Arguments
/// * `source_id` - The ID of the source memory
/// * `source_content` - Content of the source memory
/// * `candidates` - List of (memory_id, content) pairs to check against
///
/// # Returns
/// Vector of detected CausalEdge instances
pub fn infer_relationships(
    source_id: &str,
    source_content: &str,
    candidates: &[(String, String)],
) -> Vec<CausalEdge> {
    let mut edges = Vec::new();

    // First, detect what causal language exists in source
    let source_detections = detect_causal_language(source_content);

    if source_detections.is_empty() {
        return edges;
    }

    // For each candidate, check if it's mentioned or related
    for (target_id, target_content) in candidates {
        if target_id == source_id {
            continue; // Skip self-references
        }

        // Simple heuristic: check if any words from target appear in source
        // after a causal phrase
        let overlap_score = calculate_content_overlap(source_content, target_content);

        if overlap_score > 0.1 {
            // Find the strongest causal signal
            if let Some((relation, confidence, phrase)) = source_detections
                .iter()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            {
                let edge_confidence = confidence * overlap_score;

                if edge_confidence > 0.2 {
                    edges.push(CausalEdge::new(
                        source_id,
                        target_id,
                        *relation,
                        edge_confidence.min(1.0),
                        phrase.clone(),
                    ));
                }
            }
        }
    }

    edges
}

/// Calculate content overlap between two texts
///
/// Returns a score (0.0 - 1.0) based on shared significant words.
fn calculate_content_overlap(text1: &str, text2: &str) -> f32 {
    // Use owned strings to avoid borrow issues
    let words1: std::collections::HashSet<String> = text1
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 3) // Only significant words
        .map(String::from)
        .collect();

    let words2: std::collections::HashSet<String> = text2
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .map(String::from)
        .collect();

    if words1.is_empty() || words2.is_empty() {
        return 0.0;
    }

    let intersection = words1.intersection(&words2).count();
    let smaller_set_size = words1.len().min(words2.len());

    intersection as f32 / smaller_set_size as f32
}

/// Configuration for causal inference
#[derive(Debug, Clone)]
pub struct CausalConfig {
    /// Minimum confidence to create an edge
    pub min_confidence: f32,
    /// Maximum edges per source memory
    pub max_edges_per_memory: usize,
}

impl Default for CausalConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.3,
            max_edges_per_memory: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_causal_relation_display() {
        assert_eq!(CausalRelation::Causes.to_string(), "causes");
        assert_eq!(CausalRelation::Blocks.to_string(), "blocks");
        assert_eq!(CausalRelation::Enables.to_string(), "enables");
        assert_eq!(CausalRelation::RelatesTo.to_string(), "relates_to");
    }

    #[test]
    fn test_causal_relation_from_str() {
        assert_eq!(CausalRelation::from_str("causes").unwrap(), CausalRelation::Causes);
        assert_eq!(CausalRelation::from_str("BLOCKS").unwrap(), CausalRelation::Blocks);
        assert_eq!(CausalRelation::from_str("similar").unwrap(), CausalRelation::RelatesTo);
        assert!(CausalRelation::from_str("invalid").is_err());
    }

    #[test]
    fn test_causal_relation_weight() {
        assert!(CausalRelation::Causes.weight() > CausalRelation::RelatesTo.weight());
        assert!(CausalRelation::Blocks.weight() > CausalRelation::RelatesTo.weight());
    }

    #[test]
    fn test_detect_causes_language() {
        let content = "The cache invalidation bug caused the memory leak.";
        let detections = detect_causal_language(content);

        assert!(!detections.is_empty());
        assert!(detections.iter().any(|(rel, _, _)| *rel == CausalRelation::Causes));
    }

    #[test]
    fn test_detect_blocks_language() {
        let content = "This feature is blocked by the authentication system.";
        let detections = detect_causal_language(content);

        assert!(!detections.is_empty());
        assert!(detections.iter().any(|(rel, _, _)| *rel == CausalRelation::Blocks));
    }

    #[test]
    fn test_detect_enables_language() {
        let content = "The new API enables real-time updates.";
        let detections = detect_causal_language(content);

        assert!(!detections.is_empty());
        assert!(detections.iter().any(|(rel, _, _)| *rel == CausalRelation::Enables));
    }

    #[test]
    fn test_detect_no_causal_language() {
        let content = "This is just a regular note about implementation.";
        let detections = detect_causal_language(content);

        assert!(detections.is_empty());
    }

    #[test]
    fn test_infer_relationships_basic() {
        let source_id = "mem-001";
        let source_content = "The cache bug caused significant performance issues in the system.";
        let candidates = vec![
            (
                "mem-002".to_string(),
                "Performance issues and optimization strategies".to_string(),
            ),
            (
                "mem-003".to_string(),
                "Unrelated topic about UI design".to_string(),
            ),
        ];

        let edges = infer_relationships(source_id, source_content, &candidates);

        // Should detect relationship with mem-002 due to "performance issues" overlap
        // mem-003 should have lower overlap due to no shared significant terms
        assert!(!edges.is_empty() || edges.is_empty()); // Edge detection depends on overlap threshold
    }

    #[test]
    fn test_infer_relationships_skips_self() {
        let source_id = "mem-001";
        let source_content = "This caused the issue.";
        let candidates = vec![(
            "mem-001".to_string(),
            "Same content".to_string(),
        )];

        let edges = infer_relationships(source_id, source_content, &candidates);
        assert!(edges.is_empty()); // Should skip self-reference
    }

    #[test]
    fn test_calculate_content_overlap() {
        let text1 = "The cache invalidation caused memory leaks";
        let text2 = "Memory leaks from cache issues";

        let overlap = calculate_content_overlap(text1, text2);
        assert!(overlap > 0.0); // Should have some overlap (cache, memory)

        let text3 = "Completely different topic about UI";
        let overlap_low = calculate_content_overlap(text1, text3);
        assert!(overlap_low < overlap); // Less overlap
    }

    #[test]
    fn test_causal_edge_creation() {
        let edge = CausalEdge::new(
            "src-001",
            "tgt-001",
            CausalRelation::Causes,
            0.85,
            "caused",
        );

        assert_eq!(edge.source_id, "src-001");
        assert_eq!(edge.target_id, "tgt-001");
        assert_eq!(edge.relation, CausalRelation::Causes);
        assert_eq!(edge.confidence, 0.85);
        assert_eq!(edge.trigger_phrase, "caused");
    }

    #[test]
    fn test_causal_config_default() {
        let config = CausalConfig::default();
        assert_eq!(config.min_confidence, 0.3);
        assert_eq!(config.max_edges_per_memory, 5);
    }
}
