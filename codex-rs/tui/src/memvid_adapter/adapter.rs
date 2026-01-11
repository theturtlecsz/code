//! SPEC-KIT-971: MemvidMemoryAdapter implementing Stage0 LocalMemoryClient
//!
//! This adapter wraps CapsuleHandle and provides the search_memories interface
//! that Stage0 expects. All Memvid concepts are isolated here.
//!
//! ## Architecture Rule (from kickoff)
//! > Stage0 core must not depend on Memvid concepts.
//! > Implement Memvid behind existing traits via an adapter.
//! > Any CapsuleStore abstraction is INTERNAL to the adapter crate/module only.

use crate::memvid_adapter::capsule::{CapsuleConfig, CapsuleError, CapsuleHandle};
use crate::memvid_adapter::types::{BranchId, CheckpointId, LogicalUri, ObjectType};
use async_trait::async_trait;
use codex_stage0::dcc::{LocalMemoryClient, LocalMemorySearchParams, LocalMemorySummary};
use codex_stage0::errors::Result as Stage0Result;
use std::sync::Arc;
use tokio::sync::RwLock;

// =============================================================================
// MemvidMemoryAdapter
// =============================================================================

/// Adapter implementing Stage0's LocalMemoryClient using Memvid capsules.
///
/// ## Design
/// - Wraps CapsuleHandle (internal)
/// - Implements LocalMemoryClient trait (external interface)
/// - Provides fallback to local-memory if capsule unavailable
///
/// ## Fallback Behavior (SPEC-KIT-971)
/// If capsule is missing/corrupt, system falls back to local-memory
/// and records evidence.
pub struct MemvidMemoryAdapter {
    /// The capsule handle (if open)
    capsule: Arc<RwLock<Option<CapsuleHandle>>>,

    /// Configuration
    config: CapsuleConfig,

    /// Fallback client for local-memory
    fallback: Option<Arc<dyn LocalMemoryClient>>,

    /// Whether to use fallback mode
    use_fallback: Arc<RwLock<bool>>,
}

impl MemvidMemoryAdapter {
    /// Create a new adapter with the given configuration.
    pub fn new(config: CapsuleConfig) -> Self {
        Self {
            capsule: Arc::new(RwLock::new(None)),
            config,
            fallback: None,
            use_fallback: Arc::new(RwLock::new(false)),
        }
    }

    /// Create adapter with a fallback client.
    pub fn with_fallback(mut self, fallback: Arc<dyn LocalMemoryClient>) -> Self {
        self.fallback = Some(fallback);
        self
    }

    /// Open or create the capsule.
    ///
    /// Returns Ok(true) if capsule opened successfully.
    /// Returns Ok(false) if fallback mode activated.
    /// Returns Err if both capsule and fallback fail.
    pub async fn open(&self) -> Result<bool, CapsuleError> {
        match CapsuleHandle::open(self.config.clone()) {
            Ok(handle) => {
                *self.capsule.write().await = Some(handle);
                *self.use_fallback.write().await = false;
                Ok(true)
            }
            Err(e) => {
                tracing::warn!("Failed to open capsule, activating fallback: {}", e);
                if self.fallback.is_some() {
                    *self.use_fallback.write().await = true;
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Check if the adapter is using fallback mode.
    pub async fn is_fallback(&self) -> bool {
        *self.use_fallback.read().await
    }

    /// Get a reference to the capsule handle (if open).
    pub async fn capsule(&self) -> Option<tokio::sync::RwLockReadGuard<'_, Option<CapsuleHandle>>> {
        let guard = self.capsule.read().await;
        if guard.is_some() {
            Some(guard)
        } else {
            None
        }
    }

    // =========================================================================
    // Write operations (forwarded to CapsuleHandle)
    // =========================================================================

    /// Ingest an artifact into the capsule.
    ///
    /// Returns a stable `mv2://...` URI.
    pub async fn ingest(
        &self,
        spec_id: &str,
        run_id: &str,
        path: &str,
        data: Vec<u8>,
        metadata: serde_json::Value,
    ) -> Result<LogicalUri, CapsuleError> {
        let capsule = self.capsule.read().await;
        let handle = capsule.as_ref().ok_or(CapsuleError::NotOpen)?;
        handle.put(spec_id, run_id, ObjectType::Artifact, path, data, metadata)
    }

    /// Create a stage checkpoint.
    pub async fn commit_stage(
        &self,
        spec_id: &str,
        run_id: &str,
        stage: &str,
        commit_hash: Option<&str>,
    ) -> Result<CheckpointId, CapsuleError> {
        let capsule = self.capsule.read().await;
        let handle = capsule.as_ref().ok_or(CapsuleError::NotOpen)?;
        handle.commit_stage(spec_id, run_id, stage, commit_hash)
    }

    /// Create a manual checkpoint.
    pub async fn commit_manual(&self, label: &str) -> Result<CheckpointId, CapsuleError> {
        let capsule = self.capsule.read().await;
        let handle = capsule.as_ref().ok_or(CapsuleError::NotOpen)?;
        handle.commit_manual(label)
    }

    /// List checkpoints.
    pub async fn list_checkpoints(&self) -> Result<Vec<crate::memvid_adapter::types::CheckpointMetadata>, CapsuleError> {
        let capsule = self.capsule.read().await;
        let handle = capsule.as_ref().ok_or(CapsuleError::NotOpen)?;
        Ok(handle.list_checkpoints())
    }

    /// Switch branch.
    pub async fn switch_branch(&self, branch: BranchId) -> Result<(), CapsuleError> {
        let capsule = self.capsule.read().await;
        let handle = capsule.as_ref().ok_or(CapsuleError::NotOpen)?;
        handle.switch_branch(branch)
    }

    /// Resolve a URI (stub for now).
    pub async fn resolve_uri(
        &self,
        uri: &LogicalUri,
        branch: Option<&BranchId>,
        as_of: Option<&CheckpointId>,
    ) -> Result<crate::memvid_adapter::types::PhysicalPointer, CapsuleError> {
        let capsule = self.capsule.read().await;
        let handle = capsule.as_ref().ok_or(CapsuleError::NotOpen)?;
        handle.resolve_uri(uri, branch, as_of)
    }
}

// =============================================================================
// LocalMemoryClient implementation
// =============================================================================

#[async_trait]
impl LocalMemoryClient for MemvidMemoryAdapter {
    /// Search memories using the capsule's hybrid retrieval.
    ///
    /// ## Implementation Notes
    /// - If in fallback mode, delegates to fallback client
    /// - Otherwise, searches the capsule using IQO parameters
    /// - Returns results as LocalMemorySummary (Stage0's type)
    async fn search_memories(
        &self,
        params: LocalMemorySearchParams,
    ) -> Stage0Result<Vec<LocalMemorySummary>> {
        // Check fallback mode
        if *self.use_fallback.read().await {
            if let Some(fallback) = &self.fallback {
                return fallback.search_memories(params).await;
            }
            return Ok(Vec::new());
        }

        // Get capsule handle
        let capsule = self.capsule.read().await;
        let Some(_handle) = capsule.as_ref() else {
            // No capsule, try fallback
            if let Some(fallback) = &self.fallback {
                return fallback.search_memories(params).await;
            }
            return Ok(Vec::new());
        };

        // TODO: Implement actual hybrid search using memvid
        // For now, return empty results (stub)
        //
        // Full implementation (SPEC-KIT-972) will:
        // 1. Parse IQO domains, keywords, tags
        // 2. Run lexical search (BM25)
        // 3. Run vector search (BGE-M3)
        // 4. Fuse results with explainable scoring
        // 5. Return as LocalMemorySummary

        tracing::debug!(
            "MemvidMemoryAdapter::search_memories (stub): keywords={:?}, max_results={}",
            params.iqo.keywords,
            params.max_results
        );

        Ok(Vec::new())
    }
}

// =============================================================================
// Factory function for creating adapter
// =============================================================================

/// Create a MemvidMemoryAdapter from environment/config.
///
/// ## Config Switch (SPEC-KIT-971)
/// Uses `memory_backend = memvid | local-memory` config.
pub fn create_memvid_adapter(
    capsule_path: Option<std::path::PathBuf>,
    workspace_id: Option<String>,
    fallback: Option<Arc<dyn LocalMemoryClient>>,
) -> MemvidMemoryAdapter {
    let config = CapsuleConfig {
        capsule_path: capsule_path.unwrap_or_else(|| {
            std::path::PathBuf::from(".speckit/memvid/workspace.mv2")
        }),
        workspace_id: workspace_id.unwrap_or_else(|| "default".to_string()),
        ..Default::default()
    };

    let adapter = MemvidMemoryAdapter::new(config);

    if let Some(fb) = fallback {
        adapter.with_fallback(fb)
    } else {
        adapter
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod adapter_tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_adapter_open_creates_capsule() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        let config = CapsuleConfig {
            capsule_path: capsule_path.clone(),
            workspace_id: "test".to_string(),
            ..Default::default()
        };

        let adapter = MemvidMemoryAdapter::new(config);
        let result = adapter.open().await;

        assert!(result.is_ok());
        assert!(result.unwrap()); // true = capsule opened, not fallback
        assert!(capsule_path.exists());
    }

    #[tokio::test]
    async fn test_adapter_put_returns_stable_uri() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        let config = CapsuleConfig {
            capsule_path,
            workspace_id: "ws1".to_string(),
            ..Default::default()
        };

        let adapter = MemvidMemoryAdapter::new(config);
        adapter.open().await.unwrap();

        let uri = adapter
            .ingest(
                "SPEC-971",
                "run1",
                "spec.md",
                b"# Test".to_vec(),
                serde_json::json!({}),
            )
            .await
            .unwrap();

        assert!(uri.is_valid());
        assert!(uri.as_str().starts_with("mv2://"));
        assert!(uri.as_str().contains("SPEC-971"));
    }

    #[tokio::test]
    async fn test_adapter_checkpoint_creates_event() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        let config = CapsuleConfig {
            capsule_path,
            workspace_id: "ws1".to_string(),
            ..Default::default()
        };

        let adapter = MemvidMemoryAdapter::new(config);
        adapter.open().await.unwrap();

        // Ingest something first
        adapter
            .ingest(
                "SPEC-971",
                "run1",
                "spec.md",
                b"# Test".to_vec(),
                serde_json::json!({}),
            )
            .await
            .unwrap();

        // Create stage checkpoint
        let checkpoint_id = adapter
            .commit_stage("SPEC-971", "run1", "plan", None)
            .await
            .unwrap();

        // List checkpoints
        let checkpoints = adapter.list_checkpoints().await.unwrap();

        assert!(!checkpoints.is_empty());
        assert_eq!(checkpoints[0].checkpoint_id.as_str(), checkpoint_id.as_str());
    }
}
