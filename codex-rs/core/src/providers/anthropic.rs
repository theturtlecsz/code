//! Anthropic / Claude OAuth 2.0 provider implementation.
//!
//! Implements the [`ProviderAuth`] trait for Anthropic authentication
//! using OAuth 2.0 with PKCE (S256).

use async_trait::async_trait;
use serde::Deserialize;

use crate::provider_auth::{
    AuthError, OAuthConfig, ProviderAuth, ProviderCredentials, ProviderId, TokenResponse, pkce,
};

/// Anthropic OAuth authentication provider.
pub struct AnthropicAuth {
    client: reqwest::Client,
}

impl AnthropicAuth {
    /// Default OAuth client ID for Claude authentication.
    /// Can be overridden with ANTHROPIC_OAUTH_CLIENT_ID environment variable.
    /// Discovered from Claude Code OAuth flow analysis.
    pub const DEFAULT_CLIENT_ID: &'static str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

    /// Authorization URL for OAuth flow.
    pub const AUTH_URL: &'static str = "https://claude.ai/oauth/authorize";

    /// Token exchange and refresh URL.
    pub const TOKEN_URL: &'static str = "https://console.anthropic.com/v1/oauth/token";

    /// OAuth redirect URI after authorization.
    pub const REDIRECT_URI: &'static str = "https://console.anthropic.com/oauth/code/callback";

    /// OAuth scopes for Anthropic.
    pub const SCOPES: &[&'static str] = &["org:create_api_key", "user:profile", "user:inference"];

    /// Get OAuth client ID from environment or use default.
    /// Environment variable: ANTHROPIC_OAUTH_CLIENT_ID
    fn client_id() -> String {
        std::env::var("ANTHROPIC_OAUTH_CLIENT_ID")
            .unwrap_or_else(|_| Self::DEFAULT_CLIENT_ID.to_string())
    }

    /// Creates a new Anthropic auth provider with the given HTTP client.
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ProviderAuth for AnthropicAuth {
    fn provider_id(&self) -> ProviderId {
        ProviderId::Anthropic
    }

    fn display_name(&self) -> &'static str {
        "Anthropic"
    }

    fn oauth_config(&self) -> OAuthConfig {
        OAuthConfig {
            client_id: Self::client_id(),
            auth_url: Self::AUTH_URL.to_string(),
            token_url: Self::TOKEN_URL.to_string(),
            redirect_uri: Self::REDIRECT_URI.to_string(),
            scopes: Self::SCOPES.iter().map(|s| (*s).to_string()).collect(),
            use_pkce: true,
        }
    }

    fn authorization_url(&self, state: &str, code_verifier: &str) -> String {
        let challenge = pkce::generate_code_challenge(code_verifier);
        let scopes = Self::SCOPES.join(" ");

        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
            Self::AUTH_URL,
            urlencoding::encode(&Self::client_id()),
            urlencoding::encode(Self::REDIRECT_URI),
            urlencoding::encode(&scopes),
            urlencoding::encode(state),
            urlencoding::encode(&challenge),
        )
    }

    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<TokenResponse, AuthError> {
        let client_id = Self::client_id();

        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", &client_id),
            ("code", code),
            ("code_verifier", code_verifier),
            ("redirect_uri", Self::REDIRECT_URI),
        ];

        let response = self
            .client
            .post(Self::TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(AuthError::OAuth {
                error: status.to_string(),
                description: error_body,
            });
        }

        // Parse Anthropic-specific response
        let anthropic_response: AnthropicTokenResponse = response.json().await?;

        // Convert to generic TokenResponse
        Ok(TokenResponse {
            access_token: anthropic_response.access_token,
            refresh_token: anthropic_response.refresh_token,
            expires_in: anthropic_response.expires_in,
            token_type: anthropic_response
                .token_type
                .unwrap_or_else(|| "Bearer".to_string()),
            scope: Some(Self::SCOPES.join(" ")),
            extra: serde_json::Value::Object(Default::default()),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, AuthError> {
        let client_id = Self::client_id();

        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", &client_id),
            ("refresh_token", refresh_token),
        ];

        let response = self
            .client
            .post(Self::TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AuthError::TokenExpired);
        }

        // Parse Anthropic-specific response
        let anthropic_response: AnthropicTokenResponse = response.json().await?;

        // Convert to generic TokenResponse
        Ok(TokenResponse {
            access_token: anthropic_response.access_token,
            refresh_token: anthropic_response.refresh_token,
            expires_in: anthropic_response.expires_in,
            token_type: anthropic_response
                .token_type
                .unwrap_or_else(|| "Bearer".to_string()),
            scope: Some(Self::SCOPES.join(" ")),
            extra: serde_json::Value::Object(Default::default()),
        })
    }

    fn extract_metadata(&self, _response: &TokenResponse) -> serde_json::Value {
        // Anthropic may include user info in token response or require separate API call
        // Token format: sk-ant-oat01-... (OAuth Access Token)
        serde_json::json!({
            "provider": "anthropic"
        })
    }

    fn needs_refresh(&self, credentials: &ProviderCredentials) -> bool {
        // Anthropic tokens typically expire; check expires_at with 5-minute buffer
        credentials
            .expires_at
            .map(|exp| exp < chrono::Utc::now() + chrono::Duration::minutes(5))
            .unwrap_or(false)
    }
}

/// Anthropic-specific token response structure.
#[derive(Deserialize)]
struct AnthropicTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    token_type: Option<String>,
}

/// Simple URL encoding for OAuth parameters.
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut result = String::with_capacity(s.len() * 3);
        for c in s.bytes() {
            match c {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(c as char);
                }
                _ => {
                    result.push('%');
                    result.push_str(&format!("{c:02X}"));
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_auth_constants() {
        // Test default client ID
        assert_eq!(
            AnthropicAuth::DEFAULT_CLIENT_ID,
            "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
        );
        assert_eq!(AnthropicAuth::AUTH_URL, "https://claude.ai/oauth/authorize");
        assert_eq!(
            AnthropicAuth::TOKEN_URL,
            "https://console.anthropic.com/v1/oauth/token"
        );
        assert_eq!(
            AnthropicAuth::REDIRECT_URI,
            "https://console.anthropic.com/oauth/code/callback"
        );
    }

    #[test]
    fn test_anthropic_client_id_from_env() {
        // Test that client_id() uses env var if set
        unsafe {
            std::env::set_var("ANTHROPIC_OAUTH_CLIENT_ID", "test-client-id");
        }
        assert_eq!(AnthropicAuth::client_id(), "test-client-id");
        unsafe {
            std::env::remove_var("ANTHROPIC_OAUTH_CLIENT_ID");
        }

        // Test fallback to default
        assert_eq!(AnthropicAuth::client_id(), AnthropicAuth::DEFAULT_CLIENT_ID);
    }

    #[test]
    fn test_anthropic_provider_id() {
        let client = reqwest::Client::new();
        let auth = AnthropicAuth::new(client);
        assert_eq!(auth.provider_id(), ProviderId::Anthropic);
        assert_eq!(auth.display_name(), "Anthropic");
    }

    #[test]
    fn test_anthropic_oauth_config() {
        let client = reqwest::Client::new();
        let auth = AnthropicAuth::new(client);
        let config = auth.oauth_config();

        assert_eq!(config.client_id, AnthropicAuth::client_id());
        assert_eq!(config.auth_url, AnthropicAuth::AUTH_URL);
        assert_eq!(config.token_url, AnthropicAuth::TOKEN_URL);
        assert_eq!(config.redirect_uri, AnthropicAuth::REDIRECT_URI);
        assert!(config.use_pkce);
        assert_eq!(config.scopes.len(), 3);
    }

    #[test]
    fn test_anthropic_scopes() {
        assert!(AnthropicAuth::SCOPES.contains(&"org:create_api_key"));
        assert!(AnthropicAuth::SCOPES.contains(&"user:profile"));
        assert!(AnthropicAuth::SCOPES.contains(&"user:inference"));
    }

    #[test]
    fn test_authorization_url_generation() {
        let client = reqwest::Client::new();
        let auth = AnthropicAuth::new(client);

        let state = "test-state";
        let verifier = "test-verifier";
        let url = auth.authorization_url(state, verifier);

        assert!(url.starts_with(AnthropicAuth::AUTH_URL));
        assert!(url.contains("client_id="));
        assert!(url.contains("state=test-state"));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("scope="));
    }

    #[test]
    fn test_needs_refresh_recent_token() {
        let client = reqwest::Client::new();
        let auth = AnthropicAuth::new(client);

        let credentials = ProviderCredentials {
            provider: ProviderId::Anthropic,
            access_token: "sk-ant-oat01-test".to_string(),
            refresh_token: Some("sk-ant-ort01-test".to_string()),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            metadata: serde_json::Value::Null,
        };

        // Token with 1 hour remaining should not need refresh
        assert!(!auth.needs_refresh(&credentials));
    }

    #[test]
    fn test_needs_refresh_expiring_token() {
        let client = reqwest::Client::new();
        let auth = AnthropicAuth::new(client);

        let credentials = ProviderCredentials {
            provider: ProviderId::Anthropic,
            access_token: "sk-ant-oat01-test".to_string(),
            refresh_token: Some("sk-ant-ort01-test".to_string()),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::minutes(3)),
            metadata: serde_json::Value::Null,
        };

        // Token expiring in 3 minutes (< 5 minute buffer) should need refresh
        assert!(auth.needs_refresh(&credentials));
    }

    #[test]
    fn test_needs_refresh_no_expiry() {
        let client = reqwest::Client::new();
        let auth = AnthropicAuth::new(client);

        let credentials = ProviderCredentials {
            provider: ProviderId::Anthropic,
            access_token: "sk-ant-oat01-test".to_string(),
            refresh_token: None,
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        // No expiry should not trigger refresh by default
        assert!(!auth.needs_refresh(&credentials));
    }

    #[test]
    fn test_extract_metadata() {
        let client = reqwest::Client::new();
        let auth = AnthropicAuth::new(client);

        let response = TokenResponse {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_in: None,
            token_type: "Bearer".to_string(),
            scope: None,
            extra: serde_json::Value::Null,
        };

        let metadata = auth.extract_metadata(&response);
        assert_eq!(metadata.get("provider").unwrap(), "anthropic");
    }
}
