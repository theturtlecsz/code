//! Hybrid retrieval backend combining lexical and semantic search
//!
//! SPEC-KIT-972: Implements hybrid retrieval formula:
//! `final_score = lex_weight * lex_score + vec_weight * vec_score`
//!
//! This backend combines:
//! - Lexical search via TF-IDF (BM25-style scoring)
//! - Semantic search via vector embeddings (BGE-M3 or similar)
//!
//! ## Architecture
//! - Both backends implement VectorBackend trait
//! - Hybrid backend wraps both and merges results
//! - Reciprocal Rank Fusion (RRF) for score normalization

use crate::errors::Result;
use crate::tfidf::TfIdfBackend;
use crate::vector::{IndexStats, ScoredVector, VectorBackend, VectorDocument, VectorFilters};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for hybrid retrieval
#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// Weight for lexical (TF-IDF) scores (0.0 - 1.0)
    pub lexical_weight: f64,

    /// Weight for semantic (vector) scores (0.0 - 1.0)
    pub semantic_weight: f64,

    /// RRF constant k for score fusion
    /// Higher k reduces the impact of ranking position
    pub rrf_k: f64,

    /// Whether to use RRF (true) or linear combination (false)
    pub use_rrf: bool,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            lexical_weight: 0.5,
            semantic_weight: 0.5,
            rrf_k: 60.0,
            use_rrf: true,
        }
    }
}

impl HybridConfig {
    /// Create config with lexical-heavy weighting (good for exact matches)
    pub fn lexical_heavy() -> Self {
        Self {
            lexical_weight: 0.7,
            semantic_weight: 0.3,
            ..Default::default()
        }
    }

    /// Create config with semantic-heavy weighting (good for conceptual search)
    pub fn semantic_heavy() -> Self {
        Self {
            lexical_weight: 0.3,
            semantic_weight: 0.7,
            ..Default::default()
        }
    }

    /// Validate weights sum to 1.0 (within tolerance)
    pub fn validate(&self) -> bool {
        (self.lexical_weight + self.semantic_weight - 1.0).abs() < 0.01
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HybridBackend
// ─────────────────────────────────────────────────────────────────────────────

/// Hybrid retrieval backend combining lexical and semantic search
///
/// ## SPEC-KIT-972: Hybrid Retrieval
/// This backend enables hybrid scoring by combining:
/// - Lexical search (TF-IDF/BM25) for exact keyword matching
/// - Semantic search (vector embeddings) for conceptual similarity
///
/// ## Score Fusion
/// Two fusion methods are supported:
///
/// 1. **Reciprocal Rank Fusion (RRF)** (default):
///    `score = 1/(k + rank_lex) + 1/(k + rank_vec)`
///    - Position-based, works with heterogeneous score scales
///    - Recommended when lex and vec scores aren't normalized
///
/// 2. **Linear Combination**:
///    `score = lex_weight * lex_score + vec_weight * vec_score`
///    - Requires normalized scores [0, 1]
///    - Use when you have calibrated backends
pub struct HybridBackend {
    /// Configuration
    config: HybridConfig,

    /// Lexical search backend (TF-IDF)
    lexical: Arc<TfIdfBackend>,

    /// Semantic search backend (optional - uses lexical as fallback)
    semantic: Option<Arc<dyn VectorBackend>>,
}

impl HybridBackend {
    /// Create a new hybrid backend with TF-IDF for lexical search
    pub fn new() -> Self {
        Self::with_config(HybridConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: HybridConfig) -> Self {
        Self {
            config,
            lexical: Arc::new(TfIdfBackend::new()),
            semantic: None,
        }
    }

    /// Set a custom semantic backend
    ///
    /// When set, hybrid search uses this for semantic scoring.
    /// When None (default), only lexical search is performed.
    pub fn with_semantic_backend(mut self, backend: Arc<dyn VectorBackend>) -> Self {
        self.semantic = Some(backend);
        self
    }

    /// Check if semantic search is enabled
    pub fn has_semantic(&self) -> bool {
        self.semantic.is_some()
    }

    /// Get reference to lexical backend
    pub fn lexical_backend(&self) -> &TfIdfBackend {
        &self.lexical
    }

    /// Merge results from lexical and semantic backends
    fn merge_results(
        &self,
        lex_results: Vec<ScoredVector>,
        vec_results: Vec<ScoredVector>,
        top_k: usize,
    ) -> Vec<ScoredVector> {
        if self.config.use_rrf {
            self.merge_rrf(lex_results, vec_results, top_k)
        } else {
            self.merge_linear(lex_results, vec_results, top_k)
        }
    }

    /// Reciprocal Rank Fusion merging
    fn merge_rrf(
        &self,
        lex_results: Vec<ScoredVector>,
        vec_results: Vec<ScoredVector>,
        top_k: usize,
    ) -> Vec<ScoredVector> {
        let k = self.config.rrf_k;
        let lex_w = self.config.lexical_weight;
        let vec_w = self.config.semantic_weight;

        // Build rank maps (1-indexed)
        let lex_ranks: HashMap<String, usize> = lex_results
            .iter()
            .enumerate()
            .map(|(i, r)| (r.id.clone(), i + 1))
            .collect();

        let vec_ranks: HashMap<String, usize> = vec_results
            .iter()
            .enumerate()
            .map(|(i, r)| (r.id.clone(), i + 1))
            .collect();

        // Collect all unique document IDs
        let mut all_ids: HashMap<String, ScoredVector> = HashMap::new();

        for result in lex_results {
            all_ids.insert(result.id.clone(), result);
        }
        for result in vec_results {
            all_ids.entry(result.id.clone()).or_insert(result);
        }

        // Compute RRF scores
        let mut scored: Vec<ScoredVector> = all_ids
            .into_iter()
            .map(|(id, mut result)| {
                let lex_rank = lex_ranks.get(&id).copied().unwrap_or(1000);
                let vec_rank = vec_ranks.get(&id).copied().unwrap_or(1000);

                let lex_rrf = 1.0 / (k + lex_rank as f64);
                let vec_rrf = 1.0 / (k + vec_rank as f64);

                result.score = lex_w * lex_rrf + vec_w * vec_rrf;
                result
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top_k
        scored.truncate(top_k);
        scored
    }

    /// Linear combination merging (requires normalized scores)
    fn merge_linear(
        &self,
        lex_results: Vec<ScoredVector>,
        vec_results: Vec<ScoredVector>,
        top_k: usize,
    ) -> Vec<ScoredVector> {
        let lex_w = self.config.lexical_weight;
        let vec_w = self.config.semantic_weight;

        // Build score maps
        let lex_scores: HashMap<String, (f64, ScoredVector)> = lex_results
            .into_iter()
            .map(|r| (r.id.clone(), (r.score, r)))
            .collect();

        let vec_scores: HashMap<String, (f64, ScoredVector)> = vec_results
            .into_iter()
            .map(|r| (r.id.clone(), (r.score, r)))
            .collect();

        // Collect all unique document IDs
        let mut all_ids: HashMap<String, ScoredVector> = HashMap::new();

        for (id, (_, result)) in &lex_scores {
            all_ids.insert(id.clone(), result.clone());
        }
        for (id, (_, result)) in &vec_scores {
            all_ids.entry(id.clone()).or_insert_with(|| result.clone());
        }

        // Compute combined scores
        let mut scored: Vec<ScoredVector> = all_ids
            .into_iter()
            .map(|(id, mut result)| {
                let lex_score = lex_scores.get(&id).map(|(s, _)| *s).unwrap_or(0.0);
                let vec_score = vec_scores.get(&id).map(|(s, _)| *s).unwrap_or(0.0);

                result.score = lex_w * lex_score + vec_w * vec_score;
                result
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top_k
        scored.truncate(top_k);
        scored
    }
}

impl Default for HybridBackend {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// VectorBackend Implementation
// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl VectorBackend for HybridBackend {
    async fn index_documents(&self, docs: Vec<VectorDocument>) -> Result<IndexStats> {
        // Index in both backends
        let lex_stats = self.lexical.index_documents(docs.clone()).await?;

        if let Some(semantic) = &self.semantic {
            semantic.index_documents(docs).await?;
        }

        Ok(lex_stats)
    }

    async fn search(
        &self,
        query_text: &str,
        filters: &VectorFilters,
        top_k: usize,
    ) -> Result<Vec<ScoredVector>> {
        // Get lexical results
        let lex_results = self.lexical.search(query_text, filters, top_k * 2).await?;

        // Get semantic results (if available)
        let vec_results = if let Some(semantic) = &self.semantic {
            semantic.search(query_text, filters, top_k * 2).await?
        } else {
            // No semantic backend - just return lexical results
            return Ok(lex_results.into_iter().take(top_k).collect());
        };

        // Merge results
        Ok(self.merge_results(lex_results, vec_results, top_k))
    }

    async fn document_count(&self) -> Result<usize> {
        self.lexical.document_count().await
    }

    async fn clear(&self) -> Result<()> {
        self.lexical.clear().await?;
        if let Some(semantic) = &self.semantic {
            semantic.clear().await?;
        }
        Ok(())
    }

    async fn get_document(&self, id: &str) -> Result<Option<VectorDocument>> {
        self.lexical.get_document(id).await
    }

    async fn delete_document(&self, id: &str) -> Result<bool> {
        let lex_deleted = self.lexical.delete_document(id).await?;
        if let Some(semantic) = &self.semantic {
            semantic.delete_document(id).await?;
        }
        Ok(lex_deleted)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::DocumentKind;

    #[test]
    fn test_hybrid_config_validate() {
        let config = HybridConfig::default();
        assert!(config.validate());

        let invalid = HybridConfig {
            lexical_weight: 0.8,
            semantic_weight: 0.5,
            ..Default::default()
        };
        assert!(!invalid.validate());
    }

    #[test]
    fn test_hybrid_config_presets() {
        let lex_heavy = HybridConfig::lexical_heavy();
        assert!(lex_heavy.lexical_weight > lex_heavy.semantic_weight);
        assert!(lex_heavy.validate());

        let sem_heavy = HybridConfig::semantic_heavy();
        assert!(sem_heavy.semantic_weight > sem_heavy.lexical_weight);
        assert!(sem_heavy.validate());
    }

    #[tokio::test]
    async fn test_hybrid_backend_lexical_only() {
        let backend = HybridBackend::new();

        // No semantic backend
        assert!(!backend.has_semantic());

        // Index some docs
        let docs = vec![
            VectorDocument::new("doc1", DocumentKind::Memory, "Rust programming language"),
            VectorDocument::new("doc2", DocumentKind::Memory, "Python machine learning"),
        ];

        backend.index_documents(docs).await.unwrap();
        assert_eq!(backend.document_count().await.unwrap(), 2);

        // Search
        let results = backend
            .search("Rust programming", &VectorFilters::default(), 5)
            .await
            .unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, "doc1");
    }

    #[tokio::test]
    async fn test_rrf_merge_basics() {
        // Create mock results
        let lex_results = vec![
            ScoredVector::new("doc1", 0.9, DocumentKind::Memory),
            ScoredVector::new("doc2", 0.7, DocumentKind::Memory),
            ScoredVector::new("doc3", 0.5, DocumentKind::Memory),
        ];

        let vec_results = vec![
            ScoredVector::new("doc2", 0.95, DocumentKind::Memory),
            ScoredVector::new("doc1", 0.6, DocumentKind::Memory),
            ScoredVector::new("doc4", 0.4, DocumentKind::Memory),
        ];

        let backend = HybridBackend::new();
        let merged = backend.merge_rrf(lex_results, vec_results, 5);

        // doc1: lex_rank=1, vec_rank=2
        // doc2: lex_rank=2, vec_rank=1
        // Should be close because doc1 is #1 in lex, doc2 is #1 in vec
        assert!(merged.len() >= 2);

        // Both doc1 and doc2 should be in top results
        let ids: Vec<&str> = merged.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"doc1"));
        assert!(ids.contains(&"doc2"));
    }

    #[tokio::test]
    async fn test_linear_merge_weighted() {
        let config = HybridConfig {
            lexical_weight: 0.7,
            semantic_weight: 0.3,
            use_rrf: false,
            ..Default::default()
        };

        let backend = HybridBackend::with_config(config);

        let lex_results = vec![
            ScoredVector::new("doc1", 1.0, DocumentKind::Memory),
            ScoredVector::new("doc2", 0.5, DocumentKind::Memory),
        ];

        let vec_results = vec![
            ScoredVector::new("doc1", 0.3, DocumentKind::Memory),
            ScoredVector::new("doc2", 1.0, DocumentKind::Memory),
        ];

        let merged = backend.merge_linear(lex_results, vec_results, 5);

        // doc1: 0.7 * 1.0 + 0.3 * 0.3 = 0.79
        // doc2: 0.7 * 0.5 + 0.3 * 1.0 = 0.65
        assert_eq!(merged[0].id, "doc1");
        assert!((merged[0].score - 0.79).abs() < 0.01);
    }
}
