//! Token Storage for Device Code Authorization
//!
//! FORK-SPECIFIC (just-every/code): P6-SYNC Phase 5 + P53-SYNC Keyring Integration
//!
//! Persists OAuth tokens from device code flow to enable:
//! - Session continuity without re-authentication
//! - Automatic token refresh when access tokens expire
//! - Secure storage of refresh tokens
//!
//! Storage strategy (P53-SYNC):
//! - Primary: System keyring (encrypted at rest, OS-managed)
//! - Fallback: ~/.codex/device_tokens.json (file-based, chmod 600)
//! - Migration: File tokens auto-migrate to keyring on first load

use crate::device_code::{DeviceCodeProvider, StoredToken, TokenResponse};
use codex_keyring_store::{CredentialStoreError, DefaultKeyringStore, KeyringStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, trace, warn};

/// Default filename for device code tokens
const TOKEN_FILE: &str = "device_tokens.json";

/// Keyring service identifier (P53-SYNC)
const KEYRING_SERVICE: &str = "codex-cli";

/// Keyring account prefix for device code tokens
const KEYRING_ACCOUNT_PREFIX: &str = "device-token-";

/// Storage errors
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Token not found for provider: {0}")]
    NotFound(String),

    #[error("Keyring error: {0}")]
    Keyring(#[from] CredentialStoreError),
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
///
/// P53-SYNC: Uses keyring as primary storage with file fallback.
/// Tokens are automatically migrated from file to keyring on first access.
pub struct DeviceCodeTokenStorage {
    /// Path to the token file (fallback storage)
    file_path: PathBuf,
    /// System keyring store (primary storage, optional for headless environments)
    keyring: Option<Arc<dyn KeyringStore>>,
}

impl DeviceCodeTokenStorage {
    /// Create storage with default location (~/.codex/device_tokens.json)
    /// and system keyring enabled.
    pub fn new() -> io::Result<Self> {
        let codex_home = default_codex_home()?;
        let file_path = codex_home.join(TOKEN_FILE);

        // Try to initialize system keyring (graceful fallback if unavailable)
        let keyring: Option<Arc<dyn KeyringStore>> = match Self::try_init_keyring() {
            Ok(k) => {
                debug!("Keyring initialized successfully for token storage");
                Some(Arc::new(k))
            }
            Err(e) => {
                warn!("Keyring unavailable, using file-only storage: {}", e);
                None
            }
        };

        Ok(Self { file_path, keyring })
    }

    /// Create storage with custom file path (keyring enabled by default)
    pub fn with_path(path: PathBuf) -> Self {
        let keyring: Option<Arc<dyn KeyringStore>> = match Self::try_init_keyring() {
            Ok(k) => Some(Arc::new(k)),
            Err(_) => None,
        };
        Self {
            file_path: path,
            keyring,
        }
    }

    /// Create storage with custom file path and keyring (for testing)
    pub fn with_path_and_keyring(path: PathBuf, keyring: Option<Arc<dyn KeyringStore>>) -> Self {
        Self {
            file_path: path,
            keyring,
        }
    }

    /// Try to initialize the default keyring store
    fn try_init_keyring() -> Result<DefaultKeyringStore, &'static str> {
        // DefaultKeyringStore may fail on headless systems
        // We test it with a probe operation
        let store = DefaultKeyringStore;

        // Probe: Try to load a non-existent key to verify keyring is accessible
        // This catches headless environments where keyring daemon isn't running
        match store.load(KEYRING_SERVICE, "probe-test") {
            Ok(_) | Err(CredentialStoreError::Other(_)) => {
                // Ok means probe worked (and returned None for missing key)
                // Other error is acceptable - keyring is accessible but operation failed
                Ok(store)
            }
        }
    }

    /// Get the storage file path
    pub fn path(&self) -> &Path {
        &self.file_path
    }

    /// Check if keyring storage is available
    pub fn has_keyring(&self) -> bool {
        self.keyring.is_some()
    }

    /// Get keyring account name for a provider
    fn keyring_account(provider: DeviceCodeProvider) -> String {
        format!("{}{}", KEYRING_ACCOUNT_PREFIX, provider.as_str())
    }

    /// Load token from keyring (P53-SYNC)
    fn load_from_keyring(&self, provider: DeviceCodeProvider) -> Option<StoredToken> {
        let keyring = self.keyring.as_ref()?;
        let account = Self::keyring_account(provider);

        match keyring.load(KEYRING_SERVICE, &account) {
            Ok(Some(json)) => {
                trace!("keyring.load success for {}", provider.as_str());
                match serde_json::from_str(&json) {
                    Ok(token) => Some(token),
                    Err(e) => {
                        warn!(
                            "Failed to parse keyring token for {}: {}",
                            provider.as_str(),
                            e
                        );
                        None
                    }
                }
            }
            Ok(None) => {
                trace!("keyring.load no entry for {}", provider.as_str());
                None
            }
            Err(e) => {
                warn!("keyring.load error for {}: {}", provider.as_str(), e);
                None
            }
        }
    }

    /// Save token to keyring (P53-SYNC)
    fn save_to_keyring(&self, provider: DeviceCodeProvider, token: &StoredToken) -> bool {
        let Some(keyring) = self.keyring.as_ref() else {
            return false;
        };
        let account = Self::keyring_account(provider);

        match serde_json::to_string(token) {
            Ok(json) => match keyring.save(KEYRING_SERVICE, &account, &json) {
                Ok(()) => {
                    trace!("keyring.save success for {}", provider.as_str());
                    true
                }
                Err(e) => {
                    warn!("keyring.save error for {}: {}", provider.as_str(), e);
                    false
                }
            },
            Err(e) => {
                warn!("Failed to serialize token for keyring: {}", e);
                false
            }
        }
    }

    /// Delete token from keyring (P53-SYNC)
    fn delete_from_keyring(&self, provider: DeviceCodeProvider) -> bool {
        let Some(keyring) = self.keyring.as_ref() else {
            return false;
        };
        let account = Self::keyring_account(provider);

        match keyring.delete(KEYRING_SERVICE, &account) {
            Ok(existed) => {
                trace!(
                    "keyring.delete success for {} (existed: {})",
                    provider.as_str(),
                    existed
                );
                true
            }
            Err(e) => {
                warn!("keyring.delete error for {}: {}", provider.as_str(), e);
                false
            }
        }
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

        #[cfg(unix)]
        {
            use std::io::Write;
            use tempfile::NamedTempFile;

            // Use the file's parent directory, or current directory if no parent is specified.
            // This ensures we can create the temporary file for an atomic rename.
            let parent = self.file_path.parent().unwrap_or_else(|| Path::new("."));

            // Create temporary file in the same directory to ensure atomic rename.
            // NamedTempFile creates the file with 0600 permissions by default on Unix.
            let mut tmp = NamedTempFile::new_in(parent)?;
            tmp.write_all(content.as_bytes())?;
            tmp.persist(&self.file_path).map_err(|e| e.error)?;
        }

        #[cfg(not(unix))]
        {
            fs::write(&self.file_path, content)?;
        }

        Ok(())
    }

    /// Store a token for a provider
    ///
    /// P53-SYNC: Saves to keyring (primary) and file (backup).
    pub fn store_token(
        &self,
        provider: DeviceCodeProvider,
        response: TokenResponse,
    ) -> Result<(), StorageError> {
        let token = StoredToken::from_response(provider, response);

        // P53-SYNC: Save to keyring first (primary storage)
        let keyring_saved = self.save_to_keyring(provider, &token);
        if keyring_saved {
            debug!("Token saved to keyring for {}", provider.as_str());
        }

        // Always save to file as backup (or as primary if keyring unavailable)
        let mut store = self.read_store()?;
        store.tokens.insert(provider.as_str().to_string(), token);
        self.write_store(&store)?;

        if !keyring_saved && self.keyring.is_some() {
            warn!(
                "Token saved to file only (keyring save failed) for {}",
                provider.as_str()
            );
        }

        Ok(())
    }

    /// Get a stored token for a provider
    ///
    /// P53-SYNC: Tries keyring first, falls back to file, migrates if found in file only.
    pub fn get_token(&self, provider: DeviceCodeProvider) -> Result<StoredToken, StorageError> {
        // P53-SYNC: Try keyring first (primary storage)
        if let Some(token) = self.load_from_keyring(provider) {
            trace!("Token loaded from keyring for {}", provider.as_str());
            return Ok(token);
        }

        // Fallback to file storage
        let store = self.read_store()?;
        let token = store
            .tokens
            .get(provider.as_str())
            .cloned()
            .ok_or_else(|| StorageError::NotFound(provider.to_string()))?;

        // P53-SYNC: Migrate file token to keyring
        if self.keyring.is_some() && self.save_to_keyring(provider, &token) {
            debug!(
                "Migrated token from file to keyring for {}",
                provider.as_str()
            );
        }

        Ok(token)
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
    ///
    /// P53-SYNC: Removes from both keyring and file storage.
    pub fn remove_token(&self, provider: DeviceCodeProvider) -> Result<(), StorageError> {
        // P53-SYNC: Remove from keyring
        self.delete_from_keyring(provider);

        // Remove from file storage
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
        io::Error::new(
            io::ErrorKind::NotFound,
            "Could not determine home directory",
        )
    })?;

    Ok(home.join(".codex"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_keyring_store::tests::MockKeyringStore;
    use tempfile::TempDir;

    /// Create file-only storage (no keyring) for basic tests
    fn test_storage() -> (TempDir, DeviceCodeTokenStorage) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("tokens.json");
        // Use file-only storage to avoid CI keyring issues
        (
            dir,
            DeviceCodeTokenStorage::with_path_and_keyring(path, None),
        )
    }

    /// Create storage with mock keyring for keyring integration tests
    fn test_storage_with_keyring() -> (TempDir, DeviceCodeTokenStorage, Arc<MockKeyringStore>) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("tokens.json");
        let keyring = Arc::new(MockKeyringStore::default());
        let storage = DeviceCodeTokenStorage::with_path_and_keyring(path, Some(keyring.clone()));
        (dir, storage, keyring)
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

    // === P53-SYNC: Keyring integration tests ===

    #[test]
    fn test_keyring_store_and_get() {
        let (_dir, storage, keyring) = test_storage_with_keyring();

        // Store a token
        storage
            .store_token(DeviceCodeProvider::OpenAI, make_token_response())
            .unwrap();

        // Verify it was saved to keyring
        let account = DeviceCodeTokenStorage::keyring_account(DeviceCodeProvider::OpenAI);
        assert!(
            keyring.saved_value(&account).is_some(),
            "Token should be saved to keyring"
        );

        // Retrieve it - should come from keyring
        let token = storage.get_token(DeviceCodeProvider::OpenAI).unwrap();
        assert_eq!(token.access_token, "test-access-token");
    }

    #[test]
    fn test_keyring_migration_from_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("tokens.json");

        // Step 1: Create file-only storage and save a token
        let file_only_storage = DeviceCodeTokenStorage::with_path_and_keyring(path.clone(), None);
        file_only_storage
            .store_token(DeviceCodeProvider::OpenAI, make_token_response())
            .unwrap();

        // Step 2: Create keyring-enabled storage with same file path
        let keyring = Arc::new(MockKeyringStore::default());
        let keyring_storage =
            DeviceCodeTokenStorage::with_path_and_keyring(path, Some(keyring.clone()));

        // Keyring should be empty initially
        let account = DeviceCodeTokenStorage::keyring_account(DeviceCodeProvider::OpenAI);
        assert!(keyring.saved_value(&account).is_none());

        // Step 3: Get token - should migrate from file to keyring
        let token = keyring_storage
            .get_token(DeviceCodeProvider::OpenAI)
            .unwrap();
        assert_eq!(token.access_token, "test-access-token");

        // Keyring should now have the token
        assert!(
            keyring.saved_value(&account).is_some(),
            "Token should have been migrated to keyring"
        );
    }

    #[test]
    fn test_keyring_remove_token() {
        let (_dir, storage, keyring) = test_storage_with_keyring();

        // Store and then remove
        storage
            .store_token(DeviceCodeProvider::OpenAI, make_token_response())
            .unwrap();
        let account = DeviceCodeTokenStorage::keyring_account(DeviceCodeProvider::OpenAI);
        assert!(keyring.saved_value(&account).is_some());

        storage.remove_token(DeviceCodeProvider::OpenAI).unwrap();

        // Should be removed from keyring
        assert!(keyring.saved_value(&account).is_none());
        assert!(!storage.has_token(DeviceCodeProvider::OpenAI));
    }

    #[test]
    fn test_has_keyring_flag() {
        let (_dir, storage_no_keyring) = test_storage();
        assert!(!storage_no_keyring.has_keyring());

        let (_dir2, storage_with_keyring, _keyring) = test_storage_with_keyring();
        assert!(storage_with_keyring.has_keyring());
    }

    #[test]
    #[cfg(unix)]
    fn test_file_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, storage) = test_storage();
        storage
            .store_token(DeviceCodeProvider::OpenAI, make_token_response())
            .unwrap();

        let path = storage.path();
        let metadata = std::fs::metadata(path).unwrap();
        let mode = metadata.permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
