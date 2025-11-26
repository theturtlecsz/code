//! Credential storage for multi-provider OAuth authentication.
//!
//! Implements the auth_accounts.json v2 schema with support for
//! multiple providers and active account tracking per provider.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AuthError, ProviderCredentials, ProviderId};

/// Storage file name for multi-provider credentials.
const AUTH_ACCOUNTS_FILE: &str = "auth_accounts.json";

/// Current schema version.
const SCHEMA_VERSION: u32 = 2;

/// Multi-provider credentials storage.
///
/// Implements the auth_accounts.json v2 schema with:
/// - Per-provider active account tracking
/// - Multiple accounts per provider
/// - Rich metadata storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthAccountsStorage {
    /// Schema version for migration support.
    pub version: u32,

    /// Active account ID per provider.
    #[serde(default)]
    pub active_accounts: HashMap<String, String>,

    /// All stored accounts.
    #[serde(default)]
    pub accounts: Vec<StoredAccount>,
}

impl Default for AuthAccountsStorage {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            active_accounts: HashMap::new(),
            accounts: Vec::new(),
        }
    }
}

/// A stored account with credentials and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAccount {
    /// Unique account identifier.
    pub id: String,

    /// Provider this account belongs to.
    pub provider: String,

    /// Authentication mode (oauth, api_key).
    pub mode: String,

    /// Display label (typically email).
    pub label: String,

    /// OAuth credentials.
    pub credentials: StoredCredentials,

    /// Provider-specific metadata.
    #[serde(default)]
    pub metadata: serde_json::Value,

    /// When the account was created.
    pub created_at: DateTime<Utc>,

    /// Last time the account was used.
    pub last_used_at: DateTime<Utc>,
}

/// Stored OAuth credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCredentials {
    /// OAuth access token.
    pub access_token: String,

    /// OAuth refresh token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    /// Token expiration time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl AuthAccountsStorage {
    /// Loads storage from the codex home directory.
    ///
    /// Creates a new empty storage if the file doesn't exist.
    pub fn load(codex_home: &Path) -> Result<Self, AuthError> {
        let file_path = codex_home.join(AUTH_ACCOUNTS_FILE);

        if !file_path.exists() {
            return Ok(Self::default());
        }

        let mut file = File::open(&file_path).map_err(AuthError::Io)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(AuthError::Io)?;

        let storage: Self = serde_json::from_str(&contents)?;
        Ok(storage)
    }

    /// Saves storage to the codex home directory.
    ///
    /// Uses secure file permissions (0o600) on Unix systems.
    pub fn save(&self, codex_home: &Path) -> Result<(), AuthError> {
        let file_path = codex_home.join(AUTH_ACCOUNTS_FILE);

        // Ensure directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(AuthError::Io)?;
        }

        // Open with secure permissions
        #[cfg(unix)]
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&file_path)
            .map_err(AuthError::Io)?;

        #[cfg(not(unix))]
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&file_path)
            .map_err(AuthError::Io)?;

        let json = serde_json::to_string_pretty(self)?;
        file.write_all(json.as_bytes()).map_err(AuthError::Io)?;

        Ok(())
    }

    /// Gets the active account for a provider.
    pub fn get_active_account(&self, provider: ProviderId) -> Option<&StoredAccount> {
        let provider_str = provider.as_str();
        let account_id = self.active_accounts.get(provider_str)?;

        self.accounts.iter().find(|a| &a.id == account_id)
    }

    /// Gets all accounts for a provider.
    pub fn get_accounts_for_provider(&self, provider: ProviderId) -> Vec<&StoredAccount> {
        let provider_str = provider.as_str();
        self.accounts
            .iter()
            .filter(|a| a.provider == provider_str)
            .collect()
    }

    /// Gets credentials for the active account of a provider.
    pub fn get_credentials(&self, provider: ProviderId) -> Option<ProviderCredentials> {
        let account = self.get_active_account(provider)?;

        Some(ProviderCredentials {
            provider,
            access_token: account.credentials.access_token.clone(),
            refresh_token: account.credentials.refresh_token.clone(),
            expires_at: account.credentials.expires_at,
            metadata: account.metadata.clone(),
        })
    }

    /// Stores or updates credentials for a provider.
    ///
    /// If the label (email) matches an existing account, updates it.
    /// Otherwise, creates a new account and sets it as active.
    pub fn store_credentials(
        &mut self,
        provider: ProviderId,
        credentials: &ProviderCredentials,
        label: &str,
    ) -> String {
        let provider_str = provider.as_str().to_string();
        let now = Utc::now();

        // Check for existing account with same label
        let existing_idx = self
            .accounts
            .iter()
            .position(|a| a.provider == provider_str && a.label == label);

        let account_id = if let Some(idx) = existing_idx {
            // Update existing account
            let account = &mut self.accounts[idx];
            account.credentials = StoredCredentials {
                access_token: credentials.access_token.clone(),
                refresh_token: credentials.refresh_token.clone(),
                expires_at: credentials.expires_at,
            };
            account.metadata = credentials.metadata.clone();
            account.last_used_at = now;
            account.id.clone()
        } else {
            // Create new account
            let account_id = Uuid::new_v4().to_string();
            let account = StoredAccount {
                id: account_id.clone(),
                provider: provider_str.clone(),
                mode: "oauth".to_string(),
                label: label.to_string(),
                credentials: StoredCredentials {
                    access_token: credentials.access_token.clone(),
                    refresh_token: credentials.refresh_token.clone(),
                    expires_at: credentials.expires_at,
                },
                metadata: credentials.metadata.clone(),
                created_at: now,
                last_used_at: now,
            };
            self.accounts.push(account);
            account_id
        };

        // Set as active account for this provider
        self.active_accounts
            .insert(provider_str, account_id.clone());

        account_id
    }

    /// Updates the access token for a provider's active account.
    ///
    /// Used after token refresh.
    pub fn update_token(
        &mut self,
        provider: ProviderId,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), AuthError> {
        let provider_str = provider.as_str();
        let account_id = self
            .active_accounts
            .get(provider_str)
            .ok_or(AuthError::NotAuthenticated)?
            .clone();

        let account = self
            .accounts
            .iter_mut()
            .find(|a| a.id == account_id)
            .ok_or(AuthError::NotAuthenticated)?;

        account.credentials.access_token = access_token.to_string();
        if let Some(rt) = refresh_token {
            account.credentials.refresh_token = Some(rt.to_string());
        }
        account.credentials.expires_at = expires_at;
        account.last_used_at = Utc::now();

        Ok(())
    }

    /// Removes an account by ID.
    pub fn remove_account(&mut self, account_id: &str) {
        // Remove from accounts
        self.accounts.retain(|a| a.id != account_id);

        // Remove from active accounts if it was active
        self.active_accounts.retain(|_, id| id != account_id);
    }

    /// Sets the active account for a provider.
    pub fn set_active_account(
        &mut self,
        provider: ProviderId,
        account_id: &str,
    ) -> Result<(), AuthError> {
        let provider_str = provider.as_str();

        // Verify account exists and belongs to this provider
        let account_exists = self
            .accounts
            .iter()
            .any(|a| a.id == account_id && a.provider == provider_str);

        if !account_exists {
            return Err(AuthError::Config(format!(
                "Account {account_id} not found for provider {provider}"
            )));
        }

        self.active_accounts
            .insert(provider_str.to_string(), account_id.to_string());
        Ok(())
    }

    /// Returns the storage file path for a codex home directory.
    pub fn file_path(codex_home: &Path) -> PathBuf {
        codex_home.join(AUTH_ACCOUNTS_FILE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_storage() {
        let storage = AuthAccountsStorage::default();
        assert_eq!(storage.version, SCHEMA_VERSION);
        assert!(storage.active_accounts.is_empty());
        assert!(storage.accounts.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let mut storage = AuthAccountsStorage::default();

        let credentials = ProviderCredentials {
            provider: ProviderId::OpenAI,
            access_token: "test-token".to_string(),
            refresh_token: Some("test-refresh".to_string()),
            expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
            metadata: serde_json::json!({"email": "test@example.com"}),
        };

        storage.store_credentials(ProviderId::OpenAI, &credentials, "test@example.com");
        storage.save(dir.path()).unwrap();

        let loaded = AuthAccountsStorage::load(dir.path()).unwrap();
        assert_eq!(loaded.version, SCHEMA_VERSION);
        assert_eq!(loaded.accounts.len(), 1);
        assert!(loaded.active_accounts.contains_key("openai"));
    }

    #[test]
    fn test_load_missing_file() {
        let dir = tempdir().unwrap();
        let storage = AuthAccountsStorage::load(dir.path()).unwrap();
        assert_eq!(storage.version, SCHEMA_VERSION);
        assert!(storage.accounts.is_empty());
    }

    #[test]
    fn test_store_credentials() {
        let mut storage = AuthAccountsStorage::default();

        let credentials = ProviderCredentials {
            provider: ProviderId::Anthropic,
            access_token: "sk-ant-oat01-test".to_string(),
            refresh_token: Some("sk-ant-ort01-test".to_string()),
            expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
            metadata: serde_json::json!({"provider": "anthropic"}),
        };

        let account_id =
            storage.store_credentials(ProviderId::Anthropic, &credentials, "user@example.com");

        assert!(!account_id.is_empty());
        assert_eq!(storage.accounts.len(), 1);
        assert_eq!(storage.active_accounts.get("anthropic"), Some(&account_id));

        let account = &storage.accounts[0];
        assert_eq!(account.provider, "anthropic");
        assert_eq!(account.label, "user@example.com");
        assert_eq!(account.credentials.access_token, "sk-ant-oat01-test");
    }

    #[test]
    fn test_update_existing_account() {
        let mut storage = AuthAccountsStorage::default();

        let credentials1 = ProviderCredentials {
            provider: ProviderId::Google,
            access_token: "token1".to_string(),
            refresh_token: Some("refresh1".to_string()),
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        let id1 = storage.store_credentials(ProviderId::Google, &credentials1, "user@gmail.com");

        let credentials2 = ProviderCredentials {
            provider: ProviderId::Google,
            access_token: "token2".to_string(),
            refresh_token: Some("refresh2".to_string()),
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        let id2 = storage.store_credentials(ProviderId::Google, &credentials2, "user@gmail.com");

        // Should update existing, not create new
        assert_eq!(id1, id2);
        assert_eq!(storage.accounts.len(), 1);
        assert_eq!(storage.accounts[0].credentials.access_token, "token2");
    }

    #[test]
    fn test_get_credentials() {
        let mut storage = AuthAccountsStorage::default();

        let credentials = ProviderCredentials {
            provider: ProviderId::OpenAI,
            access_token: "test-token".to_string(),
            refresh_token: Some("test-refresh".to_string()),
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        storage.store_credentials(ProviderId::OpenAI, &credentials, "test@example.com");

        let retrieved = storage.get_credentials(ProviderId::OpenAI);
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.access_token, "test-token");
        assert_eq!(retrieved.refresh_token, Some("test-refresh".to_string()));
    }

    #[test]
    fn test_get_credentials_not_found() {
        let storage = AuthAccountsStorage::default();
        let credentials = storage.get_credentials(ProviderId::Anthropic);
        assert!(credentials.is_none());
    }

    #[test]
    fn test_update_token() {
        let mut storage = AuthAccountsStorage::default();

        let credentials = ProviderCredentials {
            provider: ProviderId::OpenAI,
            access_token: "old-token".to_string(),
            refresh_token: Some("old-refresh".to_string()),
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        storage.store_credentials(ProviderId::OpenAI, &credentials, "test@example.com");

        storage
            .update_token(
                ProviderId::OpenAI,
                "new-token",
                Some("new-refresh"),
                Some(Utc::now() + chrono::Duration::hours(2)),
            )
            .unwrap();

        let retrieved = storage.get_credentials(ProviderId::OpenAI).unwrap();
        assert_eq!(retrieved.access_token, "new-token");
        assert_eq!(retrieved.refresh_token, Some("new-refresh".to_string()));
        assert!(retrieved.expires_at.is_some());
    }

    #[test]
    fn test_multiple_providers() {
        let mut storage = AuthAccountsStorage::default();

        let openai_creds = ProviderCredentials {
            provider: ProviderId::OpenAI,
            access_token: "openai-token".to_string(),
            refresh_token: None,
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        let anthropic_creds = ProviderCredentials {
            provider: ProviderId::Anthropic,
            access_token: "anthropic-token".to_string(),
            refresh_token: None,
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        let google_creds = ProviderCredentials {
            provider: ProviderId::Google,
            access_token: "google-token".to_string(),
            refresh_token: None,
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        storage.store_credentials(ProviderId::OpenAI, &openai_creds, "openai@test.com");
        storage.store_credentials(
            ProviderId::Anthropic,
            &anthropic_creds,
            "anthropic@test.com",
        );
        storage.store_credentials(ProviderId::Google, &google_creds, "google@test.com");

        assert_eq!(storage.accounts.len(), 3);
        assert_eq!(storage.active_accounts.len(), 3);

        let openai = storage.get_credentials(ProviderId::OpenAI).unwrap();
        let anthropic = storage.get_credentials(ProviderId::Anthropic).unwrap();
        let google = storage.get_credentials(ProviderId::Google).unwrap();

        assert_eq!(openai.access_token, "openai-token");
        assert_eq!(anthropic.access_token, "anthropic-token");
        assert_eq!(google.access_token, "google-token");
    }

    #[test]
    fn test_remove_account() {
        let mut storage = AuthAccountsStorage::default();

        let credentials = ProviderCredentials {
            provider: ProviderId::OpenAI,
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        let id = storage.store_credentials(ProviderId::OpenAI, &credentials, "test@example.com");

        storage.remove_account(&id);

        assert!(storage.accounts.is_empty());
        assert!(storage.active_accounts.is_empty());
    }

    #[test]
    fn test_set_active_account() {
        let mut storage = AuthAccountsStorage::default();

        let creds1 = ProviderCredentials {
            provider: ProviderId::OpenAI,
            access_token: "token1".to_string(),
            refresh_token: None,
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        let creds2 = ProviderCredentials {
            provider: ProviderId::OpenAI,
            access_token: "token2".to_string(),
            refresh_token: None,
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        let id1 = storage.store_credentials(ProviderId::OpenAI, &creds1, "user1@test.com");
        let id2 = storage.store_credentials(ProviderId::OpenAI, &creds2, "user2@test.com");

        // id2 should be active now
        assert_eq!(storage.active_accounts.get("openai"), Some(&id2));

        // Switch back to id1
        storage
            .set_active_account(ProviderId::OpenAI, &id1)
            .unwrap();
        assert_eq!(storage.active_accounts.get("openai"), Some(&id1));
    }

    #[test]
    fn test_set_active_account_invalid() {
        let mut storage = AuthAccountsStorage::default();

        let result = storage.set_active_account(ProviderId::OpenAI, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_accounts_for_provider() {
        let mut storage = AuthAccountsStorage::default();

        let creds1 = ProviderCredentials {
            provider: ProviderId::OpenAI,
            access_token: "t1".to_string(),
            refresh_token: None,
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        let creds2 = ProviderCredentials {
            provider: ProviderId::OpenAI,
            access_token: "t2".to_string(),
            refresh_token: None,
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        let creds3 = ProviderCredentials {
            provider: ProviderId::Anthropic,
            access_token: "t3".to_string(),
            refresh_token: None,
            expires_at: None,
            metadata: serde_json::Value::Null,
        };

        storage.store_credentials(ProviderId::OpenAI, &creds1, "user1@test.com");
        storage.store_credentials(ProviderId::OpenAI, &creds2, "user2@test.com");
        storage.store_credentials(ProviderId::Anthropic, &creds3, "user3@test.com");

        let openai_accounts = storage.get_accounts_for_provider(ProviderId::OpenAI);
        let anthropic_accounts = storage.get_accounts_for_provider(ProviderId::Anthropic);

        assert_eq!(openai_accounts.len(), 2);
        assert_eq!(anthropic_accounts.len(), 1);
    }

    #[test]
    fn test_serialization_format() {
        let mut storage = AuthAccountsStorage::default();

        let credentials = ProviderCredentials {
            provider: ProviderId::OpenAI,
            access_token: "test-token".to_string(),
            refresh_token: Some("test-refresh".to_string()),
            expires_at: Some(Utc::now()),
            metadata: serde_json::json!({"email": "test@example.com", "plan_type": "pro"}),
        };

        storage.store_credentials(ProviderId::OpenAI, &credentials, "test@example.com");

        let json = serde_json::to_string_pretty(&storage).unwrap();

        // Verify JSON structure
        assert!(json.contains("\"version\": 2"));
        assert!(json.contains("\"active_accounts\""));
        assert!(json.contains("\"openai\""));
        assert!(json.contains("\"accounts\""));
        assert!(json.contains("\"credentials\""));
        assert!(json.contains("\"access_token\""));
    }
}
