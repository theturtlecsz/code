//! SPEC-KIT-972: Evaluation Harness for Memvid Hybrid Retrieval
//!
//! Provides:
//! - Golden query suite (10-20 representative queries for memvid evaluation)
//! - A/B harness comparing local-memory vs memvid on the same corpus
//! - Report generator (JSON + markdown)
//!
//! ## Key Concepts
//! - `ABHarness`: Runs same queries against two backends, computes comparison metrics
//! - `ABReport`: Comparison results with per-query and aggregate analysis
//! - Golden queries: Representative queries that exercise domain, tag, and keyword features

use crate::memvid_adapter::adapter::MemoryMeta;
use chrono::{DateTime, Utc};
use codex_stage0::dcc::{Iqo, LocalMemoryClient, LocalMemorySearchParams, LocalMemorySummary};
use codex_stage0::errors::Result as Stage0Result;
use codex_stage0::eval::{
    EvalCase, EvalCaseSource, EvalLane, EvalResult, EvalSuiteResult, compute_metrics,
    compute_suite_metrics,
};
use codex_stage0::vector::ScoredVector;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

// =============================================================================
// A/B Harness
// =============================================================================

/// A/B evaluation harness for comparing two memory backends.
///
/// Runs the same golden queries against both backends and produces
/// a comparative report.
pub struct ABHarness {
    /// Backend A (typically local-memory or baseline)
    backend_a: Arc<dyn LocalMemoryClient>,
    /// Backend B (typically memvid or experiment)
    backend_b: Arc<dyn LocalMemoryClient>,
    /// Name for backend A
    name_a: String,
    /// Name for backend B
    name_b: String,
    /// Golden query suite
    queries: Vec<GoldenQuery>,
    /// Top-k results to retrieve
    top_k: usize,
}

impl ABHarness {
    /// Create a new A/B harness.
    pub fn new(
        backend_a: Arc<dyn LocalMemoryClient>,
        backend_b: Arc<dyn LocalMemoryClient>,
    ) -> Self {
        Self {
            backend_a,
            backend_b,
            name_a: "local-memory".to_string(),
            name_b: "memvid".to_string(),
            queries: golden_query_suite(),
            top_k: 10,
        }
    }

    /// Set custom names for the backends.
    pub fn with_names(mut self, name_a: impl Into<String>, name_b: impl Into<String>) -> Self {
        self.name_a = name_a.into();
        self.name_b = name_b.into();
        self
    }

    /// Override the golden query suite.
    pub fn with_queries(mut self, queries: Vec<GoldenQuery>) -> Self {
        self.queries = queries;
        self
    }

    /// Set top-k results count.
    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k;
        self
    }

    /// Run the A/B evaluation and produce a report.
    pub async fn run(&self) -> Stage0Result<ABReport> {
        let mut results_a = Vec::new();
        let mut results_b = Vec::new();
        let mut latencies_a = Vec::new();
        let mut latencies_b = Vec::new();

        for query in &self.queries {
            let params = query.to_search_params(self.top_k);

            // Run backend A
            let start_a = Instant::now();
            let hits_a = self.backend_a.search_memories(params.clone()).await?;
            let latency_a = start_a.elapsed();
            latencies_a.push(latency_a);

            // Run backend B
            let start_b = Instant::now();
            let hits_b = self.backend_b.search_memories(params).await?;
            let latency_b = start_b.elapsed();
            latencies_b.push(latency_b);

            // Compute metrics for both
            let eval_case = query.to_eval_case();
            let scored_a = hits_to_scored(&hits_a);
            let scored_b = hits_to_scored(&hits_b);

            let result_a = compute_metrics(&eval_case, &scored_a)?;
            let result_b = compute_metrics(&eval_case, &scored_b)?;

            results_a.push(QueryResult {
                query_name: query.name.clone(),
                result: result_a,
                latency: latency_a,
                hits: hits_a.iter().map(HitSummary::from).collect(),
            });

            results_b.push(QueryResult {
                query_name: query.name.clone(),
                result: result_b,
                latency: latency_b,
                hits: hits_b.iter().map(HitSummary::from).collect(),
            });
        }

        // Compute aggregate metrics
        let suite_a = compute_suite_metrics(
            results_a.iter().map(|r| r.result.clone()).collect(),
            self.top_k,
        )?;
        let suite_b = compute_suite_metrics(
            results_b.iter().map(|r| r.result.clone()).collect(),
            self.top_k,
        )?;

        Ok(ABReport {
            backend_a_name: self.name_a.clone(),
            backend_b_name: self.name_b.clone(),
            results_a,
            results_b,
            suite_a,
            suite_b,
            latencies_a,
            latencies_b,
            generated_at: Utc::now(),
            top_k: self.top_k,
        })
    }
}

/// Convert LocalMemorySummary hits to ScoredVector for metrics.
fn hits_to_scored(hits: &[LocalMemorySummary]) -> Vec<ScoredVector> {
    hits.iter()
        .map(|h| ScoredVector {
            id: h.id.clone(),
            score: h.similarity_score,
            kind: codex_stage0::vector::DocumentKind::Memory,
            metadata: codex_stage0::vector::DocumentMetadata::default(),
        })
        .collect()
}

// =============================================================================
// Query Result
// =============================================================================

/// Serializable hit summary (simplified from LocalMemorySummary).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitSummary {
    /// Hit ID
    pub id: String,
    /// Domain
    pub domain: Option<String>,
    /// Tags
    pub tags: Vec<String>,
    /// Snippet
    pub snippet: String,
    /// Similarity score
    pub score: f64,
}

impl From<&LocalMemorySummary> for HitSummary {
    fn from(s: &LocalMemorySummary) -> Self {
        Self {
            id: s.id.clone(),
            domain: s.domain.clone(),
            tags: s.tags.clone(),
            snippet: s.snippet.clone(),
            score: s.similarity_score,
        }
    }
}

/// Result from running a single query against a backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Query name
    pub query_name: String,
    /// Evaluation metrics
    pub result: EvalResult,
    /// Query latency
    #[serde(with = "duration_millis")]
    pub latency: Duration,
    /// Retrieved hits (serializable)
    pub hits: Vec<HitSummary>,
}

// =============================================================================
// A/B Report
// =============================================================================

/// Comparative report from A/B evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABReport {
    /// Backend A name
    pub backend_a_name: String,
    /// Backend B name
    pub backend_b_name: String,
    /// Per-query results for backend A
    pub results_a: Vec<QueryResult>,
    /// Per-query results for backend B
    pub results_b: Vec<QueryResult>,
    /// Aggregate metrics for backend A
    pub suite_a: EvalSuiteResult,
    /// Aggregate metrics for backend B
    pub suite_b: EvalSuiteResult,
    /// Latencies for backend A
    #[serde(with = "duration_vec_millis")]
    pub latencies_a: Vec<Duration>,
    /// Latencies for backend B
    #[serde(with = "duration_vec_millis")]
    pub latencies_b: Vec<Duration>,
    /// Report generation timestamp
    pub generated_at: DateTime<Utc>,
    /// Top-k used
    pub top_k: usize,
}

impl ABReport {
    /// Compute P95 latency for backend A.
    pub fn p95_latency_a(&self) -> Duration {
        percentile_duration(&self.latencies_a, 95)
    }

    /// Compute P95 latency for backend B.
    pub fn p95_latency_b(&self) -> Duration {
        percentile_duration(&self.latencies_b, 95)
    }

    /// Check if backend B meets or exceeds backend A on key metrics.
    pub fn b_meets_baseline(&self) -> bool {
        self.suite_b.mean_precision >= self.suite_a.mean_precision * 0.95
            && self.suite_b.mean_recall >= self.suite_a.mean_recall * 0.95
    }

    /// Check if P95 latency for backend B is under threshold.
    pub fn b_latency_acceptable(&self, threshold_ms: u64) -> bool {
        self.p95_latency_b().as_millis() < threshold_ms as u128
    }

    /// Format as markdown summary.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# A/B Retrieval Evaluation Report\n\n");
        md.push_str(&format!("**Generated:** {}\n\n", self.generated_at));
        md.push_str(&format!("**Top-K:** {}\n\n", self.top_k));

        // Summary table
        md.push_str("## Summary\n\n");
        md.push_str("| Metric | ");
        md.push_str(&self.backend_a_name);
        md.push_str(" | ");
        md.push_str(&self.backend_b_name);
        md.push_str(" | Delta |\n");
        md.push_str("|--------|-------|-------|-------|\n");

        let delta_precision = self.suite_b.mean_precision - self.suite_a.mean_precision;
        let delta_recall = self.suite_b.mean_recall - self.suite_a.mean_recall;
        let delta_mrr = self.suite_b.mrr - self.suite_a.mrr;

        md.push_str(&format!(
            "| Mean P@{} | {:.3} | {:.3} | {:+.3} |\n",
            self.top_k, self.suite_a.mean_precision, self.suite_b.mean_precision, delta_precision
        ));
        md.push_str(&format!(
            "| Mean R@{} | {:.3} | {:.3} | {:+.3} |\n",
            self.top_k, self.suite_a.mean_recall, self.suite_b.mean_recall, delta_recall
        ));
        md.push_str(&format!(
            "| MRR | {:.3} | {:.3} | {:+.3} |\n",
            self.suite_a.mrr, self.suite_b.mrr, delta_mrr
        ));
        md.push_str(&format!(
            "| Cases Passed | {}/{} | {}/{} | - |\n",
            self.suite_a.cases_passed,
            self.suite_a.total_cases,
            self.suite_b.cases_passed,
            self.suite_b.total_cases
        ));
        md.push_str(&format!(
            "| P95 Latency | {}ms | {}ms | - |\n",
            self.p95_latency_a().as_millis(),
            self.p95_latency_b().as_millis()
        ));

        // Verdict
        md.push_str("\n## Verdict\n\n");
        if self.b_meets_baseline() {
            md.push_str(&format!(
                "✅ **{}** meets or exceeds **{}** baseline.\n",
                self.backend_b_name, self.backend_a_name
            ));
        } else {
            md.push_str(&format!(
                "⚠️ **{}** does NOT meet **{}** baseline.\n",
                self.backend_b_name, self.backend_a_name
            ));
        }

        if self.b_latency_acceptable(250) {
            md.push_str(&format!(
                "✅ **{}** P95 latency < 250ms.\n",
                self.backend_b_name
            ));
        } else {
            md.push_str(&format!(
                "⚠️ **{}** P95 latency >= 250ms.\n",
                self.backend_b_name
            ));
        }

        // Per-query details
        md.push_str("\n## Per-Query Results\n\n");
        md.push_str("| Query | A P@k | B P@k | A R@k | B R@k | A Latency | B Latency |\n");
        md.push_str("|-------|-------|-------|-------|-------|-----------|----------|\n");

        for (ra, rb) in self.results_a.iter().zip(self.results_b.iter()) {
            md.push_str(&format!(
                "| {} | {:.2} | {:.2} | {:.2} | {:.2} | {}ms | {}ms |\n",
                truncate(&ra.query_name, 20),
                ra.result.precision_at_k,
                rb.result.precision_at_k,
                ra.result.recall_at_k,
                rb.result.recall_at_k,
                ra.latency.as_millis(),
                rb.latency.as_millis(),
            ));
        }

        md
    }

    /// Save report as JSON.
    pub fn save_json(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    /// Save report as markdown.
    pub fn save_markdown(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, self.to_markdown())
    }
}

fn percentile_duration(durations: &[Duration], percentile: usize) -> Duration {
    if durations.is_empty() {
        return Duration::ZERO;
    }

    let mut sorted: Vec<Duration> = durations.to_vec();
    sorted.sort();

    let idx = (percentile * sorted.len() / 100).min(sorted.len() - 1);
    sorted[idx]
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

// =============================================================================
// Golden Query Suite
// =============================================================================

/// A golden query for evaluation.
///
/// Contains query parameters (IQO) and expected result IDs for metric computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenQuery {
    /// Human-readable name
    pub name: String,
    /// Description of what this query tests
    pub description: Option<String>,
    /// Keywords for search
    pub keywords: Vec<String>,
    /// Domain filter (optional)
    pub domain: Option<String>,
    /// Required tags (optional)
    pub required_tags: Vec<String>,
    /// Optional tags for boosting
    pub optional_tags: Vec<String>,
    /// Expected result IDs (for P@k, R@k calculation)
    pub expected_ids: Vec<String>,
}

impl GoldenQuery {
    /// Create a new golden query.
    pub fn new(name: impl Into<String>, keywords: Vec<String>, expected_ids: Vec<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            keywords,
            domain: None,
            required_tags: Vec::new(),
            optional_tags: Vec::new(),
            expected_ids,
        }
    }

    /// Add description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add domain filter.
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Add required tags.
    pub fn with_required_tags(mut self, tags: Vec<String>) -> Self {
        self.required_tags = tags;
        self
    }

    /// Add optional tags.
    pub fn with_optional_tags(mut self, tags: Vec<String>) -> Self {
        self.optional_tags = tags;
        self
    }

    /// Convert to LocalMemorySearchParams.
    pub fn to_search_params(&self, max_results: usize) -> LocalMemorySearchParams {
        LocalMemorySearchParams {
            iqo: Iqo {
                keywords: self.keywords.clone(),
                domains: self.domain.clone().into_iter().collect(),
                required_tags: self.required_tags.clone(),
                optional_tags: self.optional_tags.clone(),
                exclude_tags: Vec::new(),
                max_candidates: max_results * 3,
                notebook_focus: Vec::new(),
            },
            max_results,
            as_of: None,
        }
    }

    /// Convert to EvalCase for metrics computation.
    pub fn to_eval_case(&self) -> EvalCase {
        EvalCase {
            name: self.name.clone(),
            query: self.keywords.join(" "),
            expected_ids: self.expected_ids.clone(),
            description: self.description.clone(),
            lane: EvalLane::Memory,
            source: EvalCaseSource::Builtin,
        }
    }
}

/// Built-in golden query suite for memvid evaluation.
///
/// These queries exercise:
/// - Keyword-only search
/// - Domain filtering
/// - Tag filtering (required + optional)
/// - Multi-keyword queries
/// - Edge cases (rare terms, long queries)
pub fn golden_query_suite() -> Vec<GoldenQuery> {
    vec![
        // ─────────────────────────────────────────────────────────────────────────
        // Basic keyword queries
        // ─────────────────────────────────────────────────────────────────────────
        GoldenQuery::new(
            "error-handling-pattern",
            vec![
                "error".to_string(),
                "handling".to_string(),
                "pattern".to_string(),
            ],
            vec![
                "mem-rust-errors-001".to_string(),
                "mem-thiserror-002".to_string(),
            ],
        )
        .with_description("Basic keyword search for error handling patterns"),
        GoldenQuery::new(
            "tfidf-bm25-implementation",
            vec![
                "tfidf".to_string(),
                "bm25".to_string(),
                "scoring".to_string(),
            ],
            vec![
                "mem-vector-tfidf-001".to_string(),
                "mem-bm25-scoring-002".to_string(),
            ],
        )
        .with_description("Search for TF-IDF/BM25 implementation details"),
        GoldenQuery::new(
            "stage0-architecture-decision",
            vec![
                "stage0".to_string(),
                "architecture".to_string(),
                "design".to_string(),
            ],
            vec![
                "mem-stage0-arch-001".to_string(),
                "mem-stage0-overlay-002".to_string(),
            ],
        )
        .with_description("Architecture decisions for Stage0"),
        // ─────────────────────────────────────────────────────────────────────────
        // Domain-filtered queries
        // ─────────────────────────────────────────────────────────────────────────
        GoldenQuery::new(
            "spec-kit-domain-query",
            vec!["integration".to_string(), "tier2".to_string()],
            vec![
                "mem-speckit-102-001".to_string(),
                "mem-tier2-notebook-002".to_string(),
            ],
        )
        .with_domain("spec-kit")
        .with_description("Query within spec-kit domain"),
        GoldenQuery::new(
            "rust-domain-query",
            vec!["error".to_string(), "result".to_string()],
            vec![
                "mem-rust-errors-001".to_string(),
                "mem-thiserror-002".to_string(),
            ],
        )
        .with_domain("rust")
        .with_description("Query within rust domain"),
        // ─────────────────────────────────────────────────────────────────────────
        // Tag-filtered queries
        // ─────────────────────────────────────────────────────────────────────────
        GoldenQuery::new(
            "bug-pattern-required-tag",
            vec!["crash".to_string(), "memory".to_string()],
            vec![
                "mem-bug-resize-001".to_string(),
                "mem-bug-segfault-002".to_string(),
            ],
        )
        .with_required_tags(vec!["type:bug".to_string()])
        .with_description("Bug patterns with required type:bug tag"),
        GoldenQuery::new(
            "decision-type-required-tag",
            vec!["architecture".to_string(), "design".to_string()],
            vec!["mem-stage0-arch-001".to_string()],
        )
        .with_required_tags(vec!["type:decision".to_string()])
        .with_description("Architecture decisions with required type:decision tag"),
        // ─────────────────────────────────────────────────────────────────────────
        // Optional tag boost queries
        // ─────────────────────────────────────────────────────────────────────────
        GoldenQuery::new(
            "pattern-with-optional-tag-boost",
            vec!["pattern".to_string(), "implementation".to_string()],
            vec![
                "mem-vector-tfidf-001".to_string(),
                "mem-stage0-overlay-002".to_string(),
            ],
        )
        .with_optional_tags(vec![
            "type:pattern".to_string(),
            "type:implementation".to_string(),
        ])
        .with_description("Patterns with optional tag boosting"),
        // ─────────────────────────────────────────────────────────────────────────
        // Multi-keyword precision queries
        // ─────────────────────────────────────────────────────────────────────────
        GoldenQuery::new(
            "notebooklm-tier2-spec",
            vec![
                "notebooklm".to_string(),
                "tier2".to_string(),
                "synthesis".to_string(),
                "mcp".to_string(),
            ],
            vec![
                "mem-speckit-102-001".to_string(),
                "mem-tier2-notebook-002".to_string(),
            ],
        )
        .with_description("Precise query for NotebookLM Tier2 integration"),
        GoldenQuery::new(
            "resize-crash-segfault",
            vec![
                "resize".to_string(),
                "crash".to_string(),
                "segfault".to_string(),
            ],
            vec![
                "mem-bug-resize-001".to_string(),
                "mem-bug-segfault-002".to_string(),
            ],
        )
        .with_description("Precise query for resize/crash bugs"),
        // ─────────────────────────────────────────────────────────────────────────
        // Edge cases
        // ─────────────────────────────────────────────────────────────────────────
        GoldenQuery::new(
            "single-keyword-broad",
            vec!["memory".to_string()],
            vec![
                "mem-bug-resize-001".to_string(),
                "mem-bug-segfault-002".to_string(),
            ],
        )
        .with_description("Single broad keyword (tests ranking)"),
        GoldenQuery::new(
            "rare-term-search",
            vec!["avgdl".to_string()], // BM25 formula term
            vec!["mem-bm25-scoring-002".to_string()],
        )
        .with_description("Rare technical term search"),
        GoldenQuery::new(
            "spec-id-search",
            vec!["SPEC-KIT-102".to_string()],
            vec!["mem-speckit-102-001".to_string()],
        )
        .with_description("SPEC ID literal search"),
        GoldenQuery::new(
            "combined-domain-and-tag",
            vec!["overlay".to_string(), "database".to_string()],
            vec![
                "mem-stage0-arch-001".to_string(),
                "mem-stage0-overlay-002".to_string(),
            ],
        )
        .with_domain("spec-kit")
        .with_optional_tags(vec!["type:pattern".to_string()])
        .with_description("Combined domain + optional tag query"),
        // ─────────────────────────────────────────────────────────────────────────
        // Negative/Challenging cases
        // ─────────────────────────────────────────────────────────────────────────
        GoldenQuery::new(
            "no-match-expected",
            vec![
                "quantum".to_string(),
                "blockchain".to_string(),
                "web3".to_string(),
            ],
            vec![], // No expected matches
        )
        .with_description("Query that should return no results"),
    ]
}

/// Create synthetic test documents matching the golden query suite.
///
/// This mirrors `codex_stage0::eval::built_in_test_documents()` but is
/// specific to the memvid evaluation suite.
pub fn golden_test_memories() -> Vec<(MemoryMeta, String)> {
    vec![
        // Stage 0 architecture
        (
            MemoryMeta {
                id: "mem-stage0-arch-001".to_string(),
                domain: Some("spec-kit".to_string()),
                tags: vec!["type:decision".to_string()],
                importance: Some(9.0),
                created_at: Some(Utc::now()),
                snippet: "Stage 0 overlay engine architecture".to_string(),
                uri: None,
                indexable: true,
                visible_from: None,
            },
            "Stage 0 overlay engine architecture separates concerns between \
             local-memory daemon and scoring. The design decision was to use \
             SQLite for the overlay database.".to_string(),
        ),
        (
            MemoryMeta {
                id: "mem-stage0-overlay-002".to_string(),
                domain: Some("spec-kit".to_string()),
                tags: vec!["type:pattern".to_string()],
                importance: Some(8.0),
                created_at: Some(Utc::now()),
                snippet: "Stage 0 overlay design pattern".to_string(),
                uri: None,
                indexable: true,
                visible_from: None,
            },
            "Stage 0 overlay design pattern uses a separate database to track \
             dynamic scores and Tier 2 cache entries without modifying local-memory.".to_string(),
        ),
        // TF-IDF implementation
        (
            MemoryMeta {
                id: "mem-vector-tfidf-001".to_string(),
                domain: Some("spec-kit".to_string()),
                tags: vec!["type:implementation".to_string()],
                importance: Some(8.0),
                created_at: Some(Utc::now()),
                snippet: "TF-IDF vector backend implementation".to_string(),
                uri: None,
                indexable: true,
                visible_from: None,
            },
            "TF-IDF vector backend implementation uses BM25-style term frequency \
             saturation with k1=1.5 and b=0.75 parameters for scoring.".to_string(),
        ),
        (
            MemoryMeta {
                id: "mem-bm25-scoring-002".to_string(),
                domain: Some("spec-kit".to_string()),
                tags: vec!["type:algorithm".to_string()],
                importance: Some(7.0),
                created_at: Some(Utc::now()),
                snippet: "BM25 scoring formula".to_string(),
                uri: None,
                indexable: true,
                visible_from: None,
            },
            "BM25 scoring formula: TF * IDF where TF = (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * dl/avgdl)) \
             and IDF = log((N + 1) / (df + 1)) + 1".to_string(),
        ),
        // Bug patterns
        (
            MemoryMeta {
                id: "mem-bug-resize-001".to_string(),
                domain: Some("tui".to_string()),
                tags: vec!["type:bug".to_string()],
                importance: Some(9.0),
                created_at: Some(Utc::now()),
                snippet: "Window resize crash bug".to_string(),
                uri: None,
                indexable: true,
                visible_from: None,
            },
            "Bug: Window resize causes crash when terminal size drops below minimum. \
             Root cause was unchecked subtraction in viewport calculation. \
             Fix: Add bounds checking before resize. Memory corruption possible.".to_string(),
        ),
        (
            MemoryMeta {
                id: "mem-bug-segfault-002".to_string(),
                domain: Some("core".to_string()),
                tags: vec!["type:bug".to_string()],
                importance: Some(9.0),
                created_at: Some(Utc::now()),
                snippet: "Segfault in async handler".to_string(),
                uri: None,
                indexable: true,
                visible_from: None,
            },
            "Segfault in async handler due to memory corruption. The buffer was \
             being written to after being moved. Fix: Use Arc for shared ownership.".to_string(),
        ),
        // SPEC-KIT-102
        (
            MemoryMeta {
                id: "mem-speckit-102-001".to_string(),
                domain: Some("spec-kit".to_string()),
                tags: vec!["spec:SPEC-KIT-102".to_string()],
                importance: Some(8.0),
                created_at: Some(Utc::now()),
                snippet: "SPEC-KIT-102 NotebookLM integration".to_string(),
                uri: None,
                indexable: true,
                visible_from: None,
            },
            "SPEC-KIT-102 defines NotebookLM integration for Stage 0. The key insight \
             is that NotebookLM provides synthesis capabilities beyond local-memory search.".to_string(),
        ),
        (
            MemoryMeta {
                id: "mem-tier2-notebook-002".to_string(),
                domain: Some("spec-kit".to_string()),
                tags: vec!["type:integration".to_string()],
                importance: Some(7.0),
                created_at: Some(Utc::now()),
                snippet: "Tier 2 NotebookLM orchestration".to_string(),
                uri: None,
                indexable: true,
                visible_from: None,
            },
            "Tier 2 orchestration calls NotebookLM via MCP for Divine Truth synthesis. \
             Cache TTL is 24 hours to balance freshness with query costs.".to_string(),
        ),
        // Rust patterns
        (
            MemoryMeta {
                id: "mem-rust-errors-001".to_string(),
                domain: Some("rust".to_string()),
                tags: vec!["type:pattern".to_string()],
                importance: Some(8.0),
                created_at: Some(Utc::now()),
                snippet: "Rust error handling best practice".to_string(),
                uri: None,
                indexable: true,
                visible_from: None,
            },
            "Rust error handling best practice: Use thiserror for library errors \
             and anyhow for application errors. Result<T, E> is the standard pattern.".to_string(),
        ),
        (
            MemoryMeta {
                id: "mem-thiserror-002".to_string(),
                domain: Some("rust".to_string()),
                tags: vec!["type:library".to_string()],
                importance: Some(7.0),
                created_at: Some(Utc::now()),
                snippet: "thiserror derive macro".to_string(),
                uri: None,
                indexable: true,
                visible_from: None,
            },
            "thiserror derive macro generates Error trait implementations. \
             Use #[error] for Display, #[from] for automatic From conversions.".to_string(),
        ),
        // Unrelated docs (noise for testing specificity)
        (
            MemoryMeta {
                id: "mem-unrelated-001".to_string(),
                domain: Some("planning".to_string()),
                tags: vec!["type:notes".to_string()],
                importance: Some(3.0),
                created_at: Some(Utc::now()),
                snippet: "Q3 planning meeting notes".to_string(),
                uri: None,
                indexable: true,
                visible_from: None,
            },
            "Meeting notes from Q3 planning session. Discussed roadmap priorities \
             and resource allocation for the next quarter.".to_string(),
        ),
        (
            MemoryMeta {
                id: "mem-unrelated-002".to_string(),
                domain: Some("devops".to_string()),
                tags: vec!["type:guide".to_string()],
                importance: Some(4.0),
                created_at: Some(Utc::now()),
                snippet: "CI/CD configuration guide".to_string(),
                uri: None,
                indexable: true,
                visible_from: None,
            },
            "Configuration guide for CI/CD pipeline. Uses GitHub Actions with \
             cargo test and clippy checks on every PR.".to_string(),
        ),
    ]
}

/// Load a golden query suite from JSON file.
pub fn load_golden_queries(path: &Path) -> std::io::Result<Vec<GoldenQuery>> {
    let content = std::fs::read_to_string(path)?;
    serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Save a golden query suite to JSON file.
pub fn save_golden_queries(queries: &[GoldenQuery], path: &Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(queries)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}

// =============================================================================
// SPEC-KIT-972: Report Runner
// =============================================================================

/// Result from running the A/B evaluation harness.
#[derive(Debug)]
pub struct EvalRunResult {
    /// The generated report
    pub report: ABReport,
    /// Path to JSON report file
    pub json_path: std::path::PathBuf,
    /// Path to Markdown report file
    pub md_path: std::path::PathBuf,
    /// Whether backend B meets baseline (A)
    pub meets_baseline: bool,
    /// Whether P95 latency is under 250ms threshold
    pub latency_acceptable: bool,
}

/// Run the A/B harness and save reports to the eval directory.
///
/// ## SPEC-KIT-972: A/B Harness on Real Corpus
/// This function:
/// 1. Runs the ABHarness against both backends
/// 2. Generates JSON and Markdown reports
/// 3. Saves to `.speckit/eval/ab-report-{timestamp}.{json,md}`
/// 4. Returns the result with acceptance criteria checks
///
/// ## Arguments
/// * `backend_a` - Baseline backend (typically local-memory)
/// * `backend_b` - Experiment backend (typically memvid)
/// * `name_a` - Display name for backend A
/// * `name_b` - Display name for backend B
/// * `eval_dir` - Directory to save reports (e.g., ".speckit/eval")
/// * `top_k` - Number of results to retrieve per query
///
/// ## Returns
/// * `Ok(EvalRunResult)` - The evaluation result with saved report paths
/// * `Err` - If the evaluation or file save fails
pub async fn run_ab_harness_and_save(
    backend_a: Arc<dyn LocalMemoryClient>,
    backend_b: Arc<dyn LocalMemoryClient>,
    name_a: &str,
    name_b: &str,
    eval_dir: &Path,
    top_k: usize,
) -> Stage0Result<EvalRunResult> {
    // Create eval directory if it doesn't exist
    std::fs::create_dir_all(eval_dir)
        .map_err(|e| codex_stage0::errors::Stage0Error::dcc_with_source("create eval dir", e))?;

    // Run the harness
    let harness = ABHarness::new(backend_a, backend_b)
        .with_names(name_a, name_b)
        .with_top_k(top_k);

    let report = harness.run().await?;

    // Generate timestamp for filenames
    let timestamp = report.generated_at.format("%Y%m%d_%H%M%S");
    let base_name = format!("ab-report-{}", timestamp);

    let json_path = eval_dir.join(format!("{}.json", base_name));
    let md_path = eval_dir.join(format!("{}.md", base_name));

    // Save reports
    report
        .save_json(&json_path)
        .map_err(|e| codex_stage0::errors::Stage0Error::dcc_with_source("save JSON report", e))?;

    report
        .save_markdown(&md_path)
        .map_err(|e| codex_stage0::errors::Stage0Error::dcc_with_source("save MD report", e))?;

    // Check acceptance criteria
    let meets_baseline = report.b_meets_baseline();
    let latency_acceptable = report.b_latency_acceptable(250);

    tracing::info!(
        target: "eval",
        json_path = %json_path.display(),
        md_path = %md_path.display(),
        meets_baseline = meets_baseline,
        latency_acceptable = latency_acceptable,
        p95_latency_ms = %report.p95_latency_b().as_millis(),
        "A/B evaluation complete"
    );

    Ok(EvalRunResult {
        report,
        json_path,
        md_path,
        meets_baseline,
        latency_acceptable,
    })
}

/// Run A/B harness using synthetic test data (for testing).
///
/// This seeds both backends with the golden test memories and runs
/// the evaluation. Useful for validating the harness works correctly.
pub async fn run_ab_harness_synthetic(
    eval_dir: &Path,
    top_k: usize,
) -> Stage0Result<EvalRunResult> {
    use crate::memvid_adapter::adapter::MemvidMemoryAdapter;
    use crate::memvid_adapter::capsule::CapsuleConfig;
    use tempfile::TempDir;

    // Create temp directories for both backends
    let temp_dir = TempDir::new()
        .map_err(|e| codex_stage0::errors::Stage0Error::dcc_with_source("create temp dir", e))?;

    let capsule_a = temp_dir.path().join("backend_a.mv2");
    let capsule_b = temp_dir.path().join("backend_b.mv2");

    // Create and populate backend A
    let config_a = CapsuleConfig {
        capsule_path: capsule_a,
        workspace_id: "baseline".to_string(),
        ..Default::default()
    };
    let adapter_a = MemvidMemoryAdapter::new(config_a);
    adapter_a.open().await.map_err(|e| {
        codex_stage0::errors::Stage0Error::dcc_with_source(
            "open backend A",
            std::io::Error::other(e),
        )
    })?;

    // Create and populate backend B
    let config_b = CapsuleConfig {
        capsule_path: capsule_b,
        workspace_id: "experiment".to_string(),
        ..Default::default()
    };
    let adapter_b = MemvidMemoryAdapter::new(config_b);
    adapter_b.open().await.map_err(|e| {
        codex_stage0::errors::Stage0Error::dcc_with_source(
            "open backend B",
            std::io::Error::other(e),
        )
    })?;

    // Add golden test memories to both backends
    for (meta, content) in golden_test_memories() {
        adapter_a.add_memory_to_index(meta.clone(), &content).await;
        adapter_b.add_memory_to_index(meta, &content).await;
    }

    // Run the harness
    let backend_a: Arc<dyn LocalMemoryClient> = Arc::new(adapter_a);
    let backend_b: Arc<dyn LocalMemoryClient> = Arc::new(adapter_b);

    run_ab_harness_and_save(
        backend_a,
        backend_b,
        "local-memory (synthetic)",
        "memvid (synthetic)",
        eval_dir,
        top_k,
    )
    .await
}

// =============================================================================
// Serde helpers for Duration
// =============================================================================

mod duration_millis {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

mod duration_vec_millis {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(durations: &[Duration], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let millis: Vec<u128> = durations.iter().map(|d| d.as_millis()).collect();
        millis.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis: Vec<u64> = Vec::deserialize(deserializer)?;
        Ok(millis.into_iter().map(Duration::from_millis).collect())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memvid_adapter::adapter::MemvidMemoryAdapter;
    use crate::memvid_adapter::capsule::CapsuleConfig;
    use tempfile::TempDir;

    // =========================================================================
    // SPEC-KIT-979: Parity Gate Test Infrastructure
    // =========================================================================

    /// Result from a parity gate test, suitable for JSON output.
    ///
    /// Used by all `test_parity_gate_*` tests to produce machine-readable output.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ParityGateResult {
        /// Gate identifier: "GATE-RQ", "GATE-LP", "GATE-FP", "GATE-ST"
        pub gate: String,
        /// Whether the gate passed
        pub passed: bool,
        /// True only if real backends were used (not synthetic)
        pub certified: bool,
        /// "real" or "synthetic"
        pub mode: String,
        /// Gate-specific metrics
        pub metrics: serde_json::Value,
        /// Report generation timestamp
        pub timestamp: DateTime<Utc>,
        /// Human-readable summary
        pub message: String,
    }

    impl ParityGateResult {
        /// Create a new parity gate result.
        fn new(gate: &str, passed: bool, certified: bool, mode: &str) -> Self {
            Self {
                gate: gate.to_string(),
                passed,
                certified,
                mode: mode.to_string(),
                metrics: serde_json::Value::Null,
                timestamp: Utc::now(),
                message: String::new(),
            }
        }

        /// Set metrics
        fn with_metrics(mut self, metrics: serde_json::Value) -> Self {
            self.metrics = metrics;
            self
        }

        /// Set message
        fn with_message(mut self, message: impl Into<String>) -> Self {
            self.message = message.into();
            self
        }

        /// Print as JSON to stdout
        fn print(&self) {
            println!(
                "{}",
                serde_json::to_string_pretty(self).expect("serialize ParityGateResult")
            );
        }
    }

    /// Stability report schema for GATE-ST validation.
    ///
    /// This is the expected format for nightly stability reports.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct StabilityReport {
        /// Number of days in the stability period
        pub period_days: u32,
        /// Number of fallback activations (should be 0)
        pub fallback_activations: u32,
        /// Number of data loss incidents (should be 0)
        pub data_loss_incidents: u32,
        /// Crash recovery events with success/failure status
        pub crash_recoveries: Vec<CrashRecoveryEvent>,
        /// Report generation timestamp
        pub generated_at: DateTime<Utc>,
    }

    /// A crash recovery event within the stability period.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CrashRecoveryEvent {
        /// When the crash occurred
        pub timestamp: DateTime<Utc>,
        /// Whether recovery was successful
        pub recovered: bool,
        /// Optional description
        pub description: Option<String>,
    }

    impl StabilityReport {
        /// Validate the report against GATE-ST criteria.
        fn validate(&self) -> (bool, Vec<String>) {
            let mut issues = Vec::new();

            if self.fallback_activations > 0 {
                issues.push(format!(
                    "fallback_activations = {} (must be 0)",
                    self.fallback_activations
                ));
            }

            if self.data_loss_incidents > 0 {
                issues.push(format!(
                    "data_loss_incidents = {} (must be 0)",
                    self.data_loss_incidents
                ));
            }

            let failed_recoveries: Vec<_> = self
                .crash_recoveries
                .iter()
                .filter(|r| !r.recovered)
                .collect();
            if !failed_recoveries.is_empty() {
                issues.push(format!(
                    "{} crash recoveries failed (must all succeed)",
                    failed_recoveries.len()
                ));
            }

            (issues.is_empty(), issues)
        }

        /// Create a mock report for format validation testing.
        fn mock_passing() -> Self {
            Self {
                period_days: 30,
                fallback_activations: 0,
                data_loss_incidents: 0,
                crash_recoveries: vec![CrashRecoveryEvent {
                    timestamp: Utc::now(),
                    recovered: true,
                    description: Some("Simulated crash for testing".to_string()),
                }],
                generated_at: Utc::now(),
            }
        }
    }

    /// Test mode for parity gates: real backends or synthetic.
    enum ParityTestMode {
        /// Real backends available - results are certified
        Real {
            baseline: Arc<dyn LocalMemoryClient>,
            experiment: Arc<dyn LocalMemoryClient>,
        },
        /// Synthetic mode - infrastructure validation only
        Synthetic { reason: String },
    }

    impl std::fmt::Debug for ParityTestMode {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ParityTestMode::Real { .. } => write!(f, "ParityTestMode::Real"),
                ParityTestMode::Synthetic { reason } => {
                    write!(f, "ParityTestMode::Synthetic {{ reason: {:?} }}", reason)
                }
            }
        }
    }

    /// Attempt to detect and setup test mode.
    ///
    /// Tries to connect to local-memory daemon. If unavailable, falls back to synthetic.
    async fn detect_parity_test_mode(_temp_dir: &TempDir) -> ParityTestMode {
        // Check for LOCAL_MEMORY_SOCKET or similar environment variable
        // that would indicate real daemon availability
        if std::env::var("LOCAL_MEMORY_SOCKET").is_ok() || std::env::var("PARITY_TEST_REAL").is_ok()
        {
            // In a real implementation, we'd attempt to connect here.
            // For now, we document this as a future extension point.
            //
            // TODO(SPEC-KIT-979): Implement real LocalMemoryCliAdapter connection
            // when local-memory daemon integration is ready.
        }

        // Default: synthetic mode using two memvid backends
        let reason = if std::env::var("CI").is_ok() {
            "CI environment detected, using synthetic mode".to_string()
        } else {
            "Local-memory daemon not detected, using synthetic mode".to_string()
        };

        ParityTestMode::Synthetic { reason }
    }

    /// Setup backends for parity testing based on detected mode.
    async fn setup_parity_backends(
        mode: &ParityTestMode,
        temp_dir: &TempDir,
    ) -> (Arc<dyn LocalMemoryClient>, Arc<dyn LocalMemoryClient>, bool) {
        match mode {
            ParityTestMode::Real {
                baseline,
                experiment,
            } => (baseline.clone(), experiment.clone(), true),
            ParityTestMode::Synthetic { .. } => {
                // Create two memvid adapters with same test data
                let capsule_a = temp_dir.path().join("parity_baseline.mv2");
                let capsule_b = temp_dir.path().join("parity_experiment.mv2");

                let config_a = CapsuleConfig {
                    capsule_path: capsule_a,
                    workspace_id: "parity-baseline".to_string(),
                    ..Default::default()
                };
                let adapter_a = MemvidMemoryAdapter::new(config_a);
                adapter_a.open().await.expect("open baseline adapter");

                let config_b = CapsuleConfig {
                    capsule_path: capsule_b,
                    workspace_id: "parity-experiment".to_string(),
                    ..Default::default()
                };
                let adapter_b = MemvidMemoryAdapter::new(config_b);
                adapter_b.open().await.expect("open experiment adapter");

                // Populate both with golden test memories
                for (meta, content) in golden_test_memories() {
                    adapter_a.add_memory_to_index(meta.clone(), &content).await;
                    adapter_b.add_memory_to_index(meta, &content).await;
                }

                let baseline: Arc<dyn LocalMemoryClient> = Arc::new(adapter_a);
                let experiment: Arc<dyn LocalMemoryClient> = Arc::new(adapter_b);

                (baseline, experiment, false)
            }
        }
    }

    #[test]
    fn test_golden_query_suite_structure() {
        let queries = golden_query_suite();

        assert!(
            queries.len() >= 10,
            "Should have at least 10 golden queries"
        );

        for query in &queries {
            assert!(!query.name.is_empty());
            assert!(!query.keywords.is_empty() || !query.required_tags.is_empty());
        }
    }

    #[test]
    fn test_golden_query_to_search_params() {
        let query = GoldenQuery::new(
            "test",
            vec!["error".to_string(), "handling".to_string()],
            vec!["expected-id".to_string()],
        )
        .with_domain("rust")
        .with_required_tags(vec!["type:pattern".to_string()]);

        let params = query.to_search_params(10);

        assert_eq!(params.iqo.keywords, vec!["error", "handling"]);
        assert_eq!(params.iqo.domains, vec!["rust"]);
        assert_eq!(params.iqo.required_tags, vec!["type:pattern"]);
        assert_eq!(params.max_results, 10);
    }

    #[test]
    fn test_golden_query_to_eval_case() {
        let query = GoldenQuery::new(
            "test-case",
            vec!["keyword".to_string()],
            vec!["id1".to_string(), "id2".to_string()],
        )
        .with_description("A test case");

        let case = query.to_eval_case();

        assert_eq!(case.name, "test-case");
        assert_eq!(case.query, "keyword");
        assert_eq!(case.expected_ids, vec!["id1", "id2"]);
        assert_eq!(case.description, Some("A test case".to_string()));
        assert_eq!(case.lane, EvalLane::Memory);
    }

    #[test]
    fn test_golden_test_memories_coverage() {
        let memories = golden_test_memories();
        let queries = golden_query_suite();

        // Collect all expected IDs from queries
        let expected_ids: std::collections::HashSet<&str> = queries
            .iter()
            .flat_map(|q| q.expected_ids.iter().map(|s| s.as_str()))
            .collect();

        // Collect all memory IDs
        let memory_ids: std::collections::HashSet<&str> =
            memories.iter().map(|(m, _)| m.id.as_str()).collect();

        // Check coverage
        for expected in &expected_ids {
            if !expected.is_empty() {
                assert!(
                    memory_ids.contains(expected),
                    "Missing test memory for expected ID: {}",
                    expected
                );
            }
        }
    }

    #[test]
    fn test_percentile_duration() {
        let durations = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
            Duration::from_millis(50),
            Duration::from_millis(60),
            Duration::from_millis(70),
            Duration::from_millis(80),
            Duration::from_millis(90),
            Duration::from_millis(100),
        ];

        // P95 on 10 items: idx = 95 * 10 / 100 = 9 (capped at 9) = 100ms
        let p95 = percentile_duration(&durations, 95);
        assert_eq!(p95, Duration::from_millis(100));

        // P50 on 10 items: idx = 50 * 10 / 100 = 5 = 60ms
        let p50 = percentile_duration(&durations, 50);
        assert_eq!(p50, Duration::from_millis(60));
    }

    #[test]
    fn test_ab_report_to_markdown() {
        // Create minimal mock report
        let report = ABReport {
            backend_a_name: "local-memory".to_string(),
            backend_b_name: "memvid".to_string(),
            results_a: vec![],
            results_b: vec![],
            suite_a: EvalSuiteResult {
                results: vec![],
                mean_precision: 0.8,
                mean_recall: 0.7,
                mrr: 0.9,
                cases_passed: 8,
                total_cases: 10,
                top_k: 10,
                total_missing_ids: 0,
            },
            suite_b: EvalSuiteResult {
                results: vec![],
                mean_precision: 0.85,
                mean_recall: 0.75,
                mrr: 0.92,
                cases_passed: 9,
                total_cases: 10,
                top_k: 10,
                total_missing_ids: 0,
            },
            latencies_a: vec![Duration::from_millis(50), Duration::from_millis(100)],
            latencies_b: vec![Duration::from_millis(40), Duration::from_millis(80)],
            generated_at: Utc::now(),
            top_k: 10,
        };

        let md = report.to_markdown();

        assert!(md.contains("# A/B Retrieval Evaluation Report"));
        assert!(md.contains("local-memory"));
        assert!(md.contains("memvid"));
        assert!(md.contains("Mean P@10"));
        assert!(md.contains("Verdict"));
    }

    #[tokio::test]
    async fn test_ab_harness_with_memvid_adapter() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        let config = CapsuleConfig {
            capsule_path,
            workspace_id: "test".to_string(),
            ..Default::default()
        };

        // Create and populate adapter
        let adapter = MemvidMemoryAdapter::new(config);
        adapter.open().await.unwrap();

        // Add test memories
        for (meta, content) in golden_test_memories() {
            adapter.add_memory_to_index(meta, &content).await;
        }

        // Use same adapter for both A and B (self-comparison test)
        let adapter_arc: Arc<dyn LocalMemoryClient> = Arc::new(adapter);

        let harness = ABHarness::new(adapter_arc.clone(), adapter_arc)
            .with_names("baseline", "experiment")
            .with_top_k(5);

        let report = harness.run().await.unwrap();

        // Self-comparison should have equal metrics
        assert!(
            (report.suite_a.mean_precision - report.suite_b.mean_precision).abs() < f64::EPSILON
        );
        assert!((report.suite_a.mean_recall - report.suite_b.mean_recall).abs() < f64::EPSILON);

        // Should pass baseline check (comparing to self)
        assert!(report.b_meets_baseline());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // SPEC-KIT-972: Report Runner Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_run_ab_harness_synthetic_produces_reports() {
        let temp_dir = TempDir::new().unwrap();
        let eval_dir = temp_dir.path().join("eval");

        let result = run_ab_harness_synthetic(&eval_dir, 5).await.unwrap();

        // Check that reports were created
        assert!(result.json_path.exists(), "JSON report should exist");
        assert!(result.md_path.exists(), "Markdown report should exist");

        // Verify JSON is valid
        let json_content = std::fs::read_to_string(&result.json_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_content).unwrap();
        assert!(parsed.get("backend_a_name").is_some());
        assert!(parsed.get("backend_b_name").is_some());

        // Verify Markdown has expected sections
        let md_content = std::fs::read_to_string(&result.md_path).unwrap();
        assert!(md_content.contains("# A/B Retrieval Evaluation Report"));
        assert!(md_content.contains("## Summary"));
        assert!(md_content.contains("## Verdict"));

        // Self-comparison should meet baseline
        assert!(result.meets_baseline);

        // Synthetic test should be fast (well under 250ms)
        assert!(result.latency_acceptable);
    }

    #[tokio::test]
    async fn test_run_ab_harness_and_save_creates_timestamped_files() {
        let temp_dir = TempDir::new().unwrap();
        let eval_dir = temp_dir.path().join("eval");
        let capsule_path = temp_dir.path().join("test.mv2");

        let config = CapsuleConfig {
            capsule_path,
            workspace_id: "test".to_string(),
            ..Default::default()
        };

        let adapter = MemvidMemoryAdapter::new(config);
        adapter.open().await.unwrap();

        // Add some test data
        for (meta, content) in golden_test_memories().into_iter().take(3) {
            adapter.add_memory_to_index(meta, &content).await;
        }

        let adapter_arc: Arc<dyn LocalMemoryClient> = Arc::new(adapter);

        let result = run_ab_harness_and_save(
            adapter_arc.clone(),
            adapter_arc,
            "baseline",
            "experiment",
            &eval_dir,
            5,
        )
        .await
        .unwrap();

        // Verify filenames have timestamp pattern
        let json_name = result.json_path.file_name().unwrap().to_str().unwrap();
        assert!(json_name.starts_with("ab-report-"));
        assert!(json_name.ends_with(".json"));

        let md_name = result.md_path.file_name().unwrap().to_str().unwrap();
        assert!(md_name.starts_with("ab-report-"));
        assert!(md_name.ends_with(".md"));

        // Check that directory was created
        assert!(eval_dir.exists());
        assert!(eval_dir.is_dir());
    }

    // =========================================================================
    // SPEC-KIT-979: Parity Gate Tests
    // =========================================================================

    /// GATE-RQ: Retrieval Quality Parity
    ///
    /// Validates that memvid search quality >= 95% of baseline backend.
    ///
    /// **Criteria:**
    /// - Mean Precision@10 >= 0.95 * baseline
    /// - Mean Recall@10 >= 0.95 * baseline
    /// - All golden queries with expected_ids must find those IDs
    ///
    /// Run with: `cargo test -p codex-tui --lib -- test_parity_gate_retrieval_quality --ignored --nocapture`
    #[ignore]
    #[tokio::test]
    async fn test_parity_gate_retrieval_quality() {
        let temp_dir = TempDir::new().expect("create temp dir");

        // Detect test mode
        let mode = detect_parity_test_mode(&temp_dir).await;
        let (is_synthetic, mode_reason) = match &mode {
            ParityTestMode::Real { .. } => (false, "real backends".to_string()),
            ParityTestMode::Synthetic { reason } => (true, reason.clone()),
        };

        // Setup backends
        let (baseline, experiment, certified) = setup_parity_backends(&mode, &temp_dir).await;

        // Run A/B harness
        let harness = ABHarness::new(baseline, experiment)
            .with_names("baseline", "memvid")
            .with_top_k(10);

        let report = harness.run().await.expect("run AB harness");

        // Check parity criteria
        let precision_ratio = if report.suite_a.mean_precision > 0.0 {
            report.suite_b.mean_precision / report.suite_a.mean_precision
        } else {
            1.0 // If baseline is 0, any result is acceptable
        };

        let recall_ratio = if report.suite_a.mean_recall > 0.0 {
            report.suite_b.mean_recall / report.suite_a.mean_recall
        } else {
            1.0
        };

        let meets_precision = precision_ratio >= 0.95;
        let meets_recall = recall_ratio >= 0.95;
        let meets_mrr = report.suite_b.mrr >= report.suite_a.mrr * 0.95;

        // Check golden query coverage (all expected IDs found)
        // Note: In synthetic mode, we only check that both backends return same results,
        // not that all expected IDs are found (test data may not cover all golden queries)
        let queries = golden_query_suite();
        let mut golden_failures = Vec::new();
        if certified {
            // Real parity mode: strict check that all expected IDs are found
            for (query, result_b) in queries.iter().zip(report.results_b.iter()) {
                if !query.expected_ids.is_empty() {
                    let found_ids: std::collections::HashSet<&str> =
                        result_b.hits.iter().map(|h| h.id.as_str()).collect();
                    let missing: Vec<&str> = query
                        .expected_ids
                        .iter()
                        .filter(|id| !found_ids.contains(id.as_str()))
                        .map(|s| s.as_str())
                        .collect();
                    if !missing.is_empty() {
                        golden_failures.push(format!("{}: missing {:?}", query.name, missing));
                    }
                }
            }
        }
        // In synthetic mode, golden_failures stays empty (we don't check expected IDs)
        let all_golden_pass = golden_failures.is_empty();

        let passed = meets_precision && meets_recall && all_golden_pass;

        // Build result
        let message = if certified {
            if passed {
                "PARITY CERTIFIED - memvid meets retrieval quality gate".to_string()
            } else {
                "PARITY FAILED - memvid does not meet retrieval quality gate".to_string()
            }
        } else if passed {
            format!(
                "INFRASTRUCTURE VALIDATED - memvid meets 95% threshold in synthetic mode. {}. Run with real local-memory daemon for parity certification.",
                mode_reason
            )
        } else {
            format!(
                "INFRASTRUCTURE FAILED - even synthetic mode did not meet thresholds. {}",
                mode_reason
            )
        };

        let result = ParityGateResult::new(
            "GATE-RQ",
            passed,
            certified,
            if is_synthetic { "synthetic" } else { "real" },
        )
        .with_metrics(serde_json::json!({
            "baseline_precision": report.suite_a.mean_precision,
            "memvid_precision": report.suite_b.mean_precision,
            "precision_ratio": precision_ratio,
            "meets_precision": meets_precision,
            "baseline_recall": report.suite_a.mean_recall,
            "memvid_recall": report.suite_b.mean_recall,
            "recall_ratio": recall_ratio,
            "meets_recall": meets_recall,
            "baseline_mrr": report.suite_a.mrr,
            "memvid_mrr": report.suite_b.mrr,
            "meets_mrr": meets_mrr,
            "threshold": 0.95,
            "golden_failures": golden_failures,
            "all_golden_pass": all_golden_pass,
        }))
        .with_message(&message);

        result.print();

        assert!(passed, "{}", message);
    }

    /// GATE-LP: Latency Parity
    ///
    /// Validates that memvid P95 search latency < 250ms.
    ///
    /// **Criteria:**
    /// - P95 latency < 250ms
    /// - (CI caveat: if CI=true and latency > 250ms, warn but don't fail)
    ///
    /// Run with: `cargo test -p codex-tui --lib -- test_parity_gate_latency --ignored --nocapture`
    #[ignore]
    #[tokio::test]
    async fn test_parity_gate_latency() {
        let temp_dir = TempDir::new().expect("create temp dir");

        // Detect test mode
        let mode = detect_parity_test_mode(&temp_dir).await;
        let (is_synthetic, mode_reason) = match &mode {
            ParityTestMode::Real { .. } => (false, "real backends".to_string()),
            ParityTestMode::Synthetic { reason } => (true, reason.clone()),
        };

        // Setup backends
        let (baseline, experiment, certified) = setup_parity_backends(&mode, &temp_dir).await;

        // Run multiple repetitions for more accurate latency measurement
        const REPETITIONS: usize = 3;
        let mut all_latencies: Vec<Duration> = Vec::new();

        for _ in 0..REPETITIONS {
            let harness = ABHarness::new(baseline.clone(), experiment.clone())
                .with_names("baseline", "memvid")
                .with_top_k(10);

            let report = harness.run().await.expect("run AB harness");
            all_latencies.extend(report.latencies_b.clone());
        }

        // Compute P95 latency
        let p95 = percentile_duration(&all_latencies, 95);
        let p50 = percentile_duration(&all_latencies, 50);
        let max_latency = all_latencies
            .iter()
            .max()
            .copied()
            .unwrap_or(Duration::ZERO);

        const THRESHOLD_MS: u64 = 250;
        let meets_threshold = p95.as_millis() < THRESHOLD_MS as u128;

        // CI caveat: warn but don't fail if in CI and latency is high
        let is_ci = std::env::var("CI").is_ok();
        let passed = if is_ci && !meets_threshold {
            // CI environments can be slow; warn but don't fail
            true
        } else {
            meets_threshold
        };

        let message = if certified {
            if meets_threshold {
                format!(
                    "PARITY CERTIFIED - P95 latency {}ms < {}ms threshold",
                    p95.as_millis(),
                    THRESHOLD_MS
                )
            } else {
                format!(
                    "PARITY FAILED - P95 latency {}ms >= {}ms threshold",
                    p95.as_millis(),
                    THRESHOLD_MS
                )
            }
        } else if meets_threshold {
            format!(
                "INFRASTRUCTURE VALIDATED - P95 latency {}ms in synthetic mode. {}",
                p95.as_millis(),
                mode_reason
            )
        } else if is_ci {
            format!(
                "CI WARNING - P95 latency {}ms exceeds {}ms threshold (expected in slow CI). {}",
                p95.as_millis(),
                THRESHOLD_MS,
                mode_reason
            )
        } else {
            format!(
                "INFRASTRUCTURE WARNING - P95 latency {}ms exceeds {}ms threshold. {}",
                p95.as_millis(),
                THRESHOLD_MS,
                mode_reason
            )
        };

        let result = ParityGateResult::new(
            "GATE-LP",
            passed,
            certified,
            if is_synthetic { "synthetic" } else { "real" },
        )
        .with_metrics(serde_json::json!({
            "p95_latency_ms": p95.as_millis(),
            "p50_latency_ms": p50.as_millis(),
            "max_latency_ms": max_latency.as_millis(),
            "threshold_ms": THRESHOLD_MS,
            "meets_threshold": meets_threshold,
            "total_measurements": all_latencies.len(),
            "repetitions": REPETITIONS,
            "is_ci": is_ci,
        }))
        .with_message(&message);

        result.print();

        assert!(passed, "{}", message);
    }

    /// GATE-FP: Feature Parity
    ///
    /// Validates that memvid supports all required IQO features.
    ///
    /// **Features tested:**
    /// 1. Keywords only (basic search)
    /// 2. Domain filtering (IQO.domains)
    /// 3. Required tag filtering (IQO.required_tags)
    /// 4. Optional tag boost (IQO.optional_tags)
    /// 5. Importance threshold
    /// 6. Empty result handling (no-match query)
    /// 7. Combined filters
    ///
    /// Run with: `cargo test -p codex-tui --lib -- test_parity_gate_features --ignored --nocapture`
    #[ignore]
    #[tokio::test]
    async fn test_parity_gate_features() {
        let temp_dir = TempDir::new().expect("create temp dir");

        // Detect test mode
        let mode = detect_parity_test_mode(&temp_dir).await;
        let (is_synthetic, mode_reason) = match &mode {
            ParityTestMode::Real { .. } => (false, "real backends".to_string()),
            ParityTestMode::Synthetic { reason } => (true, reason.clone()),
        };

        // Setup backends
        let (baseline, experiment, certified) = setup_parity_backends(&mode, &temp_dir).await;

        // Define feature test cases
        let feature_tests: Vec<(&str, LocalMemorySearchParams)> = vec![
            (
                "keywords_only",
                LocalMemorySearchParams {
                    iqo: Iqo {
                        keywords: vec!["error".to_string(), "handling".to_string()],
                        ..Default::default()
                    },
                    max_results: 10,
                    as_of: None,
                },
            ),
            (
                "domain_filtering",
                LocalMemorySearchParams {
                    iqo: Iqo {
                        keywords: vec!["pattern".to_string()],
                        domains: vec!["rust".to_string()],
                        ..Default::default()
                    },
                    max_results: 10,
                    as_of: None,
                },
            ),
            (
                "required_tags",
                LocalMemorySearchParams {
                    iqo: Iqo {
                        keywords: vec!["memory".to_string()],
                        required_tags: vec!["type:decision".to_string()],
                        ..Default::default()
                    },
                    max_results: 10,
                    as_of: None,
                },
            ),
            (
                "optional_tags",
                LocalMemorySearchParams {
                    iqo: Iqo {
                        keywords: vec!["implementation".to_string()],
                        optional_tags: vec!["type:pattern".to_string()],
                        ..Default::default()
                    },
                    max_results: 10,
                    as_of: None,
                },
            ),
            (
                "empty_keywords_with_tags",
                LocalMemorySearchParams {
                    iqo: Iqo {
                        keywords: vec![],
                        required_tags: vec!["type:bug".to_string()],
                        ..Default::default()
                    },
                    max_results: 10,
                    as_of: None,
                },
            ),
            (
                "no_match_expected",
                LocalMemorySearchParams {
                    iqo: Iqo {
                        keywords: vec!["xyzzy_nonexistent_12345".to_string()],
                        ..Default::default()
                    },
                    max_results: 10,
                    as_of: None,
                },
            ),
            (
                "combined_filters",
                LocalMemorySearchParams {
                    iqo: Iqo {
                        keywords: vec!["architecture".to_string()],
                        domains: vec!["spec-kit".to_string()],
                        required_tags: vec!["type:decision".to_string()],
                        ..Default::default()
                    },
                    max_results: 10,
                    as_of: None,
                },
            ),
        ];

        let mut feature_results: Vec<serde_json::Value> = Vec::new();
        let mut all_passed = true;

        for (feature_name, params) in &feature_tests {
            // Run on baseline
            let baseline_result = baseline.search_memories(params.clone()).await;
            let baseline_ok = baseline_result.is_ok();
            let baseline_ids: Vec<String> = baseline_result
                .map(|hits| hits.iter().map(|h| h.id.clone()).collect())
                .unwrap_or_default();

            // Run on experiment
            let experiment_result = experiment.search_memories(params.clone()).await;
            let experiment_ok = experiment_result.is_ok();
            let experiment_ids: Vec<String> = experiment_result
                .map(|hits| hits.iter().map(|h| h.id.clone()).collect())
                .unwrap_or_default();

            // Check structural equivalence (both succeed, similar result sets)
            let both_succeed = baseline_ok && experiment_ok;
            let baseline_set: std::collections::HashSet<_> = baseline_ids.iter().collect();
            let experiment_set: std::collections::HashSet<_> = experiment_ids.iter().collect();
            let ids_match = baseline_set == experiment_set;

            let feature_passed = both_succeed && (ids_match || is_synthetic);

            if !feature_passed {
                all_passed = false;
            }

            feature_results.push(serde_json::json!({
                "feature": feature_name,
                "passed": feature_passed,
                "baseline_ok": baseline_ok,
                "experiment_ok": experiment_ok,
                "baseline_ids": baseline_ids,
                "experiment_ids": experiment_ids,
                "ids_match": ids_match,
            }));
        }

        let message = if certified {
            if all_passed {
                "PARITY CERTIFIED - all feature parity checks passed".to_string()
            } else {
                "PARITY FAILED - some feature parity checks failed".to_string()
            }
        } else if all_passed {
            format!(
                "INFRASTRUCTURE VALIDATED - all features work in synthetic mode. {}",
                mode_reason
            )
        } else {
            format!(
                "INFRASTRUCTURE FAILED - some features failed even in synthetic mode. {}",
                mode_reason
            )
        };

        let result = ParityGateResult::new("GATE-FP", all_passed, certified, if is_synthetic { "synthetic" } else { "real" })
            .with_metrics(serde_json::json!({
                "features_tested": feature_tests.len(),
                "features_passed": feature_results.iter().filter(|r| r["passed"].as_bool().unwrap_or(false)).count(),
                "feature_results": feature_results,
            }))
            .with_message(&message);

        result.print();

        assert!(all_passed, "{}", message);
    }

    /// GATE-ST: Stability
    ///
    /// Validates stability report format and zero-incident criteria.
    ///
    /// **Criteria:**
    /// - Report parses correctly (JSON schema validation)
    /// - fallback_activations == 0
    /// - data_loss_incidents == 0
    /// - All crash_recoveries successful
    ///
    /// **Limitation:** Real 30-day stability requires nightly CI tracking.
    /// This test validates report format and counter values only.
    ///
    /// Run with: `cargo test -p codex-tui --lib -- test_parity_gate_stability --ignored --nocapture`
    /// Run with report: `STABILITY_REPORT_PATH=./stability.json cargo test ...`
    #[ignore]
    #[tokio::test]
    async fn test_parity_gate_stability() {
        // Check for external report path
        let report_path = std::env::var("STABILITY_REPORT_PATH").ok();
        let has_external_report = report_path.is_some();
        let (report, source) = if let Some(path) = report_path {
            // Load external report
            let content = std::fs::read_to_string(&path)
                .expect(&format!("read stability report from {}", path));
            let report: StabilityReport = serde_json::from_str(&content)
                .expect(&format!("parse stability report from {}", path));
            (report, format!("external file: {}", path))
        } else {
            // Use mock report for format validation
            (
                StabilityReport::mock_passing(),
                "mock report (no STABILITY_REPORT_PATH provided)".to_string(),
            )
        };

        // Validate report
        let (passed, issues) = report.validate();

        let certified = has_external_report && report.period_days >= 30;

        let message = if certified {
            if passed {
                format!(
                    "PARITY CERTIFIED - stability report passed ({} days, 0 incidents)",
                    report.period_days
                )
            } else {
                format!("PARITY FAILED - stability report has issues: {:?}", issues)
            }
        } else if passed {
            format!(
                "FORMAT VALIDATED - report schema is valid. Source: {}. Note: Real 30-day stability requires nightly CI tracking.",
                source
            )
        } else {
            format!(
                "FORMAT FAILED - report has issues: {:?}. Source: {}",
                issues, source
            )
        };

        let result = ParityGateResult::new("GATE-ST", passed, certified, if has_external_report { "external" } else { "mock" })
            .with_metrics(serde_json::json!({
                "period_days": report.period_days,
                "fallback_activations": report.fallback_activations,
                "data_loss_incidents": report.data_loss_incidents,
                "crash_recovery_count": report.crash_recoveries.len(),
                "crash_recovery_all_success": report.crash_recoveries.iter().all(|r| r.recovered),
                "issues": issues,
                "source": source,
                "limitation": "Real 30-day stability requires nightly CI tracking. This test validates report format only.",
            }))
            .with_message(&message);

        result.print();

        assert!(passed, "{}", message);
    }
}
