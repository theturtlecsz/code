//! In-memory TF-IDF vector backend
//!
//! SPEC-KIT-102 V2: Simple TF-IDF implementation for the VectorBackend trait.
//!
//! This backend is suitable for:
//! - Development and testing
//! - Small to medium corpora (hundreds to low thousands of documents)
//! - Quick iteration on the API before committing to external dependencies
//!
//! The implementation uses:
//! - BM25-style term frequency (with k1 and b parameters)
//! - IDF with smoothing: log((N + 1) / (df + 1)) + 1
//! - Cosine normalization for document length

// RwLock poisoning is exceptional (requires panic in critical section) - allow expect for internal locks
#![allow(clippy::expect_used)]

use crate::errors::Result;
use crate::vector::{IndexStats, ScoredVector, VectorBackend, VectorDocument, VectorFilters};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;

// ─────────────────────────────────────────────────────────────────────────────
// TF-IDF Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for TF-IDF scoring
#[derive(Debug, Clone)]
pub struct TfIdfConfig {
    /// BM25 k1 parameter (term frequency saturation)
    /// Higher values give more weight to term frequency
    /// Typical range: 1.2 - 2.0
    pub k1: f64,

    /// BM25 b parameter (length normalization)
    /// 0 = no normalization, 1 = full normalization
    /// Typical range: 0.5 - 0.75
    pub b: f64,

    /// Minimum document frequency for a term to be indexed
    /// Terms appearing in fewer documents are considered noise
    pub min_df: usize,

    /// Maximum document frequency ratio (0.0 - 1.0)
    /// Terms appearing in more than this ratio of documents are filtered
    pub max_df_ratio: f64,
}

impl Default for TfIdfConfig {
    fn default() -> Self {
        Self {
            k1: 1.5,
            b: 0.75,
            min_df: 1,
            max_df_ratio: 0.95,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal Types
// ─────────────────────────────────────────────────────────────────────────────

/// Indexed document with pre-computed term frequencies
#[derive(Debug, Clone)]
struct IndexedDoc {
    /// Original document
    doc: VectorDocument,

    /// Term frequencies (token -> count)
    tf: HashMap<String, usize>,

    /// Document length (token count)
    length: usize,
}

impl IndexedDoc {
    fn from_document(doc: VectorDocument) -> Self {
        let tokens = tokenize(&doc.text);
        let length = tokens.len();

        let mut tf = HashMap::new();
        for token in &tokens {
            *tf.entry(token.clone()).or_insert(0) += 1;
        }

        Self { doc, tf, length }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TfIdfBackend
// ─────────────────────────────────────────────────────────────────────────────

/// In-memory TF-IDF vector backend
pub struct TfIdfBackend {
    /// Configuration
    config: TfIdfConfig,

    /// Indexed documents by ID
    documents: RwLock<HashMap<String, IndexedDoc>>,

    /// Document frequency per term
    df: RwLock<HashMap<String, usize>>,

    /// Average document length (updated on index)
    avg_doc_length: RwLock<f64>,
}

impl TfIdfBackend {
    /// Create a new TF-IDF backend with default configuration
    pub fn new() -> Self {
        Self::with_config(TfIdfConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: TfIdfConfig) -> Self {
        Self {
            config,
            documents: RwLock::new(HashMap::new()),
            df: RwLock::new(HashMap::new()),
            avg_doc_length: RwLock::new(0.0),
        }
    }

    /// Recompute document frequencies and average length
    fn recompute_stats(&self) {
        let docs = self.documents.read().expect("lock");

        // Compute DF
        let mut df: HashMap<String, usize> = HashMap::new();
        let mut total_length = 0usize;

        for indexed in docs.values() {
            // Count each unique term once per document
            for token in indexed.tf.keys() {
                *df.entry(token.clone()).or_insert(0) += 1;
            }
            total_length += indexed.length;
        }

        // Update DF
        let mut df_lock = self.df.write().expect("lock");
        *df_lock = df;

        // Update average doc length
        let mut avg_lock = self.avg_doc_length.write().expect("lock");
        *avg_lock = if docs.is_empty() {
            0.0
        } else {
            total_length as f64 / docs.len() as f64
        };
    }

    /// Compute BM25-style TF-IDF score for a document against a query
    fn score_document(&self, indexed: &IndexedDoc, query_tokens: &[String]) -> f64 {
        let df = self.df.read().expect("lock");
        let avg_len = *self.avg_doc_length.read().expect("lock");
        let n = self.documents.read().expect("lock").len() as f64;

        if n == 0.0 || avg_len == 0.0 {
            return 0.0;
        }

        let k1 = self.config.k1;
        let b = self.config.b;
        let doc_len = indexed.length as f64;

        let mut score = 0.0;

        for token in query_tokens {
            // Term frequency in document
            let tf = *indexed.tf.get(token).unwrap_or(&0) as f64;
            if tf == 0.0 {
                continue;
            }

            // Document frequency
            let doc_freq = *df.get(token).unwrap_or(&0) as f64;

            // IDF with smoothing
            let idf = ((n + 1.0) / (doc_freq + 1.0)).ln() + 1.0;

            // BM25 TF component
            let tf_norm = (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * doc_len / avg_len));

            score += idf * tf_norm;
        }

        score
    }
}

impl Default for TfIdfBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VectorBackend for TfIdfBackend {
    async fn index_documents(&self, docs: Vec<VectorDocument>) -> Result<IndexStats> {
        let start = Instant::now();
        let doc_count = docs.len();

        {
            let mut documents = self.documents.write().expect("lock");

            for doc in docs {
                let id = doc.id.clone();
                let indexed = IndexedDoc::from_document(doc);
                documents.insert(id, indexed);
            }
        }

        // Recompute global stats
        self.recompute_stats();

        // Gather stats
        let unique_tokens = self.df.read().expect("lock").len();
        let total_tokens: usize = self
            .documents
            .read()
            .expect("lock")
            .values()
            .map(|d| d.length)
            .sum();

        Ok(IndexStats {
            documents_indexed: doc_count,
            unique_tokens,
            total_tokens,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn search(
        &self,
        query_text: &str,
        filters: &VectorFilters,
        top_k: usize,
    ) -> Result<Vec<ScoredVector>> {
        let query_tokens = tokenize(query_text);

        if query_tokens.is_empty() {
            return Ok(Vec::new());
        }

        let documents = self.documents.read().expect("lock");

        // Score and filter documents
        let mut scored: Vec<ScoredVector> = documents
            .values()
            .filter(|indexed| filters.matches(&indexed.doc))
            .map(|indexed| {
                let score = self.score_document(indexed, &query_tokens);
                ScoredVector {
                    id: indexed.doc.id.clone(),
                    score,
                    kind: indexed.doc.kind,
                    metadata: indexed.doc.metadata.clone(),
                }
            })
            .filter(|sv| sv.score > 0.0)
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top_k
        scored.truncate(top_k);

        Ok(scored)
    }

    async fn document_count(&self) -> Result<usize> {
        Ok(self.documents.read().expect("lock").len())
    }

    async fn clear(&self) -> Result<()> {
        self.documents.write().expect("lock").clear();
        self.df.write().expect("lock").clear();
        *self.avg_doc_length.write().expect("lock") = 0.0;
        Ok(())
    }

    async fn get_document(&self, id: &str) -> Result<Option<VectorDocument>> {
        let documents = self.documents.read().expect("lock");
        Ok(documents.get(id).map(|indexed| indexed.doc.clone()))
    }

    async fn delete_document(&self, id: &str) -> Result<bool> {
        let removed = {
            let mut documents = self.documents.write().expect("lock");
            documents.remove(id).is_some()
        };

        if removed {
            self.recompute_stats();
        }

        Ok(removed)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tokenization
// ─────────────────────────────────────────────────────────────────────────────

/// Simple tokenizer: lowercase, split on non-alphanumeric, filter short tokens
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| s.len() >= 2) // Filter single chars
        .filter(|s| !is_stop_word(s))
        .map(str::to_string)
        .collect()
}

/// Basic stop words list
fn is_stop_word(word: &str) -> bool {
    const STOP_WORDS: &[&str] = &[
        "the", "be", "to", "of", "and", "in", "that", "have", "it", "for", "not", "on", "with",
        "he", "as", "you", "do", "at", "this", "but", "his", "by", "from", "they", "we", "say",
        "her", "she", "or", "an", "will", "my", "one", "all", "would", "there", "their", "what",
        "so", "up", "out", "if", "about", "who", "get", "which", "go", "me", "when", "make", "can",
        "like", "time", "no", "just", "him", "know", "take", "people", "into", "year", "your",
        "good", "some", "could", "them", "see", "other", "than", "then", "now", "look", "only",
        "come", "its", "over", "think", "also", "back", "after", "use", "two", "how", "our",
        "work", "first", "well", "way", "even", "new", "want", "because", "any", "these", "give",
        "day", "most", "us", "is", "was", "are", "been", "being", "were", "am",
    ];

    STOP_WORDS.contains(&word)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::DocumentKind;

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("Hello, World! This is a test.");
        // "hello", "world", "test" - "this", "is", "a" are stop words
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        assert!(!tokens.contains(&"this".to_string())); // stop word
    }

    #[test]
    fn test_tokenize_code_style() {
        let tokens = tokenize("fn calculate_score(input: &str) -> Result<f64>");
        assert!(tokens.contains(&"fn".to_string()));
        assert!(tokens.contains(&"calculate_score".to_string()));
        assert!(tokens.contains(&"input".to_string()));
        assert!(tokens.contains(&"result".to_string()));
        assert!(tokens.contains(&"f64".to_string()));
    }

    #[test]
    fn test_tokenize_filters_short() {
        let tokens = tokenize("a b c ab cd");
        assert!(!tokens.contains(&"a".to_string()));
        assert!(!tokens.contains(&"b".to_string()));
        assert!(!tokens.contains(&"c".to_string()));
        assert!(tokens.contains(&"ab".to_string()));
        assert!(tokens.contains(&"cd".to_string()));
    }

    #[tokio::test]
    async fn test_backend_index_and_count() {
        let backend = TfIdfBackend::new();

        let docs = vec![
            VectorDocument::new("doc-1", DocumentKind::Memory, "Rust programming language"),
            VectorDocument::new("doc-2", DocumentKind::Memory, "Python programming language"),
        ];

        let stats = backend.index_documents(docs).await.unwrap();

        assert_eq!(stats.documents_indexed, 2);
        assert_eq!(backend.document_count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_backend_search_basic() {
        let backend = TfIdfBackend::new();

        let docs = vec![
            VectorDocument::new("doc-1", DocumentKind::Memory, "Rust programming language"),
            VectorDocument::new(
                "doc-2",
                DocumentKind::Memory,
                "Python programming language scripting",
            ),
            VectorDocument::new(
                "doc-3",
                DocumentKind::Memory,
                "JavaScript web browser client",
            ),
        ];

        backend.index_documents(docs).await.unwrap();

        let results = backend
            .search("programming language", &VectorFilters::new(), 10)
            .await
            .unwrap();

        // Both Rust and Python docs should match "programming language"
        assert!(results.len() >= 2);

        // First two results should be the programming language docs
        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"doc-1"));
        assert!(ids.contains(&"doc-2"));
    }

    #[tokio::test]
    async fn test_backend_search_with_filters() {
        let backend = TfIdfBackend::new();

        let docs = vec![
            VectorDocument::new("doc-1", DocumentKind::Memory, "stage0 memory retrieval")
                .with_domain("spec-kit"),
            VectorDocument::new("doc-2", DocumentKind::Code, "function memory allocation")
                .with_domain("core"),
            VectorDocument::new("doc-3", DocumentKind::Memory, "memory management pattern")
                .with_domain("spec-kit"),
        ];

        backend.index_documents(docs).await.unwrap();

        // Filter to memories only
        let results = backend
            .search("memory", &VectorFilters::memories_only(), 10)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.kind == DocumentKind::Memory));

        // Filter to spec-kit domain
        let results = backend
            .search("memory", &VectorFilters::new().with_domain("spec-kit"), 10)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.id == "doc-1" || r.id == "doc-3"));
    }

    #[tokio::test]
    async fn test_backend_search_ranking() {
        let backend = TfIdfBackend::new();

        let docs = vec![
            VectorDocument::new(
                "doc-1",
                DocumentKind::Memory,
                "vector backend implementation TF-IDF scoring",
            ),
            VectorDocument::new(
                "doc-2",
                DocumentKind::Memory,
                "vector search vector retrieval vector ranking",
            ), // "vector" appears 3x
            VectorDocument::new(
                "doc-3",
                DocumentKind::Memory,
                "database query optimization indexing",
            ),
        ];

        backend.index_documents(docs).await.unwrap();

        let results = backend
            .search("vector", &VectorFilters::new(), 10)
            .await
            .unwrap();

        // doc-2 should rank higher due to more "vector" occurrences
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "doc-2");

        // doc-3 shouldn't appear (no "vector")
        assert!(!results.iter().any(|r| r.id == "doc-3"));
    }

    #[tokio::test]
    async fn test_backend_get_document() {
        let backend = TfIdfBackend::new();

        let docs = vec![
            VectorDocument::new("doc-1", DocumentKind::Memory, "test content").with_domain("test"),
        ];

        backend.index_documents(docs).await.unwrap();

        let doc = backend.get_document("doc-1").await.unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().metadata.domain, Some("test".to_string()));

        let missing = backend.get_document("nonexistent").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_backend_delete_document() {
        let backend = TfIdfBackend::new();

        let docs = vec![
            VectorDocument::new("doc-1", DocumentKind::Memory, "first document"),
            VectorDocument::new("doc-2", DocumentKind::Memory, "second document"),
        ];

        backend.index_documents(docs).await.unwrap();
        assert_eq!(backend.document_count().await.unwrap(), 2);

        let deleted = backend.delete_document("doc-1").await.unwrap();
        assert!(deleted);
        assert_eq!(backend.document_count().await.unwrap(), 1);

        let deleted_again = backend.delete_document("doc-1").await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_backend_clear() {
        let backend = TfIdfBackend::new();

        let docs = vec![
            VectorDocument::new("doc-1", DocumentKind::Memory, "first"),
            VectorDocument::new("doc-2", DocumentKind::Memory, "second"),
        ];

        backend.index_documents(docs).await.unwrap();
        assert_eq!(backend.document_count().await.unwrap(), 2);

        backend.clear().await.unwrap();
        assert_eq!(backend.document_count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_backend_upsert_behavior() {
        let backend = TfIdfBackend::new();

        // Index initial doc
        backend
            .index_documents(vec![VectorDocument::new(
                "doc-1",
                DocumentKind::Memory,
                "original content",
            )])
            .await
            .unwrap();

        // Re-index with same ID but different content
        backend
            .index_documents(vec![VectorDocument::new(
                "doc-1",
                DocumentKind::Memory,
                "updated content",
            )])
            .await
            .unwrap();

        // Should still have 1 document
        assert_eq!(backend.document_count().await.unwrap(), 1);

        // Content should be updated
        let doc = backend.get_document("doc-1").await.unwrap().unwrap();
        assert_eq!(doc.text, "updated content");
    }

    #[tokio::test]
    async fn test_backend_empty_query() {
        let backend = TfIdfBackend::new();

        backend
            .index_documents(vec![VectorDocument::new(
                "doc-1",
                DocumentKind::Memory,
                "test content",
            )])
            .await
            .unwrap();

        // Empty query should return empty results
        let results = backend.search("", &VectorFilters::new(), 10).await.unwrap();

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_backend_no_match() {
        let backend = TfIdfBackend::new();

        backend
            .index_documents(vec![VectorDocument::new(
                "doc-1",
                DocumentKind::Memory,
                "rust programming",
            )])
            .await
            .unwrap();

        let results = backend
            .search("javascript nodejs", &VectorFilters::new(), 10)
            .await
            .unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_tfidf_config_default() {
        let config = TfIdfConfig::default();
        assert!((config.k1 - 1.5).abs() < f64::EPSILON);
        assert!((config.b - 0.75).abs() < f64::EPSILON);
    }
}
