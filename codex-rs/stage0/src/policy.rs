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
//! ## Hash Determinism (SPEC-KIT-977-A1)
//!
//! The `hash` field is a **content-hash** computed only from policy content:
//! - `schema_version`, `model_config`, `weights`, `prompts`, `source_files`
//!
//! Excluded from hash (runtime values):
//! - `policy_id` (UUID generated at capture time)
//! - `created_at` (timestamp at capture time)
//! - `hash` (obviously circular)
//!
//! This ensures: **capturing twice with identical inputs produces identical hash**.
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
/// - **Governance policy from model_policy.toml** (SPEC-KIT-977 extension)
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

    /// SPEC-KIT-977: Governance policy from model_policy.toml
    ///
    /// Contains the authoritative machine-readable governance surface:
    /// routing, capture mode, budgets, security, gates, SOR.
    #[serde(default)]
    pub governance: Option<GovernancePolicy>,
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

// =============================================================================
// SPEC-KIT-977: GovernancePolicy from model_policy.toml
// =============================================================================

/// Complete governance policy parsed from model_policy.toml.
///
/// This is the authoritative machine-readable governance surface.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GovernancePolicy {
    /// Schema metadata
    pub meta: GovernanceMeta,

    /// System of record configuration
    pub system_of_record: SystemOfRecord,

    /// Model routing configuration
    pub routing: RoutingConfig,

    /// Capture and replay configuration
    pub capture: CaptureConfig,

    /// Token and cost budgets
    pub budgets: BudgetConfig,

    /// Scoring weights for Stage0 retrieval
    pub scoring: ScoringConfig,

    /// Gate criteria for promotions/sunsets
    pub gates: GateConfig,

    /// Security controls
    pub security: SecurityConfig,
}

/// Schema metadata from [meta]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GovernanceMeta {
    pub schema_version: String,
    pub effective_date: String,
}

/// System of record configuration from [system_of_record]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemOfRecord {
    pub primary: String,
    pub fallback: String,
    pub fallback_enabled: bool,
}

/// Routing configuration from [routing.cloud] and [routing.reflex]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingConfig {
    pub cloud: CloudRouting,
    pub reflex: ReflexRouting,
}

/// Cloud model routing from [routing.cloud]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CloudRouting {
    pub architect: Vec<String>,
    pub implementer: Vec<String>,
    pub judge: Vec<String>,
    pub default_architect: String,
    pub default_implementer: String,
    pub default_judge: String,
}

/// Reflex (local inference) routing from [routing.reflex]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReflexRouting {
    pub enabled: bool,
    pub endpoint: String,
    pub model: String,
    pub timeout_ms: u64,
    pub json_schema_required: bool,
    pub fallback_to_cloud: bool,
    pub thresholds: ReflexThresholds,
}

/// Bakeoff thresholds from [routing.reflex.thresholds]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReflexThresholds {
    pub p95_latency_ms: u64,
    pub success_parity_percent: u8,
    pub json_schema_compliance_percent: u8,
}

/// Capture configuration from [capture]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CaptureConfig {
    pub mode: String,
    pub store_embeddings: bool,
}

/// Budget configuration from [budgets.tokens] and [budgets.cost]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BudgetConfig {
    pub tokens: TokenBudgets,
    pub cost: CostBudgets,
}

/// Token budgets per stage from [budgets.tokens]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenBudgets {
    pub plan: u32,
    pub tasks: u32,
    pub implement: u32,
    pub validate: u32,
    pub audit: u32,
    pub unlock: u32,
}

/// Cost thresholds from [budgets.cost]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostBudgets {
    pub warn_threshold: f64,
    pub confirm_threshold: f64,
    pub hard_limit: f64,
    pub hard_limit_enabled: bool,
}

/// Scoring configuration from [scoring]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoringConfig {
    pub usage: f32,
    pub recency: f32,
    pub priority: f32,
    pub decay: f32,
    pub vector_weight: f32,
    pub lexical_weight: f32,
}

/// Gate configuration from [gates.*]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GateConfig {
    pub reflex_promotion: ReflexPromotionGate,
    pub local_memory_sunset: LocalMemorySunsetGate,
}

/// Reflex promotion gate criteria from [gates.reflex_promotion]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReflexPromotionGate {
    pub p95_latency_ms: u64,
    pub success_parity_percent: u8,
    pub json_schema_compliance_percent: u8,
    pub golden_query_regression_allowed: bool,
}

/// Local-memory sunset gate criteria from [gates.local_memory_sunset]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalMemorySunsetGate {
    pub retrieval_p95_parity: bool,
    pub search_quality_parity: bool,
    pub stability_days: u32,
    pub zero_fallback_activations: bool,
}

/// Security configuration from [security.*]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    pub redaction: RedactionConfig,
    pub export: ExportConfig,
}

/// Redaction patterns from [security.redaction]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RedactionConfig {
    pub env_var_patterns: Vec<String>,
    pub file_patterns: Vec<String>,
}

/// Export controls from [security.export]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportConfig {
    pub require_explicit_action: bool,
    pub allow_no_export_marking: bool,
    pub encrypt_at_rest: bool,
}

impl GovernancePolicy {
    /// Load governance policy from model_policy.toml.
    ///
    /// Searches for model_policy.toml in:
    /// 1. Current directory (./model_policy.toml)
    /// 2. Parent directory (../model_policy.toml)
    /// 3. codex-rs subdirectory (codex-rs/model_policy.toml)
    /// 4. Explicit path if provided
    pub fn load(explicit_path: Option<&Path>) -> Result<Self, String> {
        let path = if let Some(p) = explicit_path {
            if p.exists() {
                p.to_path_buf()
            } else {
                return Err(format!("Explicit path not found: {}", p.display()));
            }
        } else {
            Self::find_policy_file()?
        };

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        Self::from_toml(&content)
    }

    /// Find model_policy.toml by searching standard locations.
    fn find_policy_file() -> Result<PathBuf, String> {
        let candidates = [
            PathBuf::from("model_policy.toml"),
            PathBuf::from("../model_policy.toml"),
            PathBuf::from("codex-rs/model_policy.toml"),
        ];

        for path in &candidates {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        Err("model_policy.toml not found in standard locations".to_string())
    }

    /// Parse governance policy from TOML string.
    pub fn from_toml(content: &str) -> Result<Self, String> {
        // Parse as generic toml::Value first
        let value: toml::Value =
            toml::from_str(content).map_err(|e| format!("TOML parse error: {}", e))?;

        // Extract sections with defaults for missing fields
        let meta = Self::parse_meta(&value);
        let system_of_record = Self::parse_system_of_record(&value);
        let routing = Self::parse_routing(&value);
        let capture = Self::parse_capture(&value);
        let budgets = Self::parse_budgets(&value);
        let scoring = Self::parse_scoring(&value);
        let gates = Self::parse_gates(&value);
        let security = Self::parse_security(&value);

        Ok(Self {
            meta,
            system_of_record,
            routing,
            capture,
            budgets,
            scoring,
            gates,
            security,
        })
    }

    fn parse_meta(value: &toml::Value) -> GovernanceMeta {
        let meta = value.get("meta");
        GovernanceMeta {
            schema_version: meta
                .and_then(|m| m.get("schema_version"))
                .and_then(|v| v.as_str())
                .unwrap_or("1.0")
                .to_string(),
            effective_date: meta
                .and_then(|m| m.get("effective_date"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        }
    }

    fn parse_system_of_record(value: &toml::Value) -> SystemOfRecord {
        let sor = value.get("system_of_record");
        SystemOfRecord {
            primary: sor
                .and_then(|s| s.get("primary"))
                .and_then(|v| v.as_str())
                .unwrap_or("memvid")
                .to_string(),
            fallback: sor
                .and_then(|s| s.get("fallback"))
                .and_then(|v| v.as_str())
                .unwrap_or("local-memory")
                .to_string(),
            fallback_enabled: sor
                .and_then(|s| s.get("fallback_enabled"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        }
    }

    fn parse_routing(value: &toml::Value) -> RoutingConfig {
        let routing = value.get("routing");

        let cloud = routing.and_then(|r| r.get("cloud"));
        let reflex = routing.and_then(|r| r.get("reflex"));
        let thresholds = reflex.and_then(|r| r.get("thresholds"));

        RoutingConfig {
            cloud: CloudRouting {
                architect: cloud
                    .and_then(|c| c.get("architect"))
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                implementer: cloud
                    .and_then(|c| c.get("implementer"))
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                judge: cloud
                    .and_then(|c| c.get("judge"))
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                default_architect: cloud
                    .and_then(|c| c.get("default_architect"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                default_implementer: cloud
                    .and_then(|c| c.get("default_implementer"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                default_judge: cloud
                    .and_then(|c| c.get("default_judge"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            },
            reflex: ReflexRouting {
                enabled: reflex
                    .and_then(|r| r.get("enabled"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                endpoint: reflex
                    .and_then(|r| r.get("endpoint"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("http://127.0.0.1:3009/v1")
                    .to_string(),
                model: reflex
                    .and_then(|r| r.get("model"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                timeout_ms: reflex
                    .and_then(|r| r.get("timeout_ms"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(1500) as u64,
                json_schema_required: reflex
                    .and_then(|r| r.get("json_schema_required"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                fallback_to_cloud: reflex
                    .and_then(|r| r.get("fallback_to_cloud"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                thresholds: ReflexThresholds {
                    p95_latency_ms: thresholds
                        .and_then(|t| t.get("p95_latency_ms"))
                        .and_then(|v| v.as_integer())
                        .unwrap_or(2000) as u64,
                    success_parity_percent: thresholds
                        .and_then(|t| t.get("success_parity_percent"))
                        .and_then(|v| v.as_integer())
                        .unwrap_or(85) as u8,
                    json_schema_compliance_percent: thresholds
                        .and_then(|t| t.get("json_schema_compliance_percent"))
                        .and_then(|v| v.as_integer())
                        .unwrap_or(100) as u8,
                },
            },
        }
    }

    fn parse_capture(value: &toml::Value) -> CaptureConfig {
        let capture = value.get("capture");
        CaptureConfig {
            mode: capture
                .and_then(|c| c.get("mode"))
                .and_then(|v| v.as_str())
                .unwrap_or("prompts_only")
                .to_string(),
            store_embeddings: capture
                .and_then(|c| c.get("store_embeddings"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        }
    }

    fn parse_budgets(value: &toml::Value) -> BudgetConfig {
        let budgets = value.get("budgets");
        let tokens = budgets.and_then(|b| b.get("tokens"));
        let cost = budgets.and_then(|b| b.get("cost"));

        BudgetConfig {
            tokens: TokenBudgets {
                plan: tokens
                    .and_then(|t| t.get("plan"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(8000) as u32,
                tasks: tokens
                    .and_then(|t| t.get("tasks"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(4000) as u32,
                implement: tokens
                    .and_then(|t| t.get("implement"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(6000) as u32,
                validate: tokens
                    .and_then(|t| t.get("validate"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(4000) as u32,
                audit: tokens
                    .and_then(|t| t.get("audit"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(4000) as u32,
                unlock: tokens
                    .and_then(|t| t.get("unlock"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(2000) as u32,
            },
            cost: CostBudgets {
                warn_threshold: cost
                    .and_then(|c| c.get("warn_threshold"))
                    .and_then(|v| v.as_float())
                    .unwrap_or(5.0),
                confirm_threshold: cost
                    .and_then(|c| c.get("confirm_threshold"))
                    .and_then(|v| v.as_float())
                    .unwrap_or(10.0),
                hard_limit: cost
                    .and_then(|c| c.get("hard_limit"))
                    .and_then(|v| v.as_float())
                    .unwrap_or(25.0),
                hard_limit_enabled: cost
                    .and_then(|c| c.get("hard_limit_enabled"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            },
        }
    }

    fn parse_scoring(value: &toml::Value) -> ScoringConfig {
        let scoring = value.get("scoring");
        ScoringConfig {
            usage: scoring
                .and_then(|s| s.get("usage"))
                .and_then(|v| v.as_float())
                .unwrap_or(0.30) as f32,
            recency: scoring
                .and_then(|s| s.get("recency"))
                .and_then(|v| v.as_float())
                .unwrap_or(0.30) as f32,
            priority: scoring
                .and_then(|s| s.get("priority"))
                .and_then(|v| v.as_float())
                .unwrap_or(0.25) as f32,
            decay: scoring
                .and_then(|s| s.get("decay"))
                .and_then(|v| v.as_float())
                .unwrap_or(0.15) as f32,
            vector_weight: scoring
                .and_then(|s| s.get("vector_weight"))
                .and_then(|v| v.as_float())
                .unwrap_or(0.6) as f32,
            lexical_weight: scoring
                .and_then(|s| s.get("lexical_weight"))
                .and_then(|v| v.as_float())
                .unwrap_or(0.4) as f32,
        }
    }

    fn parse_gates(value: &toml::Value) -> GateConfig {
        let gates = value.get("gates");
        let reflex = gates.and_then(|g| g.get("reflex_promotion"));
        let lm_sunset = gates.and_then(|g| g.get("local_memory_sunset"));

        GateConfig {
            reflex_promotion: ReflexPromotionGate {
                p95_latency_ms: reflex
                    .and_then(|r| r.get("p95_latency_ms"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(2000) as u64,
                success_parity_percent: reflex
                    .and_then(|r| r.get("success_parity_percent"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(85) as u8,
                json_schema_compliance_percent: reflex
                    .and_then(|r| r.get("json_schema_compliance_percent"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(100) as u8,
                golden_query_regression_allowed: reflex
                    .and_then(|r| r.get("golden_query_regression_allowed"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            },
            local_memory_sunset: LocalMemorySunsetGate {
                retrieval_p95_parity: lm_sunset
                    .and_then(|l| l.get("retrieval_p95_parity"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                search_quality_parity: lm_sunset
                    .and_then(|l| l.get("search_quality_parity"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                stability_days: lm_sunset
                    .and_then(|l| l.get("stability_days"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(30) as u32,
                zero_fallback_activations: lm_sunset
                    .and_then(|l| l.get("zero_fallback_activations"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
            },
        }
    }

    fn parse_security(value: &toml::Value) -> SecurityConfig {
        let security = value.get("security");
        let redaction = security.and_then(|s| s.get("redaction"));
        let export = security.and_then(|s| s.get("export"));

        SecurityConfig {
            redaction: RedactionConfig {
                env_var_patterns: redaction
                    .and_then(|r| r.get("env_var_patterns"))
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                file_patterns: redaction
                    .and_then(|r| r.get("file_patterns"))
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
            },
            export: ExportConfig {
                require_explicit_action: export
                    .and_then(|e| e.get("require_explicit_action"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                allow_no_export_marking: export
                    .and_then(|e| e.get("allow_no_export_marking"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                encrypt_at_rest: export
                    .and_then(|e| e.get("encrypt_at_rest"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            },
        }
    }
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
        Self::capture_with_governance(config, source_files, None)
    }

    /// Create a new PolicySnapshot with explicit governance policy.
    ///
    /// SPEC-KIT-977: Extended capture that includes model_policy.toml governance.
    pub fn capture_with_governance(
        config: &Stage0Config,
        source_files: Vec<String>,
        governance: Option<GovernancePolicy>,
    ) -> Self {
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
            governance,
        };

        // Compute hash of canonical JSON
        snapshot.hash = snapshot.compute_hash();

        snapshot
    }

    /// Compute SHA256 hash of the canonical JSON representation.
    ///
    /// ## SPEC-KIT-977-A1: Deterministic Hash
    ///
    /// The hash is computed ONLY from content fields, excluding:
    /// - `policy_id` (runtime UUID)
    /// - `created_at` (runtime timestamp)
    /// - `hash` (obviously circular)
    ///
    /// **Canonicalization for determinism:**
    /// - `prompts`: Keys are sorted alphabetically (HashMap order is nondeterministic)
    /// - `source_files`: Sorted alphabetically (filesystem discovery order may vary)
    /// - `governance`: Included as-is (structs have deterministic serialization)
    ///
    /// This ensures identical inputs produce identical hashes regardless of:
    /// - HashMap iteration order
    /// - Filesystem discovery ordering
    pub fn compute_hash(&self) -> String {
        use std::collections::BTreeMap;

        // SPEC-KIT-977-A1: Sort prompts keys for deterministic serialization
        let sorted_prompts: BTreeMap<&String, &String> = self.prompts.iter().collect();

        // SPEC-KIT-977-A1: Sort source_files for deterministic serialization
        let mut sorted_sources = self.source_files.clone();
        sorted_sources.sort();

        // Hash only content fields (not runtime identifiers)
        // This ensures deterministic hashing per SPEC-KIT-977-A1
        // SPEC-KIT-977: Include governance in hash computation
        let hashable = serde_json::json!({
            "schema_version": self.schema_version,
            "model_config": self.model_config,
            "weights": self.weights,
            "prompts": sorted_prompts,
            "source_files": sorted_sources,
            "governance": self.governance,
        });

        let canonical = serde_json::to_string(&hashable).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        let result = hasher.finalize();
        hex_encode(&result)
    }

    /// Verify the hash matches the snapshot content.
    ///
    /// Returns true if recomputing the hash produces the same value.
    /// Useful for detecting tampering or corruption.
    pub fn verify_hash(&self) -> bool {
        let computed = self.compute_hash();
        computed == self.hash
    }

    /// Check if this policy has the same content as another.
    ///
    /// ## SPEC-KIT-977-A1: Policy Changed Detection
    ///
    /// Uses content-hash comparison to determine if policy content changed.
    /// Two policies with different `policy_id` or `created_at` but identical
    /// content will return `true` (same content).
    ///
    /// ## Usage
    /// ```ignore
    /// if !new_policy.content_matches(&old_policy) {
    ///     // Policy changed, need to re-capture
    /// }
    /// ```
    pub fn content_matches(&self, other: &PolicySnapshot) -> bool {
        self.hash == other.hash
    }

    /// Check if policy content has changed compared to another snapshot.
    ///
    /// Convenience inverse of `content_matches()`.
    pub fn content_changed(&self, other: &PolicySnapshot) -> bool {
        !self.content_matches(other)
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
        self.base_path.join(format!("snapshot-{policy_id}.json"))
    }

    /// Store a policy snapshot to disk.
    pub fn store(&self, snapshot: &PolicySnapshot) -> std::io::Result<PathBuf> {
        self.ensure_dir()?;

        let path = self.snapshot_path(&snapshot.policy_id);
        let json = snapshot
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

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
        PolicySnapshot::from_json(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
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
///
/// SPEC-KIT-977: Now includes governance policy from model_policy.toml.
pub fn capture_policy_snapshot(config: &Stage0Config) -> PolicySnapshot {
    // Collect source file paths
    let source_files = collect_source_files();

    // SPEC-KIT-977: Load governance policy from model_policy.toml
    let governance = load_governance_policy();

    PolicySnapshot::capture_with_governance(config, source_files, governance)
}

/// Load governance policy from model_policy.toml.
///
/// SPEC-KIT-977: Searches standard locations for model_policy.toml.
/// Returns None if file not found or parse error (non-fatal).
fn load_governance_policy() -> Option<GovernancePolicy> {
    match GovernancePolicy::load(None) {
        Ok(policy) => {
            tracing::debug!(
                schema_version = %policy.meta.schema_version,
                effective_date = %policy.meta.effective_date,
                "Loaded governance policy from model_policy.toml"
            );
            Some(policy)
        }
        Err(e) => {
            tracing::debug!("Governance policy not loaded: {}", e);
            None
        }
    }
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
pub fn get_policy_for_run(run_id: &str, config: &Stage0Config) -> std::io::Result<PolicySnapshot> {
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
// Policy Diff (SPEC-KIT-977)
// =============================================================================

/// Represents a difference in a single field between two policy snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyFieldChange {
    /// Field path using dot notation (e.g., "routing.reflex.enabled")
    pub path: String,

    /// Old value (serialized as string for display)
    pub old_value: String,

    /// New value (serialized as string for display)
    pub new_value: String,

    /// Category of the change for grouping
    pub category: ChangeCategory,
}

/// Category of a policy change for grouping in output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChangeCategory {
    /// Governance changes (routing, capture mode, SOR)
    Governance,
    /// Model configuration changes
    ModelConfig,
    /// Scoring weights changes
    Weights,
    /// Source files changes
    SourceFiles,
    /// Prompts changes
    Prompts,
    /// Schema version
    Schema,
}

impl ChangeCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChangeCategory::Governance => "governance",
            ChangeCategory::ModelConfig => "model_config",
            ChangeCategory::Weights => "weights",
            ChangeCategory::SourceFiles => "source_files",
            ChangeCategory::Prompts => "prompts",
            ChangeCategory::Schema => "schema",
        }
    }
}

/// Complete diff result between two policy snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDiff {
    /// Policy ID of the first (older) snapshot
    pub policy_id_a: String,

    /// Policy ID of the second (newer) snapshot
    pub policy_id_b: String,

    /// Hash of snapshot A
    pub hash_a: String,

    /// Hash of snapshot B
    pub hash_b: String,

    /// Whether the policies are identical (by hash)
    pub identical: bool,

    /// List of all changes, sorted by path for determinism
    pub changes: Vec<PolicyFieldChange>,
}

impl PolicyDiff {
    /// Compute the diff between two policy snapshots.
    ///
    /// Returns a `PolicyDiff` with all field changes, sorted by path
    /// for deterministic output.
    pub fn compute(a: &PolicySnapshot, b: &PolicySnapshot) -> Self {
        let mut changes = Vec::new();

        // Schema version
        if a.schema_version != b.schema_version {
            changes.push(PolicyFieldChange {
                path: "schema_version".to_string(),
                old_value: a.schema_version.clone(),
                new_value: b.schema_version.clone(),
                category: ChangeCategory::Schema,
            });
        }

        // Model config changes
        Self::diff_model_config(&a.model_config, &b.model_config, &mut changes);

        // Weights changes
        Self::diff_weights(&a.weights, &b.weights, &mut changes);

        // Source files changes (as a single field)
        let a_sources: Vec<&str> = a.source_files.iter().map(|s| s.as_str()).collect();
        let b_sources: Vec<&str> = b.source_files.iter().map(|s| s.as_str()).collect();
        if a_sources != b_sources {
            changes.push(PolicyFieldChange {
                path: "source_files".to_string(),
                old_value: format!("{:?}", a.source_files),
                new_value: format!("{:?}", b.source_files),
                category: ChangeCategory::SourceFiles,
            });
        }

        // Governance changes (if both have governance)
        match (&a.governance, &b.governance) {
            (Some(gov_a), Some(gov_b)) => {
                Self::diff_governance(gov_a, gov_b, &mut changes);
            }
            (None, Some(_)) => {
                changes.push(PolicyFieldChange {
                    path: "governance".to_string(),
                    old_value: "(none)".to_string(),
                    new_value: "(present)".to_string(),
                    category: ChangeCategory::Governance,
                });
            }
            (Some(_), None) => {
                changes.push(PolicyFieldChange {
                    path: "governance".to_string(),
                    old_value: "(present)".to_string(),
                    new_value: "(none)".to_string(),
                    category: ChangeCategory::Governance,
                });
            }
            (None, None) => {}
        }

        // Sort by path for deterministic output
        changes.sort_by(|x, y| x.path.cmp(&y.path));

        PolicyDiff {
            policy_id_a: a.policy_id.clone(),
            policy_id_b: b.policy_id.clone(),
            hash_a: a.hash.clone(),
            hash_b: b.hash.clone(),
            identical: a.hash == b.hash,
            changes,
        }
    }

    fn diff_model_config(a: &ModelConfig, b: &ModelConfig, changes: &mut Vec<PolicyFieldChange>) {
        macro_rules! check_field {
            ($field:ident) => {
                if a.$field != b.$field {
                    changes.push(PolicyFieldChange {
                        path: format!("model_config.{}", stringify!($field)),
                        old_value: format!("{:?}", a.$field),
                        new_value: format!("{:?}", b.$field),
                        category: ChangeCategory::ModelConfig,
                    });
                }
            };
        }

        check_field!(max_tokens);
        check_field!(top_k);
        check_field!(pre_filter_limit);
        check_field!(diversity_lambda);
        check_field!(iqo_llm_enabled);
        check_field!(hybrid_enabled);
        check_field!(vector_weight);
        check_field!(tier2_enabled);
        check_field!(tier2_cache_ttl_hours);
    }

    fn diff_weights(a: &ScoringWeights, b: &ScoringWeights, changes: &mut Vec<PolicyFieldChange>) {
        macro_rules! check_weight {
            ($field:ident) => {
                if (a.$field - b.$field).abs() > f32::EPSILON {
                    changes.push(PolicyFieldChange {
                        path: format!("weights.{}", stringify!($field)),
                        old_value: format!("{:.4}", a.$field),
                        new_value: format!("{:.4}", b.$field),
                        category: ChangeCategory::Weights,
                    });
                }
            };
        }

        check_weight!(usage);
        check_weight!(recency);
        check_weight!(priority);
        check_weight!(decay);
    }

    fn diff_governance(
        a: &GovernancePolicy,
        b: &GovernancePolicy,
        changes: &mut Vec<PolicyFieldChange>,
    ) {
        // System of record
        if a.system_of_record.primary != b.system_of_record.primary {
            changes.push(PolicyFieldChange {
                path: "governance.system_of_record.primary".to_string(),
                old_value: a.system_of_record.primary.clone(),
                new_value: b.system_of_record.primary.clone(),
                category: ChangeCategory::Governance,
            });
        }
        if a.system_of_record.fallback != b.system_of_record.fallback {
            changes.push(PolicyFieldChange {
                path: "governance.system_of_record.fallback".to_string(),
                old_value: a.system_of_record.fallback.clone(),
                new_value: b.system_of_record.fallback.clone(),
                category: ChangeCategory::Governance,
            });
        }
        if a.system_of_record.fallback_enabled != b.system_of_record.fallback_enabled {
            changes.push(PolicyFieldChange {
                path: "governance.system_of_record.fallback_enabled".to_string(),
                old_value: a.system_of_record.fallback_enabled.to_string(),
                new_value: b.system_of_record.fallback_enabled.to_string(),
                category: ChangeCategory::Governance,
            });
        }

        // Routing - Cloud
        if a.routing.cloud.default_architect != b.routing.cloud.default_architect {
            changes.push(PolicyFieldChange {
                path: "governance.routing.cloud.default_architect".to_string(),
                old_value: a.routing.cloud.default_architect.clone(),
                new_value: b.routing.cloud.default_architect.clone(),
                category: ChangeCategory::Governance,
            });
        }
        if a.routing.cloud.default_implementer != b.routing.cloud.default_implementer {
            changes.push(PolicyFieldChange {
                path: "governance.routing.cloud.default_implementer".to_string(),
                old_value: a.routing.cloud.default_implementer.clone(),
                new_value: b.routing.cloud.default_implementer.clone(),
                category: ChangeCategory::Governance,
            });
        }
        if a.routing.cloud.default_judge != b.routing.cloud.default_judge {
            changes.push(PolicyFieldChange {
                path: "governance.routing.cloud.default_judge".to_string(),
                old_value: a.routing.cloud.default_judge.clone(),
                new_value: b.routing.cloud.default_judge.clone(),
                category: ChangeCategory::Governance,
            });
        }

        // Routing - Reflex
        if a.routing.reflex.enabled != b.routing.reflex.enabled {
            changes.push(PolicyFieldChange {
                path: "governance.routing.reflex.enabled".to_string(),
                old_value: a.routing.reflex.enabled.to_string(),
                new_value: b.routing.reflex.enabled.to_string(),
                category: ChangeCategory::Governance,
            });
        }
        if a.routing.reflex.endpoint != b.routing.reflex.endpoint {
            changes.push(PolicyFieldChange {
                path: "governance.routing.reflex.endpoint".to_string(),
                old_value: a.routing.reflex.endpoint.clone(),
                new_value: b.routing.reflex.endpoint.clone(),
                category: ChangeCategory::Governance,
            });
        }
        if a.routing.reflex.model != b.routing.reflex.model {
            changes.push(PolicyFieldChange {
                path: "governance.routing.reflex.model".to_string(),
                old_value: a.routing.reflex.model.clone(),
                new_value: b.routing.reflex.model.clone(),
                category: ChangeCategory::Governance,
            });
        }

        // Capture mode
        if a.capture.mode != b.capture.mode {
            changes.push(PolicyFieldChange {
                path: "governance.capture.mode".to_string(),
                old_value: a.capture.mode.clone(),
                new_value: b.capture.mode.clone(),
                category: ChangeCategory::Governance,
            });
        }
        if a.capture.store_embeddings != b.capture.store_embeddings {
            changes.push(PolicyFieldChange {
                path: "governance.capture.store_embeddings".to_string(),
                old_value: a.capture.store_embeddings.to_string(),
                new_value: b.capture.store_embeddings.to_string(),
                category: ChangeCategory::Governance,
            });
        }

        // Budgets - Token budgets
        if a.budgets.tokens.plan != b.budgets.tokens.plan {
            changes.push(PolicyFieldChange {
                path: "governance.budgets.tokens.plan".to_string(),
                old_value: a.budgets.tokens.plan.to_string(),
                new_value: b.budgets.tokens.plan.to_string(),
                category: ChangeCategory::Governance,
            });
        }
        if a.budgets.cost.hard_limit_enabled != b.budgets.cost.hard_limit_enabled {
            changes.push(PolicyFieldChange {
                path: "governance.budgets.cost.hard_limit_enabled".to_string(),
                old_value: a.budgets.cost.hard_limit_enabled.to_string(),
                new_value: b.budgets.cost.hard_limit_enabled.to_string(),
                category: ChangeCategory::Governance,
            });
        }

        // Scoring weights from governance
        if (a.scoring.usage - b.scoring.usage).abs() > f32::EPSILON {
            changes.push(PolicyFieldChange {
                path: "governance.scoring.usage".to_string(),
                old_value: format!("{:.4}", a.scoring.usage),
                new_value: format!("{:.4}", b.scoring.usage),
                category: ChangeCategory::Governance,
            });
        }
        if (a.scoring.vector_weight - b.scoring.vector_weight).abs() > f32::EPSILON {
            changes.push(PolicyFieldChange {
                path: "governance.scoring.vector_weight".to_string(),
                old_value: format!("{:.4}", a.scoring.vector_weight),
                new_value: format!("{:.4}", b.scoring.vector_weight),
                category: ChangeCategory::Governance,
            });
        }
    }

    /// Get changes grouped by category.
    pub fn changes_by_category(&self) -> HashMap<ChangeCategory, Vec<&PolicyFieldChange>> {
        let mut grouped: HashMap<ChangeCategory, Vec<&PolicyFieldChange>> = HashMap::new();
        for change in &self.changes {
            grouped.entry(change.category).or_default().push(change);
        }
        grouped
    }

    /// Get list of changed field paths (sorted for determinism).
    pub fn changed_keys(&self) -> Vec<&str> {
        self.changes.iter().map(|c| c.path.as_str()).collect()
    }

    /// Format as human-readable text.
    pub fn to_text(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "Policy Diff: {} → {}\n",
            self.policy_id_a, self.policy_id_b
        ));
        output.push_str(&format!("Hash A: {}\n", self.hash_a));
        output.push_str(&format!("Hash B: {}\n", self.hash_b));
        output.push('\n');

        if self.identical {
            output.push_str("Policies are identical.\n");
            return output;
        }

        output.push_str(&format!("{} change(s) detected:\n", self.changes.len()));
        output.push('\n');

        // Group by category
        let grouped = self.changes_by_category();

        // Fixed category order for determinism
        let categories = [
            ChangeCategory::Governance,
            ChangeCategory::ModelConfig,
            ChangeCategory::Weights,
            ChangeCategory::SourceFiles,
            ChangeCategory::Prompts,
            ChangeCategory::Schema,
        ];

        for category in categories {
            if let Some(changes) = grouped.get(&category) {
                output.push_str(&format!("[{}]\n", category.as_str()));
                for change in changes {
                    output.push_str(&format!(
                        "  {} : {} → {}\n",
                        change.path, change.old_value, change.new_value
                    ));
                }
                output.push('\n');
            }
        }

        output.push_str("Changed keys:\n");
        for key in self.changed_keys() {
            output.push_str(&format!("  - {}\n", key));
        }

        output
    }

    /// Format as JSON (machine-parseable, stable).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
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
        let snapshot =
            PolicySnapshot::capture(&config, vec!["a.toml".to_string(), "b.md".to_string()]);
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

    // =========================================================================
    // SPEC-KIT-977-A1: Deterministic Hash Tests
    // =========================================================================

    /// SPEC-KIT-977-A1: Identical inputs produce identical hashes.
    ///
    /// Capturing twice with the same config and source_files should produce
    /// the same hash, even though policy_id and created_at differ.
    #[test]
    fn test_deterministic_hash_same_inputs_same_hash() {
        let config = Stage0Config::default();
        let source_files = vec!["stage0.toml".to_string(), "model_policy.toml".to_string()];

        // Capture twice with identical inputs
        let snapshot1 = PolicySnapshot::capture(&config, source_files.clone());
        let snapshot2 = PolicySnapshot::capture(&config, source_files);

        // Different policy_id (UUID is random)
        assert_ne!(snapshot1.policy_id, snapshot2.policy_id);

        // Different created_at (captured at different moments)
        // (In practice they might be the same if fast enough, but we don't rely on that)

        // CRITICAL: Hash must be identical for same content
        assert_eq!(
            snapshot1.hash, snapshot2.hash,
            "SPEC-KIT-977-A1 violated: identical inputs should produce identical hash"
        );

        // content_matches should return true
        assert!(
            snapshot1.content_matches(&snapshot2),
            "content_matches should return true for identical content"
        );

        // content_changed should return false
        assert!(
            !snapshot1.content_changed(&snapshot2),
            "content_changed should return false for identical content"
        );
    }

    /// SPEC-KIT-977-A1: Changing a field produces different hash.
    #[test]
    fn test_deterministic_hash_different_source_files_different_hash() {
        let config = Stage0Config::default();

        let snapshot1 = PolicySnapshot::capture(&config, vec!["file1.toml".to_string()]);
        let snapshot2 = PolicySnapshot::capture(&config, vec!["file2.toml".to_string()]);

        // Different source_files should produce different hash
        assert_ne!(
            snapshot1.hash, snapshot2.hash,
            "Different source_files should produce different hash"
        );

        // content_matches should return false
        assert!(
            !snapshot1.content_matches(&snapshot2),
            "content_matches should return false for different content"
        );

        // content_changed should return true
        assert!(
            snapshot1.content_changed(&snapshot2),
            "content_changed should return true for different content"
        );
    }

    /// SPEC-KIT-977-A1: Changing model_config produces different hash.
    #[test]
    fn test_deterministic_hash_different_config_different_hash() {
        let config1 = Stage0Config::default();
        let mut config2 = Stage0Config::default();

        // Modify a config field
        config2.context_compiler.top_k = 999;

        let source_files = vec!["test.toml".to_string()];
        let snapshot1 = PolicySnapshot::capture(&config1, source_files.clone());
        let snapshot2 = PolicySnapshot::capture(&config2, source_files);

        // Different model_config should produce different hash
        assert_ne!(
            snapshot1.hash, snapshot2.hash,
            "Different model_config should produce different hash"
        );

        assert!(snapshot1.content_changed(&snapshot2));
    }

    /// SPEC-KIT-977-A1: Hash verification after JSON roundtrip.
    #[test]
    fn test_deterministic_hash_survives_json_roundtrip() {
        let config = Stage0Config::default();
        let original = PolicySnapshot::capture(&config, vec!["test.toml".to_string()]);

        // Serialize and deserialize
        let json = original.to_json().expect("serialize");
        let restored = PolicySnapshot::from_json(&json).expect("deserialize");

        // Hash should be preserved
        assert_eq!(original.hash, restored.hash);

        // verify_hash should pass
        assert!(
            restored.verify_hash(),
            "verify_hash should pass after JSON roundtrip"
        );

        // content_matches should work across roundtrip
        assert!(original.content_matches(&restored));
    }

    /// SPEC-KIT-977-A1: Empty source_files is a valid input.
    #[test]
    fn test_deterministic_hash_empty_source_files() {
        let config = Stage0Config::default();

        let snapshot1 = PolicySnapshot::capture(&config, vec![]);
        let snapshot2 = PolicySnapshot::capture(&config, vec![]);

        // Same empty input should produce same hash
        assert_eq!(
            snapshot1.hash, snapshot2.hash,
            "Empty source_files should produce deterministic hash"
        );
    }

    /// SPEC-KIT-977-A1: Source file order does NOT affect hash.
    ///
    /// Different ordering of source_files produces the SAME hash because
    /// the compute_hash function sorts source_files before hashing.
    /// This ensures filesystem discovery order doesn't affect policy identity.
    #[test]
    fn test_deterministic_hash_source_file_order_invariant() {
        let config = Stage0Config::default();

        let snapshot1 =
            PolicySnapshot::capture(&config, vec!["a.toml".to_string(), "b.toml".to_string()]);
        let snapshot2 =
            PolicySnapshot::capture(&config, vec!["b.toml".to_string(), "a.toml".to_string()]);

        // SPEC-KIT-977-A1: Different order = SAME hash (sources are sorted)
        assert_eq!(
            snapshot1.hash, snapshot2.hash,
            "Source file order should NOT affect hash (sources are sorted before hashing)"
        );

        // content_matches should return true
        assert!(
            snapshot1.content_matches(&snapshot2),
            "content_matches should return true for same content in different order"
        );
    }

    /// SPEC-KIT-977-A1: Prompts HashMap order does NOT affect hash.
    ///
    /// HashMap iteration order is nondeterministic, but the compute_hash
    /// function converts to BTreeMap before hashing, ensuring determinism.
    #[test]
    fn test_deterministic_hash_prompts_order_invariant() {
        let config = Stage0Config::default();
        let source_files = vec!["test.toml".to_string()];

        // Create two snapshots with prompts added in different order
        let mut snapshot1 = PolicySnapshot::capture(&config, source_files.clone());
        snapshot1
            .prompts
            .insert("key_a".to_string(), "value_a".to_string());
        snapshot1
            .prompts
            .insert("key_b".to_string(), "value_b".to_string());
        snapshot1.hash = snapshot1.compute_hash();

        let mut snapshot2 = PolicySnapshot::capture(&config, source_files);
        snapshot2
            .prompts
            .insert("key_b".to_string(), "value_b".to_string());
        snapshot2
            .prompts
            .insert("key_a".to_string(), "value_a".to_string());
        snapshot2.hash = snapshot2.compute_hash();

        // SPEC-KIT-977-A1: Different insertion order = SAME hash
        assert_eq!(
            snapshot1.hash, snapshot2.hash,
            "Prompts insertion order should NOT affect hash (keys are sorted before hashing)"
        );
    }

    /// SPEC-KIT-977-A1: Changing weights produces different hash.
    #[test]
    fn test_deterministic_hash_different_weights_different_hash() {
        let mut config1 = Stage0Config::default();
        let mut config2 = Stage0Config::default();

        // Modify weights
        config1.scoring.weights.usage = 0.5;
        config2.scoring.weights.usage = 0.9;

        let source_files = vec!["test.toml".to_string()];
        let snapshot1 = PolicySnapshot::capture(&config1, source_files.clone());
        let snapshot2 = PolicySnapshot::capture(&config2, source_files);

        // Different weights = different hash
        assert_ne!(
            snapshot1.hash, snapshot2.hash,
            "Different weights should produce different hash"
        );
    }

    // =========================================================================
    // SPEC-KIT-977: Governance Policy Tests
    // =========================================================================

    /// SPEC-KIT-977: Parse governance policy from TOML string.
    #[test]
    fn test_governance_policy_from_toml() {
        // Use a simple, minimal TOML to test parsing
        let toml_content = r##"[meta]
schema_version = "1.0"
effective_date = "2026-01-12"

[system_of_record]
primary = "memvid"
fallback = "local-memory"
fallback_enabled = true

[routing.cloud]
architect = ["claude-sonnet-4"]
implementer = ["claude-sonnet-4"]
judge = ["claude-sonnet-4"]
default_architect = "claude-sonnet-4"
default_implementer = "claude-sonnet-4"
default_judge = "claude-sonnet-4"

[routing.reflex]
enabled = false
endpoint = "http://127.0.0.1:3009/v1"
model = "qwen2.5-coder-7b"
timeout_ms = 1500
json_schema_required = true
fallback_to_cloud = true

[routing.reflex.thresholds]
p95_latency_ms = 2000
success_parity_percent = 85
json_schema_compliance_percent = 100

[capture]
mode = "prompts_only"
store_embeddings = true

[budgets.tokens]
plan = 8000
tasks = 4000
implement = 6000
validate = 4000
audit = 4000
unlock = 2000

[budgets.cost]
warn_threshold = 5.0
confirm_threshold = 10.0
hard_limit = 25.0
hard_limit_enabled = false

[scoring]
usage = 0.30
recency = 0.30
priority = 0.25
decay = 0.15
vector_weight = 0.6
lexical_weight = 0.4

[gates.reflex_promotion]
p95_latency_ms = 2000
success_parity_percent = 85
json_schema_compliance_percent = 100
golden_query_regression_allowed = false

[gates.local_memory_sunset]
retrieval_p95_parity = true
search_quality_parity = true
stability_days = 30
zero_fallback_activations = true

[security.redaction]
env_var_patterns = ["*_KEY", "*_SECRET"]
file_patterns = [".env", "credentials"]

[security.export]
require_explicit_action = true
allow_no_export_marking = true
encrypt_at_rest = false
"##;

        let policy = GovernancePolicy::from_toml(toml_content).expect("parse toml");

        // Verify meta
        assert_eq!(policy.meta.schema_version, "1.0");
        assert_eq!(policy.meta.effective_date, "2026-01-12");

        // Verify system_of_record
        assert_eq!(policy.system_of_record.primary, "memvid");
        assert!(policy.system_of_record.fallback_enabled);

        // Verify routing
        assert_eq!(policy.routing.cloud.architect.len(), 1);
        assert_eq!(policy.routing.reflex.endpoint, "http://127.0.0.1:3009/v1");
        assert_eq!(policy.routing.reflex.thresholds.p95_latency_ms, 2000);

        // Verify budgets
        assert_eq!(policy.budgets.tokens.plan, 8000);
        assert_eq!(policy.budgets.cost.hard_limit, 25.0);

        // Verify scoring
        assert_eq!(policy.scoring.usage, 0.30);
        assert_eq!(policy.scoring.vector_weight, 0.6);

        // Verify gates
        assert!(
            !policy
                .gates
                .reflex_promotion
                .golden_query_regression_allowed
        );
        assert_eq!(policy.gates.local_memory_sunset.stability_days, 30);

        // Verify security
        assert_eq!(policy.security.redaction.env_var_patterns.len(), 2);
        assert!(policy.security.export.require_explicit_action);
    }

    /// SPEC-KIT-977: Governance policy included in snapshot hash.
    ///
    /// Acceptance test: Changing a single routing value changes hash.
    #[test]
    fn test_governance_changes_snapshot_hash() {
        let config = Stage0Config::default();
        let source_files = vec!["test.toml".to_string()];

        // Create governance with reflex disabled
        let mut gov1 = GovernancePolicy::default();
        gov1.routing.reflex.enabled = false;
        gov1.routing.reflex.thresholds.p95_latency_ms = 2000;

        // Create governance with reflex enabled (only difference)
        let mut gov2 = GovernancePolicy::default();
        gov2.routing.reflex.enabled = true;
        gov2.routing.reflex.thresholds.p95_latency_ms = 2000;

        let snapshot1 =
            PolicySnapshot::capture_with_governance(&config, source_files.clone(), Some(gov1));
        let snapshot2 = PolicySnapshot::capture_with_governance(&config, source_files, Some(gov2));

        // Different governance = different hash
        assert_ne!(
            snapshot1.hash, snapshot2.hash,
            "Changing routing.reflex.enabled should produce different hash"
        );
    }

    /// SPEC-KIT-977: Identical governance produces identical hash.
    #[test]
    fn test_governance_identical_produces_same_hash() {
        let config = Stage0Config::default();
        let source_files = vec!["test.toml".to_string()];

        let gov1 = GovernancePolicy::default();
        let gov2 = GovernancePolicy::default();

        let snapshot1 =
            PolicySnapshot::capture_with_governance(&config, source_files.clone(), Some(gov1));
        let snapshot2 = PolicySnapshot::capture_with_governance(&config, source_files, Some(gov2));

        // Identical governance = identical hash
        assert_eq!(
            snapshot1.hash, snapshot2.hash,
            "Identical governance should produce identical hash"
        );
    }

    /// SPEC-KIT-977: Snapshot JSON roundtrip preserves governance.
    #[test]
    fn test_governance_json_roundtrip() {
        let config = Stage0Config::default();
        let source_files = vec!["test.toml".to_string()];

        let mut gov = GovernancePolicy::default();
        gov.meta.schema_version = "1.0".to_string();
        gov.meta.effective_date = "2026-01-12".to_string();
        gov.routing.reflex.enabled = true;
        gov.routing.reflex.model = "test-model".to_string();
        gov.budgets.tokens.plan = 9999;

        let original = PolicySnapshot::capture_with_governance(&config, source_files, Some(gov));

        // Serialize and deserialize
        let json = original.to_json().expect("serialize");
        let restored = PolicySnapshot::from_json(&json).expect("deserialize");

        // Verify governance preserved
        assert_eq!(original.hash, restored.hash);
        assert!(restored.verify_hash());

        let gov_restored = restored.governance.expect("governance present");
        assert_eq!(gov_restored.meta.effective_date, "2026-01-12");
        assert!(gov_restored.routing.reflex.enabled);
        assert_eq!(gov_restored.routing.reflex.model, "test-model");
        assert_eq!(gov_restored.budgets.tokens.plan, 9999);
    }

    /// SPEC-KIT-977: None governance produces consistent hash.
    #[test]
    fn test_no_governance_produces_consistent_hash() {
        let config = Stage0Config::default();
        let source_files = vec!["test.toml".to_string()];

        let snapshot1 =
            PolicySnapshot::capture_with_governance(&config, source_files.clone(), None);
        let snapshot2 = PolicySnapshot::capture_with_governance(&config, source_files, None);

        // No governance should still produce consistent hash
        assert_eq!(
            snapshot1.hash, snapshot2.hash,
            "None governance should produce consistent hash"
        );
    }

    /// SPEC-KIT-977: Governance affects hash differently than no governance.
    #[test]
    fn test_governance_vs_no_governance_hash_differs() {
        let config = Stage0Config::default();
        let source_files = vec!["test.toml".to_string()];

        let snapshot_none =
            PolicySnapshot::capture_with_governance(&config, source_files.clone(), None);

        let snapshot_some = PolicySnapshot::capture_with_governance(
            &config,
            source_files,
            Some(GovernancePolicy::default()),
        );

        // With vs without governance produces different hash
        assert_ne!(
            snapshot_none.hash, snapshot_some.hash,
            "With governance vs without should produce different hash"
        );
    }

    // =========================================================================
    // PolicyDiff Tests
    // =========================================================================

    /// Identical policies produce empty diff with identical=true.
    #[test]
    fn test_policy_diff_identical() {
        let config = Stage0Config::default();
        let source_files = vec!["test.toml".to_string()];

        let snapshot1 = PolicySnapshot::capture(&config, source_files.clone());
        let snapshot2 = PolicySnapshot::capture(&config, source_files);

        let diff = PolicyDiff::compute(&snapshot1, &snapshot2);

        assert!(
            diff.identical,
            "Identical content should produce identical=true"
        );
        assert!(
            diff.changes.is_empty(),
            "Identical policies should have no changes"
        );
    }

    /// Different source files produce changes.
    #[test]
    fn test_policy_diff_source_files_changed() {
        let config = Stage0Config::default();

        let snapshot1 = PolicySnapshot::capture(&config, vec!["file1.toml".to_string()]);
        let snapshot2 = PolicySnapshot::capture(&config, vec!["file2.toml".to_string()]);

        let diff = PolicyDiff::compute(&snapshot1, &snapshot2);

        assert!(!diff.identical);
        assert_eq!(diff.changes.len(), 1);
        assert_eq!(diff.changes[0].path, "source_files");
        assert_eq!(diff.changes[0].category, ChangeCategory::SourceFiles);
    }

    /// Different model config produces changes.
    #[test]
    fn test_policy_diff_model_config_changed() {
        let mut config1 = Stage0Config::default();
        let mut config2 = Stage0Config::default();

        config1.context_compiler.top_k = 10;
        config2.context_compiler.top_k = 20;

        let source_files = vec!["test.toml".to_string()];
        let snapshot1 = PolicySnapshot::capture(&config1, source_files.clone());
        let snapshot2 = PolicySnapshot::capture(&config2, source_files);

        let diff = PolicyDiff::compute(&snapshot1, &snapshot2);

        assert!(!diff.identical);
        let top_k_change = diff.changes.iter().find(|c| c.path == "model_config.top_k");
        assert!(
            top_k_change.is_some(),
            "Should have model_config.top_k change"
        );
        assert_eq!(top_k_change.unwrap().category, ChangeCategory::ModelConfig);
    }

    /// Different weights produce changes.
    #[test]
    fn test_policy_diff_weights_changed() {
        let mut config1 = Stage0Config::default();
        let mut config2 = Stage0Config::default();

        config1.scoring.weights.usage = 0.5;
        config2.scoring.weights.usage = 0.9;

        let source_files = vec!["test.toml".to_string()];
        let snapshot1 = PolicySnapshot::capture(&config1, source_files.clone());
        let snapshot2 = PolicySnapshot::capture(&config2, source_files);

        let diff = PolicyDiff::compute(&snapshot1, &snapshot2);

        assert!(!diff.identical);
        let usage_change = diff.changes.iter().find(|c| c.path == "weights.usage");
        assert!(usage_change.is_some(), "Should have weights.usage change");
        assert_eq!(usage_change.unwrap().category, ChangeCategory::Weights);
    }

    /// Governance changes are detected.
    #[test]
    fn test_policy_diff_governance_changed() {
        let config = Stage0Config::default();
        let source_files = vec!["test.toml".to_string()];

        let mut gov1 = GovernancePolicy::default();
        gov1.routing.reflex.enabled = false;

        let mut gov2 = GovernancePolicy::default();
        gov2.routing.reflex.enabled = true;

        let snapshot1 =
            PolicySnapshot::capture_with_governance(&config, source_files.clone(), Some(gov1));
        let snapshot2 = PolicySnapshot::capture_with_governance(&config, source_files, Some(gov2));

        let diff = PolicyDiff::compute(&snapshot1, &snapshot2);

        assert!(!diff.identical);
        let reflex_change = diff
            .changes
            .iter()
            .find(|c| c.path == "governance.routing.reflex.enabled");
        assert!(reflex_change.is_some(), "Should have reflex enabled change");
        assert_eq!(reflex_change.unwrap().category, ChangeCategory::Governance);
    }

    /// Diff output is deterministic (same ordering).
    #[test]
    fn test_policy_diff_deterministic_ordering() {
        let mut config1 = Stage0Config::default();
        let mut config2 = Stage0Config::default();

        // Change multiple fields
        config1.context_compiler.top_k = 10;
        config2.context_compiler.top_k = 20;
        config1.scoring.weights.usage = 0.3;
        config2.scoring.weights.usage = 0.5;

        let snapshot1 = PolicySnapshot::capture(&config1, vec!["a.toml".to_string()]);
        let snapshot2 = PolicySnapshot::capture(&config2, vec!["b.toml".to_string()]);

        // Compute diff multiple times
        let diff1 = PolicyDiff::compute(&snapshot1, &snapshot2);
        let diff2 = PolicyDiff::compute(&snapshot1, &snapshot2);

        // Same order every time
        assert_eq!(diff1.changes.len(), diff2.changes.len());
        for (a, b) in diff1.changes.iter().zip(diff2.changes.iter()) {
            assert_eq!(a.path, b.path, "Order should be deterministic");
        }

        // Paths should be sorted
        let paths: Vec<_> = diff1.changes.iter().map(|c| c.path.as_str()).collect();
        let mut sorted_paths = paths.clone();
        sorted_paths.sort();
        assert_eq!(paths, sorted_paths, "Changes should be sorted by path");
    }

    /// JSON output is stable and machine-parseable.
    #[test]
    fn test_policy_diff_json_stable() {
        let config = Stage0Config::default();

        let snapshot1 = PolicySnapshot::capture(&config, vec!["file1.toml".to_string()]);
        let snapshot2 = PolicySnapshot::capture(&config, vec!["file2.toml".to_string()]);

        let diff = PolicyDiff::compute(&snapshot1, &snapshot2);
        let json = diff.to_json().expect("JSON serialization should succeed");

        // Parse back
        let parsed: PolicyDiff = serde_json::from_str(&json).expect("JSON should parse back");

        assert_eq!(parsed.policy_id_a, diff.policy_id_a);
        assert_eq!(parsed.policy_id_b, diff.policy_id_b);
        assert_eq!(parsed.identical, diff.identical);
        assert_eq!(parsed.changes.len(), diff.changes.len());
    }

    /// changed_keys returns sorted list.
    #[test]
    fn test_policy_diff_changed_keys() {
        let mut config1 = Stage0Config::default();
        let mut config2 = Stage0Config::default();

        config1.context_compiler.top_k = 10;
        config2.context_compiler.top_k = 20;

        let snapshot1 = PolicySnapshot::capture(&config1, vec!["a.toml".to_string()]);
        let snapshot2 = PolicySnapshot::capture(&config2, vec!["b.toml".to_string()]);

        let diff = PolicyDiff::compute(&snapshot1, &snapshot2);
        let keys = diff.changed_keys();

        // Should contain expected keys
        assert!(keys.contains(&"model_config.top_k"));
        assert!(keys.contains(&"source_files"));
    }
}
