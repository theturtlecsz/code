//! Stage0 configuration loading
//!
//! Loads configuration from `~/.config/code/stage0.toml` with fallback to
//! `~/.config/codex/stage0.toml` for backward compatibility.
//!
//! Environment variable overrides (checked in order):
//! 1. `CODE_STAGE0_CONFIG` (preferred)
//! 2. `CODEX_STAGE0_CONFIG` (legacy)
//!
//! See docs/stage0/STAGE0_CONFIG_AND_PROMPTS.md for the full schema.

use crate::errors::{Result, Stage0Error};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// P91/SPEC-KIT-105: Constitution Gate Mode
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// SPEC-KIT-971: Memory Backend Selection
// ─────────────────────────────────────────────────────────────────────────────

/// Memory backend for Stage0 retrieval
///
/// Controls which backend is used for local memory operations.
/// Per SPEC-KIT-971: "memory_backend = memvid | local-memory config"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryBackend {
    /// Use Memvid capsule for memory storage (default)
    #[default]
    Memvid,
    /// Use local-memory REST/CLI backend
    LocalMemory,
}

/// Mode for Phase -1 constitution readiness gate
///
/// Controls how /speckit.auto, /speckit.plan, and /speckit.new behave when
/// constitution is missing or incomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateMode {
    /// Warn about missing constitution but proceed (default)
    #[default]
    Warn,
    /// Skip gate check entirely (no warnings)
    Skip,
    /// Block pipeline execution when constitution is missing or incomplete (P92)
    Block,
}

/// Root configuration for Stage0 overlay engine
#[derive(Debug, Deserialize, Clone)]
pub struct Stage0Config {
    /// Master enable switch for Stage0
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Whether to log explain/debug info
    #[serde(default)]
    pub explain: bool,

    /// Path to overlay SQLite database
    #[serde(default = "default_db_path")]
    pub db_path: String,

    /// Ingestion settings
    #[serde(default)]
    pub ingestion: IngestionConfig,

    /// Dynamic scoring settings
    #[serde(default)]
    pub scoring: ScoringConfig,

    /// Context compiler (DCC) settings
    #[serde(default)]
    pub context_compiler: ContextCompilerConfig,

    /// Tier 2 (NotebookLM) settings
    #[serde(default)]
    pub tier2: Tier2Config,

    /// Vector index settings (V2.5b)
    #[serde(default)]
    pub vector_index: VectorIndexConfig,

    // ─────────────────────────────────────────────────────────────────────────────
    // P91/SPEC-KIT-105: Constitution gate settings
    // ─────────────────────────────────────────────────────────────────────────────
    /// Phase -1 readiness gate mode
    ///
    /// Controls behavior when constitution is missing before /speckit.auto, /speckit.plan, or /speckit.new
    /// - `warn`: Print warnings but proceed (default)
    /// - `skip`: No gate check at all
    /// - `block`: Abort pipeline execution when constitution is incomplete (P92)
    #[serde(default)]
    pub phase1_gate_mode: GateMode,

    // ─────────────────────────────────────────────────────────────────────────────
    // CONVERGENCE: System pointer memory settings
    // ─────────────────────────────────────────────────────────────────────────────
    /// Store Stage0 outputs as pointer memories in local-memory
    ///
    /// When true (default), Stage0 stores metadata pointers after completing:
    /// - domain: spec-tracker
    /// - tags: system:true, spec:<id>, stage:0, artifact:<type>, tier2:<status>
    /// - content: paths, hashes, summary (no raw Divine Truth)
    ///
    /// These enable traceability without polluting normal recall (excluded by system:true).
    #[serde(default = "default_store_system_pointers")]
    pub store_system_pointers: bool,

    // ─────────────────────────────────────────────────────────────────────────────
    // SPEC-KIT-971: Memory Backend Selection
    // ─────────────────────────────────────────────────────────────────────────────
    /// Memory backend to use for Tier1 retrieval
    ///
    /// Controls which backend is used for local memory operations:
    /// - `memvid`: Use Memvid capsule for memory storage (default)
    /// - `local-memory`: Use local-memory REST/CLI backend
    ///
    /// Per SPEC-KIT-971: "memory_backend = memvid | local-memory config"
    #[serde(default)]
    pub memory_backend: MemoryBackend,
}

fn default_enabled() -> bool {
    true
}

fn default_store_system_pointers() -> bool {
    true
}

fn default_db_path() -> String {
    dirs::home_dir()
        .map(|h| {
            // Prefer ~/.config/code/ path (canonical)
            let code_path = h
                .join(".config")
                .join("code")
                .join("local-memory-overlay.db");

            // Check if new path exists or old path doesn't exist (use new)
            let codex_path = h
                .join(".config")
                .join("codex")
                .join("local-memory-overlay.db");

            if code_path.exists() || !codex_path.exists() {
                code_path.to_string_lossy().into_owned()
            } else {
                // Fall back to legacy path if it exists and new doesn't
                codex_path.to_string_lossy().into_owned()
            }
        })
        .unwrap_or_else(|| "local-memory-overlay.db".to_string())
}

/// Ingestion configuration
#[derive(Debug, Deserialize, Clone)]
pub struct IngestionConfig {
    /// If true, normalize/require metadata for writes
    #[serde(default = "default_strict_metadata")]
    pub strict_metadata: bool,
}

fn default_strict_metadata() -> bool {
    true
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            strict_metadata: default_strict_metadata(),
        }
    }
}

/// Dynamic scoring configuration
#[derive(Debug, Deserialize, Clone)]
pub struct ScoringConfig {
    /// How often to recalculate scores (e.g., "6h0m0s")
    #[serde(default = "default_recalculation_interval")]
    pub recalculation_interval: String,

    /// Scoring weights
    #[serde(default)]
    pub weights: ScoringWeights,

    /// Usage count below which novelty boost applies
    #[serde(default = "default_novelty_boost_threshold")]
    pub novelty_boost_threshold: u32,

    /// Maximum novelty boost factor
    #[serde(default = "default_novelty_boost_factor_max")]
    pub novelty_boost_factor_max: f32,
}

fn default_recalculation_interval() -> String {
    "6h0m0s".to_string()
}

fn default_novelty_boost_threshold() -> u32 {
    5
}

fn default_novelty_boost_factor_max() -> f32 {
    0.5
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            recalculation_interval: default_recalculation_interval(),
            weights: ScoringWeights::default(),
            novelty_boost_threshold: default_novelty_boost_threshold(),
            novelty_boost_factor_max: default_novelty_boost_factor_max(),
        }
    }
}

/// Scoring weight distribution
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScoringWeights {
    /// Weight for usage count
    #[serde(default = "default_usage_weight")]
    pub usage: f32,

    /// Weight for recency
    #[serde(default = "default_recency_weight")]
    pub recency: f32,

    /// Weight for initial priority
    #[serde(default = "default_priority_weight")]
    pub priority: f32,

    /// Decay factor weight
    #[serde(default = "default_decay_weight")]
    pub decay: f32,
}

fn default_usage_weight() -> f32 {
    0.30
}
fn default_recency_weight() -> f32 {
    0.30
}
fn default_priority_weight() -> f32 {
    0.25
}
fn default_decay_weight() -> f32 {
    0.15
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            usage: default_usage_weight(),
            recency: default_recency_weight(),
            priority: default_priority_weight(),
            decay: default_decay_weight(),
        }
    }
}

/// Context compiler (DCC) configuration
#[derive(Debug, Deserialize, Clone)]
pub struct ContextCompilerConfig {
    /// Maximum tokens for TASK_BRIEF output
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,

    /// Number of top memories to include
    #[serde(default = "default_top_k")]
    pub top_k: usize,

    /// Weight for dynamic score in combined ranking
    #[serde(default = "default_dynamic_score_weight")]
    pub dynamic_score_weight: f32,

    /// Weight for semantic similarity in combined ranking
    #[serde(default = "default_semantic_similarity_weight")]
    pub semantic_similarity_weight: f32,

    /// Maximum candidates to consider before ranking
    #[serde(default = "default_pre_filter_limit")]
    pub pre_filter_limit: usize,

    /// MMR diversity lambda (0=pure relevance, 1=pure diversity)
    #[serde(default = "default_diversity_lambda")]
    pub diversity_lambda: f32,

    /// Whether to use LLM for IQO generation
    #[serde(default = "default_iqo_llm_enabled")]
    pub iqo_llm_enabled: bool,

    // ─────────────────────────────────────────────────────────────────────────────
    // V2.5: Hybrid retrieval configuration
    // ─────────────────────────────────────────────────────────────────────────────
    /// Enable hybrid retrieval (TF-IDF + local-memory)
    #[serde(default = "default_hybrid_enabled")]
    pub hybrid_enabled: bool,

    /// Weight for vector/TF-IDF score in combined ranking
    /// Combined formula: sim_weight*sim + dyn_weight*dyn + vec_weight*vec
    #[serde(default = "default_vector_weight")]
    pub vector_weight: f32,

    /// Maximum results from vector backend before merge
    #[serde(default = "default_vector_top_k")]
    pub vector_top_k: usize,

    // ─────────────────────────────────────────────────────────────────────────────
    // P85: Code lane configuration
    // ─────────────────────────────────────────────────────────────────────────────
    /// Enable code lane in TASK_BRIEF (requires indexed code units)
    #[serde(default = "default_code_lane_enabled")]
    pub code_lane_enabled: bool,

    /// Number of code units to include in TASK_BRIEF
    #[serde(default = "default_code_top_k")]
    pub code_top_k: usize,

    // ─────────────────────────────────────────────────────────────────────────────
    // ADR-003: Product Knowledge lane configuration
    // ─────────────────────────────────────────────────────────────────────────────
    /// Product Knowledge lane settings
    ///
    /// Controls retrieval of curated product knowledge from local-memory
    /// domain `codex-product` for inclusion in TASK_BRIEF.
    #[serde(default)]
    pub product_knowledge: ProductKnowledgeConfig,
}

fn default_max_tokens() -> usize {
    8000
}
fn default_top_k() -> usize {
    15
}
fn default_dynamic_score_weight() -> f32 {
    0.40
}
fn default_semantic_similarity_weight() -> f32 {
    0.60
}
fn default_pre_filter_limit() -> usize {
    150
}
fn default_diversity_lambda() -> f32 {
    0.70
}
fn default_iqo_llm_enabled() -> bool {
    true
}

// V2.5: Hybrid retrieval defaults
fn default_hybrid_enabled() -> bool {
    true
}
fn default_vector_weight() -> f32 {
    0.20
}
fn default_vector_top_k() -> usize {
    50
}

// P85: Code lane defaults
fn default_code_lane_enabled() -> bool {
    true
}
fn default_code_top_k() -> usize {
    10
}

impl Default for ContextCompilerConfig {
    fn default() -> Self {
        Self {
            max_tokens: default_max_tokens(),
            top_k: default_top_k(),
            dynamic_score_weight: default_dynamic_score_weight(),
            semantic_similarity_weight: default_semantic_similarity_weight(),
            pre_filter_limit: default_pre_filter_limit(),
            diversity_lambda: default_diversity_lambda(),
            iqo_llm_enabled: default_iqo_llm_enabled(),
            hybrid_enabled: default_hybrid_enabled(),
            vector_weight: default_vector_weight(),
            vector_top_k: default_vector_top_k(),
            code_lane_enabled: default_code_lane_enabled(),
            code_top_k: default_code_top_k(),
            product_knowledge: ProductKnowledgeConfig::default(),
        }
    }
}

/// Tier 2 (NotebookLM) configuration
#[derive(Debug, Deserialize, Clone)]
pub struct Tier2Config {
    /// Enable Tier 2 synthesis
    #[serde(default = "default_tier2_enabled")]
    pub enabled: bool,

    /// Notebook identifier (ID or URL) for the "Shadow Stage 0" notebook.
    ///
    /// notebooklm-mcp HTTP service accepts `notebook`, `notebook_id`, or `notebook_url`.
    #[serde(
        default,
        alias = "notebook_id_shadow",
        alias = "notebook_id",
        alias = "notebook_url"
    )]
    pub notebook: String,

    /// Optional HTTP service base URL (default: http://127.0.0.1:3456)
    #[serde(default)]
    pub base_url: Option<String>,

    /// Cache TTL in hours
    #[serde(default = "default_cache_ttl_hours")]
    pub cache_ttl_hours: u64,

    /// Deprecated (v2.0.0+): notebooklm-mcp no longer exposes MCP tools.
    /// Kept for backward-compatible config parsing only.
    #[serde(default)]
    pub mcp_tool_name: Option<String>,

    /// Call timeout (e.g., "30s")
    #[serde(default = "default_call_timeout")]
    pub call_timeout: String,
}

fn default_tier2_enabled() -> bool {
    true
}
fn default_cache_ttl_hours() -> u64 {
    24
}
fn default_call_timeout() -> String {
    "30s".to_string()
}

impl Default for Tier2Config {
    fn default() -> Self {
        Self {
            enabled: default_tier2_enabled(),
            notebook: String::new(),
            base_url: None,
            cache_ttl_hours: default_cache_ttl_hours(),
            mcp_tool_name: None,
            call_timeout: default_call_timeout(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// V2.5b: Vector Index Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Vector index configuration
///
/// Controls how many memories and code units are indexed into the TF-IDF vector backend
/// for hybrid retrieval.
#[derive(Debug, Deserialize, Clone)]
pub struct VectorIndexConfig {
    /// Maximum memories to index (0 = no limit, index all)
    /// When set, indexes top N by dynamic_score DESC
    #[serde(default = "default_vector_max_memories")]
    pub max_memories_to_index: usize,

    /// P85: Maximum code units to index (0 = no limit, index all)
    /// Code units are extracted from stage0/src/, tui/src/, core/src/
    #[serde(default = "default_vector_max_code_units")]
    pub max_code_units_to_index: usize,
}

fn default_vector_max_memories() -> usize {
    0 // No limit by default - index all
}

fn default_vector_max_code_units() -> usize {
    0 // No limit by default - index all
}

impl Default for VectorIndexConfig {
    fn default() -> Self {
        Self {
            max_memories_to_index: default_vector_max_memories(),
            max_code_units_to_index: default_vector_max_code_units(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ADR-003: Product Knowledge Lane Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Product Knowledge lane configuration (ADR-003)
///
/// Controls how Stage0 retrieves curated product knowledge from local-memory
/// domain `codex-product` and includes it in the TASK_BRIEF.
///
/// Feature is OFF by default. When enabled:
/// - Queries local-memory domain `codex-product` for curated insights
/// - Filters by importance >= 8 and canonical type tags
/// - Injects bounded markdown lane into TASK_BRIEF
/// - Snapshots used inputs to capsule for deterministic replay
#[derive(Debug, Deserialize, Clone)]
pub struct ProductKnowledgeConfig {
    /// Enable product knowledge lane (default: OFF)
    ///
    /// When enabled, Stage0 will query local-memory for curated product
    /// knowledge and include it in TASK_BRIEF context.
    #[serde(default)]
    pub enabled: bool,

    /// Domain to query (default: "codex-product")
    ///
    /// Must be the curated product knowledge domain, not the spec-kit domain.
    #[serde(default = "default_pk_domain")]
    pub domain: String,

    /// Maximum items to retrieve before filtering (default: 10)
    #[serde(default = "default_pk_max_items")]
    pub max_items: usize,

    /// Maximum characters per item content (default: 3000)
    #[serde(default = "default_pk_max_chars_per_item")]
    pub max_chars_per_item: usize,

    /// Maximum total characters for lane (default: 10000)
    #[serde(default = "default_pk_max_total_chars")]
    pub max_total_chars: usize,

    /// Minimum importance threshold (default: 8)
    ///
    /// Only include memories with importance >= this value.
    /// Per ADR-003, product knowledge must be importance >= 8.
    #[serde(default = "default_pk_min_importance")]
    pub min_importance: u8,

    // ─────────────────────────────────────────────────────────────────────────────
    // Prompt F: Pre-check + Curation settings
    // ─────────────────────────────────────────────────────────────────────────────

    /// Enable pre-check against codex-product before Tier2 calls (default: true when enabled)
    ///
    /// When enabled, Stage0 searches codex-product for existing insights before
    /// calling NotebookLM. If a strong match is found (relevance >= threshold),
    /// the cached insight is reused and Tier2 is skipped.
    #[serde(default = "default_pk_precheck_enabled")]
    pub precheck_enabled: bool,

    /// Relevance threshold for pre-check hits (default: 0.85)
    ///
    /// If max(relevance_score) >= threshold, the pre-check returns a hit
    /// and Tier2 is skipped. Lower values = more aggressive caching.
    #[serde(default = "default_pk_precheck_threshold")]
    pub precheck_threshold: f64,

    /// Enable post-curation of Tier2 outputs into codex-product (default: true when enabled)
    ///
    /// When enabled, Stage0 distills actionable insights from Tier2 outputs
    /// and stores them in codex-product for future reuse. Runs in background
    /// thread, never blocks pipeline.
    #[serde(default = "default_pk_curation_enabled")]
    pub curation_enabled: bool,
}

fn default_pk_domain() -> String {
    "codex-product".to_string()
}

fn default_pk_max_items() -> usize {
    10
}

fn default_pk_max_chars_per_item() -> usize {
    3000
}

fn default_pk_max_total_chars() -> usize {
    10000
}

fn default_pk_min_importance() -> u8 {
    8
}

fn default_pk_precheck_enabled() -> bool {
    true // ON by default when product_knowledge.enabled is true
}

fn default_pk_precheck_threshold() -> f64 {
    0.85 // Recommended threshold per ADR-003 Prompt F
}

fn default_pk_curation_enabled() -> bool {
    true // ON by default when product_knowledge.enabled is true
}

impl Default for ProductKnowledgeConfig {
    fn default() -> Self {
        Self {
            enabled: false, // OFF by default
            domain: default_pk_domain(),
            max_items: default_pk_max_items(),
            max_chars_per_item: default_pk_max_chars_per_item(),
            max_total_chars: default_pk_max_total_chars(),
            min_importance: default_pk_min_importance(),
            precheck_enabled: default_pk_precheck_enabled(),
            precheck_threshold: default_pk_precheck_threshold(),
            curation_enabled: default_pk_curation_enabled(),
        }
    }
}

impl Default for Stage0Config {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            explain: false,
            db_path: default_db_path(),
            ingestion: IngestionConfig::default(),
            scoring: ScoringConfig::default(),
            context_compiler: ContextCompilerConfig::default(),
            tier2: Tier2Config::default(),
            vector_index: VectorIndexConfig::default(),
            phase1_gate_mode: GateMode::default(),
            store_system_pointers: default_store_system_pointers(),
            memory_backend: MemoryBackend::default(),
        }
    }
}

impl Stage0Config {
    /// Environment variable for config path override (preferred)
    pub const ENV_CONFIG_PATH: &'static str = "CODE_STAGE0_CONFIG";

    /// Legacy environment variable for config path override
    pub const ENV_CONFIG_PATH_LEGACY: &'static str = "CODEX_STAGE0_CONFIG";

    /// Default config filename
    pub const DEFAULT_CONFIG_FILENAME: &'static str = "stage0.toml";

    /// Load configuration from file
    ///
    /// Resolution order:
    /// 1. `CODE_STAGE0_CONFIG` environment variable (preferred)
    /// 2. `CODEX_STAGE0_CONFIG` environment variable (legacy)
    /// 3. `~/.config/code/stage0.toml` (preferred)
    /// 4. `~/.config/codex/stage0.toml` (legacy fallback)
    ///
    /// If the config file doesn't exist, returns default configuration.
    pub fn load() -> Result<Self> {
        let path = Self::resolve_config_path();

        if !path.exists() {
            tracing::info!(
                path = %path.display(),
                "Stage0 config not found, using defaults"
            );
            return Ok(Self::default());
        }

        Self::load_from_path(&path)
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &PathBuf) -> Result<Self> {
        let contents = std::fs::read_to_string(path).map_err(|e| {
            Stage0Error::config_with_source(
                format!("failed to read config at {}", path.display()),
                e,
            )
        })?;

        Self::parse(&contents)
    }

    /// Parse configuration from TOML string
    pub fn parse(contents: &str) -> Result<Self> {
        let cfg: Stage0Config = toml::from_str(contents)
            .map_err(|e| Stage0Error::config_with_source("failed to parse config", e))?;

        cfg.validate()?;
        Ok(cfg)
    }

    /// Resolve the configuration file path
    ///
    /// Priority:
    /// 1. CODE_STAGE0_CONFIG env var (preferred)
    /// 2. CODEX_STAGE0_CONFIG env var (legacy)
    /// 3. ~/.config/code/stage0.toml (preferred path)
    /// 4. ~/.config/codex/stage0.toml (legacy fallback)
    fn resolve_config_path() -> PathBuf {
        // Check preferred env var first
        if let Ok(path) = std::env::var(Self::ENV_CONFIG_PATH) {
            return PathBuf::from(path);
        }

        // Check legacy env var
        if let Ok(path) = std::env::var(Self::ENV_CONFIG_PATH_LEGACY) {
            return PathBuf::from(path);
        }

        // Check file paths
        if let Some(home) = dirs::home_dir() {
            // Preferred path: ~/.config/code/stage0.toml
            let code_path = home
                .join(".config")
                .join("code")
                .join(Self::DEFAULT_CONFIG_FILENAME);

            if code_path.exists() {
                return code_path;
            }

            // Legacy fallback: ~/.config/codex/stage0.toml
            let codex_path = home
                .join(".config")
                .join("codex")
                .join(Self::DEFAULT_CONFIG_FILENAME);

            if codex_path.exists() {
                return codex_path;
            }

            // Neither exists - return preferred path (for "not found" message)
            return code_path;
        }

        PathBuf::from(Self::DEFAULT_CONFIG_FILENAME)
    }

    /// Get the canonical config path (for user-facing messages)
    pub fn canonical_config_path() -> PathBuf {
        dirs::home_dir()
            .map(|h| {
                h.join(".config")
                    .join("code")
                    .join(Self::DEFAULT_CONFIG_FILENAME)
            })
            .unwrap_or_else(|| PathBuf::from(Self::DEFAULT_CONFIG_FILENAME))
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        // Validate weights sum to ~1.0 (allow some tolerance)
        let weight_sum = self.scoring.weights.usage
            + self.scoring.weights.recency
            + self.scoring.weights.priority
            + self.scoring.weights.decay;

        if (weight_sum - 1.0).abs() > 0.01 {
            tracing::warn!(
                weight_sum,
                "Scoring weights don't sum to 1.0, results may be unexpected"
            );
        }

        // Validate DCC weights sum to 1.0
        let dcc_weight_sum = self.context_compiler.dynamic_score_weight
            + self.context_compiler.semantic_similarity_weight;

        if (dcc_weight_sum - 1.0).abs() > 0.01 {
            tracing::warn!(
                dcc_weight_sum,
                "DCC weights don't sum to 1.0, results may be unexpected"
            );
        }

        // Warn if Tier2 enabled but no notebook configured
        if self.tier2.enabled && self.tier2.notebook.trim().is_empty() {
            tracing::warn!("Tier2 enabled but notebook is empty; Tier2 will fail at runtime");
        }

        Ok(())
    }

    /// Get the resolved database path (expanding ~ if needed)
    pub fn resolved_db_path(&self) -> PathBuf {
        let path = &self.db_path;
        if let Some(stripped) = path.strip_prefix("~/")
            && let Some(home) = dirs::home_dir()
        {
            return home.join(stripped);
        }
        PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Stage0Config::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.scoring.weights.usage, 0.30);
        assert_eq!(cfg.context_compiler.max_tokens, 8000);
        assert!(cfg.tier2.enabled);
        // P91: Gate mode defaults to Warn
        assert_eq!(cfg.phase1_gate_mode, GateMode::Warn);
        // SPEC-KIT-971: Memory backend defaults to Memvid
        assert_eq!(cfg.memory_backend, MemoryBackend::Memvid);
    }

    // P91/SPEC-KIT-105: GateMode tests
    #[test]
    fn test_gate_mode_default() {
        let mode = GateMode::default();
        assert_eq!(mode, GateMode::Warn);
    }

    #[test]
    fn test_gate_mode_parse_warn() {
        let toml = r#"
            enabled = true
            phase1_gate_mode = "warn"
        "#;
        let cfg = Stage0Config::parse(toml).expect("should parse");
        assert_eq!(cfg.phase1_gate_mode, GateMode::Warn);
    }

    #[test]
    fn test_gate_mode_parse_skip() {
        let toml = r#"
            enabled = true
            phase1_gate_mode = "skip"
        "#;
        let cfg = Stage0Config::parse(toml).expect("should parse");
        assert_eq!(cfg.phase1_gate_mode, GateMode::Skip);
    }

    // P92/SPEC-KIT-105: Block mode test
    #[test]
    fn test_gate_mode_parse_block() {
        let toml = r#"
            enabled = true
            phase1_gate_mode = "block"
        "#;
        let cfg = Stage0Config::parse(toml).expect("should parse");
        assert_eq!(cfg.phase1_gate_mode, GateMode::Block);
    }

    // SPEC-KIT-971: MemoryBackend tests
    #[test]
    fn test_memory_backend_default() {
        let backend = MemoryBackend::default();
        assert_eq!(backend, MemoryBackend::Memvid);
    }

    #[test]
    fn test_memory_backend_parse_memvid() {
        let toml = r#"
            enabled = true
            memory_backend = "memvid"
        "#;
        let cfg = Stage0Config::parse(toml).expect("should parse");
        assert_eq!(cfg.memory_backend, MemoryBackend::Memvid);
    }

    #[test]
    fn test_memory_backend_parse_local_memory() {
        let toml = r#"
            enabled = true
            memory_backend = "local-memory"
        "#;
        let cfg = Stage0Config::parse(toml).expect("should parse");
        assert_eq!(cfg.memory_backend, MemoryBackend::LocalMemory);
    }

    #[test]
    fn test_memory_backend_default_in_config() {
        let toml = r#"
            enabled = true
        "#;
        let cfg = Stage0Config::parse(toml).expect("should parse");
        // Default should be Memvid
        assert_eq!(cfg.memory_backend, MemoryBackend::Memvid);
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
            enabled = true
            db_path = "/tmp/test.db"
        "#;

        let cfg = Stage0Config::parse(toml).expect("should parse");
        assert!(cfg.enabled);
        assert_eq!(cfg.db_path, "/tmp/test.db");
        // Defaults should be applied
        assert_eq!(cfg.scoring.weights.usage, 0.30);
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
            enabled = true
            explain = true
            db_path = "~/.config/codex/overlay.db"

            [ingestion]
            strict_metadata = false

            [scoring]
            recalculation_interval = "12h0m0s"
            novelty_boost_threshold = 10
            novelty_boost_factor_max = 0.3

            [scoring.weights]
            usage = 0.25
            recency = 0.25
            priority = 0.25
            decay = 0.25

            [context_compiler]
            max_tokens = 4000
            top_k = 10
            dynamic_score_weight = 0.50
            semantic_similarity_weight = 0.50
            pre_filter_limit = 100
            diversity_lambda = 0.80
            iqo_llm_enabled = false

            [tier2]
            enabled = false
            notebook_id_shadow = "test-notebook-id"
            cache_ttl_hours = 48
            mcp_tool_name = "custom-mcp"
            call_timeout = "60s"
        "#;

        let cfg = Stage0Config::parse(toml).expect("should parse");
        assert!(cfg.explain);
        assert!(!cfg.ingestion.strict_metadata);
        assert_eq!(cfg.scoring.recalculation_interval, "12h0m0s");
        assert_eq!(cfg.scoring.weights.usage, 0.25);
        assert_eq!(cfg.context_compiler.max_tokens, 4000);
        assert!(!cfg.tier2.enabled);
        assert_eq!(cfg.tier2.notebook, "test-notebook-id");
    }
}
