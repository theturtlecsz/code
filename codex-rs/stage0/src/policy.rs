//! SPEC-KIT-977: PolicySnapshot — Capture at Boundaries
//!
//! PolicySnapshot captures the active policy configuration at run boundaries
//! for traceability and reproducibility.
//!
//! ## Decision IDs
//! - D100: JSON format compiled from human-readable source
//! - D101: Dual storage (filesystem + capsule)
//! - D102: Events tagged with policy_id for traceability
//!
//! ## Storage Locations
//! - Filesystem: `.speckit/policies/snapshot-<POLICY_ID>.json`
//! - Capsule: `mv2://.../policy/<POLICY_ID>`

use crate::config::{ScoringWeights, Stage0Config};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// PolicySnapshot captures the active policy at a point in time.
///
/// ## SPEC-KIT-977 Requirements
/// - schema_version for forward compatibility
/// - policy_id (UUID) for unique identification
/// - hash (SHA256) of canonical JSON for integrity
/// - All scoring weights and model configuration
/// - Source file references for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySnapshot {
    /// Schema version for forward compatibility (e.g., "1.0")
    pub schema_version: String,

    /// Unique policy identifier (UUID v4)
    pub policy_id: String,

    /// SHA256 hash of canonical JSON (computed after creation)
    pub hash: String,

    /// When this snapshot was captured
    pub created_at: DateTime<Utc>,

    /// Model configuration (from stage0.toml or defaults)
    pub model_config: ModelConfig,

    /// Scoring weights used for memory ranking
    pub weights: ScoringWeights,

    /// Prompt templates (key -> template content)
    pub prompts: HashMap<String, String>,

    /// Source files this policy was derived from
    pub source_files: Vec<String>,
}

/// Model configuration captured in the snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelConfig {
    /// Maximum tokens for TASK_BRIEF
    pub max_tokens: usize,

    /// Number of memories to include
    pub top_k: usize,

    /// Pre-filter candidate limit
    pub pre_filter_limit: usize,

    /// MMR diversity lambda
    pub diversity_lambda: f32,

    /// Whether LLM IQO generation is enabled
    pub iqo_llm_enabled: bool,

    /// Whether hybrid retrieval is enabled
    pub hybrid_enabled: bool,

    /// Vector weight for hybrid scoring
    pub vector_weight: f32,

    /// Whether Tier 2 (NotebookLM) is enabled
    pub tier2_enabled: bool,

    /// Tier 2 cache TTL in hours
    pub tier2_cache_ttl_hours: u64,
}

/// Summary info for listing policy snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySnapshotInfo {
    /// Policy ID
    pub policy_id: String,

    /// When captured
    pub created_at: DateTime<Utc>,

    /// Hash (first 16 chars for display)
    pub hash_short: String,

    /// Source files count
    pub source_count: usize,
}

impl PolicySnapshot {
    /// Schema version constant
    pub const SCHEMA_VERSION: &'static str = "1.0";

    /// Create a new PolicySnapshot from the current Stage0Config.
    ///
    /// This is the primary capture method called at run start or stage boundaries.
    pub fn capture(config: &Stage0Config, source_files: Vec<String>) -> Self {
        let policy_id = Uuid::new_v4().to_string();
        let created_at = Utc::now();

        let model_config = ModelConfig {
            max_tokens: config.context_compiler.max_tokens,
            top_k: config.context_compiler.top_k,
            pre_filter_limit: config.context_compiler.pre_filter_limit,
            diversity_lambda: config.context_compiler.diversity_lambda,
            iqo_llm_enabled: config.context_compiler.iqo_llm_enabled,
            hybrid_enabled: config.context_compiler.hybrid_enabled,
            vector_weight: config.context_compiler.vector_weight,
            tier2_enabled: config.tier2.enabled,
            tier2_cache_ttl_hours: config.tier2.cache_ttl_hours,
        };

        let weights = config.scoring.weights.clone();

        // Create snapshot without hash first
        let mut snapshot = Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            policy_id,
            hash: String::new(), // Computed below
            created_at,
            model_config,
            weights,
            prompts: HashMap::new(), // Populated by caller if needed
            source_files,
        };

        // Compute hash of canonical JSON
        snapshot.hash = snapshot.compute_hash();

        snapshot
    }

    /// Compute SHA256 hash of the canonical JSON representation.
    fn compute_hash(&self) -> String {
        // Create a copy without the hash field for hashing
        let hashable = serde_json::json!({
            "schema_version": self.schema_version,
            "policy_id": self.policy_id,
            "created_at": self.created_at.to_rfc3339(),
            "model_config": self.model_config,
            "weights": self.weights,
            "prompts": self.prompts,
            "source_files": self.source_files,
        });

        let canonical = serde_json::to_string(&hashable).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        let result = hasher.finalize();
        hex_encode(&result)
    }

    /// Verify the hash matches the snapshot content.
    pub fn verify_hash(&self) -> bool {
        let computed = self.compute_hash();
        computed == self.hash
    }

    /// Get summary info for listing.
    pub fn info(&self) -> PolicySnapshotInfo {
        PolicySnapshotInfo {
            policy_id: self.policy_id.clone(),
            created_at: self.created_at,
            hash_short: self.hash.chars().take(16).collect(),
            source_count: self.source_files.len(),
        }
    }

    /// Serialize to canonical JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// =============================================================================
// PolicyStore — Filesystem storage (D101)
// =============================================================================

/// Filesystem storage for policy snapshots.
///
/// Stores snapshots in `.speckit/policies/snapshot-<POLICY_ID>.json`
pub struct PolicyStore {
    /// Base directory for policy storage
    base_path: PathBuf,
}

impl PolicyStore {
    /// Default policy directory
    pub const DEFAULT_DIR: &'static str = ".speckit/policies";

    /// Create a new PolicyStore at the default location.
    pub fn new() -> Self {
        Self {
            base_path: PathBuf::from(Self::DEFAULT_DIR),
        }
    }

    /// Create a PolicyStore at a custom path.
    pub fn with_path(path: impl AsRef<Path>) -> Self {
        Self {
            base_path: path.as_ref().to_path_buf(),
        }
    }

    /// Ensure the storage directory exists.
    pub fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.base_path)
    }

    /// Get the path for a policy snapshot.
    fn snapshot_path(&self, policy_id: &str) -> PathBuf {
        self.base_path.join(format!("snapshot-{}.json", policy_id))
    }

    /// Store a policy snapshot to disk.
    pub fn store(&self, snapshot: &PolicySnapshot) -> std::io::Result<PathBuf> {
        self.ensure_dir()?;

        let path = self.snapshot_path(&snapshot.policy_id);
        let json = snapshot.to_json().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;

        std::fs::write(&path, json)?;

        tracing::debug!(
            policy_id = %snapshot.policy_id,
            path = %path.display(),
            "Stored policy snapshot"
        );

        Ok(path)
    }

    /// Load a policy snapshot by ID.
    pub fn load(&self, policy_id: &str) -> std::io::Result<PolicySnapshot> {
        let path = self.snapshot_path(policy_id);
        let json = std::fs::read_to_string(&path)?;
        PolicySnapshot::from_json(&json).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })
    }

    /// List all policy snapshots.
    pub fn list(&self) -> std::io::Result<Vec<PolicySnapshotInfo>> {
        if !self.base_path.exists() {
            return Ok(Vec::new());
        }

        let mut infos = Vec::new();

        for entry in std::fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(snapshot) = PolicySnapshot::from_json(&json) {
                        infos.push(snapshot.info());
                    }
                }
            }
        }

        // Sort by created_at descending (newest first)
        infos.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(infos)
    }

    /// Get the latest policy snapshot.
    pub fn latest(&self) -> std::io::Result<Option<PolicySnapshot>> {
        let infos = self.list()?;

        if let Some(info) = infos.first() {
            let snapshot = self.load(&info.policy_id)?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    /// Delete a policy snapshot by ID.
    pub fn delete(&self, policy_id: &str) -> std::io::Result<()> {
        let path = self.snapshot_path(policy_id);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Get policy snapshot for a specific run (by run_id).
    ///
    /// This looks up the policy_id associated with a run from metadata.
    /// For now, returns the latest policy as placeholder.
    pub fn get_for_run(&self, _run_id: &str) -> std::io::Result<Option<PolicySnapshot>> {
        // TODO: Implement run -> policy_id mapping when run tracking is added
        self.latest()
    }
}

impl Default for PolicyStore {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Capture helpers
// =============================================================================

/// Capture a policy snapshot at run start.
///
/// This is the main entry point called by /speckit.auto Stage0.
pub fn capture_policy_snapshot(config: &Stage0Config) -> PolicySnapshot {
    // Collect source file paths
    let source_files = collect_source_files();

    PolicySnapshot::capture(config, source_files)
}

/// Collect paths of source files that contribute to policy.
fn collect_source_files() -> Vec<String> {
    let mut sources = Vec::new();

    // Check for stage0.toml
    let config_path = Stage0Config::canonical_config_path();
    if config_path.exists() {
        sources.push(config_path.to_string_lossy().into_owned());
    }

    // Check for MODEL-POLICY.md
    let policy_md = PathBuf::from("docs/MODEL-POLICY.md");
    if policy_md.exists() {
        sources.push(policy_md.to_string_lossy().into_owned());
    }

    // Check for model_policy.toml
    let policy_toml = PathBuf::from("model_policy.toml");
    if policy_toml.exists() {
        sources.push(policy_toml.to_string_lossy().into_owned());
    }

    sources
}

/// Get the policy snapshot for a run, loading from store or capturing fresh.
pub fn get_policy_for_run(
    run_id: &str,
    config: &Stage0Config,
) -> std::io::Result<PolicySnapshot> {
    let store = PolicyStore::new();

    // Try to get existing policy for this run
    if let Some(snapshot) = store.get_for_run(run_id)? {
        return Ok(snapshot);
    }

    // Capture fresh snapshot
    let snapshot = capture_policy_snapshot(config);

    // Store it
    store.store(&snapshot)?;

    Ok(snapshot)
}

// =============================================================================
// Hex encoding helper
// =============================================================================

fn hex_encode(bytes: &[u8]) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX_CHARS[(b >> 4) as usize] as char);
        s.push(HEX_CHARS[(b & 0xf) as usize] as char);
    }
    s
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_snapshot_capture() {
        let config = Stage0Config::default();
        let snapshot = PolicySnapshot::capture(&config, vec!["test.toml".to_string()]);

        assert_eq!(snapshot.schema_version, "1.0");
        assert!(!snapshot.policy_id.is_empty());
        assert!(!snapshot.hash.is_empty());
        assert_eq!(snapshot.hash.len(), 64); // SHA256 = 64 hex chars
        assert_eq!(snapshot.source_files, vec!["test.toml"]);
    }

    #[test]
    fn test_policy_snapshot_hash_verification() {
        let config = Stage0Config::default();
        let snapshot = PolicySnapshot::capture(&config, vec![]);

        // Hash should verify correctly
        assert!(snapshot.verify_hash());
    }

    #[test]
    fn test_policy_snapshot_hash_changes_on_modification() {
        let config = Stage0Config::default();
        let snapshot1 = PolicySnapshot::capture(&config, vec![]);
        let snapshot2 = PolicySnapshot::capture(&config, vec!["file.toml".to_string()]);

        // Different source files should produce different hashes
        assert_ne!(snapshot1.hash, snapshot2.hash);
    }

    #[test]
    fn test_policy_snapshot_json_roundtrip() {
        let config = Stage0Config::default();
        let original = PolicySnapshot::capture(&config, vec!["test.toml".to_string()]);

        let json = original.to_json().expect("serialize");
        let restored = PolicySnapshot::from_json(&json).expect("deserialize");

        assert_eq!(original.policy_id, restored.policy_id);
        assert_eq!(original.hash, restored.hash);
        assert_eq!(original.schema_version, restored.schema_version);
        assert_eq!(original.source_files, restored.source_files);
    }

    #[test]
    fn test_policy_snapshot_info() {
        let config = Stage0Config::default();
        let snapshot = PolicySnapshot::capture(&config, vec!["a.toml".to_string(), "b.md".to_string()]);
        let info = snapshot.info();

        assert_eq!(info.policy_id, snapshot.policy_id);
        assert_eq!(info.hash_short.len(), 16);
        assert_eq!(info.source_count, 2);
    }

    #[test]
    fn test_policy_store_lifecycle() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let store = PolicyStore::with_path(temp_dir.path().join("policies"));

        let config = Stage0Config::default();
        let snapshot = PolicySnapshot::capture(&config, vec!["test.toml".to_string()]);
        let policy_id = snapshot.policy_id.clone();

        // Store
        let path = store.store(&snapshot).expect("store");
        assert!(path.exists());

        // Load
        let loaded = store.load(&policy_id).expect("load");
        assert_eq!(loaded.policy_id, snapshot.policy_id);
        assert_eq!(loaded.hash, snapshot.hash);

        // List
        let infos = store.list().expect("list");
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].policy_id, policy_id);

        // Latest
        let latest = store.latest().expect("latest").expect("exists");
        assert_eq!(latest.policy_id, policy_id);

        // Delete
        store.delete(&policy_id).expect("delete");
        let infos = store.list().expect("list");
        assert!(infos.is_empty());
    }

    #[test]
    fn test_model_config_defaults() {
        let config = Stage0Config::default();
        let snapshot = PolicySnapshot::capture(&config, vec![]);

        // Verify model config matches Stage0Config defaults
        assert_eq!(snapshot.model_config.max_tokens, 8000);
        assert_eq!(snapshot.model_config.top_k, 15);
        assert!(snapshot.model_config.hybrid_enabled);
        assert!(snapshot.model_config.tier2_enabled);
    }

    #[test]
    fn test_scoring_weights_captured() {
        let config = Stage0Config::default();
        let snapshot = PolicySnapshot::capture(&config, vec![]);

        // Verify scoring weights are captured
        assert_eq!(snapshot.weights.usage, 0.30);
        assert_eq!(snapshot.weights.recency, 0.30);
        assert_eq!(snapshot.weights.priority, 0.25);
        assert_eq!(snapshot.weights.decay, 0.15);
    }

    #[test]
    fn test_capture_policy_snapshot_function() {
        let config = Stage0Config::default();
        let snapshot = capture_policy_snapshot(&config);

        assert!(!snapshot.policy_id.is_empty());
        assert_eq!(snapshot.schema_version, "1.0");
        // source_files depends on actual filesystem state
    }
}
