use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during configuration loading and validation
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to load configuration file: {0}")]
    LoadError(String),

    #[error("Configuration file not found at path: {0}")]
    FileNotFound(PathBuf),

    #[error("Failed to parse configuration: {0}")]
    ParseError(String),

    #[error("Invalid configuration value: {0}")]
    ValidationError(String),

    #[error("Missing required configuration field: {0}")]
    MissingField(String),

    #[error("Environment variable error: {0}")]
    EnvError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON schema validation failed: {0}")]
    SchemaValidationError(String),

    #[error("Hot-reload error: {0}")]
    HotReloadError(String),
}

/// Type alias for Results using ConfigError
pub type Result<T> = std::result::Result<T, ConfigError>;

impl From<config::ConfigError> for ConfigError {
    fn from(err: config::ConfigError) -> Self {
        ConfigError::LoadError(err.to_string())
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        ConfigError::ParseError(err.to_string())
    }
}
