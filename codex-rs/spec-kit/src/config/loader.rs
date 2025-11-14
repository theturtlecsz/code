use crate::config::error::{ConfigError, Result};
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Root application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Model configurations (provider → model settings)
    #[serde(default)]
    pub models: HashMap<String, ModelConfig>,

    /// Quality gate configurations
    #[serde(default)]
    pub quality_gates: QualityGateConfig,

    /// Cost tracking and limits
    #[serde(default)]
    pub cost: CostConfig,

    /// Evidence collection settings
    #[serde(default)]
    pub evidence: EvidenceConfig,

    /// Consensus settings
    #[serde(default)]
    pub consensus: ConsensusConfig,
}

/// Model-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model identifier (e.g., "gpt-4", "claude-3-opus")
    pub model: String,

    /// API endpoint URL (optional override)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    /// Temperature setting
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Max tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Cost per 1M input tokens (USD)
    #[serde(default)]
    pub cost_per_input_million: f64,

    /// Cost per 1M output tokens (USD)
    #[serde(default)]
    pub cost_per_output_million: f64,

    /// Retry configuration
    #[serde(default)]
    pub retry: RetryConfig,
}

/// Retry configuration for model calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Base delay in milliseconds
    #[serde(default = "default_base_delay_ms")]
    pub base_delay_ms: u64,

    /// Maximum delay in milliseconds
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,
}

/// Quality gate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGateConfig {
    /// Enable quality gates
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Minimum consensus agreement threshold (0.0-1.0)
    #[serde(default = "default_consensus_threshold")]
    pub consensus_threshold: f32,

    /// Minimum test coverage percentage
    #[serde(default)]
    pub min_test_coverage: Option<f32>,

    /// Enable schema validation
    #[serde(default = "default_true")]
    pub schema_validation: bool,
}

/// Cost tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    /// Enable cost tracking
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Daily cost limit in USD (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_limit_usd: Option<f64>,

    /// Monthly cost limit in USD (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monthly_limit_usd: Option<f64>,

    /// Alert threshold as percentage of limit (0.0-1.0)
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold: f32,
}

/// Evidence collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceConfig {
    /// Enable evidence collection
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Base directory for evidence storage
    #[serde(default = "default_evidence_dir")]
    pub base_dir: PathBuf,

    /// Maximum evidence size per SPEC in MB
    #[serde(default = "default_max_evidence_size_mb")]
    pub max_size_per_spec_mb: u64,

    /// Retention period in days
    #[serde(default = "default_evidence_retention_days")]
    pub retention_days: u32,
}

/// Consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Minimum number of agents required
    #[serde(default = "default_min_agents")]
    pub min_agents: u32,

    /// Maximum number of agents
    #[serde(default = "default_max_agents")]
    pub max_agents: u32,

    /// Timeout for consensus in seconds
    #[serde(default = "default_consensus_timeout")]
    pub timeout_seconds: u64,
}

// Default value functions
fn default_temperature() -> f32 {
    0.7
}
fn default_max_retries() -> u32 {
    3
}
fn default_base_delay_ms() -> u64 {
    1000
}
fn default_max_delay_ms() -> u64 {
    30000
}
fn default_true() -> bool {
    true
}
fn default_consensus_threshold() -> f32 {
    0.67
}
fn default_alert_threshold() -> f32 {
    0.8
}
fn default_evidence_dir() -> PathBuf {
    PathBuf::from("docs/evidence")
}
fn default_max_evidence_size_mb() -> u64 {
    25
}
fn default_evidence_retention_days() -> u32 {
    90
}
fn default_min_agents() -> u32 {
    2
}
fn default_max_agents() -> u32 {
    5
}
fn default_consensus_timeout() -> u64 {
    300
}

// Default implementations
impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            base_delay_ms: default_base_delay_ms(),
            max_delay_ms: default_max_delay_ms(),
        }
    }
}

impl Default for QualityGateConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            consensus_threshold: default_consensus_threshold(),
            min_test_coverage: None,
            schema_validation: default_true(),
        }
    }
}

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            daily_limit_usd: None,
            monthly_limit_usd: None,
            alert_threshold: default_alert_threshold(),
        }
    }
}

impl Default for EvidenceConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            base_dir: default_evidence_dir(),
            max_size_per_spec_mb: default_max_evidence_size_mb(),
            retention_days: default_evidence_retention_days(),
        }
    }
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            min_agents: default_min_agents(),
            max_agents: default_max_agents(),
            timeout_seconds: default_consensus_timeout(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            models: HashMap::new(),
            quality_gates: QualityGateConfig::default(),
            cost: CostConfig::default(),
            evidence: EvidenceConfig::default(),
            consensus: ConsensusConfig::default(),
        }
    }
}

/// Configuration loader with layered merging support
pub struct ConfigLoader {
    config_path: Option<PathBuf>,
}

impl ConfigLoader {
    /// Create a new ConfigLoader
    pub fn new() -> Self {
        Self { config_path: None }
    }

    /// Set the configuration file path
    pub fn with_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Load configuration with layered merging:
    /// 1. Start with defaults (from Default implementations)
    /// 2. Merge config file if provided
    /// 3. Override with environment variables (SPECKIT_ prefix)
    pub fn load(&self) -> Result<AppConfig> {
        let mut builder = Config::builder();

        // Layer 1: Defaults (serialize defaults to JSON and load as base)
        let defaults = AppConfig::default();
        let defaults_json = serde_json::to_string(&defaults)?;
        builder = builder.add_source(config::File::from_str(
            &defaults_json,
            config::FileFormat::Json,
        ));

        // Layer 2: Config file (if provided)
        if let Some(ref path) = self.config_path {
            if path.exists() {
                builder = builder.add_source(File::from(path.as_ref()));
            } else {
                return Err(ConfigError::FileNotFound(path.clone()));
            }
        }

        // Layer 3: Environment variables (SPECKIT_ prefix, double underscore for nesting)
        // Example: SPECKIT_QUALITY_GATES__ENABLED=false
        // Note: try_parsing(true) enables type conversion for booleans, numbers, etc.
        // Note: prefix_separator("_") adds underscore between prefix and key
        builder = builder.add_source(
            Environment::with_prefix("SPECKIT")
                .prefix_separator("_")
                .separator("__")
                .try_parsing(true)
                .list_separator(","),
        );

        // Build and deserialize
        let config = builder.build()?;
        let app_config: AppConfig = config.try_deserialize()?;

        // Validate with schema if enabled
        if app_config.quality_gates.schema_validation {
            let validator = crate::config::validator::SchemaValidator::new()?;
            validator.validate(&app_config)?;
        }

        Ok(app_config)
    }

    /// Locate the default config file in standard locations:
    /// 1. Current directory: ./speckit.toml
    /// 2. XDG config: ~/.config/speckit/config.toml
    /// 3. Home directory: ~/.speckit.toml
    pub fn find_config_file() -> Option<PathBuf> {
        // Check current directory
        let cwd_config = PathBuf::from("./speckit.toml");
        if cwd_config.exists() {
            return Some(cwd_config);
        }

        // Check XDG config directory
        if let Some(config_dir) = dirs::config_dir() {
            let xdg_config = config_dir.join("speckit").join("config.toml");
            if xdg_config.exists() {
                return Some(xdg_config);
            }
        }

        // Check home directory
        if let Some(home_dir) = dirs::home_dir() {
            let home_config = home_dir.join(".speckit.toml");
            if home_config.exists() {
                return Some(home_config);
            }
        }

        None
    }

    /// Load configuration from default locations
    pub fn load_default() -> Result<AppConfig> {
        let loader = if let Some(config_path) = Self::find_config_file() {
            ConfigLoader::new().with_file(config_path)
        } else {
            ConfigLoader::new()
        };

        loader.load()
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.quality_gates.enabled, true);
        assert_eq!(config.quality_gates.consensus_threshold, 0.67);
        assert_eq!(config.cost.enabled, true);
        assert_eq!(config.evidence.max_size_per_spec_mb, 25);
        assert_eq!(config.consensus.min_agents, 2);
        assert_eq!(config.consensus.max_agents, 5);
    }

    #[test]
    #[serial]
    fn test_load_with_defaults_only() {
        let loader = ConfigLoader::new();
        let config = loader.load().expect("Failed to load default config");
        assert_eq!(config.quality_gates.enabled, true);
        assert_eq!(config.consensus.min_agents, 2);
    }

    #[test]
    #[serial]
    fn test_load_with_env_override() {
        unsafe {
            env::set_var("SPECKIT_QUALITY_GATES__ENABLED", "false");
            env::set_var("SPECKIT_CONSENSUS__MIN_AGENTS", "3");
        }

        // Verify env vars are actually set
        assert_eq!(env::var("SPECKIT_QUALITY_GATES__ENABLED").unwrap(), "false");
        assert_eq!(env::var("SPECKIT_CONSENSUS__MIN_AGENTS").unwrap(), "3");

        let loader = ConfigLoader::new();
        let config = loader.load().expect("Failed to load config");

        // Debug output
        eprintln!("Quality gates enabled: {}", config.quality_gates.enabled);
        eprintln!("Consensus min agents: {}", config.consensus.min_agents);

        assert_eq!(config.quality_gates.enabled, false);
        assert_eq!(config.consensus.min_agents, 3);

        // Cleanup
        unsafe {
            env::remove_var("SPECKIT_QUALITY_GATES__ENABLED");
            env::remove_var("SPECKIT_CONSENSUS__MIN_AGENTS");
        }
    }

    #[test]
    #[serial]
    fn test_load_with_toml_file() {
        let toml_content = r#"
[quality_gates]
enabled = false
consensus_threshold = 0.8

[cost]
daily_limit_usd = 10.0

[consensus]
min_agents = 3
max_agents = 7
"#;

        // Create a temp file with .toml extension
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("test_config.toml");
        std::fs::write(&config_path, toml_content).expect("Failed to write temp file");

        let loader = ConfigLoader::new().with_file(&config_path);
        let config = loader.load().expect("Failed to load config");

        assert_eq!(config.quality_gates.enabled, false);
        assert_eq!(config.quality_gates.consensus_threshold, 0.8);
        assert_eq!(config.cost.daily_limit_usd, Some(10.0));
        assert_eq!(config.consensus.min_agents, 3);
        assert_eq!(config.consensus.max_agents, 7);
    }

    #[test]
    #[serial]
    fn test_layered_merging() {
        // Create TOML file
        let toml_content = r#"
[quality_gates]
enabled = false
consensus_threshold = 0.8
"#;

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("test_config.toml");
        std::fs::write(&config_path, toml_content).expect("Failed to write temp file");

        // Set env var that should override TOML
        unsafe {
            env::set_var("SPECKIT_QUALITY_GATES__ENABLED", "true");
        }

        let loader = ConfigLoader::new().with_file(&config_path);
        let config = loader.load().expect("Failed to load config");

        // Env var should win over file
        assert_eq!(config.quality_gates.enabled, true);
        // File value should be preserved for non-overridden fields
        assert_eq!(config.quality_gates.consensus_threshold, 0.8);

        // Cleanup
        unsafe {
            env::remove_var("SPECKIT_QUALITY_GATES__ENABLED");
        }
    }

    #[test]
    fn test_missing_file_error() {
        let loader = ConfigLoader::new().with_file("/nonexistent/config.toml");
        let result = loader.load();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::FileNotFound(_)));
    }

    #[test]
    fn test_retry_config_defaults() {
        let retry = RetryConfig::default();
        assert_eq!(retry.max_retries, 3);
        assert_eq!(retry.base_delay_ms, 1000);
        assert_eq!(retry.max_delay_ms, 30000);
    }

    #[test]
    fn test_evidence_config_defaults() {
        let evidence = EvidenceConfig::default();
        assert_eq!(evidence.enabled, true);
        assert_eq!(evidence.max_size_per_spec_mb, 25);
        assert_eq!(evidence.retention_days, 90);
        assert_eq!(evidence.base_dir, PathBuf::from("docs/evidence"));
    }

    #[test]
    fn test_cost_config_defaults() {
        let cost = CostConfig::default();
        assert_eq!(cost.enabled, true);
        assert_eq!(cost.daily_limit_usd, None);
        assert_eq!(cost.monthly_limit_usd, None);
        assert_eq!(cost.alert_threshold, 0.8);
    }

    #[test]
    fn test_consensus_config_defaults() {
        let consensus = ConsensusConfig::default();
        assert_eq!(consensus.min_agents, 2);
        assert_eq!(consensus.max_agents, 5);
        assert_eq!(consensus.timeout_seconds, 300);
    }

    #[test]
    #[serial]
    fn test_model_config_with_overrides() {
        let toml_content = r#"
[models.openai]
model = "gpt-4"
temperature = 0.5
cost_per_input_million = 10.0
cost_per_output_million = 30.0

[models.openai.retry]
max_retries = 5
"#;

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("test_config.toml");
        std::fs::write(&config_path, toml_content).expect("Failed to write temp file");

        let loader = ConfigLoader::new().with_file(&config_path);
        let config = loader.load().expect("Failed to load config");

        let openai_config = config.models.get("openai").expect("openai config missing");
        assert_eq!(openai_config.model, "gpt-4");
        assert_eq!(openai_config.temperature, 0.5);
        assert_eq!(openai_config.cost_per_input_million, 10.0);
        assert_eq!(openai_config.cost_per_output_million, 30.0);
        assert_eq!(openai_config.retry.max_retries, 5);
    }

    // Integration tests for schema validation

    #[test]
    #[serial]
    fn test_load_with_invalid_config_file() {
        let toml_content = r#"
[quality_gates]
enabled = true
consensus_threshold = 1.5  # Invalid: > 1.0
schema_validation = true
"#;

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("invalid_config.toml");
        std::fs::write(&config_path, toml_content).expect("Failed to write");

        let loader = ConfigLoader::new().with_file(&config_path);
        let result = loader.load();

        assert!(
            result.is_err(),
            "Expected validation error for invalid config"
        );
        let err = result.unwrap_err();
        assert!(
            matches!(err, ConfigError::SchemaValidationError(_)),
            "Expected SchemaValidationError, got: {:?}",
            err
        );
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("consensus_threshold") || err_msg.contains("quality_gates"),
            "Error should mention consensus_threshold, got: {}",
            err_msg
        );
    }

    #[test]
    #[serial]
    fn test_load_with_schema_validation_disabled() {
        let toml_content = r#"
[quality_gates]
enabled = true
consensus_threshold = 1.5  # Invalid but validation is disabled
schema_validation = false
"#;

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config_no_validation.toml");
        std::fs::write(&config_path, toml_content).expect("Failed to write");

        let loader = ConfigLoader::new().with_file(&config_path);
        let result = loader.load();

        // Should succeed because schema_validation = false
        assert!(
            result.is_ok(),
            "Config should load when validation is disabled: {:?}",
            result
        );
        let config = result.unwrap();
        assert_eq!(config.quality_gates.consensus_threshold, 1.5);
        assert_eq!(config.quality_gates.schema_validation, false);
    }

    #[test]
    #[serial]
    fn test_env_override_triggers_validation() {
        // Start with valid config from file
        let toml_content = r#"
[quality_gates]
consensus_threshold = 0.7
schema_validation = true
"#;

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");
        std::fs::write(&config_path, toml_content).expect("Failed to write");

        // Override with invalid value via env var
        unsafe {
            env::set_var("SPECKIT_CONSENSUS__MIN_AGENTS", "1"); // Invalid: < 2
        }

        let loader = ConfigLoader::new().with_file(&config_path);
        let result = loader.load();

        assert!(
            result.is_err(),
            "Expected validation error for env override: {:?}",
            result
        );
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::SchemaValidationError(_)));

        // Cleanup
        unsafe {
            env::remove_var("SPECKIT_CONSENSUS__MIN_AGENTS");
        }
    }

    #[test]
    #[serial]
    fn test_validation_error_message_quality() {
        let toml_content = r#"
[quality_gates]
consensus_threshold = 1.5
schema_validation = true

[consensus]
min_agents = 1
"#;

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("bad_config.toml");
        std::fs::write(&config_path, toml_content).expect("Failed to write");

        let loader = ConfigLoader::new().with_file(&config_path);
        let result = loader.load();

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();

        // Error message should be helpful and descriptive
        assert!(
            err_msg.contains("validation failed") || err_msg.contains("Configuration"),
            "Should have clear error prefix: {}",
            err_msg
        );
        assert!(
            err_msg.len() > 50,
            "Error message should be descriptive: {}",
            err_msg
        );

        // Should mention at least one of the invalid fields
        let has_field_mention = err_msg.contains("consensus_threshold")
            || err_msg.contains("min_agents")
            || err_msg.contains("quality_gates")
            || err_msg.contains("consensus");

        assert!(
            has_field_mention,
            "Error should mention invalid field(s), got: {}",
            err_msg
        );
    }

    #[test]
    #[serial]
    fn test_multiple_invalid_fields_caught() {
        let toml_content = r#"
[quality_gates]
consensus_threshold = -0.5  # Invalid: < 0.0
schema_validation = true

[cost]
alert_threshold = 2.0  # Invalid: > 1.0

[consensus]
max_agents = 20  # Invalid: > 10
"#;

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("multi_invalid.toml");
        std::fs::write(&config_path, toml_content).expect("Failed to write");

        let loader = ConfigLoader::new().with_file(&config_path);
        let result = loader.load();

        assert!(result.is_err(), "Multiple invalid fields should be caught");
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::SchemaValidationError(_)));

        // Error message should indicate multiple errors
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("error"),
            "Should mention errors: {}",
            err_msg
        );
    }
}
