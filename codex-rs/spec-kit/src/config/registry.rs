//! Canonical configuration field path registry.
//!
//! Provides type-safe access to configuration fields with bidirectional conversion
//! between environment variables, TOML keys, and strongly-typed field paths.
//!
//! # Overview
//!
//! This module centralizes configuration field names to prevent typos and ensure
//! consistency across environment variables, TOML configuration files, and Rust code.
//!
//! ## Key Features
//!
//! - **Type-safe paths**: Use [`ConfigPath`] builder for compile-time field validation
//! - **Bidirectional conversion**: Convert between env vars, TOML keys, and [`FieldPath`]
//! - **Validation helpers**: Get field descriptions, types, and constraints
//! - **Unknown var detection**: Warn about typos in environment variable names
//!
//! # Usage Examples
//!
//! ## Type-safe Path Construction
//!
//! ```
//! use codex_spec_kit::config::registry::ConfigPath;
//!
//! // Quality gates configuration
//! let enabled = ConfigPath::quality_gates().enabled();
//! assert_eq!(enabled.to_env_var(), "SPECKIT_QUALITY_GATES__ENABLED");
//! assert_eq!(enabled.to_toml_key(), "quality_gates.enabled");
//!
//! // Cost limits
//! let daily_limit = ConfigPath::cost().daily_limit_usd();
//! assert_eq!(daily_limit.to_toml_key(), "cost.daily_limit_usd");
//!
//! // Model-specific settings (dynamic)
//! let temp = ConfigPath::model("openai").temperature();
//! assert_eq!(temp.to_env_var(), "SPECKIT_MODELS__OPENAI__TEMPERATURE");
//! ```
//!
//! ## Environment Variable Parsing
//!
//! ```
//! use codex_spec_kit::config::registry::FieldPath;
//!
//! // Parse from environment variable
//! let path = FieldPath::from_env_var("SPECKIT_COST__DAILY_LIMIT_USD");
//! assert!(matches!(path, Some(FieldPath::Cost_DailyLimitUsd)));
//!
//! // Parse from TOML key
//! let path = FieldPath::from_toml_key("consensus.min_agents");
//! assert!(matches!(path, Some(FieldPath::Consensus_MinAgents)));
//!
//! // Detect unknown variables
//! assert!(!FieldPath::is_known_env_var("SPECKIT_UNKNOWN__FIELD"));
//! ```
//!
//! ## Validation and Metadata
//!
//! ```
//! use codex_spec_kit::config::registry::{FieldPath, ValueType};
//!
//! let path = FieldPath::QualityGates_ConsensusThreshold;
//!
//! // Get human-readable description
//! assert_eq!(
//!     path.description(),
//!     "Minimum consensus agreement threshold (0.0-1.0)"
//! );
//!
//! // Check expected type
//! assert_eq!(path.value_type(), ValueType::Float);
//!
//! // Get constraints
//! let constraints = path.constraints().unwrap();
//! assert_eq!(constraints.min, Some(0.0));
//! assert_eq!(constraints.max, Some(1.0));
//! ```
//!
//! # Adding New Fields
//!
//! To add a new configuration field:
//!
//! 1. Add variant to [`FieldPath`] enum
//! 2. Implement conversion in `to_env_var()` and `to_toml_key()`
//! 3. Add parsing in `from_env_var()` and `from_toml_key()`
//! 4. Add to `all_known_env_prefixes()` (for static fields)
//! 5. Add builder method to appropriate path struct
//! 6. Add description, value type, and constraints (if applicable)
//!
//! # Naming Conventions
//!
//! - **FieldPath variants**: `Section_FieldName` (e.g., `Cost_DailyLimitUsd`)
//! - **Environment variables**: `SPECKIT_SECTION__FIELD` (double underscore for nesting)
//! - **TOML keys**: `section.field` (dot-separated, snake_case)
//! - **Builder methods**: `snake_case` (e.g., `cost().daily_limit_usd()`)

use std::fmt;

/// Canonical field paths for all configuration values.
///
/// Naming convention: `Section_FieldName` (e.g., `QualityGates_Enabled`)
///
/// Dynamic variants (e.g., model configs) use `String` parameters to support
/// arbitrary provider names.
///
/// Note: Underscore naming is intentional for clear mapping to env vars/TOML keys.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldPath {
    // Quality Gates
    QualityGates_Enabled,
    QualityGates_ConsensusThreshold,
    QualityGates_MinTestCoverage,
    QualityGates_SchemaValidation,

    // Cost
    Cost_Enabled,
    Cost_DailyLimitUsd,
    Cost_MonthlyLimitUsd,
    Cost_AlertThreshold,

    // Evidence
    Evidence_Enabled,
    Evidence_BaseDir,
    Evidence_MaxSizePerSpecMb,
    Evidence_RetentionDays,

    // Consensus
    Consensus_MinAgents,
    Consensus_MaxAgents,
    Consensus_TimeoutSeconds,

    // Models (dynamic - provider name as parameter)
    Model_Name(String),
    Model_Endpoint(String),
    Model_Temperature(String),
    Model_MaxTokens(String),
    Model_CostPerInputMillion(String),
    Model_CostPerOutputMillion(String),
    Model_Retry_MaxRetries(String),
    Model_Retry_BaseDelayMs(String),
    Model_Retry_MaxDelayMs(String),
}

impl FieldPath {
    /// Convert to environment variable name.
    ///
    /// Format: `SPECKIT_SECTION__FIELD` (double underscore for nesting)
    ///
    /// # Examples
    ///
    /// ```
    /// # use codex_spec_kit::config::registry::FieldPath;
    /// assert_eq!(
    ///     FieldPath::QualityGates_Enabled.to_env_var(),
    ///     "SPECKIT_QUALITY_GATES__ENABLED"
    /// );
    /// assert_eq!(
    ///     FieldPath::Model_Temperature("openai".into()).to_env_var(),
    ///     "SPECKIT_MODELS__OPENAI__TEMPERATURE"
    /// );
    /// ```
    pub fn to_env_var(&self) -> String {
        match self {
            // Quality Gates
            Self::QualityGates_Enabled => "SPECKIT_QUALITY_GATES__ENABLED".into(),
            Self::QualityGates_ConsensusThreshold => "SPECKIT_QUALITY_GATES__CONSENSUS_THRESHOLD".into(),
            Self::QualityGates_MinTestCoverage => "SPECKIT_QUALITY_GATES__MIN_TEST_COVERAGE".into(),
            Self::QualityGates_SchemaValidation => "SPECKIT_QUALITY_GATES__SCHEMA_VALIDATION".into(),

            // Cost
            Self::Cost_Enabled => "SPECKIT_COST__ENABLED".into(),
            Self::Cost_DailyLimitUsd => "SPECKIT_COST__DAILY_LIMIT_USD".into(),
            Self::Cost_MonthlyLimitUsd => "SPECKIT_COST__MONTHLY_LIMIT_USD".into(),
            Self::Cost_AlertThreshold => "SPECKIT_COST__ALERT_THRESHOLD".into(),

            // Evidence
            Self::Evidence_Enabled => "SPECKIT_EVIDENCE__ENABLED".into(),
            Self::Evidence_BaseDir => "SPECKIT_EVIDENCE__BASE_DIR".into(),
            Self::Evidence_MaxSizePerSpecMb => "SPECKIT_EVIDENCE__MAX_SIZE_PER_SPEC_MB".into(),
            Self::Evidence_RetentionDays => "SPECKIT_EVIDENCE__RETENTION_DAYS".into(),

            // Consensus
            Self::Consensus_MinAgents => "SPECKIT_CONSENSUS__MIN_AGENTS".into(),
            Self::Consensus_MaxAgents => "SPECKIT_CONSENSUS__MAX_AGENTS".into(),
            Self::Consensus_TimeoutSeconds => "SPECKIT_CONSENSUS__TIMEOUT_SECONDS".into(),

            // Models (dynamic)
            Self::Model_Name(provider) => format!("SPECKIT_MODELS__{provider}__MODEL", provider = provider.to_uppercase().replace('-', "_")),
            Self::Model_Endpoint(provider) => format!("SPECKIT_MODELS__{provider}__ENDPOINT", provider = provider.to_uppercase().replace('-', "_")),
            Self::Model_Temperature(provider) => format!("SPECKIT_MODELS__{provider}__TEMPERATURE", provider = provider.to_uppercase().replace('-', "_")),
            Self::Model_MaxTokens(provider) => format!("SPECKIT_MODELS__{provider}__MAX_TOKENS", provider = provider.to_uppercase().replace('-', "_")),
            Self::Model_CostPerInputMillion(provider) => format!("SPECKIT_MODELS__{provider}__COST_PER_INPUT_MILLION", provider = provider.to_uppercase().replace('-', "_")),
            Self::Model_CostPerOutputMillion(provider) => format!("SPECKIT_MODELS__{provider}__COST_PER_OUTPUT_MILLION", provider = provider.to_uppercase().replace('-', "_")),
            Self::Model_Retry_MaxRetries(provider) => format!("SPECKIT_MODELS__{provider}__RETRY__MAX_RETRIES", provider = provider.to_uppercase().replace('-', "_")),
            Self::Model_Retry_BaseDelayMs(provider) => format!("SPECKIT_MODELS__{provider}__RETRY__BASE_DELAY_MS", provider = provider.to_uppercase().replace('-', "_")),
            Self::Model_Retry_MaxDelayMs(provider) => format!("SPECKIT_MODELS__{provider}__RETRY__MAX_DELAY_MS", provider = provider.to_uppercase().replace('-', "_")),
        }
    }

    /// Convert to TOML key path.
    ///
    /// Format: `section.field` (dot-separated)
    ///
    /// # Examples
    ///
    /// ```
    /// # use codex_spec_kit::config::registry::FieldPath;
    /// assert_eq!(
    ///     FieldPath::Cost_DailyLimitUsd.to_toml_key(),
    ///     "cost.daily_limit_usd"
    /// );
    /// assert_eq!(
    ///     FieldPath::Model_Temperature("gemini".into()).to_toml_key(),
    ///     "models.gemini.temperature"
    /// );
    /// ```
    pub fn to_toml_key(&self) -> String {
        match self {
            // Quality Gates
            Self::QualityGates_Enabled => "quality_gates.enabled".into(),
            Self::QualityGates_ConsensusThreshold => "quality_gates.consensus_threshold".into(),
            Self::QualityGates_MinTestCoverage => "quality_gates.min_test_coverage".into(),
            Self::QualityGates_SchemaValidation => "quality_gates.schema_validation".into(),

            // Cost
            Self::Cost_Enabled => "cost.enabled".into(),
            Self::Cost_DailyLimitUsd => "cost.daily_limit_usd".into(),
            Self::Cost_MonthlyLimitUsd => "cost.monthly_limit_usd".into(),
            Self::Cost_AlertThreshold => "cost.alert_threshold".into(),

            // Evidence
            Self::Evidence_Enabled => "evidence.enabled".into(),
            Self::Evidence_BaseDir => "evidence.base_dir".into(),
            Self::Evidence_MaxSizePerSpecMb => "evidence.max_size_per_spec_mb".into(),
            Self::Evidence_RetentionDays => "evidence.retention_days".into(),

            // Consensus
            Self::Consensus_MinAgents => "consensus.min_agents".into(),
            Self::Consensus_MaxAgents => "consensus.max_agents".into(),
            Self::Consensus_TimeoutSeconds => "consensus.timeout_seconds".into(),

            // Models (dynamic)
            Self::Model_Name(provider) => format!("models.{provider}.model"),
            Self::Model_Endpoint(provider) => format!("models.{provider}.endpoint"),
            Self::Model_Temperature(provider) => format!("models.{provider}.temperature"),
            Self::Model_MaxTokens(provider) => format!("models.{provider}.max_tokens"),
            Self::Model_CostPerInputMillion(provider) => format!("models.{provider}.cost_per_input_million"),
            Self::Model_CostPerOutputMillion(provider) => format!("models.{provider}.cost_per_output_million"),
            Self::Model_Retry_MaxRetries(provider) => format!("models.{provider}.retry.max_retries"),
            Self::Model_Retry_BaseDelayMs(provider) => format!("models.{provider}.retry.base_delay_ms"),
            Self::Model_Retry_MaxDelayMs(provider) => format!("models.{provider}.retry.max_delay_ms"),
        }
    }

    /// Parse from environment variable name.
    ///
    /// Returns `None` if the variable name doesn't match the `SPECKIT_*` pattern
    /// or doesn't correspond to a known field.
    ///
    /// # Examples
    ///
    /// ```
    /// # use codex_spec_kit::config::registry::FieldPath;
    /// let path = FieldPath::from_env_var("SPECKIT_EVIDENCE__ENABLED");
    /// assert!(matches!(path, Some(FieldPath::Evidence_Enabled)));
    ///
    /// let invalid = FieldPath::from_env_var("UNKNOWN_VAR");
    /// assert_eq!(invalid, None);
    /// ```
    pub fn from_env_var(s: &str) -> Option<Self> {
        // Must start with SPECKIT_
        let s = s.strip_prefix("SPECKIT_")?;

        match s {
            // Quality Gates
            "QUALITY_GATES__ENABLED" => Some(Self::QualityGates_Enabled),
            "QUALITY_GATES__CONSENSUS_THRESHOLD" => Some(Self::QualityGates_ConsensusThreshold),
            "QUALITY_GATES__MIN_TEST_COVERAGE" => Some(Self::QualityGates_MinTestCoverage),
            "QUALITY_GATES__SCHEMA_VALIDATION" => Some(Self::QualityGates_SchemaValidation),

            // Cost
            "COST__ENABLED" => Some(Self::Cost_Enabled),
            "COST__DAILY_LIMIT_USD" => Some(Self::Cost_DailyLimitUsd),
            "COST__MONTHLY_LIMIT_USD" => Some(Self::Cost_MonthlyLimitUsd),
            "COST__ALERT_THRESHOLD" => Some(Self::Cost_AlertThreshold),

            // Evidence
            "EVIDENCE__ENABLED" => Some(Self::Evidence_Enabled),
            "EVIDENCE__BASE_DIR" => Some(Self::Evidence_BaseDir),
            "EVIDENCE__MAX_SIZE_PER_SPEC_MB" => Some(Self::Evidence_MaxSizePerSpecMb),
            "EVIDENCE__RETENTION_DAYS" => Some(Self::Evidence_RetentionDays),

            // Consensus
            "CONSENSUS__MIN_AGENTS" => Some(Self::Consensus_MinAgents),
            "CONSENSUS__MAX_AGENTS" => Some(Self::Consensus_MaxAgents),
            "CONSENSUS__TIMEOUT_SECONDS" => Some(Self::Consensus_TimeoutSeconds),

            // Models (dynamic) - e.g., MODELS__OPENAI__TEMPERATURE
            other if other.starts_with("MODELS__") => {
                Self::parse_model_env_var(other)
            }

            _ => None,
        }
    }

    /// Parse from TOML key path.
    ///
    /// Returns `None` if the key doesn't correspond to a known field.
    ///
    /// # Examples
    ///
    /// ```
    /// # use codex_spec_kit::config::registry::FieldPath;
    /// let path = FieldPath::from_toml_key("consensus.min_agents");
    /// assert!(matches!(path, Some(FieldPath::Consensus_MinAgents)));
    ///
    /// let invalid = FieldPath::from_toml_key("unknown.field");
    /// assert_eq!(invalid, None);
    /// ```
    pub fn from_toml_key(s: &str) -> Option<Self> {
        match s {
            // Quality Gates
            "quality_gates.enabled" => Some(Self::QualityGates_Enabled),
            "quality_gates.consensus_threshold" => Some(Self::QualityGates_ConsensusThreshold),
            "quality_gates.min_test_coverage" => Some(Self::QualityGates_MinTestCoverage),
            "quality_gates.schema_validation" => Some(Self::QualityGates_SchemaValidation),

            // Cost
            "cost.enabled" => Some(Self::Cost_Enabled),
            "cost.daily_limit_usd" => Some(Self::Cost_DailyLimitUsd),
            "cost.monthly_limit_usd" => Some(Self::Cost_MonthlyLimitUsd),
            "cost.alert_threshold" => Some(Self::Cost_AlertThreshold),

            // Evidence
            "evidence.enabled" => Some(Self::Evidence_Enabled),
            "evidence.base_dir" => Some(Self::Evidence_BaseDir),
            "evidence.max_size_per_spec_mb" => Some(Self::Evidence_MaxSizePerSpecMb),
            "evidence.retention_days" => Some(Self::Evidence_RetentionDays),

            // Consensus
            "consensus.min_agents" => Some(Self::Consensus_MinAgents),
            "consensus.max_agents" => Some(Self::Consensus_MaxAgents),
            "consensus.timeout_seconds" => Some(Self::Consensus_TimeoutSeconds),

            // Models (dynamic) - e.g., models.openai.temperature
            other if other.starts_with("models.") => {
                Self::parse_model_toml_key(other)
            }

            _ => None,
        }
    }

    // Helper: Parse model environment variable (e.g., MODELS__OPENAI__TEMPERATURE)
    fn parse_model_env_var(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split("__").collect();

        if parts.len() < 3 || parts[0] != "MODELS" {
            return None;
        }

        let provider = parts[1].to_lowercase().replace('_', "-");

        match parts.len() {
            3 => {
                // MODELS__PROVIDER__FIELD
                match parts[2] {
                    "MODEL" => Some(Self::Model_Name(provider)),
                    "ENDPOINT" => Some(Self::Model_Endpoint(provider)),
                    "TEMPERATURE" => Some(Self::Model_Temperature(provider)),
                    "MAX_TOKENS" => Some(Self::Model_MaxTokens(provider)),
                    "COST_PER_INPUT_MILLION" => Some(Self::Model_CostPerInputMillion(provider)),
                    "COST_PER_OUTPUT_MILLION" => Some(Self::Model_CostPerOutputMillion(provider)),
                    _ => None,
                }
            }
            4 if parts[2] == "RETRY" => {
                // MODELS__PROVIDER__RETRY__FIELD
                match parts[3] {
                    "MAX_RETRIES" => Some(Self::Model_Retry_MaxRetries(provider)),
                    "BASE_DELAY_MS" => Some(Self::Model_Retry_BaseDelayMs(provider)),
                    "MAX_DELAY_MS" => Some(Self::Model_Retry_MaxDelayMs(provider)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    // Helper: Parse model TOML key (e.g., models.openai.temperature)
    fn parse_model_toml_key(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();

        if parts.len() < 3 || parts[0] != "models" {
            return None;
        }

        let provider = parts[1].to_string();

        match parts.len() {
            3 => {
                // models.provider.field
                match parts[2] {
                    "model" => Some(Self::Model_Name(provider)),
                    "endpoint" => Some(Self::Model_Endpoint(provider)),
                    "temperature" => Some(Self::Model_Temperature(provider)),
                    "max_tokens" => Some(Self::Model_MaxTokens(provider)),
                    "cost_per_input_million" => Some(Self::Model_CostPerInputMillion(provider)),
                    "cost_per_output_million" => Some(Self::Model_CostPerOutputMillion(provider)),
                    _ => None,
                }
            }
            4 if parts[2] == "retry" => {
                // models.provider.retry.field
                match parts[3] {
                    "max_retries" => Some(Self::Model_Retry_MaxRetries(provider)),
                    "base_delay_ms" => Some(Self::Model_Retry_BaseDelayMs(provider)),
                    "max_delay_ms" => Some(Self::Model_Retry_MaxDelayMs(provider)),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl fmt::Display for FieldPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_toml_key())
    }
}

// ============================================================================
// Task 2: ConfigPath Builder
// ============================================================================

/// Type-safe builder for configuration field paths.
///
/// Provides fluent API for constructing `FieldPath` values without string literals.
///
/// # Examples
///
/// ```
/// use codex_spec_kit::config::registry::ConfigPath;
///
/// // Quality gates
/// let enabled = ConfigPath::quality_gates().enabled();
/// assert_eq!(enabled.to_env_var(), "SPECKIT_QUALITY_GATES__ENABLED");
///
/// // Cost limits
/// let daily_limit = ConfigPath::cost().daily_limit_usd();
/// assert_eq!(daily_limit.to_toml_key(), "cost.daily_limit_usd");
///
/// // Model configs (dynamic)
/// let temp = ConfigPath::model("openai").temperature();
/// assert_eq!(temp.to_toml_key(), "models.openai.temperature");
/// ```
pub struct ConfigPath;

impl ConfigPath {
    /// Quality gate configuration paths.
    pub fn quality_gates() -> QualityGatesPath {
        QualityGatesPath
    }

    /// Cost tracking configuration paths.
    pub fn cost() -> CostPath {
        CostPath
    }

    /// Evidence collection configuration paths.
    pub fn evidence() -> EvidencePath {
        EvidencePath
    }

    /// Consensus configuration paths.
    pub fn consensus() -> ConsensusPath {
        ConsensusPath
    }

    /// Model-specific configuration paths.
    ///
    /// # Arguments
    ///
    /// * `provider` - Model provider name (e.g., "openai", "gemini", "claude")
    pub fn model(provider: &str) -> ModelPath {
        ModelPath {
            provider: provider.to_string(),
        }
    }
}

/// Quality gate configuration paths.
#[derive(Debug, Clone)]
pub struct QualityGatesPath;

impl QualityGatesPath {
    /// Quality gates enabled flag.
    pub fn enabled(&self) -> FieldPath {
        FieldPath::QualityGates_Enabled
    }

    /// Consensus agreement threshold (0.0-1.0).
    pub fn consensus_threshold(&self) -> FieldPath {
        FieldPath::QualityGates_ConsensusThreshold
    }

    /// Minimum test coverage percentage.
    pub fn min_test_coverage(&self) -> FieldPath {
        FieldPath::QualityGates_MinTestCoverage
    }

    /// Schema validation enabled flag.
    pub fn schema_validation(&self) -> FieldPath {
        FieldPath::QualityGates_SchemaValidation
    }
}

/// Cost tracking configuration paths.
#[derive(Debug, Clone)]
pub struct CostPath;

impl CostPath {
    /// Cost tracking enabled flag.
    pub fn enabled(&self) -> FieldPath {
        FieldPath::Cost_Enabled
    }

    /// Daily cost limit in USD.
    pub fn daily_limit_usd(&self) -> FieldPath {
        FieldPath::Cost_DailyLimitUsd
    }

    /// Monthly cost limit in USD.
    pub fn monthly_limit_usd(&self) -> FieldPath {
        FieldPath::Cost_MonthlyLimitUsd
    }

    /// Alert threshold (percentage of limit).
    pub fn alert_threshold(&self) -> FieldPath {
        FieldPath::Cost_AlertThreshold
    }
}

/// Evidence collection configuration paths.
#[derive(Debug, Clone)]
pub struct EvidencePath;

impl EvidencePath {
    /// Evidence collection enabled flag.
    pub fn enabled(&self) -> FieldPath {
        FieldPath::Evidence_Enabled
    }

    /// Base directory for evidence storage.
    pub fn base_dir(&self) -> FieldPath {
        FieldPath::Evidence_BaseDir
    }

    /// Maximum evidence size per SPEC in MB.
    pub fn max_size_per_spec_mb(&self) -> FieldPath {
        FieldPath::Evidence_MaxSizePerSpecMb
    }

    /// Evidence retention period in days.
    pub fn retention_days(&self) -> FieldPath {
        FieldPath::Evidence_RetentionDays
    }
}

/// Consensus configuration paths.
#[derive(Debug, Clone)]
pub struct ConsensusPath;

impl ConsensusPath {
    /// Minimum number of agents required.
    pub fn min_agents(&self) -> FieldPath {
        FieldPath::Consensus_MinAgents
    }

    /// Maximum number of agents.
    pub fn max_agents(&self) -> FieldPath {
        FieldPath::Consensus_MaxAgents
    }

    /// Consensus timeout in seconds.
    pub fn timeout_seconds(&self) -> FieldPath {
        FieldPath::Consensus_TimeoutSeconds
    }
}

/// Model-specific configuration paths.
#[derive(Debug, Clone)]
pub struct ModelPath {
    provider: String,
}

impl ModelPath {
    /// Model identifier.
    pub fn model(&self) -> FieldPath {
        FieldPath::Model_Name(self.provider.clone())
    }

    /// API endpoint URL.
    pub fn endpoint(&self) -> FieldPath {
        FieldPath::Model_Endpoint(self.provider.clone())
    }

    /// Temperature setting.
    pub fn temperature(&self) -> FieldPath {
        FieldPath::Model_Temperature(self.provider.clone())
    }

    /// Maximum tokens.
    pub fn max_tokens(&self) -> FieldPath {
        FieldPath::Model_MaxTokens(self.provider.clone())
    }

    /// Cost per 1M input tokens (USD).
    pub fn cost_per_input_million(&self) -> FieldPath {
        FieldPath::Model_CostPerInputMillion(self.provider.clone())
    }

    /// Cost per 1M output tokens (USD).
    pub fn cost_per_output_million(&self) -> FieldPath {
        FieldPath::Model_CostPerOutputMillion(self.provider.clone())
    }

    /// Retry configuration builder.
    pub fn retry(&self) -> ModelRetryPath {
        ModelRetryPath {
            provider: self.provider.clone(),
        }
    }
}

/// Model retry configuration paths.
#[derive(Debug, Clone)]
pub struct ModelRetryPath {
    provider: String,
}

impl ModelRetryPath {
    /// Maximum retry attempts.
    pub fn max_retries(&self) -> FieldPath {
        FieldPath::Model_Retry_MaxRetries(self.provider.clone())
    }

    /// Base delay in milliseconds.
    pub fn base_delay_ms(&self) -> FieldPath {
        FieldPath::Model_Retry_BaseDelayMs(self.provider.clone())
    }

    /// Maximum delay in milliseconds.
    pub fn max_delay_ms(&self) -> FieldPath {
        FieldPath::Model_Retry_MaxDelayMs(self.provider.clone())
    }
}

// ============================================================================
// Task 3: Validation Helpers
// ============================================================================

/// Expected value type for a configuration field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    /// Boolean (true/false)
    Bool,
    /// Integer number
    Integer,
    /// Floating-point number
    Float,
    /// String value
    String,
    /// Filesystem path
    Path,
}

/// Validation constraints for a configuration field.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraints {
    /// Minimum value (for numeric fields)
    pub min: Option<f64>,
    /// Maximum value (for numeric fields)
    pub max: Option<f64>,
    /// Regular expression pattern (for string fields)
    pub pattern: Option<&'static str>,
}

impl FieldPath {
    /// Get all known static environment variable names (excludes dynamic model paths).
    ///
    /// This is useful for validating environment variables during configuration loading.
    ///
    /// # Examples
    ///
    /// ```
    /// # use codex_spec_kit::config::registry::FieldPath;
    /// let known_vars = FieldPath::all_known_env_prefixes();
    /// assert!(known_vars.contains(&"SPECKIT_QUALITY_GATES__ENABLED"));
    /// assert!(known_vars.contains(&"SPECKIT_COST__DAILY_LIMIT_USD"));
    /// ```
    pub fn all_known_env_prefixes() -> Vec<&'static str> {
        vec![
            // Quality Gates
            "SPECKIT_QUALITY_GATES__ENABLED",
            "SPECKIT_QUALITY_GATES__CONSENSUS_THRESHOLD",
            "SPECKIT_QUALITY_GATES__MIN_TEST_COVERAGE",
            "SPECKIT_QUALITY_GATES__SCHEMA_VALIDATION",
            // Cost
            "SPECKIT_COST__ENABLED",
            "SPECKIT_COST__DAILY_LIMIT_USD",
            "SPECKIT_COST__MONTHLY_LIMIT_USD",
            "SPECKIT_COST__ALERT_THRESHOLD",
            // Evidence
            "SPECKIT_EVIDENCE__ENABLED",
            "SPECKIT_EVIDENCE__BASE_DIR",
            "SPECKIT_EVIDENCE__MAX_SIZE_PER_SPEC_MB",
            "SPECKIT_EVIDENCE__RETENTION_DAYS",
            // Consensus
            "SPECKIT_CONSENSUS__MIN_AGENTS",
            "SPECKIT_CONSENSUS__MAX_AGENTS",
            "SPECKIT_CONSENSUS__TIMEOUT_SECONDS",
            // Known model prefixes (for pattern matching)
            "SPECKIT_MODELS__",
        ]
    }

    /// Check if an environment variable name is known/valid.
    ///
    /// Validates both static fields and dynamic model paths.
    ///
    /// # Examples
    ///
    /// ```
    /// # use codex_spec_kit::config::registry::FieldPath;
    /// assert!(FieldPath::is_known_env_var("SPECKIT_COST__ENABLED"));
    /// assert!(FieldPath::is_known_env_var("SPECKIT_MODELS__OPENAI__TEMPERATURE"));
    /// assert!(!FieldPath::is_known_env_var("SPECKIT_UNKNOWN__FIELD"));
    /// ```
    pub fn is_known_env_var(var_name: &str) -> bool {
        // Try parsing - if it succeeds, it's known
        Self::from_env_var(var_name).is_some()
    }

    /// Get human-readable description of the field.
    ///
    /// # Examples
    ///
    /// ```
    /// # use codex_spec_kit::config::registry::FieldPath;
    /// let path = FieldPath::QualityGates_Enabled;
    /// assert_eq!(path.description(), "Enable quality gate validation");
    /// ```
    pub fn description(&self) -> &'static str {
        match self {
            // Quality Gates
            Self::QualityGates_Enabled => "Enable quality gate validation",
            Self::QualityGates_ConsensusThreshold => "Minimum consensus agreement threshold (0.0-1.0)",
            Self::QualityGates_MinTestCoverage => "Minimum test coverage percentage required",
            Self::QualityGates_SchemaValidation => "Enable JSON schema validation",

            // Cost
            Self::Cost_Enabled => "Enable cost tracking and limits",
            Self::Cost_DailyLimitUsd => "Daily cost limit in USD",
            Self::Cost_MonthlyLimitUsd => "Monthly cost limit in USD",
            Self::Cost_AlertThreshold => "Alert threshold as percentage of limit (0.0-1.0)",

            // Evidence
            Self::Evidence_Enabled => "Enable evidence collection",
            Self::Evidence_BaseDir => "Base directory for evidence storage",
            Self::Evidence_MaxSizePerSpecMb => "Maximum evidence size per SPEC in megabytes",
            Self::Evidence_RetentionDays => "Evidence retention period in days",

            // Consensus
            Self::Consensus_MinAgents => "Minimum number of agents required for consensus",
            Self::Consensus_MaxAgents => "Maximum number of agents allowed in consensus",
            Self::Consensus_TimeoutSeconds => "Consensus timeout in seconds",

            // Models
            Self::Model_Name(_) => "Model identifier (e.g., 'gpt-4', 'claude-3-opus')",
            Self::Model_Endpoint(_) => "API endpoint URL override",
            Self::Model_Temperature(_) => "Temperature setting for response randomness (0.0-2.0)",
            Self::Model_MaxTokens(_) => "Maximum tokens in model response",
            Self::Model_CostPerInputMillion(_) => "Cost per 1M input tokens in USD",
            Self::Model_CostPerOutputMillion(_) => "Cost per 1M output tokens in USD",
            Self::Model_Retry_MaxRetries(_) => "Maximum number of retry attempts",
            Self::Model_Retry_BaseDelayMs(_) => "Base delay between retries in milliseconds",
            Self::Model_Retry_MaxDelayMs(_) => "Maximum delay between retries in milliseconds",
        }
    }

    /// Get expected value type for the field.
    ///
    /// # Examples
    ///
    /// ```
    /// # use codex_spec_kit::config::registry::{FieldPath, ValueType};
    /// assert_eq!(FieldPath::QualityGates_Enabled.value_type(), ValueType::Bool);
    /// assert_eq!(FieldPath::Cost_DailyLimitUsd.value_type(), ValueType::Float);
    /// assert_eq!(FieldPath::Evidence_BaseDir.value_type(), ValueType::Path);
    /// ```
    pub fn value_type(&self) -> ValueType {
        match self {
            // Booleans
            Self::QualityGates_Enabled
            | Self::QualityGates_SchemaValidation
            | Self::Cost_Enabled
            | Self::Evidence_Enabled => ValueType::Bool,

            // Floats
            Self::QualityGates_ConsensusThreshold
            | Self::QualityGates_MinTestCoverage
            | Self::Cost_DailyLimitUsd
            | Self::Cost_MonthlyLimitUsd
            | Self::Cost_AlertThreshold
            | Self::Model_Temperature(_)
            | Self::Model_CostPerInputMillion(_)
            | Self::Model_CostPerOutputMillion(_) => ValueType::Float,

            // Integers
            Self::Evidence_MaxSizePerSpecMb
            | Self::Evidence_RetentionDays
            | Self::Consensus_MinAgents
            | Self::Consensus_MaxAgents
            | Self::Consensus_TimeoutSeconds
            | Self::Model_MaxTokens(_)
            | Self::Model_Retry_MaxRetries(_)
            | Self::Model_Retry_BaseDelayMs(_)
            | Self::Model_Retry_MaxDelayMs(_) => ValueType::Integer,

            // Paths
            Self::Evidence_BaseDir => ValueType::Path,

            // Strings
            Self::Model_Name(_) | Self::Model_Endpoint(_) => ValueType::String,
        }
    }

    /// Get validation constraints for the field.
    ///
    /// Returns `None` if the field has no specific constraints.
    ///
    /// # Examples
    ///
    /// ```
    /// # use codex_spec_kit::config::registry::FieldPath;
    /// let constraints = FieldPath::QualityGates_ConsensusThreshold.constraints();
    /// assert!(constraints.is_some());
    ///
    /// let c = constraints.unwrap();
    /// assert_eq!(c.min, Some(0.0));
    /// assert_eq!(c.max, Some(1.0));
    /// ```
    pub fn constraints(&self) -> Option<Constraints> {
        match self {
            // Threshold fields: 0.0-1.0
            Self::QualityGates_ConsensusThreshold | Self::Cost_AlertThreshold => Some(Constraints {
                min: Some(0.0),
                max: Some(1.0),
                pattern: None,
            }),

            // Test coverage: 0.0-100.0
            Self::QualityGates_MinTestCoverage => Some(Constraints {
                min: Some(0.0),
                max: Some(100.0),
                pattern: None,
            }),

            // Temperature: 0.0-2.0 (common range for LLMs)
            Self::Model_Temperature(_) => Some(Constraints {
                min: Some(0.0),
                max: Some(2.0),
                pattern: None,
            }),

            // Cost limits: positive values only
            Self::Cost_DailyLimitUsd | Self::Cost_MonthlyLimitUsd => Some(Constraints {
                min: Some(0.0),
                max: None,
                pattern: None,
            }),

            // Agent counts: 1-10 (reasonable range)
            Self::Consensus_MinAgents | Self::Consensus_MaxAgents => Some(Constraints {
                min: Some(1.0),
                max: Some(10.0),
                pattern: None,
            }),

            // Timeout: positive values only
            Self::Consensus_TimeoutSeconds => Some(Constraints {
                min: Some(1.0),
                max: None,
                pattern: None,
            }),

            // Retention: at least 1 day
            Self::Evidence_RetentionDays => Some(Constraints {
                min: Some(1.0),
                max: None,
                pattern: None,
            }),

            // Max size: positive values only
            Self::Evidence_MaxSizePerSpecMb => Some(Constraints {
                min: Some(1.0),
                max: None,
                pattern: None,
            }),

            // Retry delays: positive values only
            Self::Model_Retry_BaseDelayMs(_) | Self::Model_Retry_MaxDelayMs(_) => Some(Constraints {
                min: Some(0.0),
                max: None,
                pattern: None,
            }),

            // Max retries: 0-10 (reasonable range)
            Self::Model_Retry_MaxRetries(_) => Some(Constraints {
                min: Some(0.0),
                max: Some(10.0),
                pattern: None,
            }),

            // No constraints
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Task 1 Tests: Conversion roundtrips (8-10 tests)

    #[test]
    fn test_quality_gates_env_var_conversion() {
        let path = FieldPath::QualityGates_Enabled;
        assert_eq!(path.to_env_var(), "SPECKIT_QUALITY_GATES__ENABLED");
        assert_eq!(
            FieldPath::from_env_var("SPECKIT_QUALITY_GATES__ENABLED"),
            Some(FieldPath::QualityGates_Enabled)
        );
    }

    #[test]
    fn test_quality_gates_toml_conversion() {
        let path = FieldPath::QualityGates_ConsensusThreshold;
        assert_eq!(path.to_toml_key(), "quality_gates.consensus_threshold");
        assert_eq!(
            FieldPath::from_toml_key("quality_gates.consensus_threshold"),
            Some(FieldPath::QualityGates_ConsensusThreshold)
        );
    }

    #[test]
    fn test_cost_env_var_roundtrip() {
        let original = FieldPath::Cost_DailyLimitUsd;
        let env_var = original.to_env_var();
        let parsed = FieldPath::from_env_var(&env_var);
        assert_eq!(parsed, Some(original));
    }

    #[test]
    fn test_evidence_toml_roundtrip() {
        let original = FieldPath::Evidence_MaxSizePerSpecMb;
        let toml_key = original.to_toml_key();
        let parsed = FieldPath::from_toml_key(&toml_key);
        assert_eq!(parsed, Some(original));
    }

    #[test]
    fn test_consensus_roundtrip() {
        let original = FieldPath::Consensus_TimeoutSeconds;

        // Env var roundtrip
        let env_var = original.to_env_var();
        assert_eq!(FieldPath::from_env_var(&env_var), Some(original.clone()));

        // TOML roundtrip
        let toml_key = original.to_toml_key();
        assert_eq!(FieldPath::from_toml_key(&toml_key), Some(original));
    }

    #[test]
    fn test_model_temperature_env_var() {
        let path = FieldPath::Model_Temperature("openai".into());
        assert_eq!(path.to_env_var(), "SPECKIT_MODELS__OPENAI__TEMPERATURE");
        assert_eq!(
            FieldPath::from_env_var("SPECKIT_MODELS__OPENAI__TEMPERATURE"),
            Some(FieldPath::Model_Temperature("openai".into()))
        );
    }

    #[test]
    fn test_model_temperature_toml() {
        let path = FieldPath::Model_Temperature("gemini".into());
        assert_eq!(path.to_toml_key(), "models.gemini.temperature");
        assert_eq!(
            FieldPath::from_toml_key("models.gemini.temperature"),
            Some(FieldPath::Model_Temperature("gemini".into()))
        );
    }

    #[test]
    fn test_model_retry_field() {
        let path = FieldPath::Model_Retry_MaxRetries("claude".into());
        assert_eq!(path.to_env_var(), "SPECKIT_MODELS__CLAUDE__RETRY__MAX_RETRIES");
        assert_eq!(path.to_toml_key(), "models.claude.retry.max_retries");

        assert_eq!(
            FieldPath::from_env_var("SPECKIT_MODELS__CLAUDE__RETRY__MAX_RETRIES"),
            Some(FieldPath::Model_Retry_MaxRetries("claude".into()))
        );
        assert_eq!(
            FieldPath::from_toml_key("models.claude.retry.max_retries"),
            Some(FieldPath::Model_Retry_MaxRetries("claude".into()))
        );
    }

    #[test]
    fn test_invalid_env_var() {
        assert_eq!(FieldPath::from_env_var("INVALID_VAR"), None);
        assert_eq!(FieldPath::from_env_var("SPECKIT_UNKNOWN__FIELD"), None);
        assert_eq!(FieldPath::from_env_var("OTHER_PREFIX__FIELD"), None);
    }

    #[test]
    fn test_invalid_toml_key() {
        assert_eq!(FieldPath::from_toml_key("unknown.field"), None);
        assert_eq!(FieldPath::from_toml_key("quality_gates.nonexistent"), None);
        assert_eq!(FieldPath::from_toml_key("models.provider.unknown"), None);
    }

    #[test]
    fn test_model_provider_with_hyphens() {
        // Provider names with hyphens should work
        let path = FieldPath::Model_Temperature("gpt-4".into());
        assert_eq!(path.to_env_var(), "SPECKIT_MODELS__GPT_4__TEMPERATURE");
        assert_eq!(path.to_toml_key(), "models.gpt-4.temperature");

        // Roundtrip
        assert_eq!(
            FieldPath::from_env_var("SPECKIT_MODELS__GPT_4__TEMPERATURE"),
            Some(FieldPath::Model_Temperature("gpt-4".into()))
        );
        assert_eq!(
            FieldPath::from_toml_key("models.gpt-4.temperature"),
            Some(FieldPath::Model_Temperature("gpt-4".into()))
        );
    }

    // Task 2 Tests: ConfigPath builder (6-8 tests)

    #[test]
    fn test_builder_quality_gates() {
        let path = ConfigPath::quality_gates().enabled();
        assert_eq!(path, FieldPath::QualityGates_Enabled);
        assert_eq!(path.to_env_var(), "SPECKIT_QUALITY_GATES__ENABLED");

        let path = ConfigPath::quality_gates().consensus_threshold();
        assert_eq!(path, FieldPath::QualityGates_ConsensusThreshold);
        assert_eq!(path.to_toml_key(), "quality_gates.consensus_threshold");
    }

    #[test]
    fn test_builder_cost() {
        let path = ConfigPath::cost().daily_limit_usd();
        assert_eq!(path, FieldPath::Cost_DailyLimitUsd);
        assert_eq!(path.to_toml_key(), "cost.daily_limit_usd");

        let path = ConfigPath::cost().alert_threshold();
        assert_eq!(path, FieldPath::Cost_AlertThreshold);
        assert_eq!(path.to_env_var(), "SPECKIT_COST__ALERT_THRESHOLD");
    }

    #[test]
    fn test_builder_evidence() {
        let path = ConfigPath::evidence().base_dir();
        assert_eq!(path, FieldPath::Evidence_BaseDir);
        assert_eq!(path.to_toml_key(), "evidence.base_dir");

        let path = ConfigPath::evidence().max_size_per_spec_mb();
        assert_eq!(path, FieldPath::Evidence_MaxSizePerSpecMb);
    }

    #[test]
    fn test_builder_consensus() {
        let path = ConfigPath::consensus().min_agents();
        assert_eq!(path, FieldPath::Consensus_MinAgents);

        let path = ConfigPath::consensus().timeout_seconds();
        assert_eq!(path, FieldPath::Consensus_TimeoutSeconds);
        assert_eq!(path.to_toml_key(), "consensus.timeout_seconds");
    }

    #[test]
    fn test_builder_model_basic() {
        let path = ConfigPath::model("openai").temperature();
        assert_eq!(path, FieldPath::Model_Temperature("openai".into()));
        assert_eq!(path.to_env_var(), "SPECKIT_MODELS__OPENAI__TEMPERATURE");

        let path = ConfigPath::model("gemini").max_tokens();
        assert_eq!(path, FieldPath::Model_MaxTokens("gemini".into()));
        assert_eq!(path.to_toml_key(), "models.gemini.max_tokens");
    }

    #[test]
    fn test_builder_model_retry() {
        let path = ConfigPath::model("claude").retry().max_retries();
        assert_eq!(path, FieldPath::Model_Retry_MaxRetries("claude".into()));
        assert_eq!(path.to_toml_key(), "models.claude.retry.max_retries");

        let path = ConfigPath::model("gpt-4").retry().base_delay_ms();
        assert_eq!(path, FieldPath::Model_Retry_BaseDelayMs("gpt-4".into()));
        assert_eq!(path.to_env_var(), "SPECKIT_MODELS__GPT_4__RETRY__BASE_DELAY_MS");
    }

    #[test]
    fn test_builder_method_chaining() {
        // Verify path objects are independent
        let cost_path = ConfigPath::cost();
        let daily = cost_path.daily_limit_usd();
        let monthly = cost_path.monthly_limit_usd();

        assert_eq!(daily, FieldPath::Cost_DailyLimitUsd);
        assert_eq!(monthly, FieldPath::Cost_MonthlyLimitUsd);
        assert_ne!(daily, monthly);
    }

    #[test]
    fn test_builder_path_correctness() {
        // Comprehensive path validation
        let paths = vec![
            (ConfigPath::quality_gates().schema_validation(), "quality_gates.schema_validation"),
            (ConfigPath::cost().monthly_limit_usd(), "cost.monthly_limit_usd"),
            (ConfigPath::evidence().retention_days(), "evidence.retention_days"),
            (ConfigPath::consensus().max_agents(), "consensus.max_agents"),
            (ConfigPath::model("test").endpoint(), "models.test.endpoint"),
            (ConfigPath::model("test").retry().max_delay_ms(), "models.test.retry.max_delay_ms"),
        ];

        for (path, expected_toml) in paths {
            assert_eq!(path.to_toml_key(), expected_toml);
        }
    }

    // Task 3 Tests: Validation helpers (5-7 tests)

    #[test]
    fn test_description_accuracy() {
        assert_eq!(
            FieldPath::QualityGates_Enabled.description(),
            "Enable quality gate validation"
        );
        assert_eq!(
            FieldPath::Cost_DailyLimitUsd.description(),
            "Daily cost limit in USD"
        );
        assert_eq!(
            FieldPath::Model_Temperature("openai".into()).description(),
            "Temperature setting for response randomness (0.0-2.0)"
        );
    }

    #[test]
    fn test_value_type_bool() {
        assert_eq!(FieldPath::QualityGates_Enabled.value_type(), ValueType::Bool);
        assert_eq!(FieldPath::Cost_Enabled.value_type(), ValueType::Bool);
        assert_eq!(FieldPath::Evidence_Enabled.value_type(), ValueType::Bool);
    }

    #[test]
    fn test_value_type_numeric() {
        // Floats
        assert_eq!(FieldPath::Cost_DailyLimitUsd.value_type(), ValueType::Float);
        assert_eq!(FieldPath::QualityGates_ConsensusThreshold.value_type(), ValueType::Float);
        assert_eq!(FieldPath::Model_Temperature("test".into()).value_type(), ValueType::Float);

        // Integers
        assert_eq!(FieldPath::Consensus_MinAgents.value_type(), ValueType::Integer);
        assert_eq!(FieldPath::Evidence_RetentionDays.value_type(), ValueType::Integer);
        assert_eq!(FieldPath::Model_MaxTokens("test".into()).value_type(), ValueType::Integer);
    }

    #[test]
    fn test_value_type_string_and_path() {
        assert_eq!(FieldPath::Model_Name("test".into()).value_type(), ValueType::String);
        assert_eq!(FieldPath::Model_Endpoint("test".into()).value_type(), ValueType::String);
        assert_eq!(FieldPath::Evidence_BaseDir.value_type(), ValueType::Path);
    }

    #[test]
    fn test_constraints_thresholds() {
        // Consensus threshold: 0.0-1.0
        let c = FieldPath::QualityGates_ConsensusThreshold.constraints().unwrap();
        assert_eq!(c.min, Some(0.0));
        assert_eq!(c.max, Some(1.0));

        // Test coverage: 0.0-100.0
        let c = FieldPath::QualityGates_MinTestCoverage.constraints().unwrap();
        assert_eq!(c.min, Some(0.0));
        assert_eq!(c.max, Some(100.0));

        // Temperature: 0.0-2.0
        let c = FieldPath::Model_Temperature("test".into()).constraints().unwrap();
        assert_eq!(c.min, Some(0.0));
        assert_eq!(c.max, Some(2.0));
    }

    #[test]
    fn test_constraints_agent_counts() {
        // Min/max agents: 1-10
        let c = FieldPath::Consensus_MinAgents.constraints().unwrap();
        assert_eq!(c.min, Some(1.0));
        assert_eq!(c.max, Some(10.0));

        let c = FieldPath::Consensus_MaxAgents.constraints().unwrap();
        assert_eq!(c.min, Some(1.0));
        assert_eq!(c.max, Some(10.0));
    }

    #[test]
    fn test_constraints_no_constraints() {
        // Boolean fields have no numeric constraints
        assert!(FieldPath::QualityGates_Enabled.constraints().is_none());
        assert!(FieldPath::Cost_Enabled.constraints().is_none());

        // String fields have no constraints (pattern could be added later)
        assert!(FieldPath::Model_Name("test".into()).constraints().is_none());
    }
}
