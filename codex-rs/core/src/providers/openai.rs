//! OpenAI / ChatGPT OAuth 2.0 provider implementation.
//!
//! Implements the [`ProviderAuth`] trait for OpenAI authentication,
//! maintaining backward compatibility with existing auth flows.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::provider_auth::{
    AuthError, OAuthConfig, ProviderAuth, ProviderCredentials, ProviderId, TokenResponse, pkce,
};

/// OpenAI OAuth authentication provider.
pub struct OpenAIAuth {
    client: reqwest::Client,
}

impl OpenAIAuth {
    /// Default OAuth client ID for ChatGPT authentication.
    /// Can be overridden with OPENAI_OAUTH_CLIENT_ID environment variable.
    pub const DEFAULT_CLIENT_ID: &'static str = "app_EMoamEEZ73f0CkXaXp7hrann";

    /// Authorization URL for OAuth flow.
    pub const AUTH_URL: &'static str = "https://auth.openai.com/authorize";

    /// Token exchange and refresh URL.
    pub const TOKEN_URL: &'static str = "https://auth.openai.com/oauth/token";

    /// OAuth redirect URI after authorization.
    pub const REDIRECT_URI: &'static str = "https://platform.openai.com/auth/callback";

    /// OAuth scopes for OpenAI.
    pub const SCOPES: &[&'static str] = &["openid", "profile", "email"];

    /// Get OAuth client ID from environment or use default.
    /// Environment variable: OPENAI_OAUTH_CLIENT_ID
    fn client_id() -> String {
        std::env::var("OPENAI_OAUTH_CLIENT_ID")
            .unwrap_or_else(|_| Self::DEFAULT_CLIENT_ID.to_string())
    }

    /// Creates a new OpenAI auth provider with the given HTTP client.
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ProviderAuth for OpenAIAuth {
    fn provider_id(&self) -> ProviderId {
        ProviderId::OpenAI
    }

    fn display_name(&self) -> &'static str {
        "OpenAI"
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
        let params = OpenAITokenRequest {
            grant_type: "authorization_code",
            client_id: &client_id,
            code,
            code_verifier,
            redirect_uri: Self::REDIRECT_URI,
        };

        let response = self
            .client
            .post(Self::TOKEN_URL)
            .header("Content-Type", "application/json")
            .json(&params)
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

        // Parse OpenAI-specific response
        let openai_response: OpenAITokenResponse = response.json().await?;

        // Convert to generic TokenResponse
        Ok(TokenResponse {
            access_token: openai_response.access_token.unwrap_or_default(),
            refresh_token: openai_response.refresh_token,
            expires_in: openai_response.expires_in,
            token_type: "Bearer".to_string(),
            scope: Some(Self::SCOPES.join(" ")),
            extra: serde_json::json!({
                "id_token": openai_response.id_token,
            }),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, AuthError> {
        let client_id = Self::client_id();
        let params = OpenAIRefreshRequest {
            grant_type: "refresh_token",
            client_id: &client_id,
            refresh_token,
            scope: &Self::SCOPES.join(" "),
        };

        let response = self
            .client
            .post(Self::TOKEN_URL)
            .header("Content-Type", "application/json")
            .json(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AuthError::TokenExpired);
        }

        // Parse OpenAI-specific response
        let openai_response: OpenAITokenResponse = response.json().await?;

        // Convert to generic TokenResponse
        Ok(TokenResponse {
            access_token: openai_response.access_token.unwrap_or_default(),
            refresh_token: openai_response.refresh_token,
            expires_in: openai_response.expires_in,
            token_type: "Bearer".to_string(),
            scope: Some(Self::SCOPES.join(" ")),
            extra: serde_json::json!({
                "id_token": openai_response.id_token,
            }),
        })
    }

    fn extract_metadata(&self, response: &TokenResponse) -> serde_json::Value {
        // OpenAI includes id_token with user info as JWT
        // Extract email and account info if present
        let id_token = response
            .extra
            .get("id_token")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Decode JWT payload (middle part) to get user info
        if let Some(payload) = decode_jwt_payload(id_token) {
            serde_json::json!({
                "provider": "openai",
                "email": payload.get("email").cloned(),
                "name": payload.get("name").cloned(),
                "account_id": payload.get("https://api.openai.com/auth").and_then(|auth| auth.get("user_id")).cloned(),
                "plan_type": payload.get("https://api.openai.com/auth").and_then(|auth| auth.get("chatgpt_plan_type")).cloned(),
            })
        } else {
            serde_json::json!({
                "provider": "openai"
            })
        }
    }

    fn needs_refresh(&self, credentials: &ProviderCredentials) -> bool {
        // OpenAI uses 28-day token expiry check
        credentials
            .expires_at
            .map(|exp| exp < chrono::Utc::now() + chrono::Duration::days(28))
            .unwrap_or(true) // If no expiry set, assume refresh needed
    }
}

/// OpenAI-specific token request structure.
#[derive(Serialize)]
struct OpenAITokenRequest<'a> {
    grant_type: &'a str,
    client_id: &'a str,
    code: &'a str,
    code_verifier: &'a str,
    redirect_uri: &'a str,
}

/// OpenAI-specific refresh request structure.
#[derive(Serialize)]
struct OpenAIRefreshRequest<'a> {
    grant_type: &'a str,
    client_id: &'a str,
    refresh_token: &'a str,
    scope: &'a str,
}

/// OpenAI-specific token response structure.
#[derive(Deserialize)]
struct OpenAITokenResponse {
    id_token: String,
    access_token: Option<String>,
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
}

/// Decodes the payload portion of a JWT token.
fn decode_jwt_payload(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    // JWT payload is base64url encoded
    let payload = parts[1];

    // Decode base64url (handle padding)
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;

    // Parse JSON
    serde_json::from_slice(&decoded).ok()
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
    fn test_openai_auth_constants() {
        assert_eq!(
            OpenAIAuth::DEFAULT_CLIENT_ID,
            "app_EMoamEEZ73f0CkXaXp7hrann"
        );
        assert_eq!(OpenAIAuth::AUTH_URL, "https://auth.openai.com/authorize");
        assert_eq!(OpenAIAuth::TOKEN_URL, "https://auth.openai.com/oauth/token");
        assert_eq!(
            OpenAIAuth::REDIRECT_URI,
            "https://platform.openai.com/auth/callback"
        );
    }

    #[test]
    fn test_openai_client_id_from_env() {
        // Test that client_id() uses env var if set
        unsafe {
            std::env::set_var("OPENAI_OAUTH_CLIENT_ID", "test-client-id");
        }
        assert_eq!(OpenAIAuth::client_id(), "test-client-id");
        unsafe {
            std::env::remove_var("OPENAI_OAUTH_CLIENT_ID");
        }

        // Test fallback to default
        assert_eq!(OpenAIAuth::client_id(), OpenAIAuth::DEFAULT_CLIENT_ID);
    }

    #[test]
    fn test_openai_provider_id() {
        let client = reqwest::Client::new();
        let auth = OpenAIAuth::new(client);
        assert_eq!(auth.provider_id(), ProviderId::OpenAI);
        assert_eq!(auth.display_name(), "OpenAI");
    }

    #[test]
    fn test_openai_oauth_config() {
        let client = reqwest::Client::new();
        let auth = OpenAIAuth::new(client);
        let config = auth.oauth_config();

        assert_eq!(config.client_id, OpenAIAuth::client_id());
        assert_eq!(config.auth_url, OpenAIAuth::AUTH_URL);
        assert_eq!(config.token_url, OpenAIAuth::TOKEN_URL);
        assert_eq!(config.redirect_uri, OpenAIAuth::REDIRECT_URI);
        assert!(config.use_pkce);
    }

    #[test]
    fn test_authorization_url_generation() {
        let client = reqwest::Client::new();
        let auth = OpenAIAuth::new(client);

        let state = "test-state";
        let verifier = "test-verifier";
        let url = auth.authorization_url(state, verifier);

        assert!(url.starts_with(OpenAIAuth::AUTH_URL));
        assert!(url.contains("client_id="));
        assert!(url.contains("state=test-state"));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
    }

    #[test]
    fn test_decode_jwt_payload() {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

        // Test with a simple JWT structure
        // Header: {"alg":"none"}
        // Payload: {"email":"test@example.com","name":"Test User"}
        // Signature: empty
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let payload = URL_SAFE_NO_PAD.encode(r#"{"email":"test@example.com","name":"Test User"}"#);
        let token = format!("{header}.{payload}.signature");

        let decoded = decode_jwt_payload(&token);
        assert!(decoded.is_some());

        let data = decoded.unwrap();
        assert_eq!(data.get("email").unwrap(), "test@example.com");
        assert_eq!(data.get("name").unwrap(), "Test User");
    }

    #[test]
    fn test_decode_jwt_payload_invalid() {
        assert!(decode_jwt_payload("not-a-jwt").is_none());
        assert!(decode_jwt_payload("only.two").is_none());
        assert!(decode_jwt_payload("").is_none());
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding::encode("hello"), "hello");
        assert_eq!(urlencoding::encode("hello world"), "hello%20world");
        assert_eq!(urlencoding::encode("a=b&c=d"), "a%3Db%26c%3Dd");
        assert_eq!(
            urlencoding::encode("test@example.com"),
            "test%40example.com"
        );
    }

    #[test]
    fn test_needs_refresh_no_expiry() {
        let client = reqwest::Client::new();
        let auth = OpenAIAuth::new(client);

        let credentials = ProviderCredentials {
            provider: ProviderId::OpenAI,
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        // Should need refresh if no expiry is set
        assert!(auth.needs_refresh(&credentials));
    }

    #[test]
    fn test_needs_refresh_fresh_token() {
        let client = reqwest::Client::new();
        let auth = OpenAIAuth::new(client);

        let credentials = ProviderCredentials {
            provider: ProviderId::OpenAI,
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::days(60)),
            metadata: serde_json::Value::Null,
        };

        // 60 days out should not need refresh (28 day threshold)
        assert!(!auth.needs_refresh(&credentials));
    }
}
