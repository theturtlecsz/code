//! Vector state management for Stage0
//!
//! SPEC-KIT-102 V2.5b: Provides shared TfIdfBackend for hybrid retrieval.
//!
//! This module manages a shared TF-IDF vector backend that:
//! - Is populated by `/stage0.index` command
//! - Is consumed by `run_stage0_blocking` for hybrid retrieval
//! - Uses Arc<RwLock<>> for thread-safe access

use codex_stage0::TfIdfBackend;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global shared vector state
///
/// This is lazily initialized and shared across the TUI.
/// The TfIdfBackend is ephemeral (in-memory only).
pub static VECTOR_STATE: Lazy<VectorState> = Lazy::new(VectorState::new);

/// Shared vector backend state
///
/// Holds the TfIdfBackend that is populated by `/stage0.index`
/// and consumed by Stage0 DCC for hybrid retrieval.
pub struct VectorState {
    /// The TF-IDF backend wrapped in Arc<RwLock> for async access
    backend: Arc<RwLock<Option<TfIdfBackend>>>,
    /// Statistics about the last indexing operation
    stats: Arc<RwLock<Option<IndexingStats>>>,
}

/// Statistics from the last indexing operation
#[derive(Debug, Clone)]
pub struct IndexingStats {
    /// Number of documents indexed
    pub doc_count: usize,
    /// Number of unique tokens
    pub unique_tokens: usize,
    /// Total tokens processed
    pub total_tokens: usize,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Timestamp of indexing
    pub indexed_at: chrono::DateTime<chrono::Utc>,
}

impl VectorState {
    /// Create a new empty VectorState
    pub fn new() -> Self {
        Self {
            backend: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(None)),
        }
    }

    /// Get a clone of the Arc for async access to the backend
    pub fn backend_handle(&self) -> Arc<RwLock<Option<TfIdfBackend>>> {
        Arc::clone(&self.backend)
    }

    /// Set the backend (called by /stage0.index)
    pub async fn set_backend(&self, backend: TfIdfBackend, stats: IndexingStats) {
        let mut lock = self.backend.write().await;
        *lock = Some(backend);
        drop(lock);

        let mut stats_lock = self.stats.write().await;
        *stats_lock = Some(stats);
    }

    /// Clear the backend (for testing or reset)
    pub async fn clear(&self) {
        let mut lock = self.backend.write().await;
        *lock = None;
        drop(lock);

        let mut stats_lock = self.stats.write().await;
        *stats_lock = None;
    }

    /// Check if a backend is available
    pub async fn has_backend(&self) -> bool {
        let lock = self.backend.read().await;
        lock.is_some()
    }

    /// Get the last indexing stats
    pub async fn get_stats(&self) -> Option<IndexingStats> {
        let lock = self.stats.read().await;
        lock.clone()
    }
}

impl Default for VectorState {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper that provides VectorBackend trait access to a locked backend
///
/// This is used to pass the shared backend to Stage0 without exposing
/// the locking mechanism.
pub struct SharedVectorBackend {
    backend: TfIdfBackend,
}

impl SharedVectorBackend {
    /// Create from a TfIdfBackend (takes ownership for the duration of use)
    pub fn new(backend: TfIdfBackend) -> Self {
        Self { backend }
    }

    /// Get a reference to the inner backend
    pub fn inner(&self) -> &TfIdfBackend {
        &self.backend
    }
}

// Re-export VectorBackend trait for convenience
pub use codex_stage0::VectorBackend;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vector_state_empty() {
        let state = VectorState::new();
        assert!(!state.has_backend().await);
        assert!(state.get_stats().await.is_none());
    }

    #[tokio::test]
    async fn test_vector_state_set_and_clear() {
        let state = VectorState::new();

        // Set backend
        let backend = TfIdfBackend::new();
        let stats = IndexingStats {
            doc_count: 10,
            unique_tokens: 100,
            total_tokens: 500,
            duration_ms: 50,
            indexed_at: chrono::Utc::now(),
        };

        state.set_backend(backend, stats.clone()).await;
        assert!(state.has_backend().await);

        let retrieved_stats = state.get_stats().await.unwrap();
        assert_eq!(retrieved_stats.doc_count, 10);

        // Clear
        state.clear().await;
        assert!(!state.has_backend().await);
        assert!(state.get_stats().await.is_none());
    }

    #[test]
    fn test_global_vector_state() {
        // Just verify the global state is accessible
        let _ = &*VECTOR_STATE;
    }
}
