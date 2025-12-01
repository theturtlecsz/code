//! Vector backend evaluation harness
//!
//! SPEC-KIT-102 V2: Provides evaluation infrastructure for testing
//! and benchmarking VectorBackend implementations.
//!
//! Key concepts:
//! - `EvalCase`: A test case with query and expected results
//! - `EvalResult`: Precision/recall metrics for a single case
//! - `EvalSuite`: Collection of cases with aggregate metrics
//!
//! Metrics computed:
//! - Precision@k: Fraction of retrieved documents that are relevant
//! - Recall@k: Fraction of relevant documents that are retrieved
//! - MRR (Mean Reciprocal Rank): Average 1/rank of first relevant result

use crate::errors::{Result, Stage0Error};
use crate::vector::{ScoredVector, VectorBackend, VectorFilters};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// A single evaluation test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCase {
    /// Human-readable name for the case
    pub name: String,

    /// Query text (spec snippet, user question, etc.)
    pub query: String,

    /// IDs of documents expected to be relevant
    pub expected_ids: Vec<String>,

    /// Optional description of what this case tests
    #[serde(default)]
    pub description: Option<String>,
}

impl EvalCase {
    /// Create a new eval case
    pub fn new(name: impl Into<String>, query: impl Into<String>, expected_ids: Vec<String>) -> Self {
        Self {
            name: name.into(),
            query: query.into(),
            expected_ids,
            description: None,
        }
    }

    /// Add a description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Results from evaluating a single case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    /// Case name
    pub case_name: String,

    /// Precision@k: |relevant ∩ retrieved| / |retrieved|
    pub precision_at_k: f64,

    /// Recall@k: |relevant ∩ retrieved| / |relevant|
    pub recall_at_k: f64,

    /// Reciprocal rank of first relevant result (0 if none found)
    pub reciprocal_rank: f64,

    /// IDs of relevant documents that were retrieved (in order)
    pub hits: Vec<String>,

    /// IDs of relevant documents that were missed
    pub misses: Vec<String>,

    /// Top k retrieved document IDs (for debugging)
    pub retrieved: Vec<String>,
}

impl EvalResult {
    /// Check if this result meets minimum thresholds
    pub fn passes(&self, min_precision: f64, min_recall: f64) -> bool {
        self.precision_at_k >= min_precision && self.recall_at_k >= min_recall
    }
}

/// Aggregate results from an evaluation suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSuiteResult {
    /// Individual case results
    pub results: Vec<EvalResult>,

    /// Mean precision across all cases
    pub mean_precision: f64,

    /// Mean recall across all cases
    pub mean_recall: f64,

    /// Mean reciprocal rank across all cases
    pub mrr: f64,

    /// Number of cases that passed (P@k >= 0.5 and R@k >= 0.5)
    pub cases_passed: usize,

    /// Total cases evaluated
    pub total_cases: usize,

    /// Evaluation parameters
    pub top_k: usize,
}

impl EvalSuiteResult {
    /// Get overall pass rate
    pub fn pass_rate(&self) -> f64 {
        if self.total_cases == 0 {
            0.0
        } else {
            self.cases_passed as f64 / self.total_cases as f64
        }
    }

    /// Format as a summary string
    pub fn summary(&self) -> String {
        format!(
            "EvalSuite: {}/{} passed ({:.1}%), P@{}={:.2}, R@{}={:.2}, MRR={:.2}",
            self.cases_passed,
            self.total_cases,
            self.pass_rate() * 100.0,
            self.top_k,
            self.mean_precision,
            self.top_k,
            self.mean_recall,
            self.mrr,
        )
    }

    /// Format as a detailed table
    pub fn format_table(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "{:<30} {:>8} {:>8} {:>8} {}\n",
            "Case", "P@k", "R@k", "RR", "Hits"
        ));
        out.push_str(&"-".repeat(70));
        out.push('\n');

        for result in &self.results {
            let hits_str = if result.hits.is_empty() {
                "(none)".to_string()
            } else if result.hits.len() <= 3 {
                result.hits.join(", ")
            } else {
                format!("{}, ... (+{})", result.hits[..2].join(", "), result.hits.len() - 2)
            };

            out.push_str(&format!(
                "{:<30} {:>8.2} {:>8.2} {:>8.2} {}\n",
                truncate(&result.case_name, 30),
                result.precision_at_k,
                result.recall_at_k,
                result.reciprocal_rank,
                hits_str,
            ));
        }

        out.push_str(&"-".repeat(70));
        out.push('\n');
        out.push_str(&self.summary());
        out
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Evaluation Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Evaluate a single case against a backend
pub async fn evaluate_case<B: VectorBackend>(
    backend: &B,
    case: &EvalCase,
    filters: &VectorFilters,
    top_k: usize,
) -> Result<EvalResult> {
    let results = backend.search(&case.query, filters, top_k).await?;

    compute_metrics(case, &results)
}

/// Compute precision/recall metrics for search results
pub fn compute_metrics(case: &EvalCase, results: &[ScoredVector]) -> Result<EvalResult> {
    let expected: HashSet<&str> = case.expected_ids.iter().map(String::as_str).collect();
    let retrieved: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
    let retrieved_set: HashSet<&str> = retrieved.iter().map(String::as_str).collect();

    // Compute hits and misses
    let hits: Vec<String> = retrieved
        .iter()
        .filter(|id| expected.contains(id.as_str()))
        .cloned()
        .collect();

    let misses: Vec<String> = case
        .expected_ids
        .iter()
        .filter(|id| !retrieved_set.contains(id.as_str()))
        .cloned()
        .collect();

    // Precision@k
    let precision = if retrieved.is_empty() {
        0.0
    } else {
        hits.len() as f64 / retrieved.len() as f64
    };

    // Recall@k
    let recall = if case.expected_ids.is_empty() {
        1.0 // No expected = perfect recall vacuously
    } else {
        hits.len() as f64 / case.expected_ids.len() as f64
    };

    // Reciprocal rank
    let reciprocal_rank = retrieved
        .iter()
        .position(|id| expected.contains(id.as_str()))
        .map(|pos| 1.0 / (pos + 1) as f64)
        .unwrap_or(0.0);

    Ok(EvalResult {
        case_name: case.name.clone(),
        precision_at_k: precision,
        recall_at_k: recall,
        reciprocal_rank,
        hits,
        misses,
        retrieved,
    })
}

/// Evaluate a suite of cases against a backend
pub async fn evaluate_backend<B: VectorBackend>(
    backend: &B,
    cases: &[EvalCase],
    filters: &VectorFilters,
    top_k: usize,
) -> Result<EvalSuiteResult> {
    let mut results = Vec::with_capacity(cases.len());

    for case in cases {
        let result = evaluate_case(backend, case, filters, top_k).await?;
        results.push(result);
    }

    compute_suite_metrics(results, top_k)
}

/// Compute aggregate metrics from individual results
pub fn compute_suite_metrics(results: Vec<EvalResult>, top_k: usize) -> Result<EvalSuiteResult> {
    let total_cases = results.len();

    if total_cases == 0 {
        return Ok(EvalSuiteResult {
            results: Vec::new(),
            mean_precision: 0.0,
            mean_recall: 0.0,
            mrr: 0.0,
            cases_passed: 0,
            total_cases: 0,
            top_k,
        });
    }

    let sum_precision: f64 = results.iter().map(|r| r.precision_at_k).sum();
    let sum_recall: f64 = results.iter().map(|r| r.recall_at_k).sum();
    let sum_rr: f64 = results.iter().map(|r| r.reciprocal_rank).sum();

    let cases_passed = results.iter().filter(|r| r.passes(0.5, 0.5)).count();

    Ok(EvalSuiteResult {
        mean_precision: sum_precision / total_cases as f64,
        mean_recall: sum_recall / total_cases as f64,
        mrr: sum_rr / total_cases as f64,
        cases_passed,
        total_cases,
        top_k,
        results,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Built-in Eval Cases
// ─────────────────────────────────────────────────────────────────────────────

/// Built-in evaluation cases for unit tests
///
/// These provide a stable baseline for CI testing without external dependencies.
pub fn built_in_eval_cases() -> Vec<EvalCase> {
    vec![
        EvalCase::new(
            "stage0-architecture",
            "Stage 0 overlay engine architecture design decisions",
            vec![
                "mem-stage0-arch-001".to_string(),
                "mem-stage0-overlay-002".to_string(),
            ],
        )
        .with_description("Tests retrieval of Stage 0 architecture memories"),

        EvalCase::new(
            "tfidf-implementation",
            "TF-IDF vector search implementation BM25 scoring",
            vec![
                "mem-vector-tfidf-001".to_string(),
                "mem-bm25-scoring-002".to_string(),
            ],
        )
        .with_description("Tests retrieval of TF-IDF implementation details"),

        EvalCase::new(
            "bug-pattern-resize",
            "resize window crash segfault memory corruption",
            vec![
                "mem-bug-resize-001".to_string(),
                "mem-bug-segfault-002".to_string(),
            ],
        )
        .with_description("Tests retrieval of bug pattern memories"),

        EvalCase::new(
            "spec-kit-notebooklm",
            "SPEC-KIT-102 NotebookLM integration Tier 2 synthesis",
            vec![
                "mem-speckit-102-001".to_string(),
                "mem-tier2-notebook-002".to_string(),
            ],
        )
        .with_description("Tests SPEC-KIT-102 related memories"),

        EvalCase::new(
            "rust-error-handling",
            "Rust error handling Result thiserror anyhow",
            vec![
                "mem-rust-errors-001".to_string(),
                "mem-thiserror-002".to_string(),
            ],
        )
        .with_description("Tests Rust-specific pattern memories"),
    ]
}

/// Create synthetic test documents matching built-in eval cases
///
/// Useful for setting up test backends with known-good data.
pub fn built_in_test_documents() -> Vec<crate::vector::VectorDocument> {
    use crate::vector::{DocumentKind, VectorDocument};

    vec![
        // Stage 0 architecture
        VectorDocument::new(
            "mem-stage0-arch-001",
            DocumentKind::Memory,
            "Stage 0 overlay engine architecture separates concerns between \
             local-memory daemon and scoring. The design decision was to use \
             SQLite for the overlay database.",
        )
        .with_domain("spec-kit")
        .with_tag("type:decision"),

        VectorDocument::new(
            "mem-stage0-overlay-002",
            DocumentKind::Memory,
            "Stage 0 overlay design pattern uses a separate database to track \
             dynamic scores and Tier 2 cache entries without modifying local-memory.",
        )
        .with_domain("spec-kit")
        .with_tag("type:pattern"),

        // TF-IDF implementation
        VectorDocument::new(
            "mem-vector-tfidf-001",
            DocumentKind::Memory,
            "TF-IDF vector backend implementation uses BM25-style term frequency \
             saturation with k1=1.5 and b=0.75 parameters for scoring.",
        )
        .with_domain("spec-kit")
        .with_tag("type:implementation"),

        VectorDocument::new(
            "mem-bm25-scoring-002",
            DocumentKind::Memory,
            "BM25 scoring formula: TF * IDF where TF = (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * dl/avgdl)) \
             and IDF = log((N + 1) / (df + 1)) + 1",
        )
        .with_domain("spec-kit")
        .with_tag("type:algorithm"),

        // Bug patterns
        VectorDocument::new(
            "mem-bug-resize-001",
            DocumentKind::Memory,
            "Bug: Window resize causes crash when terminal size drops below minimum. \
             Root cause was unchecked subtraction in viewport calculation. \
             Fix: Add bounds checking before resize.",
        )
        .with_domain("tui")
        .with_tag("type:bug"),

        VectorDocument::new(
            "mem-bug-segfault-002",
            DocumentKind::Memory,
            "Segfault in async handler due to memory corruption. The buffer was \
             being written to after being moved. Fix: Use Arc for shared ownership.",
        )
        .with_domain("core")
        .with_tag("type:bug"),

        // SPEC-KIT-102
        VectorDocument::new(
            "mem-speckit-102-001",
            DocumentKind::Memory,
            "SPEC-KIT-102 defines NotebookLM integration for Stage 0. The key insight \
             is that NotebookLM provides synthesis capabilities beyond local-memory search.",
        )
        .with_domain("spec-kit")
        .with_tag("spec:SPEC-KIT-102"),

        VectorDocument::new(
            "mem-tier2-notebook-002",
            DocumentKind::Memory,
            "Tier 2 orchestration calls NotebookLM via MCP for Divine Truth synthesis. \
             Cache TTL is 24 hours to balance freshness with query costs.",
        )
        .with_domain("spec-kit")
        .with_tag("type:integration"),

        // Rust patterns
        VectorDocument::new(
            "mem-rust-errors-001",
            DocumentKind::Memory,
            "Rust error handling best practice: Use thiserror for library errors \
             and anyhow for application errors. Result<T, E> is the standard pattern.",
        )
        .with_domain("rust")
        .with_tag("type:pattern"),

        VectorDocument::new(
            "mem-thiserror-002",
            DocumentKind::Memory,
            "thiserror derive macro generates Error trait implementations. \
             Use #[error] for Display, #[from] for automatic From conversions.",
        )
        .with_domain("rust")
        .with_tag("type:library"),

        // Extra documents for diversity
        VectorDocument::new(
            "mem-unrelated-001",
            DocumentKind::Memory,
            "Meeting notes from Q3 planning session. Discussed roadmap priorities \
             and resource allocation for the next quarter.",
        )
        .with_domain("planning")
        .with_tag("type:notes"),

        VectorDocument::new(
            "mem-unrelated-002",
            DocumentKind::Memory,
            "Configuration guide for CI/CD pipeline. Uses GitHub Actions with \
             cargo test and clippy checks on every PR.",
        )
        .with_domain("devops")
        .with_tag("type:guide"),
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// JSON Loading
// ─────────────────────────────────────────────────────────────────────────────

/// Load eval cases from a JSON file
pub fn load_eval_cases_from_file(path: &Path) -> Result<Vec<EvalCase>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Stage0Error::config_with_source(format!("Failed to read eval cases file: {}", path.display()), e))?;

    let cases: Vec<EvalCase> = serde_json::from_str(&content)
        .map_err(|e| Stage0Error::config_with_source("Failed to parse eval cases JSON", e))?;

    Ok(cases)
}

/// Save eval cases to a JSON file
pub fn save_eval_cases_to_file(cases: &[EvalCase], path: &Path) -> Result<()> {
    let content = serde_json::to_string_pretty(cases)
        .map_err(|e| Stage0Error::internal(format!("Failed to serialize eval cases: {e}")))?;

    std::fs::write(path, content)
        .map_err(|e| Stage0Error::config_with_source(format!("Failed to write eval cases file: {}", path.display()), e))?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::uninlined_format_args)]
mod tests {
    use super::*;
    use crate::tfidf::TfIdfBackend;
    use crate::vector::VectorFilters;

    #[test]
    fn test_compute_metrics_perfect() {
        let case = EvalCase::new("test", "query", vec!["a".to_string(), "b".to_string()]);

        let results = vec![
            ScoredVector::new("a", 0.9, crate::vector::DocumentKind::Memory),
            ScoredVector::new("b", 0.8, crate::vector::DocumentKind::Memory),
        ];

        let metrics = compute_metrics(&case, &results).unwrap();

        assert!((metrics.precision_at_k - 1.0).abs() < f64::EPSILON);
        assert!((metrics.recall_at_k - 1.0).abs() < f64::EPSILON);
        assert!((metrics.reciprocal_rank - 1.0).abs() < f64::EPSILON);
        assert_eq!(metrics.hits.len(), 2);
        assert!(metrics.misses.is_empty());
    }

    #[test]
    fn test_compute_metrics_partial() {
        let case = EvalCase::new(
            "test",
            "query",
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );

        let results = vec![
            ScoredVector::new("a", 0.9, crate::vector::DocumentKind::Memory),
            ScoredVector::new("x", 0.8, crate::vector::DocumentKind::Memory), // Not relevant
            ScoredVector::new("b", 0.7, crate::vector::DocumentKind::Memory),
        ];

        let metrics = compute_metrics(&case, &results).unwrap();

        // 2 hits out of 3 retrieved = 0.67 precision
        assert!((metrics.precision_at_k - 2.0 / 3.0).abs() < 0.01);

        // 2 hits out of 3 expected = 0.67 recall
        assert!((metrics.recall_at_k - 2.0 / 3.0).abs() < 0.01);

        // First relevant at position 1 = RR 1.0
        assert!((metrics.reciprocal_rank - 1.0).abs() < f64::EPSILON);

        assert_eq!(metrics.hits, vec!["a", "b"]);
        assert_eq!(metrics.misses, vec!["c"]);
    }

    #[test]
    fn test_compute_metrics_no_hits() {
        let case = EvalCase::new("test", "query", vec!["a".to_string(), "b".to_string()]);

        let results = vec![
            ScoredVector::new("x", 0.9, crate::vector::DocumentKind::Memory),
            ScoredVector::new("y", 0.8, crate::vector::DocumentKind::Memory),
        ];

        let metrics = compute_metrics(&case, &results).unwrap();

        assert!((metrics.precision_at_k).abs() < f64::EPSILON);
        assert!((metrics.recall_at_k).abs() < f64::EPSILON);
        assert!((metrics.reciprocal_rank).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_metrics_empty_results() {
        let case = EvalCase::new("test", "query", vec!["a".to_string()]);
        let results: Vec<ScoredVector> = vec![];

        let metrics = compute_metrics(&case, &results).unwrap();

        assert!((metrics.precision_at_k).abs() < f64::EPSILON);
        assert!((metrics.recall_at_k).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_metrics_empty_expected() {
        let case = EvalCase::new("test", "query", vec![]);

        let results = vec![ScoredVector::new(
            "x",
            0.9,
            crate::vector::DocumentKind::Memory,
        )];

        let metrics = compute_metrics(&case, &results).unwrap();

        // No expected = perfect recall (vacuous truth)
        assert!((metrics.recall_at_k - 1.0).abs() < f64::EPSILON);
        // No expected = 0 precision (nothing relevant in retrieved)
        assert!((metrics.precision_at_k).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_suite_metrics() {
        let results = vec![
            EvalResult {
                case_name: "case1".to_string(),
                precision_at_k: 1.0,
                recall_at_k: 1.0,
                reciprocal_rank: 1.0,
                hits: vec!["a".to_string()],
                misses: vec![],
                retrieved: vec!["a".to_string()],
            },
            EvalResult {
                case_name: "case2".to_string(),
                precision_at_k: 0.5,
                recall_at_k: 0.5,
                reciprocal_rank: 0.5,
                hits: vec!["b".to_string()],
                misses: vec!["c".to_string()],
                retrieved: vec!["b".to_string(), "x".to_string()],
            },
        ];

        let suite = compute_suite_metrics(results, 10).unwrap();

        assert!((suite.mean_precision - 0.75).abs() < f64::EPSILON);
        assert!((suite.mean_recall - 0.75).abs() < f64::EPSILON);
        assert!((suite.mrr - 0.75).abs() < f64::EPSILON);
        assert_eq!(suite.cases_passed, 2); // Both pass threshold 0.5
        assert_eq!(suite.total_cases, 2);
    }

    #[test]
    fn test_eval_result_passes() {
        let result = EvalResult {
            case_name: "test".to_string(),
            precision_at_k: 0.6,
            recall_at_k: 0.8,
            reciprocal_rank: 1.0,
            hits: vec![],
            misses: vec![],
            retrieved: vec![],
        };

        assert!(result.passes(0.5, 0.5));
        assert!(result.passes(0.6, 0.8));
        assert!(!result.passes(0.7, 0.5));
        assert!(!result.passes(0.5, 0.9));
    }

    #[tokio::test]
    async fn test_evaluate_backend_with_builtin_cases() {
        let backend = TfIdfBackend::new();

        // Index test documents
        let docs = built_in_test_documents();
        backend.index_documents(docs).await.unwrap();

        // Run evaluation
        let cases = built_in_eval_cases();
        let suite = evaluate_backend(&backend, &cases, &VectorFilters::new(), 10)
            .await
            .unwrap();

        // We should get reasonable results
        assert_eq!(suite.total_cases, 5);
        assert!(suite.mean_precision > 0.0, "Should have some precision");
        assert!(suite.mean_recall > 0.0, "Should have some recall");
    }

    #[test]
    fn test_builtin_cases_structure() {
        let cases = built_in_eval_cases();

        assert!(!cases.is_empty());

        for case in &cases {
            assert!(!case.name.is_empty());
            assert!(!case.query.is_empty());
            assert!(!case.expected_ids.is_empty());
        }
    }

    #[test]
    fn test_builtin_test_documents_structure() {
        let docs = built_in_test_documents();

        assert!(docs.len() >= 10);

        // Check all expected IDs from cases are present
        let doc_ids: std::collections::HashSet<&str> =
            docs.iter().map(|d| d.id.as_str()).collect();

        let cases = built_in_eval_cases();
        for case in &cases {
            for expected_id in &case.expected_ids {
                assert!(
                    doc_ids.contains(expected_id.as_str()),
                    "Missing expected document: {}",
                    expected_id
                );
            }
        }
    }

    #[test]
    fn test_eval_case_builder() {
        let case = EvalCase::new("test-case", "test query", vec!["id1".to_string()])
            .with_description("A test description");

        assert_eq!(case.name, "test-case");
        assert_eq!(case.query, "test query");
        assert_eq!(case.description, Some("A test description".to_string()));
    }

    #[test]
    fn test_format_table() {
        let results = vec![EvalResult {
            case_name: "test-case-with-long-name".to_string(),
            precision_at_k: 0.85,
            recall_at_k: 0.75,
            reciprocal_rank: 1.0,
            hits: vec!["hit1".to_string(), "hit2".to_string()],
            misses: vec!["miss1".to_string()],
            retrieved: vec!["hit1".to_string(), "hit2".to_string(), "other".to_string()],
        }];

        let suite = compute_suite_metrics(results, 10).unwrap();
        let table = suite.format_table();

        assert!(table.contains("test-case-with-long-name"));
        assert!(table.contains("0.85"));
        assert!(table.contains("0.75"));
        assert!(table.contains("hit1"));
    }
}
