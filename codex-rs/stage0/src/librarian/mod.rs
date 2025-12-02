//! SPEC-KIT-103: Librarian — Memory Corpus Quality Engine
//!
//! This module provides tools for improving memory corpus quality:
//! - Classification: Assign semantic types (Pattern, Decision, Problem, etc.)
//! - Templating: Restructure content into CONTEXT/REASONING/OUTCOME format
//! - Causal inference: Detect relationships beyond `similar`
//!
//! ## Usage
//!
//! ```rust,ignore
//! use codex_stage0::librarian::{classify_memory, apply_template, MemoryType};
//!
//! let content = "Decision: Use SQLite because we need embedded storage.";
//! let classification = classify_memory(content);
//! let templated = apply_template(content, classification.memory_type);
//! ```

pub mod causal;
pub mod classifier;
pub mod templater;

// Re-export main types and functions
pub use causal::{CausalConfig, CausalEdge, CausalRelation, detect_causal_language, infer_relationships};
pub use classifier::{
    ClassificationResult, ClassifierConfig, MemoryType, classify_memory, has_type_tag,
};
pub use templater::{TemplatedMemory, TemplaterConfig, apply_template, apply_template_with_config};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Configuration for a librarian sweep operation
#[derive(Debug, Clone, Default)]
pub struct SweepConfig {
    /// Run in dry-run mode (no writes)
    pub dry_run: bool,
    /// Filter by domains (empty = all domains)
    pub domains: Vec<String>,
    /// Maximum memories to process (0 = unlimited)
    pub limit: usize,
    /// Minimum importance to process (0 = all)
    pub min_importance: i32,
    /// Output JSON report
    pub json_report: bool,
    /// Classifier configuration
    pub classifier: ClassifierConfig,
    /// Templater configuration
    pub templater: TemplaterConfig,
    /// Causal inference configuration
    pub causal: CausalConfig,
}

impl SweepConfig {
    /// Create a dry-run configuration (safe for testing)
    pub fn dry_run() -> Self {
        Self {
            dry_run: true,
            ..Default::default()
        }
    }

    /// Create a limited sweep (for incremental processing)
    pub fn limited(limit: usize) -> Self {
        Self {
            limit,
            ..Default::default()
        }
    }
}

/// Change action in a sweep
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum SweepChange {
    /// Memory was reclassified
    Retype {
        memory_id: String,
        old_type: Option<String>,
        new_type: String,
        confidence: f32,
    },
    /// Memory was restructured with template
    Template {
        memory_id: String,
        memory_type: String,
        preserved_original: bool,
        warnings: Vec<String>,
    },
    /// Causal edge was inferred
    CausalEdge {
        source_id: String,
        target_id: String,
        relation: String,
        confidence: f32,
    },
    /// Memory flagged for review (Unknown type)
    FlaggedForReview {
        memory_id: String,
        reason: String,
    },
}

/// Summary statistics for a sweep
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SweepSummary {
    /// Total memories scanned
    pub memories_scanned: usize,
    /// Memories that were reclassified
    pub memories_retyped: usize,
    /// Memories that were restructured
    pub memories_templated: usize,
    /// Causal edges created
    pub causal_edges_created: usize,
    /// Memories flagged as Unknown
    pub unknown_flagged: usize,
    /// Processing duration in milliseconds
    pub duration_ms: u64,
}

/// Result of a librarian sweep operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepResult {
    /// Unique identifier for this sweep
    pub sweep_id: String,
    /// Whether this was a dry run
    pub dry_run: bool,
    /// Configuration used
    #[serde(skip)]
    pub config: Option<SweepConfig>,
    /// Summary statistics
    pub summary: SweepSummary,
    /// Individual changes (for JSON report)
    pub changes: Vec<SweepChange>,
    /// Timestamp of sweep
    pub timestamp: DateTime<Utc>,
}

impl SweepResult {
    /// Create a new sweep result
    pub fn new(sweep_id: impl Into<String>, dry_run: bool) -> Self {
        Self {
            sweep_id: sweep_id.into(),
            dry_run,
            config: None,
            summary: SweepSummary::default(),
            changes: Vec::new(),
            timestamp: Utc::now(),
        }
    }

    /// Add a retype change
    pub fn add_retype(
        &mut self,
        memory_id: impl Into<String>,
        old_type: Option<&str>,
        new_type: &MemoryType,
        confidence: f32,
    ) {
        self.changes.push(SweepChange::Retype {
            memory_id: memory_id.into(),
            old_type: old_type.map(String::from),
            new_type: new_type.to_string(),
            confidence,
        });
        self.summary.memories_retyped += 1;
    }

    /// Add a template change
    pub fn add_template(
        &mut self,
        memory_id: impl Into<String>,
        memory_type: &MemoryType,
        preserved_original: bool,
        warnings: Vec<String>,
    ) {
        self.changes.push(SweepChange::Template {
            memory_id: memory_id.into(),
            memory_type: memory_type.to_string(),
            preserved_original,
            warnings,
        });
        self.summary.memories_templated += 1;
    }

    /// Add a causal edge change
    pub fn add_causal_edge(&mut self, edge: &CausalEdge) {
        self.changes.push(SweepChange::CausalEdge {
            source_id: edge.source_id.clone(),
            target_id: edge.target_id.clone(),
            relation: edge.relation.to_string(),
            confidence: edge.confidence,
        });
        self.summary.causal_edges_created += 1;
    }

    /// Flag a memory for review
    pub fn flag_for_review(&mut self, memory_id: impl Into<String>, reason: impl Into<String>) {
        self.changes.push(SweepChange::FlaggedForReview {
            memory_id: memory_id.into(),
            reason: reason.into(),
        });
        self.summary.unknown_flagged += 1;
    }

    /// Generate human-readable summary
    pub fn summary_text(&self) -> String {
        let mode = if self.dry_run { "DRY RUN" } else { "LIVE" };
        format!(
            "Librarian Sweep {} [{}]\n\
             Scanned: {} memories\n\
             Retyped: {}\n\
             Templated: {}\n\
             Causal edges: {}\n\
             Flagged for review: {}\n\
             Duration: {}ms",
            self.sweep_id,
            mode,
            self.summary.memories_scanned,
            self.summary.memories_retyped,
            self.summary.memories_templated,
            self.summary.causal_edges_created,
            self.summary.unknown_flagged,
            self.summary.duration_ms,
        )
    }

    /// Serialize to JSON for CI output
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Process a single memory through the librarian pipeline
///
/// This is the core processing function that:
/// 1. Classifies the memory
/// 2. Applies template structure
/// 3. Optionally detects causal language
///
/// Returns the classification, templated content, and any detected causal patterns.
pub fn process_memory(
    memory_id: &str,
    content: &str,
    config: &SweepConfig,
) -> (ClassificationResult, TemplatedMemory, Vec<(CausalRelation, f32, String)>) {
    // Classify
    let classification = classify_memory(content);

    // Template
    let templated = apply_template_with_config(content, classification.memory_type, &config.templater);

    // Detect causal language (for later edge creation)
    let causal_patterns = detect_causal_language(content);

    tracing::debug!(
        memory_id = memory_id,
        memory_type = %classification.memory_type,
        confidence = classification.confidence,
        templated = !templated.preserved_original,
        causal_patterns = causal_patterns.len(),
        "Processed memory"
    );

    (classification, templated, causal_patterns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sweep_config_dry_run() {
        let config = SweepConfig::dry_run();
        assert!(config.dry_run);
        assert!(config.domains.is_empty());
        assert_eq!(config.limit, 0);
    }

    #[test]
    fn test_sweep_config_limited() {
        let config = SweepConfig::limited(100);
        assert!(!config.dry_run);
        assert_eq!(config.limit, 100);
    }

    #[test]
    fn test_sweep_result_creation() {
        let result = SweepResult::new("sweep-001", true);
        assert_eq!(result.sweep_id, "sweep-001");
        assert!(result.dry_run);
        assert!(result.changes.is_empty());
    }

    #[test]
    fn test_sweep_result_add_retype() {
        let mut result = SweepResult::new("sweep-001", true);
        result.add_retype("mem-001", None, &MemoryType::Pattern, 0.85);

        assert_eq!(result.summary.memories_retyped, 1);
        assert_eq!(result.changes.len(), 1);

        if let SweepChange::Retype { memory_id, new_type, confidence, .. } = &result.changes[0] {
            assert_eq!(memory_id, "mem-001");
            assert_eq!(new_type, "pattern");
            assert_eq!(*confidence, 0.85);
        } else {
            panic!("Expected Retype change");
        }
    }

    #[test]
    fn test_sweep_result_add_template() {
        let mut result = SweepResult::new("sweep-001", true);
        result.add_template("mem-001", &MemoryType::Decision, false, vec!["warning".to_string()]);

        assert_eq!(result.summary.memories_templated, 1);
        assert_eq!(result.changes.len(), 1);
    }

    #[test]
    fn test_sweep_result_add_causal_edge() {
        let mut result = SweepResult::new("sweep-001", true);
        let edge = CausalEdge::new("src", "tgt", CausalRelation::Causes, 0.9, "caused");
        result.add_causal_edge(&edge);

        assert_eq!(result.summary.causal_edges_created, 1);
    }

    #[test]
    fn test_sweep_result_flag_for_review() {
        let mut result = SweepResult::new("sweep-001", true);
        result.flag_for_review("mem-001", "No clear type signals");

        assert_eq!(result.summary.unknown_flagged, 1);
    }

    #[test]
    fn test_sweep_result_summary_text() {
        let mut result = SweepResult::new("sweep-001", true);
        result.summary.memories_scanned = 100;
        result.summary.memories_retyped = 25;
        result.summary.duration_ms = 500;

        let text = result.summary_text();
        assert!(text.contains("sweep-001"));
        assert!(text.contains("DRY RUN"));
        assert!(text.contains("100 memories"));
        assert!(text.contains("25"));
        assert!(text.contains("500ms"));
    }

    #[test]
    fn test_sweep_result_to_json() {
        let result = SweepResult::new("sweep-001", true);
        let json = result.to_json().unwrap();

        assert!(json.contains("sweep-001"));
        assert!(json.contains("dry_run"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_process_memory_decision() {
        let config = SweepConfig::dry_run();
        let content = "Decision: Use SQLite because we need embedded storage.";

        let (classification, templated, _) = process_memory("mem-001", content, &config);

        assert_eq!(classification.memory_type, MemoryType::Decision);
        assert!(classification.confidence > 0.0);
        assert!(templated.content.contains("## CONTEXT") || templated.content.contains("CONTEXT"));
        assert!(templated.content.contains("type:decision"));
    }

    #[test]
    fn test_process_memory_with_causal_language() {
        let config = SweepConfig::dry_run();
        let content = "The cache bug caused memory leaks in production.";

        let (_, _, causal_patterns) = process_memory("mem-001", content, &config);

        assert!(!causal_patterns.is_empty());
        assert!(causal_patterns.iter().any(|(rel, _, _)| *rel == CausalRelation::Causes));
    }

    #[test]
    fn test_process_memory_unknown() {
        let config = SweepConfig::dry_run();
        let content = "Random text with no clear signals.";

        let (classification, templated, _) = process_memory("mem-001", content, &config);

        assert_eq!(classification.memory_type, MemoryType::Unknown);
        assert!(templated.content.contains("type:unknown"));
    }
}
