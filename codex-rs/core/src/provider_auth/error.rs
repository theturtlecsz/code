//! Authentication error types for the provider auth framework.

use thiserror::Error;

/// Authentication errors for provider OAuth flows.
#[derive(Debug, Error)]
pub enum AuthError {
    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    Network(#[from] reqwest::Error),

    /// OAuth error from the provider.
    #[error("OAuth error: {error} - {description}")]
    OAuth {
        /// OAuth error code.
        error: String,
        /// Human-readable description.
        description: String,
    },

    /// Token has expired and refresh failed.
    #[error("Token expired and refresh failed")]
    TokenExpired,

    /// Invalid or malformed token response from provider.
    #[error("Invalid token response: {0}")]
    InvalidResponse(String),

    /// IO error during file operations or callback server.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Provider is not authenticated.
    #[error("Provider not authenticated")]
    NotAuthenticated,

    /// Failed to open browser for OAuth flow.
    #[error("Failed to open browser: {0}")]
    BrowserLaunchFailed(String),

    /// Timeout waiting for OAuth callback.
    #[error("Timeout waiting for OAuth callback")]
    CallbackTimeout,

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),
}
