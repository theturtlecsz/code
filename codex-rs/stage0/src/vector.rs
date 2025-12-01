//! VectorBackend trait and types for Stage 0 memory retrieval
//!
//! SPEC-KIT-102 V2: Provides an abstraction layer for vector-based
//! semantic search over memories and other indexed content.
//!
//! This module defines:
//! - `VectorDocument`: Documents to be indexed
//! - `VectorFilters`: Query-time filtering options
//! - `ScoredVector`: Search results with relevance scores
//! - `VectorBackend`: Trait for pluggable vector backends

use crate::errors::{Result, Stage0Error};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Document kind enum for type-safe filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    /// Local-memory entry
    Memory,
    /// Code snippet or function
    Code,
    /// Spec document fragment
    Spec,
    /// Architecture decision record
    Adr,
    /// Generic/other document type
    Other,
}

impl DocumentKind {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Code => "code",
            Self::Spec => "spec",
            Self::Adr => "adr",
            Self::Other => "other",
        }
    }

    /// Parse from string value
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "memory" => Self::Memory,
            "code" => Self::Code,
            "spec" => Self::Spec,
            "adr" => Self::Adr,
            _ => Self::Other,
        }
    }
}

impl Default for DocumentKind {
    fn default() -> Self {
        Self::Other
    }
}

impl std::fmt::Display for DocumentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Document to be indexed in the vector backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    /// Unique identifier (e.g., local-memory UUID)
    pub id: String,

    /// Document kind for filtering
    pub kind: DocumentKind,

    /// Raw text content to be indexed/embedded
    pub text: String,

    /// Additional metadata for filtering and retrieval
    #[serde(default)]
    pub metadata: DocumentMetadata,
}

impl VectorDocument {
    /// Create a new VectorDocument
    pub fn new(id: impl Into<String>, kind: DocumentKind, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            text: text.into(),
            metadata: DocumentMetadata::default(),
        }
    }

    /// Create with metadata
    pub fn with_metadata(mut self, metadata: DocumentMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add a tag to the document
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.metadata.tags.push(tag.into());
        self
    }

    /// Set the domain
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.metadata.domain = Some(domain.into());
        self
    }
}

/// Metadata associated with a document
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Tags for categorical filtering
    #[serde(default)]
    pub tags: Vec<String>,

    /// Knowledge domain (e.g., "spec-kit", "stage0")
    #[serde(default)]
    pub domain: Option<String>,

    /// Source file path (for code/spec documents)
    #[serde(default)]
    pub source_path: Option<String>,

    /// Creation timestamp (ISO 8601)
    #[serde(default)]
    pub created_at: Option<String>,

    /// Dynamic score from overlay DB
    #[serde(default)]
    pub overlay_score: Option<f64>,

    /// Arbitrary extra fields
    #[serde(default, flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl DocumentMetadata {
    /// Create new empty metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set domain
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Set source path
    pub fn with_source_path(mut self, path: impl Into<String>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    /// Set overlay score
    pub fn with_overlay_score(mut self, score: f64) -> Self {
        self.overlay_score = Some(score);
        self
    }
}

/// Query-time filters for vector search
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorFilters {
    /// Filter by document kinds (empty = all kinds)
    #[serde(default)]
    pub kinds: Vec<DocumentKind>,

    /// Filter by tags (match any)
    #[serde(default)]
    pub tags: Vec<String>,

    /// Filter by domain
    #[serde(default)]
    pub domain: Option<String>,

    /// Minimum overlay score threshold
    #[serde(default)]
    pub min_overlay_score: Option<f64>,
}

impl VectorFilters {
    /// Create empty filters (matches everything)
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter to specific kinds
    pub fn with_kinds(mut self, kinds: Vec<DocumentKind>) -> Self {
        self.kinds = kinds;
        self
    }

    /// Filter to memories only
    pub fn memories_only() -> Self {
        Self {
            kinds: vec![DocumentKind::Memory],
            ..Default::default()
        }
    }

    /// Add tag filter
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set domain filter
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Set minimum overlay score
    pub fn with_min_score(mut self, score: f64) -> Self {
        self.min_overlay_score = Some(score);
        self
    }

    /// Check if a document matches these filters
    pub fn matches(&self, doc: &VectorDocument) -> bool {
        // Kind filter
        if !self.kinds.is_empty() && !self.kinds.contains(&doc.kind) {
            return false;
        }

        // Tag filter (match any)
        if !self.tags.is_empty() {
            let has_matching_tag = self.tags.iter().any(|t| doc.metadata.tags.contains(t));
            if !has_matching_tag {
                return false;
            }
        }

        // Domain filter
        if self.domain.is_some() && doc.metadata.domain.as_ref() != self.domain.as_ref() {
            return false;
        }

        // Min score filter
        if let Some(min_score) = self.min_overlay_score {
            if let Some(score) = doc.metadata.overlay_score {
                if score < min_score {
                    return false;
                }
            } else {
                // No score = doesn't pass minimum score filter
                return false;
            }
        }

        true
    }
}

/// Search result with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredVector {
    /// Document ID
    pub id: String,

    /// Relevance score (higher = more relevant)
    pub score: f64,

    /// Document kind
    pub kind: DocumentKind,

    /// Associated metadata
    pub metadata: DocumentMetadata,
}

impl ScoredVector {
    /// Create a new scored result
    pub fn new(id: impl Into<String>, score: f64, kind: DocumentKind) -> Self {
        Self {
            id: id.into(),
            score,
            kind,
            metadata: DocumentMetadata::default(),
        }
    }

    /// With metadata
    pub fn with_metadata(mut self, metadata: DocumentMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Statistics from indexing operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    /// Number of documents indexed
    pub documents_indexed: usize,

    /// Number of unique tokens
    pub unique_tokens: usize,

    /// Total tokens across all documents
    pub total_tokens: usize,

    /// Indexing duration in milliseconds
    pub duration_ms: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// VectorBackend Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Trait for pluggable vector search backends
///
/// Implementations can range from simple in-memory TF-IDF to
/// full vector databases like Qdrant or pgvector.
#[async_trait]
pub trait VectorBackend: Send + Sync {
    /// Index a batch of documents
    ///
    /// Implementations should handle deduplication (updating existing docs
    /// with the same ID) and compute any necessary indices/embeddings.
    async fn index_documents(&self, docs: Vec<VectorDocument>) -> Result<IndexStats>;

    /// Search for documents matching the query
    ///
    /// # Arguments
    /// * `query_text` - Natural language query
    /// * `filters` - Filtering options
    /// * `top_k` - Maximum number of results to return
    ///
    /// # Returns
    /// Vec of scored results, sorted by score descending
    async fn search(
        &self,
        query_text: &str,
        filters: &VectorFilters,
        top_k: usize,
    ) -> Result<Vec<ScoredVector>>;

    /// Get document count
    async fn document_count(&self) -> Result<usize>;

    /// Clear all indexed documents
    async fn clear(&self) -> Result<()>;

    /// Get a document by ID
    async fn get_document(&self, id: &str) -> Result<Option<VectorDocument>>;

    /// Delete a document by ID
    async fn delete_document(&self, id: &str) -> Result<bool>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Error helpers
// ─────────────────────────────────────────────────────────────────────────────

impl Stage0Error {
    /// Create a vector backend error
    pub fn vector(message: impl Into<String>) -> Self {
        // Use Internal for now since we don't have a dedicated Vector category
        // This keeps the error module changes minimal
        Self::Internal {
            message: format!("vector backend: {}", message.into()),
            source: None,
        }
    }

    /// Create a vector backend error with source
    pub fn vector_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Internal {
            message: format!("vector backend: {}", message.into()),
            source: Some(Box::new(source)),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::uninlined_format_args)]
mod tests {
    use super::*;

    #[test]
    fn test_document_kind_roundtrip() {
        let kinds = [
            DocumentKind::Memory,
            DocumentKind::Code,
            DocumentKind::Spec,
            DocumentKind::Adr,
            DocumentKind::Other,
        ];

        for kind in kinds {
            let s = kind.as_str();
            let parsed = DocumentKind::from_str(s);
            assert_eq!(kind, parsed, "Failed roundtrip for {:?}", kind);
        }
    }

    #[test]
    fn test_vector_document_builder() {
        let doc = VectorDocument::new("doc-1", DocumentKind::Memory, "test content")
            .with_domain("spec-kit")
            .with_tag("type:pattern");

        assert_eq!(doc.id, "doc-1");
        assert_eq!(doc.kind, DocumentKind::Memory);
        assert_eq!(doc.text, "test content");
        assert_eq!(doc.metadata.domain, Some("spec-kit".to_string()));
        assert!(doc.metadata.tags.contains(&"type:pattern".to_string()));
    }

    #[test]
    fn test_vector_filters_matches_kind() {
        let doc = VectorDocument::new("doc-1", DocumentKind::Memory, "test");
        let filters = VectorFilters::memories_only();

        assert!(filters.matches(&doc));

        let code_doc = VectorDocument::new("doc-2", DocumentKind::Code, "test");
        assert!(!filters.matches(&code_doc));
    }

    #[test]
    fn test_vector_filters_matches_tag() {
        let doc = VectorDocument::new("doc-1", DocumentKind::Memory, "test")
            .with_tag("type:bug")
            .with_tag("spec:SPEC-KIT-102");

        let filters = VectorFilters::new().with_tag("type:bug");
        assert!(filters.matches(&doc));

        let filters_no_match = VectorFilters::new().with_tag("type:pattern");
        assert!(!filters_no_match.matches(&doc));
    }

    #[test]
    fn test_vector_filters_matches_domain() {
        let doc =
            VectorDocument::new("doc-1", DocumentKind::Memory, "test").with_domain("spec-kit");

        let filters = VectorFilters::new().with_domain("spec-kit");
        assert!(filters.matches(&doc));

        let filters_no_match = VectorFilters::new().with_domain("other");
        assert!(!filters_no_match.matches(&doc));
    }

    #[test]
    fn test_vector_filters_matches_min_score() {
        let meta = DocumentMetadata::new().with_overlay_score(0.8);
        let doc = VectorDocument::new("doc-1", DocumentKind::Memory, "test").with_metadata(meta);

        let filters = VectorFilters::new().with_min_score(0.5);
        assert!(filters.matches(&doc));

        let filters_too_high = VectorFilters::new().with_min_score(0.9);
        assert!(!filters_too_high.matches(&doc));
    }

    #[test]
    fn test_vector_filters_empty_matches_all() {
        let doc = VectorDocument::new("doc-1", DocumentKind::Memory, "test")
            .with_domain("any")
            .with_tag("any-tag");

        let filters = VectorFilters::new();
        assert!(filters.matches(&doc));
    }

    #[test]
    fn test_scored_vector_builder() {
        let meta = DocumentMetadata::new().with_domain("test");
        let scored = ScoredVector::new("doc-1", 0.95, DocumentKind::Memory).with_metadata(meta);

        assert_eq!(scored.id, "doc-1");
        assert!((scored.score - 0.95).abs() < f64::EPSILON);
        assert_eq!(scored.kind, DocumentKind::Memory);
        assert_eq!(scored.metadata.domain, Some("test".to_string()));
    }
}
