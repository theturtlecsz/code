//! Vector backend evaluation harness
//!
//! SPEC-KIT-102 V2: Provides evaluation infrastructure for testing
//! and benchmarking VectorBackend implementations.
//!
//! P86: Extended for Code Lane evaluation with P@K, R@K, MRR metrics
//! for both memory retrieval and code unit retrieval.
//!
//! Key concepts:
//! - `EvalLane`: Which retrieval lane to evaluate (Memory or Code)
//! - `EvalCase`: A test case with query, expected results, and lane
//! - `EvalResult`: Precision/recall metrics for a single case
//! - `EvalSuite`: Collection of cases with aggregate metrics
//!
//! Metrics computed:
//! - Precision@k: Fraction of retrieved documents that are relevant
//! - Recall@k: Fraction of relevant documents that are retrieved
//! - MRR (Mean Reciprocal Rank): Average 1/rank of first relevant result

use crate::dcc::CompileContextResult;
use crate::errors::{Result, Stage0Error};
use crate::vector::{ScoredVector, VectorBackend, VectorFilters};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// P86: Evaluation lane - which retrieval lane to evaluate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EvalLane {
    /// Memory retrieval lane (local-memory + overlay)
    #[default]
    Memory,
    /// Code unit retrieval lane (TF-IDF code search)
    Code,
}

impl EvalLane {
    /// Get display name for the lane
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Code => "code",
        }
    }
}

impl std::fmt::Display for EvalLane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// P86: Source of an evaluation case
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EvalCaseSource {
    /// Built-in test case
    #[default]
    Builtin,
    /// Loaded from external JSON file
    External,
}

impl EvalCaseSource {
    /// Get display name for the source
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::External => "external",
        }
    }
}

impl std::fmt::Display for EvalCaseSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

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

    /// P86: Which retrieval lane this case tests (Memory or Code)
    #[serde(default)]
    pub lane: EvalLane,

    /// P86: Source of this case (builtin or external)
    /// Note: This is not serialized in JSON files - it's set programmatically
    #[serde(skip)]
    pub source: EvalCaseSource,
}

impl EvalCase {
    /// Create a new eval case (defaults to Memory lane)
    pub fn new(
        name: impl Into<String>,
        query: impl Into<String>,
        expected_ids: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            query: query.into(),
            expected_ids,
            description: None,
            lane: EvalLane::Memory,
            source: EvalCaseSource::Builtin,
        }
    }

    /// Create a new code lane eval case
    pub fn new_code(
        name: impl Into<String>,
        query: impl Into<String>,
        expected_ids: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            query: query.into(),
            expected_ids,
            description: None,
            lane: EvalLane::Code,
            source: EvalCaseSource::Builtin,
        }
    }

    /// Add a description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// P86: Set the evaluation lane
    pub fn with_lane(mut self, lane: EvalLane) -> Self {
        self.lane = lane;
        self
    }

    /// P86: Mark as external source (loaded from file)
    pub fn mark_external(mut self) -> Self {
        self.source = EvalCaseSource::External;
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

    /// P86: Which lane this result is from
    #[serde(default)]
    pub lane: EvalLane,

    /// P86: Source of the eval case (builtin or external)
    #[serde(default)]
    pub source: EvalCaseSource,

    /// P86: Expected IDs that don't exist in the index
    /// Used for hybrid missing ID handling - these are excluded from denominators
    #[serde(default)]
    pub missing_expected_ids: Vec<String>,
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

    /// P86: Total missing expected IDs across all cases
    #[serde(default)]
    pub total_missing_ids: usize,
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

    /// P86: Check if any expected IDs were missing from the index
    pub fn has_missing_ids(&self) -> bool {
        self.total_missing_ids > 0
    }

    /// P86: Get results filtered by lane
    pub fn filter_by_lane(&self, lane: EvalLane) -> Vec<&EvalResult> {
        self.results.iter().filter(|r| r.lane == lane).collect()
    }

    /// Format as a summary string
    pub fn summary(&self) -> String {
        let mut s = format!(
            "EvalSuite: {}/{} passed ({:.1}%), P@{}={:.2}, R@{}={:.2}, MRR={:.2}",
            self.cases_passed,
            self.total_cases,
            self.pass_rate() * 100.0,
            self.top_k,
            self.mean_precision,
            self.top_k,
            self.mean_recall,
            self.mrr,
        );

        if self.total_missing_ids > 0 {
            s.push_str(&format!(" [⚠ {} missing IDs]", self.total_missing_ids));
        }

        s
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
                format!(
                    "{}, ... (+{})",
                    result.hits[..2].join(", "),
                    result.hits.len() - 2
                )
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

    /// P86: Format as detailed table with lane and source columns
    pub fn format_table_with_lanes(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "{:<24} {:>6} {:>8} {:>8} {:>8} {:>8} {}\n",
            "Case", "Lane", "Source", "P@k", "R@k", "RR", "Hits"
        ));
        out.push_str(&"-".repeat(90));
        out.push('\n');

        for result in &self.results {
            let hits_str = if result.hits.is_empty() {
                "(none)".to_string()
            } else if result.hits.len() <= 2 {
                result.hits.join(", ")
            } else {
                format!("{}, +{}", result.hits[0], result.hits.len() - 1)
            };

            let missing_marker = if !result.missing_expected_ids.is_empty() {
                format!(" [⚠{}]", result.missing_expected_ids.len())
            } else {
                String::new()
            };

            out.push_str(&format!(
                "{:<24} {:>6} {:>8} {:>8.2} {:>8.2} {:>8.2} {}{}\n",
                truncate(&result.case_name, 24),
                result.lane.as_str(),
                result.source.as_str(),
                result.precision_at_k,
                result.recall_at_k,
                result.reciprocal_rank,
                hits_str,
                missing_marker,
            ));
        }

        out.push_str(&"-".repeat(90));
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
    // P86: Empty missing_expected_ids - this version doesn't check index existence
    compute_metrics_with_missing(case, results, Vec::new())
}

/// P86: Compute precision/recall metrics with missing ID handling
///
/// When `missing_expected_ids` is provided, those IDs are excluded from the
/// denominator calculations, effectively treating them as "not expected".
pub fn compute_metrics_with_missing(
    case: &EvalCase,
    results: &[ScoredVector],
    missing_expected_ids: Vec<String>,
) -> Result<EvalResult> {
    let missing_set: HashSet<&str> = missing_expected_ids.iter().map(String::as_str).collect();

    // Filter expected IDs to exclude missing ones
    let valid_expected_ids: Vec<&String> = case
        .expected_ids
        .iter()
        .filter(|id| !missing_set.contains(id.as_str()))
        .collect();

    let expected: HashSet<&str> = valid_expected_ids.iter().map(|s| s.as_str()).collect();
    let retrieved: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
    let retrieved_set: HashSet<&str> = retrieved.iter().map(String::as_str).collect();

    // Compute hits and misses (using valid expected only)
    let hits: Vec<String> = retrieved
        .iter()
        .filter(|id| expected.contains(id.as_str()))
        .cloned()
        .collect();

    let misses: Vec<String> = valid_expected_ids
        .iter()
        .filter(|id| !retrieved_set.contains(id.as_str()))
        .map(|s| (*s).clone())
        .collect();

    // Precision@k
    let precision = if retrieved.is_empty() {
        0.0
    } else {
        hits.len() as f64 / retrieved.len() as f64
    };

    // Recall@k - uses valid expected count (excludes missing)
    let recall = if valid_expected_ids.is_empty() {
        1.0 // No expected = perfect recall vacuously
    } else {
        hits.len() as f64 / valid_expected_ids.len() as f64
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
        lane: case.lane,
        source: case.source,
        missing_expected_ids,
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
            total_missing_ids: 0,
        });
    }

    let sum_precision: f64 = results.iter().map(|r| r.precision_at_k).sum();
    let sum_recall: f64 = results.iter().map(|r| r.recall_at_k).sum();
    let sum_rr: f64 = results.iter().map(|r| r.reciprocal_rank).sum();

    let cases_passed = results.iter().filter(|r| r.passes(0.5, 0.5)).count();

    // P86: Count total missing IDs across all cases
    let total_missing_ids: usize = results.iter().map(|r| r.missing_expected_ids.len()).sum();

    Ok(EvalSuiteResult {
        mean_precision: sum_precision / total_cases as f64,
        mean_recall: sum_recall / total_cases as f64,
        mrr: sum_rr / total_cases as f64,
        cases_passed,
        total_cases,
        top_k,
        total_missing_ids,
        results,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Built-in Eval Cases
// ─────────────────────────────────────────────────────────────────────────────

/// Built-in evaluation cases for unit tests
///
/// These provide a stable baseline for CI testing without external dependencies.
/// P86: Now includes both Memory and Code lane cases.
pub fn built_in_eval_cases() -> Vec<EvalCase> {
    vec![
        // ─────────────────────────────────────────────────────────────────────
        // Memory Lane Cases (existing)
        // ─────────────────────────────────────────────────────────────────────
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
        // ─────────────────────────────────────────────────────────────────────
        // P86: Code Lane Cases
        // ─────────────────────────────────────────────────────────────────────
        EvalCase::new_code(
            "code-handle-spec-auto",
            "handle_spec_auto speckit auto command implementation",
            vec![
                "code:tui/src/chatwidget/spec_kit/pipeline_coordinator.rs::handle_spec_auto"
                    .to_string(),
            ],
        )
        .with_description("Tests retrieval of handle_spec_auto function for /speckit.auto"),
        EvalCase::new_code(
            "code-spec-auto-state",
            "SpecAutoState pipeline state machine struct",
            vec![
                "code:tui/src/chatwidget/spec_kit/pipeline_coordinator.rs::SpecAutoState"
                    .to_string(),
            ],
        )
        .with_description("Tests retrieval of SpecAutoState struct definition"),
        EvalCase::new_code(
            "code-dcc-compile-context",
            "compile_context DCC TASK_BRIEF generation function",
            vec!["code:stage0/src/dcc.rs::compile_context".to_string()],
        )
        .with_description("Tests retrieval of DCC compile_context function"),
    ]
}

/// P86: Get built-in code lane cases only
pub fn built_in_code_eval_cases() -> Vec<EvalCase> {
    built_in_eval_cases()
        .into_iter()
        .filter(|c| c.lane == EvalLane::Code)
        .collect()
}

/// P86: Get built-in memory lane cases only
pub fn built_in_memory_eval_cases() -> Vec<EvalCase> {
    built_in_eval_cases()
        .into_iter()
        .filter(|c| c.lane == EvalLane::Memory)
        .collect()
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
///
/// Cases loaded from files are automatically marked with `source: External`
pub fn load_eval_cases_from_file(path: &Path) -> Result<Vec<EvalCase>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Stage0Error::config_with_source(
            format!("Failed to read eval cases file: {}", path.display()),
            e,
        )
    })?;

    let mut cases: Vec<EvalCase> = serde_json::from_str(&content)
        .map_err(|e| Stage0Error::config_with_source("Failed to parse eval cases JSON", e))?;

    // P86: Mark all loaded cases as external
    for case in &mut cases {
        case.source = EvalCaseSource::External;
    }

    Ok(cases)
}

/// Save eval cases to a JSON file
pub fn save_eval_cases_to_file(cases: &[EvalCase], path: &Path) -> Result<()> {
    let content = serde_json::to_string_pretty(cases)
        .map_err(|e| Stage0Error::internal(format!("Failed to serialize eval cases: {e}")))?;

    std::fs::write(path, content).map_err(|e| {
        Stage0Error::config_with_source(
            format!("Failed to write eval cases file: {}", path.display()),
            e,
        )
    })?;

    Ok(())
}

/// P86: Combine built-in cases with optional external cases
///
/// Merges built-in eval cases with cases loaded from an external file.
/// External cases override built-in cases with the same name.
pub fn combined_eval_cases(
    use_builtins: bool,
    external_path: Option<&Path>,
    lane_filter: Option<EvalLane>,
) -> Result<Vec<EvalCase>> {
    let mut cases = if use_builtins {
        built_in_eval_cases()
    } else {
        Vec::new()
    };

    // Add external cases if provided
    if let Some(path) = external_path {
        let external = load_eval_cases_from_file(path)?;

        // Build set of external case names for deduplication
        let external_names: HashSet<&str> = external.iter().map(|c| c.name.as_str()).collect();

        // Remove built-in cases that are overridden by external
        cases.retain(|c| !external_names.contains(c.name.as_str()));

        // Add external cases
        cases.extend(external);
    }

    // Filter by lane if specified
    if let Some(lane) = lane_filter {
        cases.retain(|c| c.lane == lane);
    }

    Ok(cases)
}

// ─────────────────────────────────────────────────────────────────────────────
// P86: DCC Evaluation Functions
// ─────────────────────────────────────────────────────────────────────────────

/// P86: Evaluate a code lane case against CompileContextResult
///
/// Extracts code candidate IDs from the DCC result and computes metrics.
pub fn evaluate_dcc_code_result(
    case: &EvalCase,
    result: &CompileContextResult,
) -> Result<EvalResult> {
    if case.lane != EvalLane::Code {
        return Err(Stage0Error::internal(format!(
            "evaluate_dcc_code_result called with non-code case: {}",
            case.name
        )));
    }

    // Extract code candidate IDs
    let retrieved: Vec<ScoredVector> = result
        .code_candidates
        .iter()
        .map(|cc| ScoredVector {
            id: cc.id.clone(),
            score: cc.score,
            kind: crate::vector::DocumentKind::Code,
            metadata: crate::vector::DocumentMetadata::default(),
        })
        .collect();

    compute_metrics(case, &retrieved)
}

/// P86: Evaluate a memory lane case against CompileContextResult
///
/// Extracts memory IDs from the DCC result and computes metrics.
pub fn evaluate_dcc_memory_result(
    case: &EvalCase,
    result: &CompileContextResult,
    explain_scores: Option<&crate::dcc::ExplainScores>,
) -> Result<EvalResult> {
    if case.lane != EvalLane::Memory {
        return Err(Stage0Error::internal(format!(
            "evaluate_dcc_memory_result called with non-memory case: {}",
            case.name
        )));
    }

    // Extract memory IDs with scores from explain_scores if available
    let retrieved: Vec<ScoredVector> = if let Some(scores) = explain_scores {
        scores
            .memories
            .iter()
            .filter(|m| result.memories_used.contains(&m.id))
            .map(|m| ScoredVector {
                id: m.id.clone(),
                score: m.combined_score,
                kind: crate::vector::DocumentKind::Memory,
                metadata: crate::vector::DocumentMetadata::default(),
            })
            .collect()
    } else {
        // Fallback: use memories_used with uniform scores
        result
            .memories_used
            .iter()
            .enumerate()
            .map(|(i, id)| ScoredVector {
                id: id.clone(),
                score: 1.0 - (i as f64 * 0.01), // Decreasing scores by order
                kind: crate::vector::DocumentKind::Memory,
                metadata: crate::vector::DocumentMetadata::default(),
            })
            .collect()
    };

    compute_metrics(case, &retrieved)
}

/// P86: Evaluate all cases against a CompileContextResult
///
/// Routes each case to the appropriate lane handler (code or memory).
pub fn evaluate_dcc_results(
    cases: &[EvalCase],
    result: &CompileContextResult,
    top_k: usize,
) -> Result<EvalSuiteResult> {
    let mut results = Vec::with_capacity(cases.len());

    for case in cases {
        let eval_result = match case.lane {
            EvalLane::Code => evaluate_dcc_code_result(case, result)?,
            EvalLane::Memory => {
                evaluate_dcc_memory_result(case, result, result.explain_scores.as_ref())?
            }
        };
        results.push(eval_result);
    }

    compute_suite_metrics(results, top_k)
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
                lane: EvalLane::Memory,
                source: EvalCaseSource::Builtin,
                missing_expected_ids: vec![],
            },
            EvalResult {
                case_name: "case2".to_string(),
                precision_at_k: 0.5,
                recall_at_k: 0.5,
                reciprocal_rank: 0.5,
                hits: vec!["b".to_string()],
                misses: vec!["c".to_string()],
                retrieved: vec!["b".to_string(), "x".to_string()],
                lane: EvalLane::Memory,
                source: EvalCaseSource::Builtin,
                missing_expected_ids: vec![],
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
            lane: EvalLane::Memory,
            source: EvalCaseSource::Builtin,
            missing_expected_ids: vec![],
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

        // Run evaluation with memory cases only (test docs don't include code: IDs)
        let cases = built_in_memory_eval_cases();
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

        // Check all expected IDs from MEMORY cases are present
        // (Code cases expect code: prefixed IDs from code index, not test documents)
        let doc_ids: std::collections::HashSet<&str> = docs.iter().map(|d| d.id.as_str()).collect();

        let memory_cases = built_in_memory_eval_cases();
        for case in &memory_cases {
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
            lane: EvalLane::Memory,
            source: EvalCaseSource::Builtin,
            missing_expected_ids: vec![],
        }];

        let suite = compute_suite_metrics(results, 10).unwrap();
        let table = suite.format_table();

        assert!(table.contains("test-case-with-long-name"));
        assert!(table.contains("0.85"));
        assert!(table.contains("0.75"));
        assert!(table.contains("hit1"));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // P86: Code Lane Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_eval_lane_display() {
        assert_eq!(EvalLane::Memory.as_str(), "memory");
        assert_eq!(EvalLane::Code.as_str(), "code");
        assert_eq!(EvalLane::Memory.to_string(), "memory");
        assert_eq!(EvalLane::Code.to_string(), "code");
    }

    #[test]
    fn test_eval_case_source_display() {
        assert_eq!(EvalCaseSource::Builtin.as_str(), "builtin");
        assert_eq!(EvalCaseSource::External.as_str(), "external");
    }

    #[test]
    fn test_eval_case_new_code() {
        let case = EvalCase::new_code(
            "code-test",
            "test query",
            vec!["code:path::symbol".to_string()],
        );

        assert_eq!(case.name, "code-test");
        assert_eq!(case.lane, EvalLane::Code);
        assert_eq!(case.source, EvalCaseSource::Builtin);
    }

    #[test]
    fn test_eval_case_with_lane() {
        let case = EvalCase::new("test", "query", vec!["id".to_string()]).with_lane(EvalLane::Code);

        assert_eq!(case.lane, EvalLane::Code);
    }

    #[test]
    fn test_eval_case_mark_external() {
        let case = EvalCase::new("test", "query", vec!["id".to_string()]).mark_external();

        assert_eq!(case.source, EvalCaseSource::External);
    }

    #[test]
    fn test_compute_metrics_with_missing_ids() {
        let case = EvalCase::new(
            "test",
            "query",
            vec!["a".to_string(), "b".to_string(), "missing".to_string()],
        );

        let results = vec![
            ScoredVector::new("a", 0.9, crate::vector::DocumentKind::Memory),
            ScoredVector::new("b", 0.8, crate::vector::DocumentKind::Memory),
        ];

        // Without missing ID handling: recall = 2/3
        let metrics_no_missing = compute_metrics(&case, &results).unwrap();
        assert!((metrics_no_missing.recall_at_k - 2.0 / 3.0).abs() < 0.01);

        // With missing ID handling: recall = 2/2 = 1.0
        let missing = vec!["missing".to_string()];
        let metrics_with_missing = compute_metrics_with_missing(&case, &results, missing).unwrap();
        assert!((metrics_with_missing.recall_at_k - 1.0).abs() < f64::EPSILON);
        assert_eq!(metrics_with_missing.missing_expected_ids.len(), 1);
    }

    #[test]
    fn test_mrr_calculation() {
        // First relevant at position 0 → MRR = 1.0
        let case = EvalCase::new("test", "query", vec!["a".to_string()]);
        let results = vec![
            ScoredVector::new("a", 0.9, crate::vector::DocumentKind::Memory),
            ScoredVector::new("b", 0.8, crate::vector::DocumentKind::Memory),
        ];
        let metrics = compute_metrics(&case, &results).unwrap();
        assert!((metrics.reciprocal_rank - 1.0).abs() < f64::EPSILON);

        // First relevant at position 1 → MRR = 0.5
        let results2 = vec![
            ScoredVector::new("x", 0.9, crate::vector::DocumentKind::Memory),
            ScoredVector::new("a", 0.8, crate::vector::DocumentKind::Memory),
        ];
        let metrics2 = compute_metrics(&case, &results2).unwrap();
        assert!((metrics2.reciprocal_rank - 0.5).abs() < f64::EPSILON);

        // First relevant at position 2 → MRR = 0.333...
        let results3 = vec![
            ScoredVector::new("x", 0.9, crate::vector::DocumentKind::Memory),
            ScoredVector::new("y", 0.8, crate::vector::DocumentKind::Memory),
            ScoredVector::new("a", 0.7, crate::vector::DocumentKind::Memory),
        ];
        let metrics3 = compute_metrics(&case, &results3).unwrap();
        assert!((metrics3.reciprocal_rank - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_eval_suite_result_has_missing_ids() {
        let result_with_missing = EvalResult {
            case_name: "test".to_string(),
            precision_at_k: 1.0,
            recall_at_k: 1.0,
            reciprocal_rank: 1.0,
            hits: vec![],
            misses: vec![],
            retrieved: vec![],
            lane: EvalLane::Memory,
            source: EvalCaseSource::Builtin,
            missing_expected_ids: vec!["missing-id".to_string()],
        };

        let suite = compute_suite_metrics(vec![result_with_missing], 10).unwrap();
        assert!(suite.has_missing_ids());
        assert_eq!(suite.total_missing_ids, 1);
    }

    #[test]
    fn test_eval_suite_result_filter_by_lane() {
        let memory_result = EvalResult {
            case_name: "memory-test".to_string(),
            precision_at_k: 1.0,
            recall_at_k: 1.0,
            reciprocal_rank: 1.0,
            hits: vec![],
            misses: vec![],
            retrieved: vec![],
            lane: EvalLane::Memory,
            source: EvalCaseSource::Builtin,
            missing_expected_ids: vec![],
        };

        let code_result = EvalResult {
            case_name: "code-test".to_string(),
            precision_at_k: 0.8,
            recall_at_k: 0.8,
            reciprocal_rank: 0.5,
            hits: vec![],
            misses: vec![],
            retrieved: vec![],
            lane: EvalLane::Code,
            source: EvalCaseSource::Builtin,
            missing_expected_ids: vec![],
        };

        let suite = compute_suite_metrics(vec![memory_result, code_result], 10).unwrap();

        let memory_results = suite.filter_by_lane(EvalLane::Memory);
        assert_eq!(memory_results.len(), 1);
        assert_eq!(memory_results[0].case_name, "memory-test");

        let code_results = suite.filter_by_lane(EvalLane::Code);
        assert_eq!(code_results.len(), 1);
        assert_eq!(code_results[0].case_name, "code-test");
    }

    #[test]
    fn test_builtin_cases_include_code_lane() {
        let cases = built_in_eval_cases();

        let memory_cases: Vec<_> = cases
            .iter()
            .filter(|c| c.lane == EvalLane::Memory)
            .collect();
        let code_cases: Vec<_> = cases.iter().filter(|c| c.lane == EvalLane::Code).collect();

        // Should have both memory and code cases
        assert!(!memory_cases.is_empty(), "Should have memory lane cases");
        assert!(!code_cases.is_empty(), "Should have code lane cases");

        // Verify code cases have expected ID format
        for case in &code_cases {
            for id in &case.expected_ids {
                assert!(
                    id.starts_with("code:"),
                    "Code lane case should have code: prefixed IDs: {}",
                    id
                );
            }
        }
    }

    #[test]
    fn test_builtin_code_eval_cases() {
        let code_cases = built_in_code_eval_cases();
        assert!(!code_cases.is_empty());

        for case in &code_cases {
            assert_eq!(case.lane, EvalLane::Code);
        }
    }

    #[test]
    fn test_builtin_memory_eval_cases() {
        let memory_cases = built_in_memory_eval_cases();
        assert!(!memory_cases.is_empty());

        for case in &memory_cases {
            assert_eq!(case.lane, EvalLane::Memory);
        }
    }

    #[test]
    fn test_format_table_with_lanes() {
        let memory_result = EvalResult {
            case_name: "memory-case".to_string(),
            precision_at_k: 0.9,
            recall_at_k: 0.8,
            reciprocal_rank: 1.0,
            hits: vec!["hit1".to_string()],
            misses: vec![],
            retrieved: vec!["hit1".to_string()],
            lane: EvalLane::Memory,
            source: EvalCaseSource::Builtin,
            missing_expected_ids: vec![],
        };

        let code_result = EvalResult {
            case_name: "code-case".to_string(),
            precision_at_k: 0.7,
            recall_at_k: 0.6,
            reciprocal_rank: 0.5,
            hits: vec!["code-hit".to_string()],
            misses: vec!["code-miss".to_string()],
            retrieved: vec!["code-hit".to_string()],
            lane: EvalLane::Code,
            source: EvalCaseSource::External,
            missing_expected_ids: vec![],
        };

        let suite = compute_suite_metrics(vec![memory_result, code_result], 10).unwrap();
        let table = suite.format_table_with_lanes();

        // Check lane column
        assert!(table.contains("memory"));
        assert!(table.contains("code"));

        // Check source column
        assert!(table.contains("builtin"));
        assert!(table.contains("external"));
    }

    #[tokio::test]
    async fn test_evaluate_backend_with_memory_cases_only() {
        let backend = TfIdfBackend::new();

        // Index test documents
        let docs = built_in_test_documents();
        backend.index_documents(docs).await.unwrap();

        // Run evaluation with memory cases only
        let cases = built_in_memory_eval_cases();
        let suite = evaluate_backend(&backend, &cases, &VectorFilters::new(), 10)
            .await
            .unwrap();

        // All results should be memory lane
        for result in &suite.results {
            assert_eq!(result.lane, EvalLane::Memory);
        }
    }
}
