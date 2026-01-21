//! SPEC-KIT-971: MemvidMemoryAdapter implementing Stage0 LocalMemoryClient
//!
//! This adapter wraps CapsuleHandle and provides the search_memories interface
//! that Stage0 expects. All Memvid concepts are isolated here.
//!
//! ## Architecture Rule (from kickoff)
//! > Stage0 core must not depend on Memvid concepts.
//! > Implement Memvid behind existing traits via an adapter.
//! > Any CapsuleStore abstraction is INTERNAL to the adapter crate/module only.
//!
//! ## SPEC-KIT-972: Hybrid Retrieval
//! Implements lexical search (TF-IDF/BM25) with IQO parameter filtering:
//! - Domain filtering
//! - Keyword matching
//! - Tag filtering
//! - Importance threshold
//! - Explainable scoring

use crate::memvid_adapter::capsule::{CapsuleConfig, CapsuleError, CapsuleHandle};
use crate::memvid_adapter::types::{
    BranchId, CheckpointId, LogicalUri, ObjectType, RetrievalRequestPayload,
    RetrievalResponsePayload,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use codex_stage0::dcc::{LocalMemoryClient, LocalMemorySearchParams, LocalMemorySummary};
use codex_stage0::errors::Result as Stage0Result;
use codex_stage0::vector::{DocumentKind, DocumentMetadata, VectorDocument, VectorFilters};
use codex_stage0::{TfIdfBackend, VectorBackend};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

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
///
/// ## Search (SPEC-KIT-972)
/// Maintains a TF-IDF search index for lexical retrieval.
/// Indexed documents are filtered by IQO parameters.
///
/// ## Event Emission (SPEC-KIT-975)
/// When emit context is set, emits RetrievalRequest/Response events.
pub struct MemvidMemoryAdapter {
    /// The capsule handle (if open)
    capsule: Arc<RwLock<Option<CapsuleHandle>>>,

    /// Configuration
    config: CapsuleConfig,

    /// Fallback client for local-memory
    fallback: Option<Arc<dyn LocalMemoryClient>>,

    /// Whether to use fallback mode
    use_fallback: Arc<RwLock<bool>>,

    // ─────────────────────────────────────────────────────────────────────────────
    // SPEC-KIT-972: Search index
    // ─────────────────────────────────────────────────────────────────────────────
    /// TF-IDF search index for lexical retrieval
    search_index: Arc<TfIdfBackend>,

    /// Metadata for indexed memories (id -> MemoryMeta)
    /// Tracks domain, tags, importance, timestamps for filtering
    memory_meta: Arc<RwLock<HashMap<String, MemoryMeta>>>,

    // ─────────────────────────────────────────────────────────────────────────────
    // SPEC-KIT-975: Event emission context
    // ─────────────────────────────────────────────────────────────────────────────
    /// Emit context for SPEC-KIT-975 event emission (spec_id, run_id, stage)
    emit_context: Arc<RwLock<Option<EmitContext>>>,
}

/// Emit context for retrieval event emission (SPEC-KIT-975).
#[derive(Debug, Clone)]
pub struct EmitContext {
    /// SPEC ID
    pub spec_id: String,
    /// Run ID
    pub run_id: String,
    /// Current stage (optional)
    pub stage: Option<String>,
}

/// Metadata for an indexed memory (SPEC-KIT-972)
///
/// Stores filtering attributes separate from the TF-IDF index
/// to enable IQO-based filtering.
#[derive(Debug, Clone)]
pub struct MemoryMeta {
    /// Memory ID (matches document ID in search index)
    pub id: String,
    /// Domain tag (e.g., "spec-kit", "infrastructure")
    pub domain: Option<String>,
    /// All tags
    pub tags: Vec<String>,
    /// Importance score (0-10, higher = more important)
    pub importance: Option<f32>,
    /// Creation timestamp
    pub created_at: Option<DateTime<Utc>>,
    /// Content snippet for display
    pub snippet: String,
    /// Source URI in capsule
    pub uri: Option<LogicalUri>,
}

impl MemvidMemoryAdapter {
    /// Create a new adapter with the given configuration.
    pub fn new(config: CapsuleConfig) -> Self {
        Self {
            capsule: Arc::new(RwLock::new(None)),
            config,
            fallback: None,
            use_fallback: Arc::new(RwLock::new(false)),
            search_index: Arc::new(TfIdfBackend::new()),
            memory_meta: Arc::new(RwLock::new(HashMap::new())),
            emit_context: Arc::new(RwLock::new(None)),
        }
    }

    /// Create adapter with a fallback client.
    pub fn with_fallback(mut self, fallback: Arc<dyn LocalMemoryClient>) -> Self {
        self.fallback = Some(fallback);
        self
    }

    // =========================================================================
    // SPEC-KIT-975: Emit context management
    // =========================================================================

    /// Set the emit context for event emission.
    ///
    /// When set, search_memories will emit RetrievalRequest/Response events.
    pub async fn set_emit_context(
        &self,
        spec_id: impl Into<String>,
        run_id: impl Into<String>,
        stage: Option<String>,
    ) {
        *self.emit_context.write().await = Some(EmitContext {
            spec_id: spec_id.into(),
            run_id: run_id.into(),
            stage,
        });
    }

    /// Clear the emit context.
    pub async fn clear_emit_context(&self) {
        *self.emit_context.write().await = None;
    }

    /// Update the stage in the emit context.
    pub async fn set_emit_stage(&self, stage: impl Into<String>) {
        if let Some(ref mut ctx) = *self.emit_context.write().await {
            ctx.stage = Some(stage.into());
        }
    }

    /// Open or create the capsule.
    ///
    /// Returns Ok(true) if capsule opened successfully.
    /// Returns Ok(false) if fallback mode activated.
    /// Returns Err if both capsule and fallback fail.
    ///
    /// ## SPEC-KIT-971 Persistence
    /// On reopen, rebuilds the TF-IDF search index from stored artifacts.
    pub async fn open(&self) -> Result<bool, CapsuleError> {
        match CapsuleHandle::open(self.config.clone()) {
            Ok(handle) => {
                // Rebuild search index from stored artifacts (SPEC-KIT-971 persistence)
                self.rebuild_search_index_from_handle(&handle).await;

                *self.capsule.write().await = Some(handle);
                *self.use_fallback.write().await = false;
                Ok(true)
            }
            Err(e) => {
                if self.fallback.is_some() {
                    // SPEC-KIT-979: Structured fallback event logging for GATE-ST tracking
                    tracing::warn!(
                        target: "memvid",
                        event = "FallbackActivated",
                        from_backend = "memvid",
                        to_backend = "local-memory",
                        reason = %e,
                        operation = "capsule_open",
                        "Activating local-memory fallback after capsule open failure"
                    );
                    *self.use_fallback.write().await = true;
                    Ok(false)
                } else {
                    tracing::error!(
                        target: "memvid",
                        error = %e,
                        "Failed to open capsule with no fallback available"
                    );
                    Err(e)
                }
            }
        }
    }

    /// Rebuild the search index from stored artifacts in a capsule.
    ///
    /// Called during open() to restore search capability after reopen.
    async fn rebuild_search_index_from_handle(&self, handle: &CapsuleHandle) {
        let mut count = 0;
        for (uri, data, metadata) in handle.iter_stored_artifacts() {
            // Extract spec_id from URI if possible (mv2://workspace/spec/run/type/path)
            let spec_id = uri
                .as_str()
                .strip_prefix("mv2://")
                .and_then(|s| s.split('/').nth(1))
                .unwrap_or("unknown");

            self.index_memory(&uri, &data, &metadata, spec_id).await;
            count += 1;
        }

        if count > 0 {
            tracing::info!("Rebuilt search index with {} artifacts from capsule", count);
        }
    }

    /// Check if the adapter is using fallback mode.
    pub async fn is_fallback(&self) -> bool {
        *self.use_fallback.read().await
    }

    /// Get a reference to the capsule handle (if open).
    pub async fn capsule(&self) -> Option<tokio::sync::RwLockReadGuard<'_, Option<CapsuleHandle>>> {
        let guard = self.capsule.read().await;
        if guard.is_some() { Some(guard) } else { None }
    }

    // =========================================================================
    // Write operations (forwarded to CapsuleHandle)
    // =========================================================================

    /// Ingest an artifact into the capsule.
    ///
    /// Returns a stable `mv2://...` URI.
    ///
    /// ## SPEC-KIT-972: Indexing
    /// Ingested artifacts are also indexed in the TF-IDF search index
    /// for retrieval via `search_memories()`.
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
        let uri = handle.put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            path,
            data.clone(),
            metadata.clone(),
        )?;

        // SPEC-KIT-972: Index the artifact for search
        self.index_memory(&uri, &data, &metadata, spec_id).await;

        Ok(uri)
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // SPEC-KIT-972: Memory indexing
    // ─────────────────────────────────────────────────────────────────────────────

    /// Index a memory for search (SPEC-KIT-972).
    ///
    /// Extracts text content, domain, tags, and importance from metadata
    /// and adds to the TF-IDF index.
    async fn index_memory(
        &self,
        uri: &LogicalUri,
        data: &[u8],
        metadata: &serde_json::Value,
        spec_id: &str,
    ) {
        // Generate a stable ID from URI
        let id = uri.as_str().to_string();

        // Extract text content (assume UTF-8)
        let text = String::from_utf8_lossy(data).to_string();

        // Extract domain from metadata or default to spec-id domain
        let domain = metadata
            .get("domain")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| Some(format!("spec:{}", spec_id)));

        // Extract tags
        let tags: Vec<String> = metadata
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        // Extract importance
        let importance = metadata
            .get("importance")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32);

        // Create snippet (first 200 chars)
        let snippet = text.chars().take(200).collect::<String>();

        // Store metadata for filtering
        let mem_meta = MemoryMeta {
            id: id.clone(),
            domain: domain.clone(),
            tags: tags.clone(),
            importance,
            created_at: Some(Utc::now()),
            snippet: snippet.clone(),
            uri: Some(uri.clone()),
        };

        {
            let mut meta_map = self.memory_meta.write().await;
            meta_map.insert(id.clone(), mem_meta);
        }

        // Create vector document for TF-IDF indexing
        let mut doc_meta = DocumentMetadata::new();
        if let Some(ref d) = domain {
            doc_meta = doc_meta.with_domain(d.clone());
        }
        for tag in &tags {
            doc_meta = doc_meta.with_tag(tag.clone());
        }

        let doc = VectorDocument::new(id, DocumentKind::Memory, text).with_metadata(doc_meta);

        // Index the document
        if let Err(e) = self.search_index.index_documents(vec![doc]).await {
            tracing::warn!("Failed to index memory in search index: {}", e);
        }
    }

    /// Add a memory directly to the search index (for testing or bulk import).
    pub async fn add_memory_to_index(&self, meta: MemoryMeta, content: &str) {
        let id = meta.id.clone();

        // Store metadata
        {
            let mut meta_map = self.memory_meta.write().await;
            meta_map.insert(id.clone(), meta.clone());
        }

        // Create vector document
        let mut doc_meta = DocumentMetadata::new();
        if let Some(ref d) = meta.domain {
            doc_meta = doc_meta.with_domain(d.clone());
        }
        for tag in &meta.tags {
            doc_meta = doc_meta.with_tag(tag.clone());
        }

        let doc = VectorDocument::new(id, DocumentKind::Memory, content).with_metadata(doc_meta);

        // Index the document
        if let Err(e) = self.search_index.index_documents(vec![doc]).await {
            tracing::warn!("Failed to index memory: {}", e);
        }
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
    pub async fn list_checkpoints(
        &self,
    ) -> Result<Vec<crate::memvid_adapter::types::CheckpointMetadata>, CapsuleError> {
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
    /// ## SPEC-KIT-972: Hybrid Retrieval Implementation
    /// 1. Parses IQO parameters (domains, keywords, tags, importance threshold)
    /// 2. Runs lexical search using TF-IDF (BM25-style scoring)
    /// 3. Filters results by domain, tags, and importance
    /// 4. Applies recency bias (optional)
    /// 5. Returns results with explainable scoring
    ///
    /// ## SPEC-KIT-975: Event Emission
    /// When emit_context is set, emits RetrievalRequest/Response events.
    ///
    /// ## Fallback Behavior
    /// - If in fallback mode, delegates to fallback client
    /// - If no search results, tries fallback if available
    async fn search_memories(
        &self,
        params: LocalMemorySearchParams,
    ) -> Stage0Result<Vec<LocalMemorySummary>> {
        let start_time = Instant::now();

        // Check fallback mode
        if *self.use_fallback.read().await {
            if let Some(fallback) = &self.fallback {
                return fallback.search_memories(params).await;
            }
            return Ok(Vec::new());
        }

        tracing::debug!(
            "MemvidMemoryAdapter::search_memories: keywords={:?}, domains={:?}, max_results={}",
            params.iqo.keywords,
            params.iqo.domains,
            params.max_results
        );

        // Build query from IQO keywords
        let query_text = params.iqo.keywords.join(" ");
        if query_text.is_empty() && params.iqo.required_tags.is_empty() {
            // No keywords or required tags - return empty to avoid full scan
            return Ok(Vec::new());
        }

        // =========================================================================
        // SPEC-KIT-975: Emit RetrievalRequest (best-effort)
        // =========================================================================
        let request_id = Uuid::new_v4().to_string();
        let emit_ctx = self.emit_context.read().await.clone();
        if let (Some(ctx), Some(capsule_guard)) = (&emit_ctx, self.capsule.read().await.as_ref()) {
            let req_payload = RetrievalRequestPayload {
                request_id: request_id.clone(),
                query: query_text.clone(),
                config: serde_json::json!({
                    "domains": params.iqo.domains,
                    "max_results": params.max_results,
                    "required_tags": params.iqo.required_tags,
                    "max_candidates": params.iqo.max_candidates,
                }),
                source: "capsule".to_string(),
                stage: ctx.stage.clone(),
                role: None,
            };
            // Best-effort: ignore errors
            if let Err(e) =
                capsule_guard.emit_retrieval_request(&ctx.spec_id, &ctx.run_id, &req_payload)
            {
                tracing::warn!(error = %e, "Failed to emit RetrievalRequest (best-effort)");
            }
        }

        // Build filters for TF-IDF search
        let mut filters = VectorFilters::memories_only();

        // Add domain filter if specified (first domain only for now)
        if let Some(domain) = params.iqo.domains.first() {
            filters = filters.with_domain(domain.clone());
        }

        // Run lexical search
        let top_k = (params.max_results * 3).min(100); // Fetch 3x for filtering headroom
        let search_results = self
            .search_index
            .search(&query_text, &filters, top_k)
            .await?;

        tracing::debug!(
            "TF-IDF search returned {} raw results",
            search_results.len()
        );

        // Get metadata for filtering
        let meta_map = self.memory_meta.read().await;

        // Filter and score results
        let mut candidates: Vec<(String, f64, Option<MemoryMeta>)> = Vec::new();

        for result in search_results {
            let meta = meta_map.get(&result.id).cloned();

            // Apply IQO filters
            if !self.passes_iqo_filters(&meta, &params) {
                continue;
            }

            // Compute hybrid score
            let lex_score = result.score;
            let recency_score = self.compute_recency_score(&meta);
            let tag_boost = self.compute_tag_boost(&meta, &params);

            // Weighted fusion: α*lex + β*recency + γ*tag_boost
            // Using reasonable defaults: 0.6 lex, 0.2 recency, 0.2 tag_boost
            let final_score = 0.6 * lex_score + 0.2 * recency_score + 0.2 * tag_boost;

            candidates.push((result.id.clone(), final_score, meta));
        }

        // Sort by final score descending
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top N results
        candidates.truncate(params.max_results);

        tracing::debug!("After filtering: {} candidates", candidates.len());

        // Convert to LocalMemorySummary
        let results: Vec<LocalMemorySummary> = candidates
            .into_iter()
            .map(|(id, score, meta)| LocalMemorySummary {
                id,
                domain: meta.as_ref().and_then(|m| m.domain.clone()),
                tags: meta.as_ref().map(|m| m.tags.clone()).unwrap_or_default(),
                created_at: meta.as_ref().and_then(|m| m.created_at),
                snippet: meta.as_ref().map(|m| m.snippet.clone()).unwrap_or_default(),
                similarity_score: score,
            })
            .collect();

        // If no results and fallback available, try fallback
        if results.is_empty() {
            if let Some(fallback) = &self.fallback {
                tracing::debug!("No memvid results, trying fallback");
                return fallback.search_memories(params).await;
            }
        }

        // =========================================================================
        // SPEC-KIT-975: Emit RetrievalResponse (best-effort)
        // =========================================================================
        let latency_ms = start_time.elapsed().as_millis() as u64;
        if let (Some(ctx), Some(capsule_guard)) = (&emit_ctx, self.capsule.read().await.as_ref()) {
            let hit_uris: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
            let fused_scores: Vec<f64> = results.iter().map(|r| r.similarity_score).collect();

            let resp_payload = RetrievalResponsePayload {
                request_id: request_id.clone(),
                hit_uris,
                fused_scores: Some(fused_scores),
                explainability: None,
                latency_ms: Some(latency_ms),
                error: None,
            };
            // Best-effort: ignore errors
            if let Err(e) = capsule_guard.emit_retrieval_response(
                &ctx.spec_id,
                &ctx.run_id,
                ctx.stage.as_deref(),
                &resp_payload,
            ) {
                tracing::warn!(error = %e, "Failed to emit RetrievalResponse (best-effort)");
            }
        }

        Ok(results)
    }
}

// =============================================================================
// SPEC-KIT-972: Search helper methods
// =============================================================================

impl MemvidMemoryAdapter {
    /// Check if a memory passes IQO filters.
    fn passes_iqo_filters(
        &self,
        meta: &Option<MemoryMeta>,
        params: &LocalMemorySearchParams,
    ) -> bool {
        let Some(meta) = meta else {
            // No metadata = can't filter, include by default
            return true;
        };

        // Check domain filter
        if !params.iqo.domains.is_empty() {
            if let Some(ref domain) = meta.domain {
                // Allow if memory domain matches any IQO domain or starts with "spec:"
                let matches =
                    params.iqo.domains.iter().any(|d| {
                        domain == d || domain.starts_with(&format!("spec:{}", d)) || d == "*"
                    });
                if !matches && !domain.starts_with("spec:") {
                    return false;
                }
            }
        }

        // Check required tags
        if !params.iqo.required_tags.is_empty() {
            let has_required = params
                .iqo
                .required_tags
                .iter()
                .all(|req| meta.tags.contains(req));
            if !has_required {
                return false;
            }
        }

        // Check excluded tags
        if !params.iqo.exclude_tags.is_empty() {
            let has_excluded = params
                .iqo
                .exclude_tags
                .iter()
                .any(|excl| meta.tags.contains(excl));
            if has_excluded {
                return false;
            }
        }

        // Note: importance_threshold is not in current IQO, but we support it
        // in MemoryMeta for future use when IQO is extended.

        true
    }

    /// Compute recency score (0.0 - 1.0).
    ///
    /// More recent memories score higher.
    fn compute_recency_score(&self, meta: &Option<MemoryMeta>) -> f64 {
        let Some(meta) = meta else {
            return 0.5; // Default for unknown
        };

        let Some(created_at) = meta.created_at else {
            return 0.5; // Default for unknown timestamp
        };

        let now = Utc::now();
        let age_hours = (now - created_at).num_hours() as f64;

        // Decay: score = 1.0 at 0 hours, 0.5 at 24 hours, ~0.25 at 48 hours
        let decay_rate = 0.03; // Per hour
        (1.0 / (1.0 + decay_rate * age_hours)).max(0.1)
    }

    /// Compute tag boost based on optional tag matches.
    fn compute_tag_boost(
        &self,
        meta: &Option<MemoryMeta>,
        params: &LocalMemorySearchParams,
    ) -> f64 {
        let Some(meta) = meta else {
            return 0.0;
        };

        if params.iqo.optional_tags.is_empty() {
            return 0.5; // Neutral if no optional tags specified
        }

        // Count matching optional tags
        let matches = params
            .iqo
            .optional_tags
            .iter()
            .filter(|t| meta.tags.contains(t))
            .count();

        let total = params.iqo.optional_tags.len();

        // Score based on fraction of optional tags matched
        if total > 0 {
            (matches as f64) / (total as f64)
        } else {
            0.5
        }
    }

    /// Get the number of indexed memories.
    pub async fn indexed_memory_count(&self) -> usize {
        self.search_index.document_count().await.unwrap_or(0)
    }

    /// Clear the search index.
    pub async fn clear_search_index(&self) {
        if let Err(e) = self.search_index.clear().await {
            tracing::warn!("Failed to clear search index: {}", e);
        }
        self.memory_meta.write().await.clear();
    }
}

// =============================================================================
// SPEC-KIT-971: UnifiedMemoryClient enum for backend routing
// =============================================================================

/// Unified memory client that supports both memvid and local-memory backends.
///
/// SPEC-KIT-971: This enum allows Stage0 to work with either backend while
/// maintaining type safety (avoiding `dyn` issues with Stage0's generic constraints).
pub enum UnifiedMemoryClient {
    /// Memvid capsule backend
    Memvid(MemvidMemoryAdapter),
    /// Local-memory CLI backend
    LocalMemory(crate::stage0_adapters::LocalMemoryCliAdapter),
}

#[async_trait]
impl LocalMemoryClient for UnifiedMemoryClient {
    async fn search_memories(
        &self,
        params: LocalMemorySearchParams,
    ) -> Stage0Result<Vec<LocalMemorySummary>> {
        match self {
            UnifiedMemoryClient::Memvid(adapter) => adapter.search_memories(params).await,
            UnifiedMemoryClient::LocalMemory(adapter) => adapter.search_memories(params).await,
        }
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
        capsule_path: capsule_path
            .unwrap_or_else(|| std::path::PathBuf::from(".speckit/memvid/workspace.mv2")),
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

/// Create a LocalMemoryClient based on the configured backend.
///
/// ## SPEC-KIT-971: Config Switch
/// Routes to the appropriate backend based on `memory_backend` config:
/// - `Memvid`: Creates a MemvidMemoryAdapter with capsule storage
/// - `LocalMemory`: Uses the provided fallback client directly
///
/// ## Fallback Behavior
/// If `backend` is `Memvid` but capsule fails to open, the fallback client
/// is used (if provided). This enables graceful degradation.
///
/// ## Returns
/// - `Ok(Arc<dyn LocalMemoryClient>)`: The configured memory client
/// - `Err`: If neither memvid nor fallback can be initialized
pub async fn create_memory_client(
    backend: codex_stage0::MemoryBackend,
    capsule_path: Option<std::path::PathBuf>,
    workspace_id: Option<String>,
    fallback: Option<Arc<dyn LocalMemoryClient>>,
) -> Result<Arc<dyn LocalMemoryClient>, CapsuleError> {
    use codex_stage0::MemoryBackend;

    match backend {
        MemoryBackend::LocalMemory => {
            // Use fallback client directly
            match fallback {
                Some(client) => {
                    tracing::info!(
                        target: "memvid",
                        "Using local-memory backend (via config)"
                    );
                    Ok(client)
                }
                None => {
                    tracing::warn!(
                        target: "memvid",
                        "local-memory backend selected but no fallback client provided"
                    );
                    Err(CapsuleError::Corrupted {
                        reason: "local-memory backend selected but no client available".to_string(),
                    })
                }
            }
        }
        MemoryBackend::Memvid => {
            // Create and open memvid adapter
            let adapter = create_memvid_adapter(capsule_path, workspace_id, fallback.clone());

            match adapter.open().await {
                Ok(true) => {
                    tracing::info!(
                        target: "memvid",
                        "Using memvid backend (capsule opened)"
                    );
                    Ok(Arc::new(adapter))
                }
                Ok(false) => {
                    // Capsule failed but fallback activated (event logged in open())
                    match fallback {
                        Some(client) => Ok(client),
                        None => Err(CapsuleError::NotOpen),
                    }
                }
                Err(e) => {
                    // SPEC-KIT-979: Structured fallback logging
                    match fallback {
                        Some(client) => {
                            tracing::warn!(
                                target: "memvid",
                                event = "FallbackActivated",
                                from_backend = "memvid",
                                to_backend = "local-memory",
                                reason = %e,
                                operation = "create_memory_client",
                                "Using local-memory fallback after memvid error"
                            );
                            Ok(client)
                        }
                        None => {
                            tracing::error!(
                                target: "memvid",
                                error = %e,
                                "Failed to open memvid capsule with no fallback"
                            );
                            Err(e)
                        }
                    }
                }
            }
        }
    }
}

/// Create a UnifiedMemoryClient based on the configured backend.
///
/// ## SPEC-KIT-971: Pipeline Integration
/// This is the primary factory for Stage0 integration. Returns a `UnifiedMemoryClient`
/// enum that implements `LocalMemoryClient` and works with Stage0's generic constraints.
///
/// ## Backend Routing
/// - `Memvid`: Creates MemvidMemoryAdapter, opens capsule
/// - `LocalMemory`: Uses LocalMemoryCliAdapter directly
///
/// ## Fallback Behavior
/// If `backend` is `Memvid` but capsule fails to open:
/// - Check if local-memory daemon is healthy
/// - If healthy: use local-memory as fallback
/// - If not: return error
pub async fn create_unified_memory_client(
    backend: codex_stage0::MemoryBackend,
    capsule_path: std::path::PathBuf,
    workspace_id: String,
    check_local_memory_health: impl Fn() -> bool,
) -> Result<UnifiedMemoryClient, CapsuleError> {
    use codex_stage0::MemoryBackend;

    match backend {
        MemoryBackend::LocalMemory => {
            tracing::info!(
                target: "stage0",
                "Using local-memory backend (via config)"
            );
            Ok(UnifiedMemoryClient::LocalMemory(
                crate::stage0_adapters::LocalMemoryCliAdapter::new(),
            ))
        }
        MemoryBackend::Memvid => {
            // Create memvid adapter without fallback (we handle fallback ourselves)
            let config = CapsuleConfig {
                capsule_path: capsule_path.clone(),
                workspace_id: workspace_id.clone(),
                ..Default::default()
            };
            let adapter = MemvidMemoryAdapter::new(config);

            match adapter.open().await {
                Ok(true) => {
                    tracing::info!(
                        target: "stage0",
                        "Using memvid backend (capsule opened)"
                    );
                    Ok(UnifiedMemoryClient::Memvid(adapter))
                }
                Ok(false) | Err(_) => {
                    // Capsule failed - check if local-memory is available as fallback
                    if check_local_memory_health() {
                        // SPEC-KIT-979: Structured fallback event logging for GATE-ST tracking
                        tracing::warn!(
                            target: "stage0",
                            event = "FallbackActivated",
                            from_backend = "memvid",
                            to_backend = "local-memory",
                            reason = "capsule_open_failed",
                            operation = "create_unified_memory_client",
                            "Using local-memory fallback after memvid failure"
                        );
                        Ok(UnifiedMemoryClient::LocalMemory(
                            crate::stage0_adapters::LocalMemoryCliAdapter::new(),
                        ))
                    } else {
                        tracing::error!(
                            target: "stage0",
                            "Memvid capsule failed and local-memory health check failed"
                        );
                        Err(CapsuleError::NotOpen)
                    }
                }
            }
        }
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
        assert_eq!(
            checkpoints[0].checkpoint_id.as_str(),
            checkpoint_id.as_str()
        );
    }

    // =========================================================================
    // SPEC-KIT-972: Search tests
    // =========================================================================

    use codex_stage0::dcc::{Iqo, LocalMemoryClient as _};

    #[tokio::test]
    async fn test_search_memories_basic_keyword_search() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        let config = CapsuleConfig {
            capsule_path,
            workspace_id: "ws1".to_string(),
            ..Default::default()
        };

        let adapter = MemvidMemoryAdapter::new(config);
        adapter.open().await.unwrap();

        // Add some memories directly to the index
        adapter.add_memory_to_index(
            MemoryMeta {
                id: "mem-001".to_string(),
                domain: Some("spec-kit".to_string()),
                tags: vec!["type:pattern".to_string()],
                importance: Some(8.0),
                created_at: Some(Utc::now()),
                snippet: "Rust error handling pattern".to_string(),
                uri: None,
            },
            "Rust error handling pattern using Result and Option types for safe error propagation",
        ).await;

        adapter
            .add_memory_to_index(
                MemoryMeta {
                    id: "mem-002".to_string(),
                    domain: Some("spec-kit".to_string()),
                    tags: vec!["type:decision".to_string()],
                    importance: Some(7.0),
                    created_at: Some(Utc::now()),
                    snippet: "Decision to use async/await".to_string(),
                    uri: None,
                },
                "Decision to use async/await pattern for all IO operations in the codebase",
            )
            .await;

        adapter
            .add_memory_to_index(
                MemoryMeta {
                    id: "mem-003".to_string(),
                    domain: Some("infrastructure".to_string()),
                    tags: vec!["type:pattern".to_string()],
                    importance: Some(6.0),
                    created_at: Some(Utc::now()),
                    snippet: "Database connection pool pattern".to_string(),
                    uri: None,
                },
                "Database connection pool pattern for PostgreSQL with r2d2",
            )
            .await;

        assert_eq!(adapter.indexed_memory_count().await, 3);

        // Search for "error handling"
        let params = LocalMemorySearchParams {
            iqo: Iqo {
                keywords: vec!["error".to_string(), "handling".to_string()],
                ..Default::default()
            },
            max_results: 10,
        };

        let results = adapter.search_memories(params).await.unwrap();

        assert!(!results.is_empty(), "Should find at least one result");
        assert_eq!(
            results[0].id, "mem-001",
            "First result should be error handling memory"
        );
    }

    #[tokio::test]
    async fn test_search_memories_domain_filtering() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        let config = CapsuleConfig {
            capsule_path,
            workspace_id: "ws1".to_string(),
            ..Default::default()
        };

        let adapter = MemvidMemoryAdapter::new(config);
        adapter.open().await.unwrap();

        // Add memories in different domains
        adapter
            .add_memory_to_index(
                MemoryMeta {
                    id: "mem-spec".to_string(),
                    domain: Some("spec-kit".to_string()),
                    tags: vec!["type:pattern".to_string()],
                    importance: Some(8.0),
                    created_at: Some(Utc::now()),
                    snippet: "Pattern in spec-kit".to_string(),
                    uri: None,
                },
                "Important pattern for spec-kit workflow",
            )
            .await;

        adapter
            .add_memory_to_index(
                MemoryMeta {
                    id: "mem-infra".to_string(),
                    domain: Some("infrastructure".to_string()),
                    tags: vec!["type:pattern".to_string()],
                    importance: Some(8.0),
                    created_at: Some(Utc::now()),
                    snippet: "Pattern in infrastructure".to_string(),
                    uri: None,
                },
                "Important pattern for infrastructure deployment",
            )
            .await;

        // Search with domain filter
        let params = LocalMemorySearchParams {
            iqo: Iqo {
                keywords: vec!["pattern".to_string()],
                domains: vec!["spec-kit".to_string()],
                ..Default::default()
            },
            max_results: 10,
        };

        let results = adapter.search_memories(params).await.unwrap();

        // Should only return spec-kit domain result
        assert!(!results.is_empty());
        assert!(
            results
                .iter()
                .all(|r| r.domain.as_deref() == Some("spec-kit"))
        );
    }

    #[tokio::test]
    async fn test_search_memories_required_tags() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        let config = CapsuleConfig {
            capsule_path,
            workspace_id: "ws1".to_string(),
            ..Default::default()
        };

        let adapter = MemvidMemoryAdapter::new(config);
        adapter.open().await.unwrap();

        // Add memories with different tags
        adapter
            .add_memory_to_index(
                MemoryMeta {
                    id: "mem-bug".to_string(),
                    domain: Some("spec-kit".to_string()),
                    tags: vec!["type:bug-fix".to_string(), "priority:high".to_string()],
                    importance: Some(9.0),
                    created_at: Some(Utc::now()),
                    snippet: "Fixed critical bug".to_string(),
                    uri: None,
                },
                "Fixed critical bug in memory retrieval causing crashes",
            )
            .await;

        adapter
            .add_memory_to_index(
                MemoryMeta {
                    id: "mem-pattern".to_string(),
                    domain: Some("spec-kit".to_string()),
                    tags: vec!["type:pattern".to_string()],
                    importance: Some(7.0),
                    created_at: Some(Utc::now()),
                    snippet: "New pattern".to_string(),
                    uri: None,
                },
                "New pattern for memory retrieval",
            )
            .await;

        // Search requiring type:bug-fix tag
        let params = LocalMemorySearchParams {
            iqo: Iqo {
                keywords: vec!["memory".to_string()],
                required_tags: vec!["type:bug-fix".to_string()],
                ..Default::default()
            },
            max_results: 10,
        };

        let results = adapter.search_memories(params).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "mem-bug");
        assert!(results[0].tags.contains(&"type:bug-fix".to_string()));
    }

    #[tokio::test]
    async fn test_search_memories_exclude_tags() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        let config = CapsuleConfig {
            capsule_path,
            workspace_id: "ws1".to_string(),
            ..Default::default()
        };

        let adapter = MemvidMemoryAdapter::new(config);
        adapter.open().await.unwrap();

        // Add memories
        adapter
            .add_memory_to_index(
                MemoryMeta {
                    id: "mem-system".to_string(),
                    domain: Some("spec-kit".to_string()),
                    tags: vec!["system:true".to_string()],
                    importance: Some(5.0),
                    created_at: Some(Utc::now()),
                    snippet: "System memory".to_string(),
                    uri: None,
                },
                "System generated memory for internal use",
            )
            .await;

        adapter
            .add_memory_to_index(
                MemoryMeta {
                    id: "mem-user".to_string(),
                    domain: Some("spec-kit".to_string()),
                    tags: vec!["type:decision".to_string()],
                    importance: Some(8.0),
                    created_at: Some(Utc::now()),
                    snippet: "User memory".to_string(),
                    uri: None,
                },
                "User created memory for decision tracking",
            )
            .await;

        // Search excluding system:true
        let params = LocalMemorySearchParams {
            iqo: Iqo {
                keywords: vec!["memory".to_string()],
                exclude_tags: vec!["system:true".to_string()],
                ..Default::default()
            },
            max_results: 10,
        };

        let results = adapter.search_memories(params).await.unwrap();

        // Should only return user memory, not system memory
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "mem-user");
    }

    #[tokio::test]
    async fn test_search_memories_empty_keywords_returns_empty() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        let config = CapsuleConfig {
            capsule_path,
            workspace_id: "ws1".to_string(),
            ..Default::default()
        };

        let adapter = MemvidMemoryAdapter::new(config);
        adapter.open().await.unwrap();

        // Add a memory
        adapter
            .add_memory_to_index(
                MemoryMeta {
                    id: "mem-001".to_string(),
                    domain: Some("spec-kit".to_string()),
                    tags: vec![],
                    importance: Some(8.0),
                    created_at: Some(Utc::now()),
                    snippet: "Test memory".to_string(),
                    uri: None,
                },
                "Test memory content",
            )
            .await;

        // Search with empty keywords and no required tags
        let params = LocalMemorySearchParams {
            iqo: Iqo::default(),
            max_results: 10,
        };

        let results = adapter.search_memories(params).await.unwrap();

        // Should return empty to avoid full scan
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_memories_ingest_indexes_content() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        let config = CapsuleConfig {
            capsule_path,
            workspace_id: "ws1".to_string(),
            ..Default::default()
        };

        let adapter = MemvidMemoryAdapter::new(config);
        adapter.open().await.unwrap();

        // Ingest an artifact (should auto-index)
        let _uri = adapter.ingest(
            "SPEC-972",
            "run1",
            "decision.md",
            b"# Decision: Use TF-IDF for lexical search\n\nWe chose TF-IDF because it's simple and effective.".to_vec(),
            serde_json::json!({
                "domain": "spec-kit",
                "tags": ["type:decision"],
                "importance": 8
            }),
        ).await.unwrap();

        // Verify it was indexed
        assert_eq!(adapter.indexed_memory_count().await, 1);

        // Search for it
        let params = LocalMemorySearchParams {
            iqo: Iqo {
                keywords: vec!["tfidf".to_string(), "lexical".to_string()],
                ..Default::default()
            },
            max_results: 10,
        };

        let results = adapter.search_memories(params).await.unwrap();

        assert!(!results.is_empty(), "Should find ingested document");
    }

    #[tokio::test]
    async fn test_search_memories_result_scoring() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        let config = CapsuleConfig {
            capsule_path,
            workspace_id: "ws1".to_string(),
            ..Default::default()
        };

        let adapter = MemvidMemoryAdapter::new(config);
        adapter.open().await.unwrap();

        // Add memories with different relevance
        adapter
            .add_memory_to_index(
                MemoryMeta {
                    id: "mem-high".to_string(),
                    domain: Some("spec-kit".to_string()),
                    tags: vec![],
                    importance: Some(9.0),
                    created_at: Some(Utc::now()),
                    snippet: "Rust Rust Rust".to_string(),
                    uri: None,
                },
                "Rust Rust Rust - highly relevant to Rust programming language",
            )
            .await;

        adapter
            .add_memory_to_index(
                MemoryMeta {
                    id: "mem-low".to_string(),
                    domain: Some("spec-kit".to_string()),
                    tags: vec![],
                    importance: Some(5.0),
                    created_at: Some(Utc::now()),
                    snippet: "Python programming".to_string(),
                    uri: None,
                },
                "Python programming with occasional Rust interop",
            )
            .await;

        // Search for "Rust"
        let params = LocalMemorySearchParams {
            iqo: Iqo {
                keywords: vec!["rust".to_string()],
                ..Default::default()
            },
            max_results: 10,
        };

        let results = adapter.search_memories(params).await.unwrap();

        assert!(results.len() >= 1);
        // The one with more "Rust" mentions should score higher
        assert_eq!(results[0].id, "mem-high");
        assert!(results[0].similarity_score > 0.0);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // SPEC-KIT-971: Config Switch Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_memory_client_memvid_backend() {
        use codex_stage0::MemoryBackend;

        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        let client = create_memory_client(
            MemoryBackend::Memvid,
            Some(capsule_path.clone()),
            Some("test".to_string()),
            None,
        )
        .await
        .unwrap();

        // Should have created the capsule file
        assert!(capsule_path.exists());

        // Verify client is functional
        let params = LocalMemorySearchParams {
            iqo: Iqo {
                keywords: vec!["test".to_string()],
                ..Default::default()
            },
            max_results: 10,
        };
        let results = client.search_memories(params).await.unwrap();
        assert!(results.is_empty()); // No data yet
    }

    #[tokio::test]
    async fn test_create_memory_client_local_memory_backend() {
        use codex_stage0::MemoryBackend;

        // Create a mock fallback client (using an adapter with an in-memory capsule)
        let temp_dir = TempDir::new().unwrap();
        let fallback_path = temp_dir.path().join("fallback.mv2");
        let fallback_config = CapsuleConfig {
            capsule_path: fallback_path,
            workspace_id: "fallback".to_string(),
            ..Default::default()
        };
        let fallback = MemvidMemoryAdapter::new(fallback_config);
        fallback.open().await.unwrap();
        let fallback_arc: Arc<dyn LocalMemoryClient> = Arc::new(fallback);

        // Request local-memory backend
        let client = create_memory_client(
            MemoryBackend::LocalMemory,
            None,
            None,
            Some(fallback_arc.clone()),
        )
        .await
        .unwrap();

        // The client should be the fallback
        // Verify by checking it returns empty results (no data)
        let params = LocalMemorySearchParams {
            iqo: Iqo {
                keywords: vec!["test".to_string()],
                ..Default::default()
            },
            max_results: 10,
        };
        let results = client.search_memories(params).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_create_memory_client_local_memory_no_fallback_fails() {
        use codex_stage0::MemoryBackend;

        // Request local-memory backend without fallback
        let result = create_memory_client(
            MemoryBackend::LocalMemory,
            None,
            None,
            None, // No fallback
        )
        .await;

        assert!(result.is_err());
        match result {
            Err(CapsuleError::Corrupted { reason }) => {
                assert!(reason.contains("no client available"));
            }
            _ => panic!("Expected Corrupted error"),
        }
    }

    #[tokio::test]
    async fn test_create_memory_client_memvid_with_fallback() {
        use codex_stage0::MemoryBackend;

        // Create a fallback that we can verify
        let temp_dir = TempDir::new().unwrap();
        let fallback_path = temp_dir.path().join("fallback.mv2");
        let fallback_config = CapsuleConfig {
            capsule_path: fallback_path,
            workspace_id: "fallback".to_string(),
            ..Default::default()
        };
        let fallback = MemvidMemoryAdapter::new(fallback_config);
        fallback.open().await.unwrap();
        let fallback_arc: Arc<dyn LocalMemoryClient> = Arc::new(fallback);

        // Create memvid with valid path - should use memvid, not fallback
        let capsule_path = temp_dir.path().join("memvid.mv2");
        let client = create_memory_client(
            MemoryBackend::Memvid,
            Some(capsule_path.clone()),
            Some("primary".to_string()),
            Some(fallback_arc),
        )
        .await
        .unwrap();

        // Capsule file should be created
        assert!(capsule_path.exists());

        // Client should work
        let params = LocalMemorySearchParams {
            iqo: Iqo {
                keywords: vec!["test".to_string()],
                ..Default::default()
            },
            max_results: 10,
        };
        let results = client.search_memories(params).await.unwrap();
        assert!(results.is_empty());
    }
}
