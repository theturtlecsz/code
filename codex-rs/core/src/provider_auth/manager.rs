//! Central manager for multi-provider authentication.
//!
//! Orchestrates OAuth flows, token management, and credential storage
//! across multiple providers.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::providers::{AnthropicAuth, GoogleAuth, OpenAIAuth};

/// Load Claude CLI credentials from ~/.claude/.credentials.json
fn load_claude_cli_token() -> Option<String> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            tracing::warn!("Could not determine home directory for Claude CLI credentials");
            return None;
        }
    };
    let creds_path = home.join(".claude").join(".credentials.json");

    let content = match std::fs::read_to_string(&creds_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!(
                "Could not read Claude CLI credentials at {:?}: {}",
                creds_path,
                e
            );
            return None;
        }
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(e) => {
            tracing::warn!("Could not parse Claude CLI credentials: {}", e);
            return None;
        }
    };

    // Extract access token from claudeAiOauth
    let token = json
        .get("claudeAiOauth")
        .and_then(|o| o.get("accessToken"))
        .and_then(|t| t.as_str())
        .map(String::from);

    if token.is_none() {
        tracing::debug!("Claude CLI credentials file missing claudeAiOauth.accessToken");
    } else {
        tracing::debug!("Found Claude CLI credentials");
    }

    token
}

/// Load Gemini CLI credentials from ~/.gemini/oauth_creds.json
fn load_gemini_cli_token() -> Option<String> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            tracing::warn!("Could not determine home directory for Gemini CLI credentials");
            return None;
        }
    };
    let creds_path = home.join(".gemini").join("oauth_creds.json");

    let content = match std::fs::read_to_string(&creds_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!(
                "Could not read Gemini CLI credentials at {:?}: {}",
                creds_path,
                e
            );
            return None;
        }
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(e) => {
            tracing::warn!("Could not parse Gemini CLI credentials: {}", e);
            return None;
        }
    };

    // Extract access token
    let token = json
        .get("access_token")
        .and_then(|t| t.as_str())
        .map(String::from);

    if token.is_none() {
        tracing::warn!("Gemini CLI credentials file missing access_token");
    } else {
        tracing::info!("Found Gemini CLI credentials");
    }

    token
}

use super::{
    AuthError, CallbackServer, ProviderAuth, ProviderCredentials, ProviderId, TokenResponse, pkce,
    storage::AuthAccountsStorage,
};

/// Indicates where a token was loaded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSource {
    /// Token from regular codex storage (~/.code)
    Storage,
    /// Token from Claude CLI (~/.claude/.credentials.json)
    ClaudeCli,
    /// Token from Gemini CLI (~/.gemini/oauth_creds.json)
    GeminiCli,
}

/// Token with its source information.
#[derive(Debug, Clone)]
pub struct TokenWithSource {
    /// The access token.
    pub token: String,
    /// Where the token was loaded from.
    pub source: TokenSource,
}

/// Central manager for all provider authentications.
///
/// Provides a unified interface for:
/// - Initiating OAuth flows
/// - Managing tokens (get, refresh)
/// - Storing and loading credentials
pub struct ProviderAuthManager {
    /// Codex home directory for credential storage.
    codex_home: PathBuf,

    /// Provider implementations.
    providers: HashMap<ProviderId, Arc<dyn ProviderAuth>>,

    /// HTTP client for OAuth requests.
    client: reqwest::Client,
}

impl ProviderAuthManager {
    /// Creates a new provider auth manager.
    ///
    /// # Arguments
    ///
    /// * `codex_home` - Path to the codex home directory for credential storage
    pub fn new(codex_home: PathBuf) -> Self {
        let client = crate::default_client::create_client("codex_cli_rs");

        let mut providers: HashMap<ProviderId, Arc<dyn ProviderAuth>> = HashMap::new();
        providers.insert(
            ProviderId::OpenAI,
            Arc::new(OpenAIAuth::new(client.clone())),
        );
        providers.insert(
            ProviderId::Anthropic,
            Arc::new(AnthropicAuth::new(client.clone())),
        );
        // Google auth uses dynamic port for redirect
        providers.insert(
            ProviderId::Google,
            Arc::new(GoogleAuth::new(client.clone(), 0)),
        );

        Self {
            codex_home,
            providers,
            client,
        }
    }

    /// Gets a provider implementation.
    pub fn provider(&self, id: ProviderId) -> Option<Arc<dyn ProviderAuth>> {
        self.providers.get(&id).cloned()
    }

    /// Gets stored credentials for a provider.
    pub fn get_credentials(
        &self,
        provider: ProviderId,
    ) -> Result<Option<ProviderCredentials>, AuthError> {
        let storage = AuthAccountsStorage::load(&self.codex_home)?;
        Ok(storage.get_credentials(provider))
    }

    /// Gets an access token for a provider, refreshing if needed.
    ///
    /// # Arguments
    ///
    /// * `provider` - Provider to get token for
    ///
    /// # Returns
    ///
    /// The access token string.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::NotAuthenticated` if the provider has no credentials.
    pub async fn get_token(&self, provider: ProviderId) -> Result<String, AuthError> {
        tracing::debug!("get_token called for {:?}", provider);

        // Try to load storage, but don't fail if it's corrupt - fall through to CLI fallback
        let storage_result = AuthAccountsStorage::load(&self.codex_home);

        if let Ok(mut storage) = storage_result {
            // First try to get credentials from regular storage
            let creds = storage.get_credentials(provider);

            if let Some(credentials) = creds {
                let provider_impl = self.provider(provider).ok_or(AuthError::NotAuthenticated)?;

                if provider_impl.needs_refresh(&credentials) {
                    // Try to refresh, but fall through to CLI fallback on failure
                    if let Some(refresh_token) = &credentials.refresh_token {
                        match provider_impl.refresh_token(refresh_token).await {
                            Ok(response) => {
                                // Calculate new expiry
                                let expires_at = response.expires_in.map(|secs| {
                                    chrono::Utc::now() + chrono::Duration::seconds(secs as i64)
                                });

                                // Update stored credentials
                                if let Err(e) = storage.update_token(
                                    provider,
                                    &response.access_token,
                                    response.refresh_token.as_deref(),
                                    expires_at,
                                ) {
                                    tracing::warn!("Failed to update stored token: {}", e);
                                } else if let Err(e) = storage.save(&self.codex_home) {
                                    tracing::warn!("Failed to save token storage: {}", e);
                                }

                                return Ok(response.access_token);
                            }
                            Err(e) => {
                                tracing::debug!(
                                    "Failed to refresh token for {:?}, trying CLI fallback: {}",
                                    provider,
                                    e
                                );
                                // Fall through to CLI fallback
                            }
                        }
                    } else {
                        tracing::debug!("No refresh token for {:?}, trying CLI fallback", provider);
                        // Fall through to CLI fallback
                    }
                } else {
                    return Ok(credentials.access_token);
                }
            }
        } else {
            tracing::debug!(
                "Failed to load storage (will try CLI fallback): {:?}",
                storage_result.err()
            );
        }

        // Fallback: Try to load from CLI credential files
        tracing::debug!(
            "No credentials in storage for {:?}, trying CLI fallback",
            provider
        );
        match provider {
            ProviderId::Anthropic => {
                if let Some(token) = load_claude_cli_token() {
                    tracing::info!("Using Claude CLI credentials from ~/.claude/.credentials.json");
                    return Ok(token);
                }
            }
            ProviderId::Google => {
                if let Some(token) = load_gemini_cli_token() {
                    tracing::info!("Using Gemini CLI credentials from ~/.gemini/oauth_creds.json");
                    return Ok(token);
                }
            }
            _ => {}
        }

        tracing::debug!(
            "Authentication failed for {:?} - no valid credentials found",
            provider
        );
        Err(AuthError::NotAuthenticated)
    }

    /// Gets an access token with source information.
    ///
    /// Same as `get_token` but also returns where the token came from,
    /// which is useful for clients that need to adjust behavior based on
    /// the token source (e.g., using different User-Agent for CLI tokens).
    pub async fn get_token_with_source(
        &self,
        provider: ProviderId,
    ) -> Result<TokenWithSource, AuthError> {
        tracing::debug!("get_token_with_source called for {:?}", provider);

        // Try to load storage
        let storage_result = AuthAccountsStorage::load(&self.codex_home);

        if let Ok(mut storage) = storage_result {
            // First try to get credentials from regular storage
            let creds = storage.get_credentials(provider);

            if let Some(credentials) = creds {
                let provider_impl = self.provider(provider).ok_or(AuthError::NotAuthenticated)?;

                if provider_impl.needs_refresh(&credentials) {
                    // Try to refresh
                    if let Some(refresh_token) = &credentials.refresh_token {
                        match provider_impl.refresh_token(refresh_token).await {
                            Ok(response) => {
                                // Calculate new expiry
                                let expires_at = response.expires_in.map(|secs| {
                                    chrono::Utc::now() + chrono::Duration::seconds(secs as i64)
                                });

                                // Update stored credentials
                                if let Err(e) = storage.update_token(
                                    provider,
                                    &response.access_token,
                                    response.refresh_token.as_deref(),
                                    expires_at,
                                ) {
                                    tracing::warn!("Failed to update stored token: {}", e);
                                } else if let Err(e) = storage.save(&self.codex_home) {
                                    tracing::warn!("Failed to save token storage: {}", e);
                                }

                                return Ok(TokenWithSource {
                                    token: response.access_token,
                                    source: TokenSource::Storage,
                                });
                            }
                            Err(e) => {
                                tracing::debug!(
                                    "Failed to refresh token for {:?}: {}",
                                    provider,
                                    e
                                );
                                // Fall through to CLI fallback
                            }
                        }
                    }
                } else {
                    return Ok(TokenWithSource {
                        token: credentials.access_token,
                        source: TokenSource::Storage,
                    });
                }
            }
        }

        // Fallback: Try to load from CLI credential files
        match provider {
            ProviderId::Anthropic => {
                if let Some(token) = load_claude_cli_token() {
                    return Ok(TokenWithSource {
                        token,
                        source: TokenSource::ClaudeCli,
                    });
                }
            }
            ProviderId::Google => {
                if let Some(token) = load_gemini_cli_token() {
                    return Ok(TokenWithSource {
                        token,
                        source: TokenSource::GeminiCli,
                    });
                }
            }
            _ => {}
        }

        Err(AuthError::NotAuthenticated)
    }

    /// Initiates the OAuth flow for a provider.
    ///
    /// Opens a browser for authorization and waits for the callback.
    ///
    /// # Arguments
    ///
    /// * `provider` - Provider to authenticate with
    ///
    /// # Returns
    ///
    /// The account ID of the stored credentials.
    pub async fn authenticate(&self, provider: ProviderId) -> Result<String, AuthError> {
        let provider_impl = self.provider(provider).ok_or(AuthError::NotAuthenticated)?;

        // Generate PKCE and state
        let state = pkce::generate_state();
        let verifier = pkce::generate_code_verifier();

        // Start callback server
        let server = CallbackServer::new()?;
        let port = server.port();

        // For Google, we need to create a new provider with the correct port
        let provider_impl: Arc<dyn ProviderAuth> = if provider == ProviderId::Google {
            Arc::new(GoogleAuth::new(self.client.clone(), port))
        } else {
            provider_impl
        };

        // Build authorization URL
        let auth_url = provider_impl.authorization_url(&state, &verifier);

        // Open browser
        webbrowser::open(&auth_url).map_err(|e| AuthError::BrowserLaunchFailed(e.to_string()))?;

        // Wait for callback (5 minute timeout)
        let code = server.wait_for_code(&state, Duration::from_secs(300))?;

        // Exchange code for tokens
        let response = provider_impl.exchange_code(&code, &verifier).await?;

        // Extract metadata and create credentials
        let metadata = provider_impl.extract_metadata(&response);
        let credentials =
            ProviderCredentials::from_token_response(provider, &response, metadata.clone());

        // Determine label (email or default)
        let label = metadata
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        // Store credentials
        let mut storage = AuthAccountsStorage::load(&self.codex_home)?;
        let account_id = storage.store_credentials(provider, &credentials, &label);
        storage.save(&self.codex_home)?;

        Ok(account_id)
    }

    /// Checks if a provider has valid credentials.
    ///
    /// Checks both regular storage and CLI credential files.
    pub fn is_authenticated(&self, provider: ProviderId) -> Result<bool, AuthError> {
        let storage = AuthAccountsStorage::load(&self.codex_home)?;

        // First check regular storage
        if storage.get_credentials(provider).is_some() {
            return Ok(true);
        }

        // Fallback: Check CLI credential files
        match provider {
            ProviderId::Anthropic => Ok(load_claude_cli_token().is_some()),
            ProviderId::Google => Ok(load_gemini_cli_token().is_some()),
            _ => Ok(false),
        }
    }

    /// Removes all credentials for a provider.
    pub fn logout(&self, provider: ProviderId) -> Result<(), AuthError> {
        let mut storage = AuthAccountsStorage::load(&self.codex_home)?;

        // Remove all accounts for this provider
        let accounts_to_remove: Vec<String> = storage
            .get_accounts_for_provider(provider)
            .iter()
            .map(|a| a.id.clone())
            .collect();

        for account_id in accounts_to_remove {
            storage.remove_account(&account_id);
        }

        storage.save(&self.codex_home)?;
        Ok(())
    }

    /// Gets all authenticated providers.
    ///
    /// Includes both regular storage and CLI-authenticated providers.
    pub fn authenticated_providers(&self) -> Result<Vec<ProviderId>, AuthError> {
        let storage = AuthAccountsStorage::load(&self.codex_home)?;

        let mut providers = Vec::new();
        for provider_id in ProviderId::all() {
            // Check regular storage first
            if storage.get_credentials(*provider_id).is_some() {
                providers.push(*provider_id);
                continue;
            }

            // Fallback: Check CLI credentials
            let has_cli_creds = match provider_id {
                ProviderId::Anthropic => load_claude_cli_token().is_some(),
                ProviderId::Google => load_gemini_cli_token().is_some(),
                _ => false,
            };

            if has_cli_creds {
                providers.push(*provider_id);
            }
        }

        Ok(providers)
    }

    /// Manually stores credentials for a provider.
    ///
    /// Useful for importing credentials from other sources.
    pub fn store_credentials(
        &self,
        provider: ProviderId,
        response: &TokenResponse,
        label: &str,
    ) -> Result<String, AuthError> {
        let provider_impl = self.provider(provider).ok_or(AuthError::NotAuthenticated)?;

        let metadata = provider_impl.extract_metadata(response);
        let credentials = ProviderCredentials::from_token_response(provider, response, metadata);

        let mut storage = AuthAccountsStorage::load(&self.codex_home)?;
        let account_id = storage.store_credentials(provider, &credentials, label);
        storage.save(&self.codex_home)?;

        Ok(account_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_manager_creation() {
        let dir = tempdir().unwrap();
        let manager = ProviderAuthManager::new(dir.path().to_path_buf());

        assert!(manager.provider(ProviderId::OpenAI).is_some());
        assert!(manager.provider(ProviderId::Anthropic).is_some());
        assert!(manager.provider(ProviderId::Google).is_some());
    }

    #[test]
    fn test_get_credentials_empty() {
        let dir = tempdir().unwrap();
        let manager = ProviderAuthManager::new(dir.path().to_path_buf());

        let credentials = manager.get_credentials(ProviderId::OpenAI).unwrap();
        assert!(credentials.is_none());
    }

    #[test]
    fn test_is_authenticated_false() {
        let dir = tempdir().unwrap();
        let manager = ProviderAuthManager::new(dir.path().to_path_buf());

        assert!(!manager.is_authenticated(ProviderId::OpenAI).unwrap());
        assert!(!manager.is_authenticated(ProviderId::Anthropic).unwrap());
        assert!(!manager.is_authenticated(ProviderId::Google).unwrap());
    }

    #[test]
    fn test_authenticated_providers_empty() {
        let dir = tempdir().unwrap();
        let manager = ProviderAuthManager::new(dir.path().to_path_buf());

        let providers = manager.authenticated_providers().unwrap();
        assert!(providers.is_empty());
    }

    #[test]
    fn test_store_credentials() {
        let dir = tempdir().unwrap();
        let manager = ProviderAuthManager::new(dir.path().to_path_buf());

        let response = TokenResponse {
            access_token: "test-token".to_string(),
            refresh_token: Some("test-refresh".to_string()),
            expires_in: Some(3600),
            token_type: "Bearer".to_string(),
            scope: Some("openid profile email".to_string()),
            extra: serde_json::json!({"id_token": "test-id-token"}),
        };

        let account_id = manager
            .store_credentials(ProviderId::OpenAI, &response, "test@example.com")
            .unwrap();

        assert!(!account_id.is_empty());
        assert!(manager.is_authenticated(ProviderId::OpenAI).unwrap());

        let credentials = manager.get_credentials(ProviderId::OpenAI).unwrap();
        assert!(credentials.is_some());
        assert_eq!(credentials.unwrap().access_token, "test-token");
    }

    #[test]
    fn test_logout() {
        let dir = tempdir().unwrap();
        let manager = ProviderAuthManager::new(dir.path().to_path_buf());

        // Store credentials
        let response = TokenResponse {
            access_token: "test-token".to_string(),
            refresh_token: Some("test-refresh".to_string()),
            expires_in: Some(3600),
            token_type: "Bearer".to_string(),
            scope: None,
            extra: serde_json::Value::Null,
        };

        manager
            .store_credentials(ProviderId::Anthropic, &response, "test@example.com")
            .unwrap();
        assert!(manager.is_authenticated(ProviderId::Anthropic).unwrap());

        // Logout
        manager.logout(ProviderId::Anthropic).unwrap();
        assert!(!manager.is_authenticated(ProviderId::Anthropic).unwrap());
    }

    #[test]
    fn test_multiple_providers() {
        let dir = tempdir().unwrap();
        let manager = ProviderAuthManager::new(dir.path().to_path_buf());

        let response = TokenResponse {
            access_token: "token".to_string(),
            refresh_token: None,
            expires_in: Some(3600),
            token_type: "Bearer".to_string(),
            scope: None,
            extra: serde_json::Value::Null,
        };

        manager
            .store_credentials(ProviderId::OpenAI, &response, "openai@test.com")
            .unwrap();
        manager
            .store_credentials(ProviderId::Anthropic, &response, "anthropic@test.com")
            .unwrap();

        let providers = manager.authenticated_providers().unwrap();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&ProviderId::OpenAI));
        assert!(providers.contains(&ProviderId::Anthropic));
        assert!(!providers.contains(&ProviderId::Google));
    }

    #[tokio::test]
    async fn test_get_token_not_authenticated() {
        let dir = tempdir().unwrap();
        let manager = ProviderAuthManager::new(dir.path().to_path_buf());

        let result = manager.get_token(ProviderId::OpenAI).await;
        assert!(result.is_err());
        match result {
            Err(AuthError::NotAuthenticated) => {}
            _ => panic!("Expected NotAuthenticated error"),
        }
    }

    #[tokio::test]
    async fn test_get_token_fresh() {
        let dir = tempdir().unwrap();
        let manager = ProviderAuthManager::new(dir.path().to_path_buf());

        // Store fresh credentials (expires in 30 days - past OpenAI's 28-day refresh threshold)
        let mut storage = AuthAccountsStorage::load(dir.path()).unwrap();
        let credentials = ProviderCredentials {
            provider: ProviderId::OpenAI,
            access_token: "fresh-token".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::days(30)),
            metadata: serde_json::Value::Null,
        };
        storage.store_credentials(ProviderId::OpenAI, &credentials, "test@example.com");
        storage.save(dir.path()).unwrap();

        // Get token should return stored token without refresh
        let token = manager.get_token(ProviderId::OpenAI).await.unwrap();
        assert_eq!(token, "fresh-token");
    }
}
