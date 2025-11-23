//! Provider authentication framework for multi-provider OAuth 2.0 PKCE support.
//!
//! This module provides a unified trait abstraction for authenticating with
//! multiple providers (OpenAI, Anthropic, Google) using OAuth 2.0 with PKCE.
//!
//! # Architecture
//!
//! The framework consists of:
//! - [`ProviderAuth`] trait: Core abstraction for provider authentication
//! - [`ProviderId`]: Enum identifying supported providers
//! - [`OAuthConfig`]: OAuth configuration for each provider
//! - [`TokenResponse`]: Token data from OAuth exchange
//! - [`ProviderCredentials`]: Stored credentials with metadata
//!
//! # Example
//!
//! ```rust,ignore
//! use codex_core::provider_auth::{ProviderId, ProviderAuthManager};
//!
//! let manager = ProviderAuthManager::new(codex_home);
//!
//! // Authenticate with a provider
//! manager.authenticate(ProviderId::Anthropic).await?;
//!
//! // Get access token (auto-refreshes if needed)
//! let token = manager.get_token(ProviderId::Anthropic).await?;
//! ```

pub mod callback_server;
mod error;
pub mod manager;
pub mod pkce;
pub mod storage;

pub use callback_server::CallbackServer;
pub use error::AuthError;
pub use manager::{ProviderAuthManager, TokenSource, TokenWithSource};
pub use storage::AuthAccountsStorage;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Provider identifier for routing and storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    /// OpenAI / ChatGPT provider.
    OpenAI,
    /// Anthropic / Claude provider.
    Anthropic,
    /// Google / Gemini provider.
    Google,
}

impl ProviderId {
    /// Returns all available provider IDs.
    pub fn all() -> &'static [ProviderId] {
        &[
            ProviderId::OpenAI,
            ProviderId::Anthropic,
            ProviderId::Google,
        ]
    }

    /// Returns the lowercase string representation for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderId::OpenAI => "openai",
            ProviderId::Anthropic => "anthropic",
            ProviderId::Google => "google",
        }
    }
}

impl fmt::Display for ProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderId::OpenAI => write!(f, "OpenAI"),
            ProviderId::Anthropic => write!(f, "Anthropic"),
            ProviderId::Google => write!(f, "Google"),
        }
    }
}

/// OAuth configuration for a provider.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// OAuth client ID.
    pub client_id: String,
    /// Authorization URL for the OAuth flow.
    pub auth_url: String,
    /// Token exchange URL.
    pub token_url: String,
    /// OAuth redirect URI.
    pub redirect_uri: String,
    /// OAuth scopes to request.
    pub scopes: Vec<String>,
    /// Whether to use PKCE (S256).
    pub use_pkce: bool,
}

/// Token response from OAuth exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    /// The access token for API calls.
    pub access_token: String,
    /// Refresh token for obtaining new access tokens.
    pub refresh_token: Option<String>,
    /// Token lifetime in seconds.
    pub expires_in: Option<u64>,
    /// Token type (usually "Bearer").
    pub token_type: String,
    /// Scopes granted.
    pub scope: Option<String>,
    /// Provider-specific additional fields (id_token for OpenAI, etc.).
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Provider-specific stored credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCredentials {
    /// Provider this credential belongs to.
    pub provider: ProviderId,
    /// OAuth access token.
    pub access_token: String,
    /// OAuth refresh token for token renewal.
    pub refresh_token: Option<String>,
    /// When the access token expires.
    pub expires_at: Option<DateTime<Utc>>,
    /// Provider-specific metadata (email, account_id, plan_type, etc.).
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl ProviderCredentials {
    /// Creates new credentials from a token response.
    pub fn from_token_response(
        provider: ProviderId,
        response: &TokenResponse,
        metadata: serde_json::Value,
    ) -> Self {
        let expires_at = response
            .expires_in
            .map(|secs| Utc::now() + chrono::Duration::seconds(secs as i64));

        Self {
            provider,
            access_token: response.access_token.clone(),
            refresh_token: response.refresh_token.clone(),
            expires_at,
            metadata,
        }
    }
}

/// Main trait for provider authentication.
///
/// Implementations handle provider-specific OAuth flows while maintaining
/// a consistent interface for the [`ProviderAuthManager`].
#[async_trait]
pub trait ProviderAuth: Send + Sync {
    /// Returns the provider identifier.
    fn provider_id(&self) -> ProviderId;

    /// Returns the display name for UI purposes.
    fn display_name(&self) -> &'static str;

    /// Returns the OAuth configuration for this provider.
    fn oauth_config(&self) -> OAuthConfig;

    /// Generates the full authorization URL with PKCE challenge.
    ///
    /// # Arguments
    ///
    /// * `state` - CSRF protection state parameter
    /// * `code_verifier` - PKCE code verifier for S256 challenge generation
    fn authorization_url(&self, state: &str, code_verifier: &str) -> String;

    /// Exchanges an authorization code for tokens.
    ///
    /// # Arguments
    ///
    /// * `code` - Authorization code from OAuth callback
    /// * `code_verifier` - PKCE code verifier used in the authorization request
    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<TokenResponse, AuthError>;

    /// Refreshes an expired access token.
    ///
    /// # Arguments
    ///
    /// * `refresh_token` - The refresh token from initial authentication
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, AuthError>;

    /// Extracts user metadata from a token response.
    ///
    /// Provider-specific implementation to decode JWT id_token or fetch
    /// user info from the response.
    fn extract_metadata(&self, response: &TokenResponse) -> serde_json::Value;

    /// Checks if the credentials need refreshing.
    ///
    /// Default implementation checks if expires_at is within 5 minutes.
    fn needs_refresh(&self, credentials: &ProviderCredentials) -> bool {
        credentials
            .expires_at
            .map(|exp| exp < Utc::now() + chrono::Duration::minutes(5))
            .unwrap_or(false)
    }
}

// Submodule exports (to be added as we implement them)
// pub mod callback_server;
// pub mod manager;
// pub mod pkce;
// pub mod storage;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id_serialization() {
        assert_eq!(
            serde_json::to_string(&ProviderId::OpenAI).unwrap(),
            r#""openai""#
        );
        assert_eq!(
            serde_json::to_string(&ProviderId::Anthropic).unwrap(),
            r#""anthropic""#
        );
        assert_eq!(
            serde_json::to_string(&ProviderId::Google).unwrap(),
            r#""google""#
        );
    }

    #[test]
    fn test_provider_id_deserialization() {
        assert_eq!(
            serde_json::from_str::<ProviderId>(r#""openai""#).unwrap(),
            ProviderId::OpenAI
        );
        assert_eq!(
            serde_json::from_str::<ProviderId>(r#""anthropic""#).unwrap(),
            ProviderId::Anthropic
        );
        assert_eq!(
            serde_json::from_str::<ProviderId>(r#""google""#).unwrap(),
            ProviderId::Google
        );
    }

    #[test]
    fn test_provider_id_display() {
        assert_eq!(ProviderId::OpenAI.to_string(), "OpenAI");
        assert_eq!(ProviderId::Anthropic.to_string(), "Anthropic");
        assert_eq!(ProviderId::Google.to_string(), "Google");
    }

    #[test]
    fn test_provider_id_as_str() {
        assert_eq!(ProviderId::OpenAI.as_str(), "openai");
        assert_eq!(ProviderId::Anthropic.as_str(), "anthropic");
        assert_eq!(ProviderId::Google.as_str(), "google");
    }

    #[test]
    fn test_provider_id_all() {
        let all = ProviderId::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&ProviderId::OpenAI));
        assert!(all.contains(&ProviderId::Anthropic));
        assert!(all.contains(&ProviderId::Google));
    }
}
