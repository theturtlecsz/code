//! Layered configuration loader for codex-rs (SPEC-939 Task 6)
//!
//! Implements a three-layer configuration system with precedence:
//! 1. Default config (hardcoded sensible defaults)
//! 2. File config (loaded from ~/.code/config.toml)
//! 3. Environment overrides (CODEX_* environment variables)
//!
//! ## Example
//!
//! ```no_run
//! use codex_core::config_loader::ConfigLoader;
//! use std::path::PathBuf;
//!
//! let config = ConfigLoader::new()
//!     .with_codex_home(PathBuf::from("~/.code"))
//!     .with_env_prefix("CODEX")
//!     .load()
//!     .expect("Failed to load config");
//! ```

use crate::config_types::{
    AceConfig, AgentConfig, HotReloadConfig, QualityGateConfig, ValidationConfig,
};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;

/// Errors that can occur during configuration loading.
#[derive(Debug)]
pub enum ConfigLoadError {
    /// I/O error reading config file
    IoError(std::io::Error),

    /// TOML parsing error
    TomlParseError(toml::de::Error),

    /// Configuration validation error
    ValidationError(String),

    /// Missing required field
    MissingRequiredField { field: String, context: String },

    /// Invalid environment variable value
    InvalidEnvValue {
        var: String,
        value: String,
        expected: String,
    },

    /// Schema validation failed (when using JSON Schema)
    SchemaValidationFailed(Vec<String>),
}

impl std::fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigLoadError::IoError(e) => write!(f, "I/O error loading config: {e}"),
            ConfigLoadError::TomlParseError(e) => write!(f, "TOML parsing error: {e}"),
            ConfigLoadError::ValidationError(msg) => write!(f, "Config validation error: {msg}"),
            ConfigLoadError::MissingRequiredField { field, context } => {
                write!(f, "Missing required field '{field}' in {context}")
            }
            ConfigLoadError::InvalidEnvValue {
                var,
                value,
                expected,
            } => write!(
                f,
                "Invalid value for ${var}: '{value}' (expected: {expected})"
            ),
            ConfigLoadError::SchemaValidationFailed(errors) => {
                write!(f, "Schema validation failed: {}", errors.join("; "))
            }
        }
    }
}

impl std::error::Error for ConfigLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigLoadError::IoError(e) => Some(e),
            ConfigLoadError::TomlParseError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ConfigLoadError {
    fn from(err: std::io::Error) -> Self {
        ConfigLoadError::IoError(err)
    }
}

impl From<toml::de::Error> for ConfigLoadError {
    fn from(err: toml::de::Error) -> Self {
        ConfigLoadError::TomlParseError(err)
    }
}

/// Configuration data with all layers merged.
///
/// This is an intermediate representation before final Config construction.
/// All fields are Option to support layering: default → file → env.
#[derive(Debug, Clone, Default)]
pub struct LayeredConfig {
    /// ACE configuration
    pub ace: Option<AceConfig>,

    /// Agent configurations
    pub agents: Vec<AgentConfig>,

    /// Quality gate configurations
    pub quality_gates: Option<QualityGateConfig>,

    /// Hot-reload configuration
    pub hot_reload: Option<HotReloadConfig>,

    /// Validation configuration
    pub validation: Option<ValidationConfig>,

    /// Model provider selection
    pub model_provider: Option<String>,

    /// Model name
    pub model: Option<String>,

    /// Auto-upgrade enabled
    pub auto_upgrade_enabled: Option<bool>,

    /// Additional TOML fields (for forward compatibility)
    pub extra: HashMap<String, TomlValue>,
}

/// Builder for layered configuration loading.
///
/// Supports three layers with precedence (later layers override earlier):
/// 1. Default config (via `with_defaults()`)
/// 2. File config (via `with_file()`)
/// 3. Environment overrides (via `with_env_overrides()`)
///
/// ## Example
///
/// ```no_run
/// use codex_core::config_loader::ConfigLoader;
/// use std::path::PathBuf;
///
/// let config = ConfigLoader::new()
///     .with_codex_home(PathBuf::from("~/.code"))
///     .with_env_prefix("CODEX")
///     .load()
///     .expect("Failed to load config");
/// ```
pub struct ConfigLoader {
    codex_home: Option<PathBuf>,
    env_prefix: String,
    skip_file: bool,
    skip_env: bool,
}

impl ConfigLoader {
    /// Create a new ConfigLoader with default settings.
    ///
    /// By default:
    /// - CODEX_HOME is auto-detected from environment
    /// - Environment prefix is "CODEX"
    /// - All layers are enabled
    pub fn new() -> Self {
        Self {
            codex_home: None,
            env_prefix: "CODEX".to_string(),
            skip_file: false,
            skip_env: false,
        }
    }

    /// Set the CODEX_HOME directory explicitly.
    ///
    /// If not set, will be auto-detected from $CODEX_HOME or $CODE_HOME,
    /// defaulting to ~/.code.
    pub fn with_codex_home(mut self, path: PathBuf) -> Self {
        self.codex_home = Some(path);
        self
    }

    /// Set the environment variable prefix for overrides.
    ///
    /// Default is "CODEX", which means CODEX_MODEL, CODEX_PROVIDER, etc.
    pub fn with_env_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.env_prefix = prefix.into();
        self
    }

    /// Skip loading from config file (only use defaults + env).
    ///
    /// Useful for testing or minimal configurations.
    pub fn skip_file_layer(mut self) -> Self {
        self.skip_file = true;
        self
    }

    /// Skip environment variable overrides (only use defaults + file).
    ///
    /// Useful for testing or when env vars should be ignored.
    pub fn skip_env_layer(mut self) -> Self {
        self.skip_env = true;
        self
    }

    /// Load configuration with all enabled layers.
    ///
    /// Precedence: default < file < environment
    ///
    /// # Returns
    ///
    /// Returns the fully merged LayeredConfig or an error if loading fails.
    ///
    /// # Errors
    ///
    /// - IoError: File not readable
    /// - TomlParseError: Invalid TOML syntax
    /// - ValidationError: Config fails validation rules
    pub fn load(self) -> Result<LayeredConfig, ConfigLoadError> {
        // Layer 1: Default config
        let mut config = Self::default_config();

        // Layer 2: File config (if not skipped)
        if !self.skip_file {
            let codex_home = self.resolve_codex_home()?;
            let file_config = Self::load_from_file(&codex_home)?;
            Self::merge_config(&mut config, file_config);
        }

        // Layer 3: Environment overrides (if not skipped)
        if !self.skip_env {
            Self::apply_env_overrides(&mut config, &self.env_prefix)?;
        }

        Ok(config)
    }

    /// Resolve CODEX_HOME directory.
    ///
    /// Priority:
    /// 1. Explicit codex_home from builder
    /// 2. $CODEX_HOME environment variable
    /// 3. $CODE_HOME environment variable
    /// 4. ~/.code (default)
    fn resolve_codex_home(&self) -> Result<PathBuf, ConfigLoadError> {
        if let Some(ref path) = self.codex_home {
            return Ok(path.clone());
        }

        // Check environment variables
        if let Ok(path) = env::var("CODEX_HOME") {
            return Ok(PathBuf::from(path));
        }

        if let Ok(path) = env::var("CODE_HOME") {
            return Ok(PathBuf::from(path));
        }

        // Default to ~/.code
        dirs::home_dir()
            .map(|home| home.join(".code"))
            .ok_or_else(|| {
                ConfigLoadError::ValidationError("Cannot determine home directory".to_string())
            })
    }

    /// Provide sensible default configuration.
    ///
    /// Returns a LayeredConfig with reasonable defaults that work out-of-the-box.
    ///
    /// Default values:
    /// - auto_upgrade_enabled: false
    /// - model_provider: "openai"
    /// - model: "gpt-5-codex"
    /// - validation: default ValidationConfig
    /// - quality_gates: None (use runtime defaults)
    /// - hot_reload: None (disabled by default)
    pub fn default_config() -> LayeredConfig {
        LayeredConfig {
            ace: Some(AceConfig::default()),
            agents: Vec::new(),
            quality_gates: None, // Use runtime defaults
            hot_reload: None,    // Disabled by default
            validation: Some(ValidationConfig::default()),
            model_provider: Some("openai".to_string()),
            model: Some("gpt-5-codex".to_string()),
            auto_upgrade_enabled: Some(false),
            extra: HashMap::new(),
        }
    }

    /// Load configuration from TOML file.
    ///
    /// Reads config.toml from CODEX_HOME and parses into LayeredConfig.
    /// Returns empty config if file doesn't exist (not an error).
    ///
    /// # Arguments
    ///
    /// * `codex_home` - Path to CODEX_HOME directory
    ///
    /// # Returns
    ///
    /// Returns LayeredConfig or error if file exists but is invalid.
    pub fn load_from_file(codex_home: &Path) -> Result<LayeredConfig, ConfigLoadError> {
        let config_path = codex_home.join("config.toml");

        let contents = match std::fs::read_to_string(&config_path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!("config.toml not found at {:?}, using defaults", config_path);
                return Ok(LayeredConfig::default());
            }
            Err(e) => return Err(ConfigLoadError::IoError(e)),
        };

        let toml_value: TomlValue = toml::from_str(&contents)?;

        // Parse into LayeredConfig
        Self::parse_toml_to_layered(toml_value)
    }

    /// Parse TOML value into LayeredConfig.
    ///
    /// Extracts known fields and stores unknown fields in `extra`.
    fn parse_toml_to_layered(value: TomlValue) -> Result<LayeredConfig, ConfigLoadError> {
        let mut config = LayeredConfig::default();

        if let TomlValue::Table(table) = value {
            // Extract known top-level fields
            if let Some(ace_value) = table.get("ace") {
                let ace_str = toml::to_string(ace_value).map_err(|e| {
                    ConfigLoadError::ValidationError(format!("ace serialization: {e}"))
                })?;
                config.ace =
                    Some(toml::from_str(&ace_str).map_err(ConfigLoadError::TomlParseError)?);
            }

            if let Some(agents_value) = table.get("agents") {
                let agents_str = toml::to_string(agents_value).map_err(|e| {
                    ConfigLoadError::ValidationError(format!("agents serialization: {e}"))
                })?;
                config.agents =
                    toml::from_str(&agents_str).map_err(ConfigLoadError::TomlParseError)?;
            }

            if let Some(quality_gates_value) = table.get("quality_gates") {
                let qg_str = toml::to_string(quality_gates_value).map_err(|e| {
                    ConfigLoadError::ValidationError(format!("quality_gates serialization: {e}"))
                })?;
                config.quality_gates =
                    Some(toml::from_str(&qg_str).map_err(ConfigLoadError::TomlParseError)?);
            }

            if let Some(hot_reload_value) = table.get("hot_reload") {
                let hr_str = toml::to_string(hot_reload_value).map_err(|e| {
                    ConfigLoadError::ValidationError(format!("hot_reload serialization: {e}"))
                })?;
                config.hot_reload =
                    Some(toml::from_str(&hr_str).map_err(ConfigLoadError::TomlParseError)?);
            }

            if let Some(validation_value) = table.get("validation") {
                let val_str = toml::to_string(validation_value).map_err(|e| {
                    ConfigLoadError::ValidationError(format!("validation serialization: {e}"))
                })?;
                config.validation =
                    Some(toml::from_str(&val_str).map_err(ConfigLoadError::TomlParseError)?);
            }

            if let Some(TomlValue::String(s)) = table.get("model_provider") {
                config.model_provider = Some(s.clone());
            }

            if let Some(TomlValue::String(s)) = table.get("model") {
                config.model = Some(s.clone());
            }

            if let Some(TomlValue::Boolean(b)) = table.get("auto_upgrade_enabled") {
                config.auto_upgrade_enabled = Some(*b);
            }

            // Store unknown fields in extra for forward compatibility
            for (key, value) in table {
                if !matches!(
                    key.as_str(),
                    "ace"
                        | "agents"
                        | "quality_gates"
                        | "hot_reload"
                        | "validation"
                        | "model_provider"
                        | "model"
                        | "auto_upgrade_enabled"
                ) {
                    config.extra.insert(key, value);
                }
            }
        }

        Ok(config)
    }

    /// Merge two configurations with precedence (later overrides earlier).
    ///
    /// Merges `overlay` into `base`, preferring non-None values from overlay.
    ///
    /// # Arguments
    ///
    /// * `base` - Base configuration (modified in place)
    /// * `overlay` - Overlay configuration (takes precedence)
    pub fn merge_config(base: &mut LayeredConfig, overlay: LayeredConfig) {
        // Merge Option fields (overlay takes precedence if Some)
        if overlay.ace.is_some() {
            base.ace = overlay.ace;
        }

        if overlay.quality_gates.is_some() {
            base.quality_gates = overlay.quality_gates;
        }

        if overlay.hot_reload.is_some() {
            base.hot_reload = overlay.hot_reload;
        }

        if overlay.validation.is_some() {
            base.validation = overlay.validation;
        }

        if overlay.model_provider.is_some() {
            base.model_provider = overlay.model_provider;
        }

        if overlay.model.is_some() {
            base.model = overlay.model;
        }

        if overlay.auto_upgrade_enabled.is_some() {
            base.auto_upgrade_enabled = overlay.auto_upgrade_enabled;
        }

        // Merge agents (overlay appends/replaces by canonical_name or name)
        if !overlay.agents.is_empty() {
            // Replace agents with matching names, append new ones
            for overlay_agent in overlay.agents {
                let overlay_name = overlay_agent.get_agent_name();
                if let Some(existing) = base
                    .agents
                    .iter_mut()
                    .find(|a| a.get_agent_name() == overlay_name)
                {
                    *existing = overlay_agent;
                } else {
                    base.agents.push(overlay_agent);
                }
            }
        }

        // Merge extra fields
        for (key, value) in overlay.extra {
            base.extra.insert(key, value);
        }
    }

    /// Apply environment variable overrides.
    ///
    /// Reads environment variables with the given prefix and applies them
    /// as overrides to the configuration.
    ///
    /// Supported environment variables:
    /// - `{PREFIX}_MODEL` - Override model name
    /// - `{PREFIX}_PROVIDER` - Override model provider
    /// - `{PREFIX}_AUTO_UPGRADE` - Override auto-upgrade setting (true/false)
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration to modify (in place)
    /// * `prefix` - Environment variable prefix (e.g., "CODEX")
    ///
    /// # Errors
    ///
    /// Returns error if environment variable has invalid format/value.
    pub fn apply_env_overrides(
        config: &mut LayeredConfig,
        prefix: &str,
    ) -> Result<(), ConfigLoadError> {
        // MODEL override
        let model_var = format!("{prefix}_MODEL");
        if let Ok(value) = env::var(&model_var)
            && !value.trim().is_empty()
        {
            tracing::debug!("Applying env override: {}={}", model_var, value);
            config.model = Some(value);
        }

        // PROVIDER override
        let provider_var = format!("{prefix}_PROVIDER");
        if let Ok(value) = env::var(&provider_var)
            && !value.trim().is_empty()
        {
            tracing::debug!("Applying env override: {}={}", provider_var, value);
            config.model_provider = Some(value);
        }

        // AUTO_UPGRADE override
        let upgrade_var = format!("{prefix}_AUTO_UPGRADE");
        if let Ok(value) = env::var(&upgrade_var) {
            let trimmed = value.trim().to_lowercase();
            match trimmed.as_str() {
                "true" | "1" | "yes" | "on" => {
                    tracing::debug!("Applying env override: {}=true", upgrade_var);
                    config.auto_upgrade_enabled = Some(true);
                }
                "false" | "0" | "no" | "off" => {
                    tracing::debug!("Applying env override: {}=false", upgrade_var);
                    config.auto_upgrade_enabled = Some(false);
                }
                _ => {
                    return Err(ConfigLoadError::InvalidEnvValue {
                        var: upgrade_var,
                        value,
                        expected: "true/false, 1/0, yes/no, on/off".to_string(),
                    });
                }
            }
        }

        Ok(())
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

    #[test]
    fn test_default_config() {
        let config = ConfigLoader::default_config();

        assert_eq!(config.model_provider, Some("openai".to_string()));
        assert_eq!(config.model, Some("gpt-5-codex".to_string()));
        assert_eq!(config.auto_upgrade_enabled, Some(false));
        assert!(config.ace.is_some());
        assert!(config.validation.is_some());
    }

    #[test]
    fn test_merge_config_overlay_takes_precedence() {
        let mut base = LayeredConfig {
            model: Some("base-model".to_string()),
            model_provider: Some("base-provider".to_string()),
            auto_upgrade_enabled: Some(false),
            ..Default::default()
        };

        let overlay = LayeredConfig {
            model: Some("overlay-model".to_string()),
            model_provider: None, // Should not override
            auto_upgrade_enabled: Some(true),
            ..Default::default()
        };

        ConfigLoader::merge_config(&mut base, overlay);

        assert_eq!(base.model, Some("overlay-model".to_string()));
        assert_eq!(base.model_provider, Some("base-provider".to_string())); // Unchanged
        assert_eq!(base.auto_upgrade_enabled, Some(true));
    }

    #[test]
    fn test_merge_agents_by_name() {
        let mut base = LayeredConfig {
            agents: vec![AgentConfig {
                name: "agent1".to_string(),
                command: "old-command".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let overlay = LayeredConfig {
            agents: vec![
                AgentConfig {
                    name: "agent1".to_string(), // Same name - should replace
                    command: "new-command".to_string(),
                    ..Default::default()
                },
                AgentConfig {
                    name: "agent2".to_string(), // New agent - should append
                    command: "command2".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        ConfigLoader::merge_config(&mut base, overlay);

        assert_eq!(base.agents.len(), 2);
        assert_eq!(base.agents[0].name, "agent1");
        assert_eq!(base.agents[0].command, "new-command"); // Updated
        assert_eq!(base.agents[1].name, "agent2"); // Appended
    }

    #[test]
    fn test_apply_env_overrides_model() {
        unsafe {
            std::env::set_var("TEST_MODEL", "env-model");
            std::env::set_var("TEST_PROVIDER", "env-provider");
        }

        let mut config = LayeredConfig::default();
        ConfigLoader::apply_env_overrides(&mut config, "TEST").unwrap();

        assert_eq!(config.model, Some("env-model".to_string()));
        assert_eq!(config.model_provider, Some("env-provider".to_string()));

        unsafe {
            std::env::remove_var("TEST_MODEL");
            std::env::remove_var("TEST_PROVIDER");
        }
    }

    #[test]
    fn test_apply_env_overrides_auto_upgrade_variations() {
        let test_cases = vec![
            ("true", true),
            ("TRUE", true),
            ("1", true),
            ("yes", true),
            ("on", true),
            ("false", false),
            ("FALSE", false),
            ("0", false),
            ("no", false),
            ("off", false),
        ];

        for (input, expected) in test_cases {
            unsafe {
                std::env::set_var("TEST_AUTO_UPGRADE", input);
            }

            let mut config = LayeredConfig::default();
            ConfigLoader::apply_env_overrides(&mut config, "TEST").unwrap();

            assert_eq!(
                config.auto_upgrade_enabled,
                Some(expected),
                "Failed for input: {input}"
            );

            unsafe {
                std::env::remove_var("TEST_AUTO_UPGRADE");
            }
        }
    }

    #[test]
    fn test_apply_env_overrides_invalid_bool() {
        unsafe {
            std::env::set_var("TEST_AUTO_UPGRADE", "invalid");
        }

        let mut config = LayeredConfig::default();
        let result = ConfigLoader::apply_env_overrides(&mut config, "TEST");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigLoadError::InvalidEnvValue { .. }
        ));

        unsafe {
            std::env::remove_var("TEST_AUTO_UPGRADE");
        }
    }

    #[test]
    fn test_load_with_skip_layers() {
        // Test skip_file_layer
        let config = ConfigLoader::new()
            .skip_file_layer()
            .load()
            .expect("Should load with defaults only");

        assert!(config.model.is_some()); // Defaults present

        // Test skip_env_layer
        unsafe {
            std::env::set_var("CODEX_MODEL", "should-be-ignored");
        }

        let config = ConfigLoader::new()
            .skip_file_layer()
            .skip_env_layer()
            .load()
            .expect("Should load with defaults only");

        assert_eq!(config.model, Some("gpt-5-codex".to_string())); // Default, not env

        unsafe {
            std::env::remove_var("CODEX_MODEL");
        }
    }

    #[test]
    fn test_parse_toml_with_unknown_fields() {
        let toml_str = r#"
            model = "test-model"
            unknown_field = "should-be-preserved"
            [unknown_section]
            key = "value"
        "#;

        let toml_value: TomlValue = toml::from_str(toml_str).unwrap();
        let config = ConfigLoader::parse_toml_to_layered(toml_value).unwrap();

        assert_eq!(config.model, Some("test-model".to_string()));
        assert!(config.extra.contains_key("unknown_field"));
        assert!(config.extra.contains_key("unknown_section"));
    }

    #[test]
    fn test_config_load_error_display() {
        let err = ConfigLoadError::MissingRequiredField {
            field: "model".to_string(),
            context: "quality_gates.plan".to_string(),
        };

        let msg = format!("{err}");
        assert!(msg.contains("model"));
        assert!(msg.contains("quality_gates.plan"));
    }

    #[test]
    fn test_builder_pattern() {
        let loader = ConfigLoader::new()
            .with_env_prefix("CUSTOM")
            .skip_file_layer()
            .skip_env_layer();

        assert_eq!(loader.env_prefix, "CUSTOM");
        assert!(loader.skip_file);
        assert!(loader.skip_env);
    }
}
