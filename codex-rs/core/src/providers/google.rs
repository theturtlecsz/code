//! Google / Gemini OAuth 2.0 provider implementation.
//!
//! Implements the [`ProviderAuth`] trait for Google authentication
//! using OAuth 2.0 with PKCE (S256).

use async_trait::async_trait;
use serde::Deserialize;

use crate::provider_auth::{
    AuthError, OAuthConfig, ProviderAuth, ProviderCredentials, ProviderId, TokenResponse, pkce,
};

/// Google OAuth authentication provider for Gemini.
pub struct GoogleAuth {
    client: reqwest::Client,
    redirect_port: u16,
}

impl GoogleAuth {
    /// Default OAuth client ID for Gemini CLI authentication.
    /// Can be overridden with GOOGLE_OAUTH_CLIENT_ID environment variable.
    /// Discovered from gemini-cli OAuth flow analysis.
    pub const DEFAULT_CLIENT_ID: &'static str =
        "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";

    /// Authorization URL for Google OAuth flow.
    pub const AUTH_URL: &'static str = "https://accounts.google.com/o/oauth2/v2/auth";

    /// Token exchange and refresh URL.
    pub const TOKEN_URL: &'static str = "https://oauth2.googleapis.com/token";

    /// Get OAuth client ID from environment or use default.
    /// Environment variable: GOOGLE_OAUTH_CLIENT_ID
    fn client_id() -> String {
        std::env::var("GOOGLE_OAUTH_CLIENT_ID")
            .unwrap_or_else(|_| Self::DEFAULT_CLIENT_ID.to_string())
    }

    /// Get OAuth client secret from environment.
    /// Environment variable: GOOGLE_OAUTH_CLIENT_SECRET (required)
    ///
    /// # Panics
    /// Panics if GOOGLE_OAUTH_CLIENT_SECRET is not set.
    /// For native apps, this should be the public OAuth client secret.
    #[allow(clippy::expect_used)] // intentional panic for required config
    fn client_secret() -> String {
        std::env::var("GOOGLE_OAUTH_CLIENT_SECRET")
            .expect("GOOGLE_OAUTH_CLIENT_SECRET environment variable must be set for Google OAuth")
    }

    /// OAuth scopes for Google/Gemini.
    /// Uses generativelanguage.tuning for Gemini API access.
    pub const SCOPES: &[&'static str] = &[
        "https://www.googleapis.com/auth/generativelanguage.tuning",
        "https://www.googleapis.com/auth/userinfo.email",
        "https://www.googleapis.com/auth/userinfo.profile",
        "openid",
    ];

    /// Creates a new Google auth provider with the given HTTP client.
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client for making requests
    /// * `redirect_port` - Port for local OAuth callback server (0 for auto-assign)
    pub fn new(client: reqwest::Client, redirect_port: u16) -> Self {
        Self {
            client,
            redirect_port,
        }
    }

    /// Returns the redirect URI for the OAuth callback.
    fn redirect_uri(&self) -> String {
        format!("http://localhost:{}", self.redirect_port)
    }
}

#[async_trait]
impl ProviderAuth for GoogleAuth {
    fn provider_id(&self) -> ProviderId {
        ProviderId::Google
    }

    fn display_name(&self) -> &'static str {
        "Google"
    }

    fn oauth_config(&self) -> OAuthConfig {
        OAuthConfig {
            client_id: Self::client_id(),
            auth_url: Self::AUTH_URL.to_string(),
            token_url: Self::TOKEN_URL.to_string(),
            redirect_uri: self.redirect_uri(),
            scopes: Self::SCOPES.iter().map(|s| (*s).to_string()).collect(),
            use_pkce: true,
        }
    }

    fn authorization_url(&self, state: &str, code_verifier: &str) -> String {
        let challenge = pkce::generate_code_challenge(code_verifier);
        let scopes = Self::SCOPES.join(" ");

        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
            Self::AUTH_URL,
            urlencoding::encode(&Self::client_id()),
            urlencoding::encode(&self.redirect_uri()),
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
        let client_secret = Self::client_secret();
        let redirect_uri = self.redirect_uri();

        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("code", code),
            ("code_verifier", code_verifier),
            ("redirect_uri", &redirect_uri),
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

        // Parse Google-specific response
        let google_response: GoogleTokenResponse = response.json().await?;

        // Convert to generic TokenResponse
        Ok(TokenResponse {
            access_token: google_response.access_token,
            refresh_token: google_response.refresh_token,
            expires_in: google_response.expires_in,
            token_type: google_response
                .token_type
                .unwrap_or_else(|| "Bearer".to_string()),
            scope: google_response.scope,
            extra: serde_json::json!({
                "id_token": google_response.id_token,
            }),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, AuthError> {
        let client_id = Self::client_id();
        let client_secret = Self::client_secret();

        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
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

        // Parse Google-specific response
        let google_response: GoogleTokenResponse = response.json().await?;

        // Convert to generic TokenResponse
        // Note: Google doesn't return a new refresh_token on refresh
        Ok(TokenResponse {
            access_token: google_response.access_token,
            refresh_token: google_response.refresh_token,
            expires_in: google_response.expires_in,
            token_type: google_response
                .token_type
                .unwrap_or_else(|| "Bearer".to_string()),
            scope: google_response.scope,
            extra: serde_json::json!({
                "id_token": google_response.id_token,
            }),
        })
    }

    fn extract_metadata(&self, response: &TokenResponse) -> serde_json::Value {
        // Google includes id_token with user info as JWT
        // Extract email and profile info if present
        let id_token = response
            .extra
            .get("id_token")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Decode JWT payload to get user info
        if let Some(payload) = decode_jwt_payload(id_token) {
            serde_json::json!({
                "provider": "google",
                "email": payload.get("email").cloned(),
                "name": payload.get("name").cloned(),
                "picture": payload.get("picture").cloned(),
                "sub": payload.get("sub").cloned(), // Google user ID
            })
        } else {
            serde_json::json!({
                "provider": "google"
            })
        }
    }

    fn needs_refresh(&self, credentials: &ProviderCredentials) -> bool {
        // Google tokens expire; check with 5-minute pre-expiry buffer
        credentials
            .expires_at
            .map(|exp| exp < chrono::Utc::now() + chrono::Duration::minutes(5))
            .unwrap_or(false)
    }
}

/// Google-specific token response structure.
#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    token_type: Option<String>,
    scope: Option<String>,
    id_token: Option<String>,
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
    fn test_google_auth_constants() {
        // Test default client ID
        assert_eq!(
            GoogleAuth::DEFAULT_CLIENT_ID,
            "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com"
        );
        assert_eq!(
            GoogleAuth::AUTH_URL,
            "https://accounts.google.com/o/oauth2/v2/auth"
        );
        assert_eq!(GoogleAuth::TOKEN_URL, "https://oauth2.googleapis.com/token");
    }

    #[test]
    fn test_google_client_id_from_env() {
        // Test that client_id() uses env var if set
        unsafe {
            std::env::set_var("GOOGLE_OAUTH_CLIENT_ID", "test-client-id");
        }
        assert_eq!(GoogleAuth::client_id(), "test-client-id");
        unsafe {
            std::env::remove_var("GOOGLE_OAUTH_CLIENT_ID");
        }

        // Test fallback to default
        assert_eq!(GoogleAuth::client_id(), GoogleAuth::DEFAULT_CLIENT_ID);
    }

    #[test]
    #[should_panic(expected = "GOOGLE_OAUTH_CLIENT_SECRET environment variable must be set")]
    fn test_google_client_secret_missing() {
        // Ensure no env var is set
        unsafe {
            std::env::remove_var("GOOGLE_OAUTH_CLIENT_SECRET");
        }
        // This should panic
        let _ = GoogleAuth::client_secret();
    }

    #[test]
    fn test_google_provider_id() {
        let client = reqwest::Client::new();
        let auth = GoogleAuth::new(client, 8080);
        assert_eq!(auth.provider_id(), ProviderId::Google);
        assert_eq!(auth.display_name(), "Google");
    }

    #[test]
    fn test_google_oauth_config() {
        // Set required env var for test
        unsafe {
            std::env::set_var("GOOGLE_OAUTH_CLIENT_SECRET", "test-secret");
        }

        let client = reqwest::Client::new();
        let auth = GoogleAuth::new(client, 8080);
        let config = auth.oauth_config();

        assert_eq!(config.client_id, GoogleAuth::client_id());
        assert_eq!(config.auth_url, GoogleAuth::AUTH_URL);
        assert_eq!(config.token_url, GoogleAuth::TOKEN_URL);
        assert_eq!(config.redirect_uri, "http://localhost:8080");
        assert!(config.use_pkce);
        assert_eq!(config.scopes.len(), 4);

        unsafe {
            std::env::remove_var("GOOGLE_OAUTH_CLIENT_SECRET");
        }
    }

    #[test]
    fn test_google_scopes() {
        assert!(
            GoogleAuth::SCOPES
                .contains(&"https://www.googleapis.com/auth/generativelanguage.tuning")
        );
        assert!(GoogleAuth::SCOPES.contains(&"https://www.googleapis.com/auth/userinfo.email"));
        assert!(GoogleAuth::SCOPES.contains(&"https://www.googleapis.com/auth/userinfo.profile"));
        assert!(GoogleAuth::SCOPES.contains(&"openid"));
    }

    #[test]
    fn test_redirect_uri() {
        let client = reqwest::Client::new();
        let auth = GoogleAuth::new(client, 9999);
        assert_eq!(auth.redirect_uri(), "http://localhost:9999");
    }

    #[test]
    fn test_authorization_url_generation() {
        let client = reqwest::Client::new();
        let auth = GoogleAuth::new(client, 8080);

        let state = "test-state";
        let verifier = "test-verifier";
        let url = auth.authorization_url(state, verifier);

        assert!(url.starts_with(GoogleAuth::AUTH_URL));
        assert!(url.contains("client_id="));
        assert!(url.contains("state=test-state"));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("scope="));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("prompt=consent"));
    }

    #[test]
    fn test_needs_refresh_recent_token() {
        let client = reqwest::Client::new();
        let auth = GoogleAuth::new(client, 8080);

        let credentials = ProviderCredentials {
            provider: ProviderId::Google,
            access_token: "ya29.test-token".to_string(),
            refresh_token: Some("1//test-refresh".to_string()),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            metadata: serde_json::Value::Null,
        };

        // Token with 1 hour remaining should not need refresh
        assert!(!auth.needs_refresh(&credentials));
    }

    #[test]
    fn test_needs_refresh_expiring_token() {
        let client = reqwest::Client::new();
        let auth = GoogleAuth::new(client, 8080);

        let credentials = ProviderCredentials {
            provider: ProviderId::Google,
            access_token: "ya29.test-token".to_string(),
            refresh_token: Some("1//test-refresh".to_string()),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::minutes(3)),
            metadata: serde_json::Value::Null,
        };

        // Token expiring in 3 minutes (< 5 minute buffer) should need refresh
        assert!(auth.needs_refresh(&credentials));
    }

    #[test]
    fn test_needs_refresh_no_expiry() {
        let client = reqwest::Client::new();
        let auth = GoogleAuth::new(client, 8080);

        let credentials = ProviderCredentials {
            provider: ProviderId::Google,
            access_token: "ya29.test-token".to_string(),
            refresh_token: None,
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        // No expiry should not trigger refresh
        assert!(!auth.needs_refresh(&credentials));
    }

    #[test]
    fn test_extract_metadata_no_id_token() {
        let client = reqwest::Client::new();
        let auth = GoogleAuth::new(client, 8080);

        let response = TokenResponse {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_in: None,
            token_type: "Bearer".to_string(),
            scope: None,
            extra: serde_json::Value::Null,
        };

        let metadata = auth.extract_metadata(&response);
        assert_eq!(metadata.get("provider").unwrap(), "google");
    }

    #[test]
    fn test_decode_jwt_payload() {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

        // Create a simple JWT
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let payload = URL_SAFE_NO_PAD
            .encode(r#"{"email":"test@gmail.com","name":"Test User","sub":"12345"}"#);
        let token = format!("{header}.{payload}.signature");

        let decoded = decode_jwt_payload(&token);
        assert!(decoded.is_some());

        let data = decoded.unwrap();
        assert_eq!(data.get("email").unwrap(), "test@gmail.com");
        assert_eq!(data.get("name").unwrap(), "Test User");
        assert_eq!(data.get("sub").unwrap(), "12345");
    }

    #[test]
    fn test_decode_jwt_payload_invalid() {
        assert!(decode_jwt_payload("not-a-jwt").is_none());
        assert!(decode_jwt_payload("only.two").is_none());
        assert!(decode_jwt_payload("").is_none());
    }
}
