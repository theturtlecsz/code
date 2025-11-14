/// Configuration module for spec-kit
///
/// Implements layered configuration with the 12-factor app pattern:
/// 1. Defaults (from code)
/// 2. Config file (speckit.toml)
/// 3. Environment variables (SPECKIT_* prefix)
///
/// # Example
///
/// ```no_run
/// use codex_spec_kit::config::ConfigLoader;
///
/// // Load from default locations
/// let config = ConfigLoader::load_default().expect("Failed to load config");
///
/// // Or load from specific file
/// let config = ConfigLoader::new()
///     .with_file("./my-config.toml")
///     .load()
///     .expect("Failed to load config");
/// ```
pub mod error;
pub mod loader;
pub mod validator;

// Re-export main types
pub use error::{ConfigError, Result};
pub use loader::{
    AppConfig, ConfigLoader, ConsensusConfig, CostConfig, EvidenceConfig, ModelConfig,
    QualityGateConfig, RetryConfig,
};
pub use validator::SchemaValidator;
