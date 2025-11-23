//! API clients for direct provider integration (SPEC-KIT-953-F/G)
//!
//! This module provides native HTTP clients for Anthropic and Google APIs,
//! replacing CLI subprocess routing with direct API calls that support
//! conversation context and streaming responses.

mod anthropic;
mod google;

pub use anthropic::{AnthropicClient, AnthropicConfig};
pub use google::{GeminiClient, GeminiConfig, map_model_name as map_gemini_model};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from API client operations.
#[derive(Debug, Error)]
pub enum ApiError {
    /// Authentication error (no credentials or token expired).
    #[error("Not authenticated with provider")]
    NotAuthenticated,

    /// Token refresh failed.
    #[error("Failed to refresh access token: {0}")]
    TokenRefreshFailed(String),

    /// Network request failed.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// API returned an error response.
    #[error("API error ({status}): {message}")]
    ApiResponse {
        /// HTTP status code.
        status: u16,
        /// Error message from API.
        message: String,
        /// Error type (if provided).
        error_type: Option<String>,
    },

    /// Failed to parse API response.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Stream ended unexpectedly.
    #[error("Stream ended unexpectedly: {0}")]
    StreamError(String),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Token usage information from API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of input tokens consumed.
    pub input_tokens: u32,
    /// Number of output tokens generated.
    pub output_tokens: u32,
    /// Cache creation tokens (Anthropic-specific).
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    /// Cache read tokens (Anthropic-specific).
    #[serde(default)]
    pub cache_read_input_tokens: u32,
}

/// Events emitted during streaming response.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Stream started with message metadata.
    MessageStart {
        /// Message ID.
        id: String,
        /// Model used.
        model: String,
    },

    /// Content block started.
    ContentBlockStart {
        /// Block index.
        index: u32,
        /// Block type (text, tool_use).
        block_type: String,
    },

    /// Text delta received.
    TextDelta {
        /// Block index.
        index: u32,
        /// Text content.
        text: String,
    },

    /// Content block completed.
    ContentBlockStop {
        /// Block index.
        index: u32,
    },

    /// Message metadata update (stop reason, usage).
    MessageDelta {
        /// Stop reason (end_turn, tool_use, max_tokens).
        stop_reason: Option<String>,
        /// Token usage update.
        usage: Option<TokenUsage>,
    },

    /// Stream completed.
    MessageStop,

    /// Ping event (keepalive).
    Ping,
}

/// Result type for API client operations.
pub type ApiResult<T> = Result<T, ApiError>;
