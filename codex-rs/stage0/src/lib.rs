//! Stage 0 Overlay Engine for SPEC-KIT
//!
//! SPEC-KIT-102: Provides memory retrieval, dynamic scoring, and Tier 2
//! (NotebookLM) synthesis for `/speckit.auto` workflows.
//!
//! This crate implements an overlay layer that:
//! - Sits between `codex-rs` and the closed-source `local-memory` daemon
//! - Maintains its own SQLite overlay DB (scores, structure status, Tier 2 cache)
//! - Implements guardians, DCC, dynamic scoring, and Tier 2 orchestration
//! - Does NOT modify local-memory internals
//!
//! See the Stage 0 spec documents in the repo root for full architecture details.

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod config;
pub mod errors;
pub mod guardians;
pub mod overlay_db;

pub use config::Stage0Config;
pub use errors::{ErrorCategory, Result, Stage0Error};
pub use guardians::{
    GuardedMemory, LlmClient, MemoryDraft, MemoryKind, apply_metadata_guardian,
    apply_template_guardian, apply_template_guardian_passthrough,
};
pub use overlay_db::{OverlayDb, OverlayMemory, StructureStatus, Tier2CacheEntry};

use sha2::{Digest, Sha256};

/// Stage 0 version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Main entry point for Stage 0 operations
pub struct Stage0Engine {
    /// Configuration
    cfg: Stage0Config,
    /// Overlay database
    db: OverlayDb,
}

impl Stage0Engine {
    /// Create a new Stage0Engine, loading config and initializing the overlay DB
    pub fn new() -> Result<Self> {
        let cfg = Stage0Config::load()?;

        if !cfg.enabled {
            tracing::info!("Stage0 is disabled via configuration");
        }

        let db = OverlayDb::connect_and_init(&cfg)?;

        tracing::info!(
            version = VERSION,
            db_path = %cfg.resolved_db_path().display(),
            tier2_enabled = cfg.tier2.enabled,
            "Stage0 engine initialized"
        );

        Ok(Self { cfg, db })
    }

    /// Create a Stage0Engine with a specific config (for testing)
    pub fn with_config(cfg: Stage0Config) -> Result<Self> {
        let db = OverlayDb::connect_and_init(&cfg)?;
        Ok(Self { cfg, db })
    }

    /// Create a Stage0Engine with an in-memory database (for testing)
    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let cfg = Stage0Config::default();
        let db = OverlayDb::connect_in_memory()?;
        Ok(Self { cfg, db })
    }

    /// Check if Stage0 is enabled
    pub fn is_enabled(&self) -> bool {
        self.cfg.enabled
    }

    /// Get a reference to the configuration
    pub fn config(&self) -> &Stage0Config {
        &self.cfg
    }

    /// Get a reference to the overlay database
    pub fn db(&self) -> &OverlayDb {
        &self.db
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // V1.2: Guardian methods
    // ─────────────────────────────────────────────────────────────────────────────

    /// Apply both MetadataGuardian and TemplateGuardian to a memory draft
    ///
    /// This is the main entry point for memory ingestion:
    /// 1. Validates metadata (timestamps, agent type, priority)
    /// 2. Classifies content and restructures into template format
    ///
    /// Returns a `GuardedMemory` ready to be written to local-memory
    /// and recorded in the overlay DB.
    pub async fn guard_memory<L: LlmClient>(
        &self,
        llm: &L,
        draft: MemoryDraft,
    ) -> Result<GuardedMemory> {
        let now = chrono::Utc::now();
        let base = apply_metadata_guardian(&self.cfg, &draft, now)?;
        let guarded = apply_template_guardian(llm, base).await?;
        Ok(guarded)
    }

    /// Apply MetadataGuardian only, skipping LLM template processing
    ///
    /// Useful when:
    /// - LLM is unavailable
    /// - Quick ingestion without restructuring is needed
    /// - Testing without LLM dependencies
    pub fn guard_memory_sync(&self, draft: MemoryDraft) -> Result<GuardedMemory> {
        let now = chrono::Utc::now();
        let base = apply_metadata_guardian(&self.cfg, &draft, now)?;
        let guarded = apply_template_guardian_passthrough(base);
        Ok(guarded)
    }

    /// Record a guarded memory in the overlay DB
    ///
    /// Call this after successfully writing to local-memory to track
    /// the memory in the overlay for scoring and caching.
    pub fn record_guarded_memory(&self, memory_id: &str, guarded: &GuardedMemory) -> Result<()> {
        self.db.upsert_overlay_memory(
            memory_id,
            guarded.kind,
            guarded.created_at,
            guarded.initial_priority,
            &guarded.content_raw,
        )
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Placeholder methods for future phases (V1.3+)
    // ─────────────────────────────────────────────────────────────────────────────

    // V1.4: Dynamic Context Compiler (DCC)
    // pub async fn run_dcc(&self, spec_content: &str) -> Result<TaskBrief> { ... }

    // V1.5: Full Stage 0 run (DCC + Tier 2)
    // pub async fn run_stage0(&self, input: Stage0Input) -> Result<Stage0Result> { ... }
}

/// Compute SHA-256 hash of input, returning hex string
pub fn compute_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Compute combined cache key hash from spec and brief
pub fn compute_cache_key(spec_content: &str, brief_content: &str) -> String {
    let spec_hash = compute_hash(spec_content);
    let brief_hash = compute_hash(brief_content);
    compute_hash(&format!("{spec_hash}{brief_hash}"))
}

// Need hex encoding for hashes
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let bytes = bytes.as_ref();
        let mut s = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            s.push(HEX_CHARS[(b >> 4) as usize] as char);
            s.push(HEX_CHARS[(b & 0xf) as usize] as char);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_in_memory() {
        let engine = Stage0Engine::in_memory().expect("should create");
        assert!(engine.is_enabled());
        assert_eq!(engine.db().memory_count().expect("count"), 0);
    }

    #[test]
    fn test_compute_hash() {
        let hash = compute_hash("hello world");
        assert_eq!(hash.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_compute_cache_key() {
        let key1 = compute_cache_key("spec A", "brief B");
        let key2 = compute_cache_key("spec A", "brief C");
        let key3 = compute_cache_key("spec A", "brief B");

        assert_ne!(key1, key2); // Different briefs
        assert_eq!(key1, key3); // Same inputs = same key
    }
}
