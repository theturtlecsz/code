//! Stage0 configuration loading
//!
//! Loads configuration from `~/.config/codex/stage0.toml` (or `CODEX_STAGE0_CONFIG` env).
//! See STAGE0_CONFIG_AND_PROMPTS.md for the full schema.

use crate::errors::{Result, Stage0Error};
use serde::Deserialize;
use std::path::PathBuf;

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
}

fn default_enabled() -> bool {
    true
}

fn default_db_path() -> String {
    dirs::home_dir()
        .map(|h| {
            h.join(".config")
                .join("codex")
                .join("local-memory-overlay.db")
                .to_string_lossy()
                .into_owned()
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
#[derive(Debug, Deserialize, Clone)]
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
        }
    }
}

/// Tier 2 (NotebookLM) configuration
#[derive(Debug, Deserialize, Clone)]
pub struct Tier2Config {
    /// Enable Tier 2 synthesis
    #[serde(default = "default_tier2_enabled")]
    pub enabled: bool,

    /// NotebookLM notebook ID for the "Shadow Stage 0" notebook
    #[serde(default)]
    pub notebook_id_shadow: String,

    /// Cache TTL in hours
    #[serde(default = "default_cache_ttl_hours")]
    pub cache_ttl_hours: u64,

    /// MCP tool name for NotebookLM
    #[serde(default = "default_mcp_tool_name")]
    pub mcp_tool_name: String,

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
fn default_mcp_tool_name() -> String {
    "notebooklm-mcp".to_string()
}
fn default_call_timeout() -> String {
    "30s".to_string()
}

impl Default for Tier2Config {
    fn default() -> Self {
        Self {
            enabled: default_tier2_enabled(),
            notebook_id_shadow: String::new(),
            cache_ttl_hours: default_cache_ttl_hours(),
            mcp_tool_name: default_mcp_tool_name(),
            call_timeout: default_call_timeout(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// V2.5b: Vector Index Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Vector index configuration
///
/// Controls how many memories are indexed into the TF-IDF vector backend
/// for hybrid retrieval.
#[derive(Debug, Deserialize, Clone)]
pub struct VectorIndexConfig {
    /// Maximum memories to index (0 = no limit, index all)
    /// When set, indexes top N by dynamic_score DESC
    #[serde(default = "default_vector_max_memories")]
    pub max_memories_to_index: usize,
}

fn default_vector_max_memories() -> usize {
    0 // No limit by default - index all
}

impl Default for VectorIndexConfig {
    fn default() -> Self {
        Self {
            max_memories_to_index: default_vector_max_memories(),
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
        }
    }
}

impl Stage0Config {
    /// Environment variable for config path override
    pub const ENV_CONFIG_PATH: &'static str = "CODEX_STAGE0_CONFIG";

    /// Default config filename
    pub const DEFAULT_CONFIG_FILENAME: &'static str = "stage0.toml";

    /// Load configuration from file
    ///
    /// Resolution order:
    /// 1. `CODEX_STAGE0_CONFIG` environment variable
    /// 2. `~/.config/codex/stage0.toml`
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
    fn resolve_config_path() -> PathBuf {
        if let Ok(path) = std::env::var(Self::ENV_CONFIG_PATH) {
            return PathBuf::from(path);
        }

        dirs::home_dir()
            .map(|h| {
                h.join(".config")
                    .join("codex")
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

        // Warn if Tier2 enabled but no notebook ID
        if self.tier2.enabled && self.tier2.notebook_id_shadow.is_empty() {
            tracing::warn!(
                "Tier2 enabled but notebook_id_shadow is empty; Tier2 will fail at runtime"
            );
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
        assert_eq!(cfg.tier2.notebook_id_shadow, "test-notebook-id");
    }
}
