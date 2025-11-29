//! OpenAI Device Code Authorization
//!
//! FORK-SPECIFIC (just-every/code): P6-SYNC Phase 5
//!
//! Implements device code OAuth 2.0 flow for OpenAI/ChatGPT authentication.
//! This allows users to authenticate without a browser callback server,
//! making it suitable for headless environments and CLI tools.
//!
//! OpenAI Device Code Flow:
//! 1. POST to /oauth/device with client_id and scope
//! 2. Display user_code and verification_uri to user
//! 3. Poll /oauth/token with device_code until success
//! 4. Store access_token and refresh_token

use crate::device_code::{
    DeviceAuthError, DeviceAuthorizationResponse, DeviceCodeAuth, DeviceCodeProvider, PollError,
    RefreshError, TokenResponse,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// OpenAI Auth endpoints
const OPENAI_AUTH_BASE: &str = "https://auth.openai.com";
const DEVICE_AUTH_ENDPOINT: &str = "/oauth/device";
const TOKEN_ENDPOINT: &str = "/oauth/token";

/// Default client ID for OpenAI OAuth
/// Can be overridden with OPENAI_OAUTH_CLIENT_ID environment variable
const DEFAULT_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

/// OpenAI OAuth scopes
const OPENAI_SCOPES: &[&str] = &["openid", "profile", "email", "offline_access"];

/// OpenAI device code authorization client
pub struct OpenAIDeviceCode {
    client: reqwest::Client,
    client_id: String,
    auth_base: String,
}

impl Default for OpenAIDeviceCode {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAIDeviceCode {
    /// Create with default configuration
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            client_id: Self::get_client_id(),
            auth_base: OPENAI_AUTH_BASE.to_string(),
        }
    }

    /// Create with custom client ID
    pub fn with_client_id(client_id: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            client_id,
            auth_base: OPENAI_AUTH_BASE.to_string(),
        }
    }

    /// Get client ID from environment or use default
    fn get_client_id() -> String {
        std::env::var("OPENAI_OAUTH_CLIENT_ID").unwrap_or_else(|_| DEFAULT_CLIENT_ID.to_string())
    }

    /// Build URL for an endpoint
    fn endpoint_url(&self, path: &str) -> String {
        format!("{}{}", self.auth_base, path)
    }
}

/// OAuth error response from OpenAI
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
impl DeviceCodeAuth for OpenAIDeviceCode {
    fn provider(&self) -> DeviceCodeProvider {
        DeviceCodeProvider::OpenAI
    }

    async fn start_device_authorization(
        &self,
    ) -> Result<DeviceAuthorizationResponse, DeviceAuthError> {
        let url = self.endpoint_url(DEVICE_AUTH_ENDPOINT);
        let scope = OPENAI_SCOPES.join(" ");

        let request = DeviceAuthRequest {
            client_id: self.client_id.clone(),
            scope,
        };

        let response = self
            .client
            .post(&url)
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

            return Err(DeviceAuthError::Server(format!("HTTP {status}: {body}")));
        }

        response
            .json::<DeviceAuthorizationResponse>()
            .await
            .map_err(|e| DeviceAuthError::Parse(e.to_string()))
    }

    async fn poll_for_token(&self, device_code: &str) -> Result<TokenResponse, PollError> {
        let url = self.endpoint_url(TOKEN_ENDPOINT);

        let request = DeviceTokenRequest {
            client_id: self.client_id.clone(),
            device_code: device_code.to_string(),
            grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
        };

        let response = self
            .client
            .post(&url)
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
        let url = self.endpoint_url(TOKEN_ENDPOINT);

        let request = RefreshTokenRequest {
            client_id: self.client_id.clone(),
            refresh_token: refresh_token.to_string(),
            grant_type: "refresh_token".to_string(),
        };

        let response = self
            .client
            .post(&url)
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
        OPENAI_SCOPES
    }

    fn display_name(&self) -> &str {
        "OpenAI (ChatGPT)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_device_code_creation() {
        let client = OpenAIDeviceCode::new();
        assert_eq!(client.provider(), DeviceCodeProvider::OpenAI);
        assert_eq!(client.display_name(), "OpenAI (ChatGPT)");
        assert!(!client.scopes().is_empty());
    }

    #[test]
    fn test_endpoint_urls() {
        let client = OpenAIDeviceCode::new();
        assert_eq!(
            client.endpoint_url(DEVICE_AUTH_ENDPOINT),
            "https://auth.openai.com/oauth/device"
        );
        assert_eq!(
            client.endpoint_url(TOKEN_ENDPOINT),
            "https://auth.openai.com/oauth/token"
        );
    }

    #[test]
    fn test_custom_client_id() {
        let client = OpenAIDeviceCode::with_client_id("custom-id".to_string());
        assert_eq!(client.client_id, "custom-id");
    }

    #[test]
    fn test_scopes_include_offline_access() {
        let client = OpenAIDeviceCode::new();
        assert!(client.scopes().contains(&"offline_access"));
    }
}
