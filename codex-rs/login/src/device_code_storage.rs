//! Token Storage for Device Code Authorization
//!
//! FORK-SPECIFIC (just-every/code): P6-SYNC Phase 5
//!
//! Persists OAuth tokens from device code flow to enable:
//! - Session continuity without re-authentication
//! - Automatic token refresh when access tokens expire
//! - Secure storage of refresh tokens
//!
//! Storage location: ~/.codex/device_tokens.json (alongside auth.json)

use crate::device_code::{DeviceCodeProvider, StoredToken, TokenResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Default filename for device code tokens
const TOKEN_FILE: &str = "device_tokens.json";

/// Storage errors
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Token not found for provider: {0}")]
    NotFound(String),
}

/// Token storage file format
#[derive(Debug, Default, Serialize, Deserialize)]
struct TokenStore {
    /// Map of provider name to stored token
    tokens: HashMap<String, StoredToken>,
    /// Schema version for migration
    #[serde(default = "default_version")]
    version: u32,
}

fn default_version() -> u32 {
    1
}

/// Device code token storage manager
pub struct DeviceCodeTokenStorage {
    /// Path to the token file
    file_path: PathBuf,
}

impl DeviceCodeTokenStorage {
    /// Create storage with default location (~/.codex/device_tokens.json)
    pub fn new() -> io::Result<Self> {
        let codex_home = default_codex_home()?;
        Ok(Self::with_path(codex_home.join(TOKEN_FILE)))
    }

    /// Create storage with custom file path
    pub fn with_path(path: PathBuf) -> Self {
        Self { file_path: path }
    }

    /// Get the storage file path
    pub fn path(&self) -> &Path {
        &self.file_path
    }

    /// Read all stored tokens
    fn read_store(&self) -> Result<TokenStore, StorageError> {
        if !self.file_path.exists() {
            return Ok(TokenStore::default());
        }

        let content = fs::read_to_string(&self.file_path)?;
        let store: TokenStore = serde_json::from_str(&content)?;
        Ok(store)
    }

    /// Write token store to disk
    fn write_store(&self, store: &TokenStore) -> Result<(), StorageError> {
        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(store)?;
        fs::write(&self.file_path, content)?;

        // Set file permissions to user-only (Unix)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&self.file_path, permissions)?;
        }

        Ok(())
    }

    /// Store a token for a provider
    pub fn store_token(
        &self,
        provider: DeviceCodeProvider,
        response: TokenResponse,
    ) -> Result<(), StorageError> {
        let mut store = self.read_store()?;
        let token = StoredToken::from_response(provider, response);
        store.tokens.insert(provider.as_str().to_string(), token);
        self.write_store(&store)
    }

    /// Get a stored token for a provider
    pub fn get_token(&self, provider: DeviceCodeProvider) -> Result<StoredToken, StorageError> {
        let store = self.read_store()?;
        store
            .tokens
            .get(provider.as_str())
            .cloned()
            .ok_or_else(|| StorageError::NotFound(provider.to_string()))
    }

    /// Check if a token exists for a provider
    pub fn has_token(&self, provider: DeviceCodeProvider) -> bool {
        self.get_token(provider).is_ok()
    }

    /// Check if a provider's token needs refresh
    pub fn needs_refresh(&self, provider: DeviceCodeProvider) -> bool {
        match self.get_token(provider) {
            Ok(token) => token.is_expired(),
            Err(_) => false, // No token = no refresh needed
        }
    }

    /// Update the access token after a refresh
    pub fn update_access_token(
        &self,
        provider: DeviceCodeProvider,
        response: TokenResponse,
    ) -> Result<(), StorageError> {
        let mut store = self.read_store()?;
        let key = provider.as_str().to_string();

        if let Some(existing) = store.tokens.get_mut(&key) {
            let now = chrono::Utc::now().timestamp();
            existing.access_token = response.access_token;
            existing.expires_at = response.expires_in.map(|secs| now + secs as i64);

            // Update refresh token if a new one was issued
            if let Some(new_refresh) = response.refresh_token {
                existing.refresh_token = Some(new_refresh);
            }

            existing.stored_at = now;
            self.write_store(&store)
        } else {
            // No existing token, store as new
            self.store_token(provider, response)
        }
    }

    /// Remove a token for a provider
    pub fn remove_token(&self, provider: DeviceCodeProvider) -> Result<(), StorageError> {
        let mut store = self.read_store()?;
        store.tokens.remove(provider.as_str());
        self.write_store(&store)
    }

    /// List all providers with stored tokens
    pub fn list_providers(&self) -> Result<Vec<DeviceCodeProvider>, StorageError> {
        let store = self.read_store()?;
        Ok(store
            .tokens
            .keys()
            .filter_map(|k| match k.as_str() {
                "openai" => Some(DeviceCodeProvider::OpenAI),
                "google" => Some(DeviceCodeProvider::Google),
                "anthropic" => Some(DeviceCodeProvider::Anthropic),
                _ => None,
            })
            .collect())
    }

    /// Get status summary for all providers
    pub fn status_summary(&self) -> Result<Vec<(DeviceCodeProvider, TokenStatus)>, StorageError> {
        let store = self.read_store()?;
        let mut results = Vec::new();

        for provider in [
            DeviceCodeProvider::OpenAI,
            DeviceCodeProvider::Google,
            DeviceCodeProvider::Anthropic,
        ] {
            let status = match store.tokens.get(provider.as_str()) {
                Some(token) => {
                    if token.is_expired() {
                        if token.can_refresh() {
                            TokenStatus::NeedsRefresh
                        } else {
                            TokenStatus::Expired
                        }
                    } else {
                        TokenStatus::Valid
                    }
                }
                None => TokenStatus::NotAuthenticated,
            };
            results.push((provider, status));
        }

        Ok(results)
    }
}

impl Default for DeviceCodeTokenStorage {
    /// Note: Panics if home directory cannot be determined.
    /// Use `DeviceCodeTokenStorage::new()` for fallible construction.
    #[allow(clippy::expect_used)]
    fn default() -> Self {
        Self::new().expect("Failed to create token storage with default path")
    }
}

/// Token status for UI display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenStatus {
    /// No token stored
    NotAuthenticated,
    /// Token is valid
    Valid,
    /// Token expired but can be refreshed
    NeedsRefresh,
    /// Token expired and cannot be refreshed
    Expired,
}

impl TokenStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotAuthenticated => "not authenticated",
            Self::Valid => "authenticated",
            Self::NeedsRefresh => "needs refresh",
            Self::Expired => "expired",
        }
    }

    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Valid | Self::NeedsRefresh)
    }
}

impl std::fmt::Display for TokenStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Get default codex home directory
fn default_codex_home() -> io::Result<PathBuf> {
    // Check CODEX_HOME env first
    if let Ok(home) = std::env::var("CODEX_HOME") {
        return Ok(PathBuf::from(home));
    }

    // Fall back to ~/.codex
    let home = dirs::home_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "Could not determine home directory")
    })?;

    Ok(home.join(".codex"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_storage() -> (TempDir, DeviceCodeTokenStorage) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("tokens.json");
        (dir, DeviceCodeTokenStorage::with_path(path))
    }

    fn make_token_response() -> TokenResponse {
        TokenResponse {
            access_token: "test-access-token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            refresh_token: Some("test-refresh-token".to_string()),
            scope: Some("openid profile".to_string()),
            id_token: None,
        }
    }

    #[test]
    fn test_store_and_get_token() {
        let (_dir, storage) = test_storage();

        // Store a token
        storage
            .store_token(DeviceCodeProvider::OpenAI, make_token_response())
            .unwrap();

        // Retrieve it
        let token = storage.get_token(DeviceCodeProvider::OpenAI).unwrap();
        assert_eq!(token.access_token, "test-access-token");
        assert_eq!(token.refresh_token, Some("test-refresh-token".to_string()));
        assert!(!token.is_expired());
    }

    #[test]
    fn test_has_token() {
        let (_dir, storage) = test_storage();

        assert!(!storage.has_token(DeviceCodeProvider::OpenAI));

        storage
            .store_token(DeviceCodeProvider::OpenAI, make_token_response())
            .unwrap();

        assert!(storage.has_token(DeviceCodeProvider::OpenAI));
        assert!(!storage.has_token(DeviceCodeProvider::Google));
    }

    #[test]
    fn test_remove_token() {
        let (_dir, storage) = test_storage();

        storage
            .store_token(DeviceCodeProvider::OpenAI, make_token_response())
            .unwrap();
        assert!(storage.has_token(DeviceCodeProvider::OpenAI));

        storage.remove_token(DeviceCodeProvider::OpenAI).unwrap();
        assert!(!storage.has_token(DeviceCodeProvider::OpenAI));
    }

    #[test]
    fn test_list_providers() {
        let (_dir, storage) = test_storage();

        storage
            .store_token(DeviceCodeProvider::OpenAI, make_token_response())
            .unwrap();
        storage
            .store_token(DeviceCodeProvider::Google, make_token_response())
            .unwrap();

        let providers = storage.list_providers().unwrap();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&DeviceCodeProvider::OpenAI));
        assert!(providers.contains(&DeviceCodeProvider::Google));
    }

    #[test]
    fn test_status_summary() {
        let (_dir, storage) = test_storage();

        storage
            .store_token(DeviceCodeProvider::OpenAI, make_token_response())
            .unwrap();

        let summary = storage.status_summary().unwrap();
        assert_eq!(summary.len(), 3); // OpenAI, Google, Anthropic

        let openai_status = summary
            .iter()
            .find(|(p, _)| *p == DeviceCodeProvider::OpenAI)
            .map(|(_, s)| *s)
            .unwrap();
        assert_eq!(openai_status, TokenStatus::Valid);

        let google_status = summary
            .iter()
            .find(|(p, _)| *p == DeviceCodeProvider::Google)
            .map(|(_, s)| *s)
            .unwrap();
        assert_eq!(google_status, TokenStatus::NotAuthenticated);

        let anthropic_status = summary
            .iter()
            .find(|(p, _)| *p == DeviceCodeProvider::Anthropic)
            .map(|(_, s)| *s)
            .unwrap();
        assert_eq!(anthropic_status, TokenStatus::NotAuthenticated);
    }

    #[test]
    fn test_update_access_token() {
        let (_dir, storage) = test_storage();

        storage
            .store_token(DeviceCodeProvider::OpenAI, make_token_response())
            .unwrap();

        let new_response = TokenResponse {
            access_token: "new-access-token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: Some(7200),
            refresh_token: None, // No new refresh token
            scope: None,
            id_token: None,
        };

        storage
            .update_access_token(DeviceCodeProvider::OpenAI, new_response)
            .unwrap();

        let token = storage.get_token(DeviceCodeProvider::OpenAI).unwrap();
        assert_eq!(token.access_token, "new-access-token");
        // Original refresh token should be preserved
        assert_eq!(token.refresh_token, Some("test-refresh-token".to_string()));
    }

    #[test]
    fn test_token_status_display() {
        assert_eq!(TokenStatus::Valid.as_str(), "authenticated");
        assert_eq!(TokenStatus::NotAuthenticated.as_str(), "not authenticated");
        assert!(TokenStatus::Valid.is_usable());
        assert!(TokenStatus::NeedsRefresh.is_usable());
        assert!(!TokenStatus::Expired.is_usable());
        assert!(!TokenStatus::NotAuthenticated.is_usable());
    }
}
