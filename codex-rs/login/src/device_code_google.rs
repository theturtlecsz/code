//! Google Device Code Authorization
//!
//! FORK-SPECIFIC (just-every/code): P6-SYNC Phase 5
//!
//! Implements device code OAuth 2.0 flow for Google/Gemini authentication.
//! This allows users to authenticate for Gemini API access without a browser
//! callback server, making it suitable for headless environments and CLI tools.
//!
//! Google Device Code Flow:
//! 1. POST to /device/code with client_id and scope
//! 2. Display user_code and verification_url to user
//! 3. Poll /token with device_code until success
//! 4. Store access_token and refresh_token
//!
//! Note: Requires a Google Cloud project with OAuth consent screen configured
//! and the Generative Language API enabled.

use crate::device_code::{
    DeviceAuthError, DeviceAuthorizationResponse, DeviceCodeAuth, DeviceCodeProvider,
    PollError, RefreshError, TokenResponse,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Google OAuth2 endpoints
const GOOGLE_DEVICE_AUTH_URL: &str = "https://oauth2.googleapis.com/device/code";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Default client ID for Google OAuth (Gemini API access)
/// Users should configure their own client ID via environment variable
#[allow(dead_code)]
const DEFAULT_CLIENT_ID: &str = "";  // Must be configured

/// Google OAuth scopes for Gemini API
/// generative-language scope for Gemini API access
const GOOGLE_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/generative-language",
    "https://www.googleapis.com/auth/userinfo.email",
];

/// Google device code authorization client
pub struct GoogleDeviceCode {
    client: reqwest::Client,
    client_id: String,
    client_secret: Option<String>,
}

impl GoogleDeviceCode {
    /// Create with configuration from environment
    ///
    /// Requires GOOGLE_OAUTH_CLIENT_ID environment variable.
    /// Optionally uses GOOGLE_OAUTH_CLIENT_SECRET if configured.
    pub fn from_env() -> Result<Self, DeviceAuthError> {
        let client_id = std::env::var("GOOGLE_OAUTH_CLIENT_ID")
            .map_err(|_| DeviceAuthError::Config(
                "GOOGLE_OAUTH_CLIENT_ID environment variable not set. \
                 Create OAuth credentials at https://console.cloud.google.com/apis/credentials"
                    .to_string()
            ))?;

        let client_secret = std::env::var("GOOGLE_OAUTH_CLIENT_SECRET").ok();

        Ok(Self {
            client: reqwest::Client::new(),
            client_id,
            client_secret,
        })
    }

    /// Create with explicit credentials
    pub fn new(client_id: String, client_secret: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            client_id,
            client_secret,
        }
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        !self.client_id.is_empty()
    }
}

/// Google's device authorization response format
/// Slightly different field names than standard OAuth
#[derive(Debug, Deserialize)]
struct GoogleDeviceAuthResponse {
    device_code: String,
    user_code: String,
    verification_url: String,  // Google uses verification_url not verification_uri
    expires_in: u64,
    interval: u64,
}

impl From<GoogleDeviceAuthResponse> for DeviceAuthorizationResponse {
    fn from(resp: GoogleDeviceAuthResponse) -> Self {
        let verification_uri_complete = Some(format!(
            "{}?user_code={}",
            resp.verification_url,
            resp.user_code
        ));
        Self {
            device_code: resp.device_code,
            user_code: resp.user_code,
            verification_uri: resp.verification_url,
            verification_uri_complete,
            expires_in: resp.expires_in,
            interval: resp.interval,
        }
    }
}

/// OAuth error response from Google
#[derive(Debug, Deserialize)]
struct GoogleOAuthError {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

/// Device authorization request body
#[derive(Debug, Serialize)]
struct GoogleDeviceAuthRequest {
    client_id: String,
    scope: String,
}

/// Token request body for device code grant
#[derive(Debug, Serialize)]
struct GoogleDeviceTokenRequest {
    client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_secret: Option<String>,
    device_code: String,
    grant_type: String,
}

/// Token request body for refresh grant
#[derive(Debug, Serialize)]
struct GoogleRefreshTokenRequest {
    client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_secret: Option<String>,
    refresh_token: String,
    grant_type: String,
}

#[async_trait]
impl DeviceCodeAuth for GoogleDeviceCode {
    fn provider(&self) -> DeviceCodeProvider {
        DeviceCodeProvider::Google
    }

    async fn start_device_authorization(&self) -> Result<DeviceAuthorizationResponse, DeviceAuthError> {
        if !self.is_configured() {
            return Err(DeviceAuthError::Config(
                "Google OAuth not configured. Set GOOGLE_OAUTH_CLIENT_ID environment variable."
                    .to_string(),
            ));
        }

        let scope = GOOGLE_SCOPES.join(" ");

        let request = GoogleDeviceAuthRequest {
            client_id: self.client_id.clone(),
            scope,
        };

        let response = self
            .client
            .post(GOOGLE_DEVICE_AUTH_URL)
            .form(&request)
            .send()
            .await
            .map_err(|e| DeviceAuthError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if let Ok(oauth_err) = serde_json::from_str::<GoogleOAuthError>(&body) {
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

        let google_response: GoogleDeviceAuthResponse = response
            .json()
            .await
            .map_err(|e| DeviceAuthError::Parse(e.to_string()))?;

        Ok(google_response.into())
    }

    async fn poll_for_token(&self, device_code: &str) -> Result<TokenResponse, PollError> {
        let request = GoogleDeviceTokenRequest {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            device_code: device_code.to_string(),
            grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
        };

        let response = self
            .client
            .post(GOOGLE_TOKEN_URL)
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
        let oauth_err: GoogleOAuthError = serde_json::from_str(&body)
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
        let request = GoogleRefreshTokenRequest {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            refresh_token: refresh_token.to_string(),
            grant_type: "refresh_token".to_string(),
        };

        let response = self
            .client
            .post(GOOGLE_TOKEN_URL)
            .form(&request)
            .send()
            .await
            .map_err(|e| RefreshError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();

            if let Ok(oauth_err) = serde_json::from_str::<GoogleOAuthError>(&body) {
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
        GOOGLE_SCOPES
    }

    fn display_name(&self) -> &str {
        "Google (Gemini)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_device_code_not_configured() {
        // Ensure env var is not set for this test
        // SAFETY: Test-only, no concurrent access to this env var
        unsafe { std::env::remove_var("GOOGLE_OAUTH_CLIENT_ID") };

        let result = GoogleDeviceCode::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn test_google_device_code_creation() {
        let client = GoogleDeviceCode::new("test-client-id".to_string(), None);
        assert_eq!(client.provider(), DeviceCodeProvider::Google);
        assert_eq!(client.display_name(), "Google (Gemini)");
        assert!(client.is_configured());
    }

    #[test]
    fn test_empty_client_id_not_configured() {
        let client = GoogleDeviceCode::new(String::new(), None);
        assert!(!client.is_configured());
    }

    #[test]
    fn test_google_auth_response_conversion() {
        let google_response = GoogleDeviceAuthResponse {
            device_code: "dev123".to_string(),
            user_code: "ABCD-1234".to_string(),
            verification_url: "https://google.com/device".to_string(),
            expires_in: 1800,
            interval: 5,
        };

        let response: DeviceAuthorizationResponse = google_response.into();
        assert_eq!(response.device_code, "dev123");
        assert_eq!(response.user_code, "ABCD-1234");
        assert_eq!(response.verification_uri, "https://google.com/device");
        assert!(response.verification_uri_complete.is_some());
        assert!(response
            .verification_uri_complete
            .unwrap()
            .contains("ABCD-1234"));
    }

    #[test]
    fn test_scopes_include_generative_language() {
        let client = GoogleDeviceCode::new("test".to_string(), None);
        assert!(client
            .scopes()
            .iter()
            .any(|s| s.contains("generative-language")));
    }
}
