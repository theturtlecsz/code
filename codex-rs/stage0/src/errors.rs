//! Stage0 error types
//!
//! Implements the error taxonomy from STAGE0_ERROR_TAXONOMY.md.
//! Default policy: soft failure (log + skip Stage 0) unless otherwise specified.

use thiserror::Error;

/// Error category for structured logging and behavior mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// `stage0.toml` or env misconfigured
    ConfigError,
    /// Errors creating/connecting/querying overlay SQLite
    OverlayDbError,
    /// Failures when Stage0 talks to local-memory via MCP/REST
    LocalMemoryError,
    /// Failures inside DCC logic (ranking, summarization)
    DccError,
    /// Errors calling NotebookLM via MCP
    Tier2Error,
    /// Prompt/response formatting issues for IQO or Tier2
    PromptError,
    /// Unexpected logic bugs/panics
    InternalError,
}

impl ErrorCategory {
    /// Machine-readable code for logging
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ConfigError => "CONFIG_ERROR",
            Self::OverlayDbError => "OVERLAY_DB_ERROR",
            Self::LocalMemoryError => "LOCAL_MEMORY_ERROR",
            Self::DccError => "DCC_ERROR",
            Self::Tier2Error => "TIER2_ERROR",
            Self::PromptError => "PROMPT_ERROR",
            Self::InternalError => "INTERNAL_ERROR",
        }
    }

    /// Whether DCC can still produce output after this error
    pub fn dcc_recoverable(&self) -> bool {
        matches!(self, Self::Tier2Error | Self::PromptError)
    }

    /// Whether Tier2 can still produce output (possibly fallback)
    pub fn tier2_recoverable(&self) -> bool {
        matches!(self, Self::PromptError)
    }
}

/// Stage0 error with category and context
#[derive(Debug, Error)]
pub enum Stage0Error {
    #[error("config error: {message}")]
    Config {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("overlay db error: {message}")]
    OverlayDb {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("local-memory error: {message}")]
    LocalMemory {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("DCC error: {message}")]
    Dcc {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Tier2 error: {message}")]
    Tier2 {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("prompt error: {message}")]
    Prompt {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("internal error: {message}")]
    Internal {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl Stage0Error {
    /// Get the error category
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::Config { .. } => ErrorCategory::ConfigError,
            Self::OverlayDb { .. } => ErrorCategory::OverlayDbError,
            Self::LocalMemory { .. } => ErrorCategory::LocalMemoryError,
            Self::Dcc { .. } => ErrorCategory::DccError,
            Self::Tier2 { .. } => ErrorCategory::Tier2Error,
            Self::Prompt { .. } => ErrorCategory::PromptError,
            Self::Internal { .. } => ErrorCategory::InternalError,
        }
    }

    /// Create a config error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
            source: None,
        }
    }

    /// Create a config error with source
    pub fn config_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Config {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an overlay DB error
    pub fn overlay_db(message: impl Into<String>) -> Self {
        Self::OverlayDb {
            message: message.into(),
            source: None,
        }
    }

    /// Create an overlay DB error with source
    pub fn overlay_db_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::OverlayDb {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            source: None,
        }
    }

    /// Create a prompt error
    pub fn prompt(message: impl Into<String>) -> Self {
        Self::Prompt {
            message: message.into(),
            source: None,
        }
    }

    /// Create a prompt error with source
    pub fn prompt_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Prompt {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

impl Clone for Stage0Error {
    fn clone(&self) -> Self {
        match self {
            Self::Config { message, .. } => Self::Config {
                message: message.clone(),
                source: None,
            },
            Self::OverlayDb { message, .. } => Self::OverlayDb {
                message: message.clone(),
                source: None,
            },
            Self::LocalMemory { message, .. } => Self::LocalMemory {
                message: message.clone(),
                source: None,
            },
            Self::Dcc { message, .. } => Self::Dcc {
                message: message.clone(),
                source: None,
            },
            Self::Tier2 { message, .. } => Self::Tier2 {
                message: message.clone(),
                source: None,
            },
            Self::Prompt { message, .. } => Self::Prompt {
                message: message.clone(),
                source: None,
            },
            Self::Internal { message, .. } => Self::Internal {
                message: message.clone(),
                source: None,
            },
        }
    }
}

/// Result type for Stage0 operations
pub type Result<T> = std::result::Result<T, Stage0Error>;
