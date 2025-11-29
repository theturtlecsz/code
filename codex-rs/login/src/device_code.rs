//! Device Code Authorization Flow (RFC 8628)
//!
//! FORK-SPECIFIC (just-every/code): P6-SYNC Phase 5
//!
//! Implements OAuth 2.0 Device Authorization Grant for:
//! - OpenAI (ChatGPT authentication for non-browser environments)
//! - Google (Gemini API authentication)
//!
//! Flow overview:
//! 1. Request device code from authorization server
//! 2. Display user code and verification URL to user
//! 3. Poll token endpoint until user completes authorization
//! 4. Store and refresh tokens as needed

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Response from device authorization endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAuthorizationResponse {
    /// The device verification code
    pub device_code: String,
    /// The end-user verification code (shown to user)
    pub user_code: String,
    /// The verification URI to display to user
    pub verification_uri: String,
    /// Optional verification URI that includes user_code
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    /// Lifetime in seconds of the device_code and user_code
    pub expires_in: u64,
    /// Minimum polling interval in seconds
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

impl DeviceAuthorizationResponse {
    /// Get the polling interval as Duration
    pub fn poll_interval(&self) -> Duration {
        Duration::from_secs(self.interval)
    }

    /// Get expiration as Duration
    pub fn expires_duration(&self) -> Duration {
        Duration::from_secs(self.expires_in)
    }

    /// Get the best URL to show to user (complete if available)
    pub fn display_uri(&self) -> &str {
        self.verification_uri_complete
            .as_deref()
            .unwrap_or(&self.verification_uri)
    }
}

/// Successful token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    /// The access token
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Lifetime in seconds of the access token
    #[serde(default)]
    pub expires_in: Option<u64>,
    /// Refresh token for obtaining new access tokens
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Scope of the access token
    #[serde(default)]
    pub scope: Option<String>,
    /// ID token (for OIDC flows)
    #[serde(default)]
    pub id_token: Option<String>,
}

impl TokenResponse {
    /// Check if this response includes a refresh token
    pub fn has_refresh_token(&self) -> bool {
        self.refresh_token.is_some()
    }

    /// Get expiration as Duration if available
    pub fn expires_duration(&self) -> Option<Duration> {
        self.expires_in.map(Duration::from_secs)
    }
}

/// Error during device code polling
#[derive(Debug, Error)]
pub enum PollError {
    /// Authorization pending - user hasn't completed auth yet
    #[error("authorization_pending: user hasn't completed authorization")]
    AuthorizationPending,

    /// Slow down - increase polling interval
    #[error("slow_down: polling too frequently, increase interval")]
    SlowDown,

    /// Access denied by user
    #[error("access_denied: user denied the authorization request")]
    AccessDenied,

    /// Device code expired
    #[error("expired_token: device code has expired, restart flow")]
    ExpiredToken,

    /// Network or HTTP error
    #[error("network error: {0}")]
    Network(String),

    /// Unexpected error from server
    #[error("server error: {0}")]
    Server(String),

    /// JSON parsing error
    #[error("parse error: {0}")]
    Parse(String),
}

/// Error during token refresh
#[derive(Debug, Error)]
pub enum RefreshError {
    /// Invalid or expired refresh token
    #[error("invalid_grant: refresh token is invalid or expired")]
    InvalidGrant,

    /// Network or HTTP error
    #[error("network error: {0}")]
    Network(String),

    /// Server error
    #[error("server error: {0}")]
    Server(String),

    /// JSON parsing error
    #[error("parse error: {0}")]
    Parse(String),
}

/// Error during device authorization request
#[derive(Debug, Error)]
pub enum DeviceAuthError {
    /// Network or HTTP error
    #[error("network error: {0}")]
    Network(String),

    /// Server error
    #[error("server error: {0}")]
    Server(String),

    /// JSON parsing error
    #[error("parse error: {0}")]
    Parse(String),

    /// Invalid configuration
    #[error("configuration error: {0}")]
    Config(String),
}

/// Provider identifier for device code auth
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceCodeProvider {
    OpenAI,
    Google,
    Anthropic,
}

impl DeviceCodeProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Google => "google",
            Self::Anthropic => "anthropic",
        }
    }

    /// Human-readable display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::OpenAI => "OpenAI",
            Self::Google => "Gemini",
            Self::Anthropic => "Claude",
        }
    }
}

impl std::fmt::Display for DeviceCodeProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Trait for device code authorization implementations
///
/// Each provider (OpenAI, Google) implements this trait with their
/// specific endpoints and client configurations.
#[async_trait::async_trait]
pub trait DeviceCodeAuth: Send + Sync {
    /// Provider identifier
    fn provider(&self) -> DeviceCodeProvider;

    /// Start device authorization flow
    ///
    /// Returns the device code and user code for the user to complete
    /// authorization in their browser.
    async fn start_device_authorization(&self) -> Result<DeviceAuthorizationResponse, DeviceAuthError>;

    /// Poll for token after user authorization
    ///
    /// Should be called repeatedly at the interval specified in
    /// DeviceAuthorizationResponse until success or terminal error.
    async fn poll_for_token(
        &self,
        device_code: &str,
    ) -> Result<TokenResponse, PollError>;

    /// Refresh an expired access token
    ///
    /// Uses the refresh token to obtain a new access token without
    /// requiring user interaction.
    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<TokenResponse, RefreshError>;

    /// Get the scopes requested for this provider
    fn scopes(&self) -> &[&str];

    /// Get human-readable provider name for UI display
    fn display_name(&self) -> &str;
}

/// Stored token data for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    /// Provider this token is for
    pub provider: DeviceCodeProvider,
    /// The access token
    pub access_token: String,
    /// Refresh token if available
    pub refresh_token: Option<String>,
    /// When the access token expires (Unix timestamp)
    pub expires_at: Option<i64>,
    /// Token scope
    pub scope: Option<String>,
    /// When this token was stored (Unix timestamp)
    pub stored_at: i64,
}

impl StoredToken {
    /// Create from TokenResponse
    pub fn from_response(provider: DeviceCodeProvider, response: TokenResponse) -> Self {
        let now = chrono::Utc::now().timestamp();
        let expires_at = response.expires_in.map(|secs| now + secs as i64);

        Self {
            provider,
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_at,
            scope: response.scope,
            stored_at: now,
        }
    }

    /// Check if the access token is expired (with 5 minute buffer)
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(exp) => {
                let now = chrono::Utc::now().timestamp();
                let buffer = 5 * 60; // 5 minutes buffer
                now >= (exp - buffer)
            }
            None => false, // No expiry means never expires
        }
    }

    /// Check if we can refresh this token
    pub fn can_refresh(&self) -> bool {
        self.refresh_token.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_auth_response_display_uri() {
        let response = DeviceAuthorizationResponse {
            device_code: "dev123".to_string(),
            user_code: "ABCD-1234".to_string(),
            verification_uri: "https://example.com/device".to_string(),
            verification_uri_complete: None,
            expires_in: 600,
            interval: 5,
        };
        assert_eq!(response.display_uri(), "https://example.com/device");

        let response_with_complete = DeviceAuthorizationResponse {
            verification_uri_complete: Some("https://example.com/device?code=ABCD-1234".to_string()),
            ..response
        };
        assert_eq!(
            response_with_complete.display_uri(),
            "https://example.com/device?code=ABCD-1234"
        );
    }

    #[test]
    fn test_stored_token_expiry() {
        let now = chrono::Utc::now().timestamp();

        // Not expired
        let token = StoredToken {
            provider: DeviceCodeProvider::OpenAI,
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: Some(now + 3600), // 1 hour from now
            scope: None,
            stored_at: now,
        };
        assert!(!token.is_expired());

        // Expired
        let expired_token = StoredToken {
            expires_at: Some(now - 100), // 100 seconds ago
            ..token.clone()
        };
        assert!(expired_token.is_expired());

        // Within buffer (5 min)
        let nearly_expired = StoredToken {
            expires_at: Some(now + 200), // 3.3 min from now (within 5 min buffer)
            ..token
        };
        assert!(nearly_expired.is_expired());
    }

    #[test]
    fn test_provider_display() {
        assert_eq!(DeviceCodeProvider::OpenAI.as_str(), "openai");
        assert_eq!(DeviceCodeProvider::Google.as_str(), "google");
        assert_eq!(DeviceCodeProvider::Anthropic.as_str(), "anthropic");
        assert_eq!(format!("{}", DeviceCodeProvider::OpenAI), "openai");
        assert_eq!(format!("{}", DeviceCodeProvider::Anthropic), "anthropic");
    }
}
