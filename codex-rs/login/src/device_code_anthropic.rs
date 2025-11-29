//! Anthropic/Claude Device Code Authorization
//!
//! FORK-SPECIFIC (just-every/code): P6-SYNC Phase 5
//!
//! Implements device code OAuth 2.0 flow for Anthropic/Claude authentication.
//! This allows users to authenticate for Claude API access without a browser
//! callback server, making it suitable for headless environments and CLI tools.
//!
//! Anthropic Device Code Flow:
//! 1. POST to /oauth/device with client_id and scope
//! 2. Display user_code and verification_url to user
//! 3. Poll /oauth/token with device_code until success
//! 4. Store access_token and refresh_token
//!
//! Based on the OAuth endpoints discovered from Claude Code's auth flow.

use crate::device_code::{
    DeviceAuthError, DeviceAuthorizationResponse, DeviceCodeAuth, DeviceCodeProvider,
    PollError, RefreshError, TokenResponse,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Anthropic OAuth2 endpoints
/// Based on analysis of Claude Code authentication flow
const ANTHROPIC_DEVICE_AUTH_URL: &str = "https://claude.ai/oauth/device";
const ANTHROPIC_TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";

/// Default client ID for Anthropic OAuth
/// Discovered from Claude Code OAuth flow analysis
const DEFAULT_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

/// Anthropic OAuth scopes
const ANTHROPIC_SCOPES: &[&str] = &[
    "org:create_api_key",
    "user:profile",
    "user:inference",
];

/// Anthropic/Claude device code authorization client
pub struct AnthropicDeviceCode {
    client: reqwest::Client,
    client_id: String,
}

impl Default for AnthropicDeviceCode {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicDeviceCode {
    /// Create with default configuration
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            client_id: Self::get_client_id(),
        }
    }

    /// Create with custom client ID
    pub fn with_client_id(client_id: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            client_id,
        }
    }

    /// Get client ID from environment or use default
    fn get_client_id() -> String {
        std::env::var("ANTHROPIC_OAUTH_CLIENT_ID")
            .unwrap_or_else(|_| DEFAULT_CLIENT_ID.to_string())
    }
}

/// OAuth error response from Anthropic
#[derive(Debug, Deserialize)]
struct OAuthError {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

/// Device authorization request body
#[derive(Debug, Serialize)]
struct DeviceAuthRequest {
    client_id: String,
    scope: String,
}

/// Token request body for device code grant
#[derive(Debug, Serialize)]
struct DeviceTokenRequest {
    client_id: String,
    device_code: String,
    grant_type: String,
}

/// Token request body for refresh grant
#[derive(Debug, Serialize)]
struct RefreshTokenRequest {
    client_id: String,
    refresh_token: String,
    grant_type: String,
}

#[async_trait]
impl DeviceCodeAuth for AnthropicDeviceCode {
    fn provider(&self) -> DeviceCodeProvider {
        DeviceCodeProvider::Anthropic
    }

    async fn start_device_authorization(&self) -> Result<DeviceAuthorizationResponse, DeviceAuthError> {
        let scope = ANTHROPIC_SCOPES.join(" ");

        let request = DeviceAuthRequest {
            client_id: self.client_id.clone(),
            scope,
        };

        let response = self
            .client
            .post(ANTHROPIC_DEVICE_AUTH_URL)
            .form(&request)
            .send()
            .await
            .map_err(|e| DeviceAuthError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // Try to parse OAuth error
            if let Ok(oauth_err) = serde_json::from_str::<OAuthError>(&body) {
                return Err(DeviceAuthError::Server(format!(
                    "{}: {}",
                    oauth_err.error,
                    oauth_err.error_description.unwrap_or_default()
                )));
            }

            return Err(DeviceAuthError::Server(format!(
                "HTTP {status}: {body}"
            )));
        }

        response
            .json::<DeviceAuthorizationResponse>()
            .await
            .map_err(|e| DeviceAuthError::Parse(e.to_string()))
    }

    async fn poll_for_token(&self, device_code: &str) -> Result<TokenResponse, PollError> {
        let request = DeviceTokenRequest {
            client_id: self.client_id.clone(),
            device_code: device_code.to_string(),
            grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
        };

        let response = self
            .client
            .post(ANTHROPIC_TOKEN_URL)
            .form(&request)
            .send()
            .await
            .map_err(|e| PollError::Network(e.to_string()))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if status.is_success() {
            return serde_json::from_str::<TokenResponse>(&body)
                .map_err(|e| PollError::Parse(e.to_string()));
        }

        // Parse OAuth error to determine poll status
        let oauth_err: OAuthError = serde_json::from_str(&body)
            .map_err(|e| PollError::Parse(format!("Failed to parse error: {e}")))?;

        match oauth_err.error.as_str() {
            "authorization_pending" => Err(PollError::AuthorizationPending),
            "slow_down" => Err(PollError::SlowDown),
            "access_denied" => Err(PollError::AccessDenied),
            "expired_token" => Err(PollError::ExpiredToken),
            _ => Err(PollError::Server(format!(
                "{}: {}",
                oauth_err.error,
                oauth_err.error_description.unwrap_or_default()
            ))),
        }
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, RefreshError> {
        let request = RefreshTokenRequest {
            client_id: self.client_id.clone(),
            refresh_token: refresh_token.to_string(),
            grant_type: "refresh_token".to_string(),
        };

        let response = self
            .client
            .post(ANTHROPIC_TOKEN_URL)
            .form(&request)
            .send()
            .await
            .map_err(|e| RefreshError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();

            if let Ok(oauth_err) = serde_json::from_str::<OAuthError>(&body) {
                if oauth_err.error == "invalid_grant" {
                    return Err(RefreshError::InvalidGrant);
                }
                return Err(RefreshError::Server(format!(
                    "{}: {}",
                    oauth_err.error,
                    oauth_err.error_description.unwrap_or_default()
                )));
            }

            return Err(RefreshError::Server(body));
        }

        response
            .json::<TokenResponse>()
            .await
            .map_err(|e| RefreshError::Parse(e.to_string()))
    }

    fn scopes(&self) -> &[&str] {
        ANTHROPIC_SCOPES
    }

    fn display_name(&self) -> &str {
        "Anthropic (Claude)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_device_code_creation() {
        let client = AnthropicDeviceCode::new();
        assert_eq!(client.provider(), DeviceCodeProvider::Anthropic);
        assert_eq!(client.display_name(), "Anthropic (Claude)");
        assert!(!client.scopes().is_empty());
    }

    #[test]
    fn test_custom_client_id() {
        let client = AnthropicDeviceCode::with_client_id("custom-id".to_string());
        assert_eq!(client.client_id, "custom-id");
    }

    #[test]
    fn test_default_client_id() {
        let client = AnthropicDeviceCode::new();
        // Should use default when env var not set
        assert!(!client.client_id.is_empty());
    }

    #[test]
    fn test_scopes_include_inference() {
        let client = AnthropicDeviceCode::new();
        assert!(client.scopes().contains(&"user:inference"));
        assert!(client.scopes().contains(&"org:create_api_key"));
    }
}
