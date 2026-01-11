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
use codex_stage0::eval::{
    compute_metrics, compute_suite_metrics, EvalCase, EvalCaseSource, EvalLane, EvalResult,
    EvalSuiteResult,
};
use codex_stage0::errors::Result as Stage0Result;
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
            vec!["error".to_string(), "handling".to_string(), "pattern".to_string()],
            vec!["mem-rust-errors-001".to_string(), "mem-thiserror-002".to_string()],
        )
        .with_description("Basic keyword search for error handling patterns"),

        GoldenQuery::new(
            "tfidf-bm25-implementation",
            vec!["tfidf".to_string(), "bm25".to_string(), "scoring".to_string()],
            vec!["mem-vector-tfidf-001".to_string(), "mem-bm25-scoring-002".to_string()],
        )
        .with_description("Search for TF-IDF/BM25 implementation details"),

        GoldenQuery::new(
            "stage0-architecture-decision",
            vec!["stage0".to_string(), "architecture".to_string(), "design".to_string()],
            vec!["mem-stage0-arch-001".to_string(), "mem-stage0-overlay-002".to_string()],
        )
        .with_description("Architecture decisions for Stage0"),

        // ─────────────────────────────────────────────────────────────────────────
        // Domain-filtered queries
        // ─────────────────────────────────────────────────────────────────────────
        GoldenQuery::new(
            "spec-kit-domain-query",
            vec!["integration".to_string(), "tier2".to_string()],
            vec!["mem-speckit-102-001".to_string(), "mem-tier2-notebook-002".to_string()],
        )
        .with_domain("spec-kit")
        .with_description("Query within spec-kit domain"),

        GoldenQuery::new(
            "rust-domain-query",
            vec!["error".to_string(), "result".to_string()],
            vec!["mem-rust-errors-001".to_string(), "mem-thiserror-002".to_string()],
        )
        .with_domain("rust")
        .with_description("Query within rust domain"),

        // ─────────────────────────────────────────────────────────────────────────
        // Tag-filtered queries
        // ─────────────────────────────────────────────────────────────────────────
        GoldenQuery::new(
            "bug-pattern-required-tag",
            vec!["crash".to_string(), "memory".to_string()],
            vec!["mem-bug-resize-001".to_string(), "mem-bug-segfault-002".to_string()],
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
            vec!["mem-vector-tfidf-001".to_string(), "mem-stage0-overlay-002".to_string()],
        )
        .with_optional_tags(vec!["type:pattern".to_string(), "type:implementation".to_string()])
        .with_description("Patterns with optional tag boosting"),

        // ─────────────────────────────────────────────────────────────────────────
        // Multi-keyword precision queries
        // ─────────────────────────────────────────────────────────────────────────
        GoldenQuery::new(
            "notebooklm-tier2-spec",
            vec!["notebooklm".to_string(), "tier2".to_string(), "synthesis".to_string(), "mcp".to_string()],
            vec!["mem-speckit-102-001".to_string(), "mem-tier2-notebook-002".to_string()],
        )
        .with_description("Precise query for NotebookLM Tier2 integration"),

        GoldenQuery::new(
            "resize-crash-segfault",
            vec!["resize".to_string(), "crash".to_string(), "segfault".to_string()],
            vec!["mem-bug-resize-001".to_string(), "mem-bug-segfault-002".to_string()],
        )
        .with_description("Precise query for resize/crash bugs"),

        // ─────────────────────────────────────────────────────────────────────────
        // Edge cases
        // ─────────────────────────────────────────────────────────────────────────
        GoldenQuery::new(
            "single-keyword-broad",
            vec!["memory".to_string()],
            vec!["mem-bug-resize-001".to_string(), "mem-bug-segfault-002".to_string()],
        )
        .with_description("Single broad keyword (tests ranking)"),

        GoldenQuery::new(
            "rare-term-search",
            vec!["avgdl".to_string()],  // BM25 formula term
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
            vec!["mem-stage0-arch-001".to_string(), "mem-stage0-overlay-002".to_string()],
        )
        .with_domain("spec-kit")
        .with_optional_tags(vec!["type:pattern".to_string()])
        .with_description("Combined domain + optional tag query"),

        // ─────────────────────────────────────────────────────────────────────────
        // Negative/Challenging cases
        // ─────────────────────────────────────────────────────────────────────────
        GoldenQuery::new(
            "no-match-expected",
            vec!["quantum".to_string(), "blockchain".to_string(), "web3".to_string()],
            vec![],  // No expected matches
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
            },
            "Configuration guide for CI/CD pipeline. Uses GitHub Actions with \
             cargo test and clippy checks on every PR.".to_string(),
        ),
    ]
}

/// Load a golden query suite from JSON file.
pub fn load_golden_queries(path: &Path) -> std::io::Result<Vec<GoldenQuery>> {
    let content = std::fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Save a golden query suite to JSON file.
pub fn save_golden_queries(queries: &[GoldenQuery], path: &Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(queries)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
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

    #[test]
    fn test_golden_query_suite_structure() {
        let queries = golden_query_suite();

        assert!(queries.len() >= 10, "Should have at least 10 golden queries");

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
        assert!((report.suite_a.mean_precision - report.suite_b.mean_precision).abs() < f64::EPSILON);
        assert!((report.suite_a.mean_recall - report.suite_b.mean_recall).abs() < f64::EPSILON);

        // Should pass baseline check (comparing to self)
        assert!(report.b_meets_baseline());
    }
}
