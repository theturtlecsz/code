# SPEC-KIT-953-D: Provider Authentication Framework

**Status**: Draft
**Created**: 2025-11-19
**Type**: Implementation SPEC
**Priority**: High
**Estimated Effort**: 30-40 hours
**Dependencies**: SPEC-953-A, SPEC-953-B, SPEC-953-C (Research SPECs - Complete)

---

## Executive Summary

Implement a unified OAuth 2.0 PKCE authentication framework that supports multiple providers (OpenAI, Anthropic, Google) through a shared trait abstraction. This enables native integration of Claude and Gemini providers while maintaining backward compatibility with existing ChatGPT/OpenAI authentication.

---

## Problem Statement

Current `codex-rs/core/src/auth.rs` is hardcoded for OpenAI:
- Token refresh URL: `https://auth.openai.com/oauth/token` (line 426)
- Client ID: `app_EMoamEEZ73f0CkXaXp7hrann` (line 476)
- `AuthMode` enum only has `ApiKey` and `ChatGPT` variants

This architecture cannot support Anthropic or Google OAuth without significant refactoring.

---

## Solution: Provider Authentication Trait

### Core Abstraction

```rust
// codex-rs/core/src/provider_auth.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Provider identifier for routing and storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderId {
    OpenAI,
    Anthropic,
    Google,
}

/// OAuth configuration for a provider
pub struct OAuthConfig {
    pub client_id: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub use_pkce: bool,
}

/// Token response from OAuth exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub token_type: String,
    pub scope: Option<String>,
    /// Provider-specific additional fields (id_token for OpenAI, etc.)
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Provider-specific stored credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCredentials {
    pub provider: ProviderId,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Provider-specific data (email, account_id, plan_type, etc.)
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Main trait for provider authentication
#[async_trait]
pub trait ProviderAuth: Send + Sync {
    /// Provider identifier
    fn provider_id(&self) -> ProviderId;

    /// Display name for UI
    fn display_name(&self) -> &'static str;

    /// OAuth configuration
    fn oauth_config(&self) -> OAuthConfig;

    /// Generate authorization URL with PKCE
    fn authorization_url(&self, state: &str, code_verifier: &str) -> String;

    /// Exchange authorization code for tokens
    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<TokenResponse, AuthError>;

    /// Refresh an expired access token
    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<TokenResponse, AuthError>;

    /// Extract user metadata from token response (email, account_id, etc.)
    fn extract_metadata(&self, response: &TokenResponse) -> serde_json::Value;

    /// Check if token needs refresh (provider-specific expiry logic)
    fn needs_refresh(&self, credentials: &ProviderCredentials) -> bool;
}

/// Authentication errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("HTTP request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("OAuth error: {error} - {description}")]
    OAuth { error: String, description: String },

    #[error("Token expired and refresh failed")]
    TokenExpired,

    #[error("Invalid token response: {0}")]
    InvalidResponse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Provider not authenticated")]
    NotAuthenticated,
}
```

---

## Provider Implementations

### OpenAI Provider (Refactor Existing)

```rust
// codex-rs/core/src/providers/openai.rs

pub struct OpenAIAuth {
    client: reqwest::Client,
}

impl OpenAIAuth {
    pub const CLIENT_ID: &'static str = "app_EMoamEEZ73f0CkXaXp7hrann";
    pub const AUTH_URL: &'static str = "https://auth.openai.com/authorize";
    pub const TOKEN_URL: &'static str = "https://auth.openai.com/oauth/token";
    pub const REDIRECT_URI: &'static str = "https://platform.openai.com/auth/callback";
    pub const SCOPES: &[&str] = &["openid", "profile", "email"];
}

#[async_trait]
impl ProviderAuth for OpenAIAuth {
    fn provider_id(&self) -> ProviderId { ProviderId::OpenAI }
    fn display_name(&self) -> &'static str { "OpenAI" }

    fn oauth_config(&self) -> OAuthConfig {
        OAuthConfig {
            client_id: Self::CLIENT_ID.to_string(),
            auth_url: Self::AUTH_URL.to_string(),
            token_url: Self::TOKEN_URL.to_string(),
            redirect_uri: Self::REDIRECT_URI.to_string(),
            scopes: Self::SCOPES.iter().map(|s| s.to_string()).collect(),
            use_pkce: true,
        }
    }

    // ... implementation details
}
```

### Anthropic Provider

```rust
// codex-rs/core/src/providers/anthropic.rs

pub struct AnthropicAuth {
    client: reqwest::Client,
}

impl AnthropicAuth {
    /// OAuth Client ID from claude-code research
    pub const CLIENT_ID: &'static str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
    pub const AUTH_URL: &'static str = "https://claude.ai/oauth/authorize";
    pub const TOKEN_URL: &'static str = "https://console.anthropic.com/v1/oauth/token";
    pub const REDIRECT_URI: &'static str = "https://console.anthropic.com/oauth/code/callback";
    pub const SCOPES: &[&str] = &["org:create_api_key", "user:profile", "user:inference"];
}

#[async_trait]
impl ProviderAuth for AnthropicAuth {
    fn provider_id(&self) -> ProviderId { ProviderId::Anthropic }
    fn display_name(&self) -> &'static str { "Anthropic" }

    fn oauth_config(&self) -> OAuthConfig {
        OAuthConfig {
            client_id: Self::CLIENT_ID.to_string(),
            auth_url: Self::AUTH_URL.to_string(),
            token_url: Self::TOKEN_URL.to_string(),
            redirect_uri: Self::REDIRECT_URI.to_string(),
            scopes: Self::SCOPES.iter().map(|s| s.to_string()).collect(),
            use_pkce: true, // S256 PKCE
        }
    }

    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<TokenResponse, AuthError> {
        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", Self::CLIENT_ID),
            ("code", code),
            ("code_verifier", code_verifier),
            ("redirect_uri", Self::REDIRECT_URI),
        ];

        let response = self.client
            .post(Self::TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(AuthError::OAuth {
                error: "token_exchange_failed".to_string(),
                description: error_body,
            });
        }

        let token_response: TokenResponse = response.json().await?;
        Ok(token_response)
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<TokenResponse, AuthError> {
        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", Self::CLIENT_ID),
            ("refresh_token", refresh_token),
        ];

        let response = self.client
            .post(Self::TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AuthError::TokenExpired);
        }

        let token_response: TokenResponse = response.json().await?;
        Ok(token_response)
    }

    fn extract_metadata(&self, response: &TokenResponse) -> serde_json::Value {
        // Anthropic returns user info in the token response or via separate endpoint
        serde_json::json!({
            "provider": "anthropic"
        })
    }

    fn needs_refresh(&self, credentials: &ProviderCredentials) -> bool {
        // Anthropic tokens typically expire; check expires_at
        credentials.expires_at
            .map(|exp| exp < chrono::Utc::now() + chrono::Duration::minutes(5))
            .unwrap_or(false)
    }
}
```

### Google Provider (Gemini)

```rust
// codex-rs/core/src/providers/google.rs

pub struct GoogleAuth {
    client: reqwest::Client,
    redirect_port: u16,
}

impl GoogleAuth {
    /// OAuth Client ID from gemini-cli research
    pub const CLIENT_ID: &'static str =
        "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";
    pub const AUTH_URL: &'static str = "https://accounts.google.com/o/oauth2/v2/auth";
    pub const TOKEN_URL: &'static str = "https://oauth2.googleapis.com/token";
    pub const SCOPES: &[&str] = &[
        "https://www.googleapis.com/auth/cloud-platform",
        "https://www.googleapis.com/auth/userinfo.email",
        "https://www.googleapis.com/auth/userinfo.profile",
    ];

    pub fn new(client: reqwest::Client, redirect_port: u16) -> Self {
        Self { client, redirect_port }
    }
}

#[async_trait]
impl ProviderAuth for GoogleAuth {
    fn provider_id(&self) -> ProviderId { ProviderId::Google }
    fn display_name(&self) -> &'static str { "Google" }

    fn oauth_config(&self) -> OAuthConfig {
        OAuthConfig {
            client_id: Self::CLIENT_ID.to_string(),
            auth_url: Self::AUTH_URL.to_string(),
            token_url: Self::TOKEN_URL.to_string(),
            redirect_uri: format!("http://localhost:{}", self.redirect_port),
            scopes: Self::SCOPES.iter().map(|s| s.to_string()).collect(),
            use_pkce: true, // S256 PKCE
        }
    }

    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<TokenResponse, AuthError> {
        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", Self::CLIENT_ID),
            ("code", code),
            ("code_verifier", code_verifier),
            ("redirect_uri", &format!("http://localhost:{}", self.redirect_port)),
        ];

        let response = self.client
            .post(Self::TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(AuthError::OAuth {
                error: "token_exchange_failed".to_string(),
                description: error_body,
            });
        }

        let token_response: TokenResponse = response.json().await?;
        Ok(token_response)
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<TokenResponse, AuthError> {
        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", Self::CLIENT_ID),
            ("refresh_token", refresh_token),
        ];

        let response = self.client
            .post(Self::TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AuthError::TokenExpired);
        }

        let token_response: TokenResponse = response.json().await?;
        Ok(token_response)
    }

    fn extract_metadata(&self, response: &TokenResponse) -> serde_json::Value {
        // Google includes id_token with user info
        // Decode JWT to get email, etc.
        serde_json::json!({
            "provider": "google"
        })
    }

    fn needs_refresh(&self, credentials: &ProviderCredentials) -> bool {
        credentials.expires_at
            .map(|exp| exp < chrono::Utc::now() + chrono::Duration::minutes(5))
            .unwrap_or(false)
    }
}
```

---

## Shared Infrastructure

### PKCE Implementation

```rust
// codex-rs/core/src/provider_auth/pkce.rs

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

/// Generate a cryptographically random code verifier
pub fn generate_code_verifier() -> String {
    let random_bytes: [u8; 32] = rand::random();
    URL_SAFE_NO_PAD.encode(random_bytes)
}

/// Generate code challenge from verifier (S256 method)
pub fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

/// Generate random state parameter for CSRF protection
pub fn generate_state() -> String {
    let random_bytes: [u8; 16] = rand::random();
    URL_SAFE_NO_PAD.encode(random_bytes)
}
```

### Browser Callback Server

```rust
// codex-rs/core/src/provider_auth/callback_server.rs

use std::net::TcpListener;
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;

/// Simple HTTP server to receive OAuth callback
pub struct CallbackServer {
    listener: TcpListener,
    port: u16,
}

impl CallbackServer {
    /// Start server on available port
    pub fn new() -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(false)?;
        Ok(Self { listener, port })
    }

    /// Start server on specific port
    pub fn on_port(port: u16) -> std::io::Result<Self> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
        listener.set_nonblocking(false)?;
        Ok(Self { listener, port })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Wait for callback with authorization code
    pub fn wait_for_code(&self, expected_state: &str, timeout: Duration) -> Result<String, AuthError> {
        self.listener.set_read_timeout(Some(timeout))?;

        let (mut stream, _) = self.listener.accept()?;
        let mut reader = BufReader::new(&stream);

        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;

        // Parse GET /?code=XXX&state=YYY HTTP/1.1
        let (code, state) = parse_callback_params(&request_line)?;

        if state != expected_state {
            // Send error response
            let response = "HTTP/1.1 400 Bad Request\r\n\r\nState mismatch";
            stream.write_all(response.as_bytes())?;
            return Err(AuthError::OAuth {
                error: "state_mismatch".to_string(),
                description: "CSRF protection: state parameter mismatch".to_string(),
            });
        }

        // Send success response
        let html = r#"<!DOCTYPE html>
<html>
<head><title>Authentication Successful</title></head>
<body>
<h1>Authentication successful!</h1>
<p>You can close this window and return to the terminal.</p>
<script>window.close();</script>
</body>
</html>"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
            html.len(),
            html
        );
        stream.write_all(response.as_bytes())?;

        Ok(code)
    }
}

fn parse_callback_params(request_line: &str) -> Result<(String, String), AuthError> {
    // Parse "GET /?code=XXX&state=YYY HTTP/1.1"
    let path = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| AuthError::InvalidResponse("Missing path in callback".to_string()))?;

    let query = path
        .split('?')
        .nth(1)
        .ok_or_else(|| AuthError::InvalidResponse("Missing query string".to_string()))?;

    let mut code = None;
    let mut state = None;

    for param in query.split('&') {
        let mut parts = param.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");

        match key {
            "code" => code = Some(value.to_string()),
            "state" => state = Some(value.to_string()),
            _ => {}
        }
    }

    Ok((
        code.ok_or_else(|| AuthError::InvalidResponse("Missing code parameter".to_string()))?,
        state.ok_or_else(|| AuthError::InvalidResponse("Missing state parameter".to_string()))?,
    ))
}
```

---

## Storage Schema

### Extended auth_accounts.json

```json
{
  "version": 2,
  "active_accounts": {
    "openai": "uuid-1",
    "anthropic": "uuid-2",
    "google": "uuid-3"
  },
  "accounts": [
    {
      "id": "uuid-1",
      "provider": "openai",
      "mode": "oauth",
      "label": "user@example.com",
      "credentials": {
        "access_token": "...",
        "refresh_token": "...",
        "expires_at": "2025-12-19T00:00:00Z"
      },
      "metadata": {
        "email": "user@example.com",
        "account_id": "acct_xxx",
        "plan_type": "pro"
      },
      "created_at": "2025-11-19T00:00:00Z",
      "last_used_at": "2025-11-19T12:00:00Z"
    },
    {
      "id": "uuid-2",
      "provider": "anthropic",
      "mode": "oauth",
      "label": "user@example.com",
      "credentials": {
        "access_token": "...",
        "refresh_token": "...",
        "expires_at": "2025-12-19T00:00:00Z"
      },
      "metadata": {
        "email": "user@example.com"
      },
      "created_at": "2025-11-19T00:00:00Z",
      "last_used_at": "2025-11-19T12:00:00Z"
    },
    {
      "id": "uuid-3",
      "provider": "google",
      "mode": "oauth",
      "label": "user@gmail.com",
      "credentials": {
        "access_token": "...",
        "refresh_token": "...",
        "expires_at": "2025-12-19T00:00:00Z"
      },
      "metadata": {
        "email": "user@gmail.com"
      },
      "created_at": "2025-11-19T00:00:00Z",
      "last_used_at": "2025-11-19T12:00:00Z"
    }
  ]
}
```

---

## Provider Manager

```rust
// codex-rs/core/src/provider_auth/manager.rs

use std::collections::HashMap;
use std::sync::Arc;

/// Central manager for all provider authentications
pub struct ProviderAuthManager {
    codex_home: PathBuf,
    providers: HashMap<ProviderId, Arc<dyn ProviderAuth>>,
    client: reqwest::Client,
}

impl ProviderAuthManager {
    pub fn new(codex_home: PathBuf) -> Self {
        let client = crate::default_client::create_client("codex_cli_rs");

        let mut providers: HashMap<ProviderId, Arc<dyn ProviderAuth>> = HashMap::new();
        providers.insert(ProviderId::OpenAI, Arc::new(OpenAIAuth::new(client.clone())));
        providers.insert(ProviderId::Anthropic, Arc::new(AnthropicAuth::new(client.clone())));
        providers.insert(ProviderId::Google, Arc::new(GoogleAuth::new(client.clone(), 0)));

        Self {
            codex_home,
            providers,
            client,
        }
    }

    /// Get provider implementation
    pub fn provider(&self, id: ProviderId) -> Option<Arc<dyn ProviderAuth>> {
        self.providers.get(&id).cloned()
    }

    /// Get credentials for a provider
    pub fn get_credentials(&self, provider: ProviderId) -> Option<ProviderCredentials> {
        // Load from auth_accounts.json
        todo!()
    }

    /// Get access token, refreshing if needed
    pub async fn get_token(&self, provider: ProviderId) -> Result<String, AuthError> {
        let credentials = self.get_credentials(provider)
            .ok_or(AuthError::NotAuthenticated)?;

        let provider_impl = self.provider(provider)
            .ok_or(AuthError::NotAuthenticated)?;

        if provider_impl.needs_refresh(&credentials) {
            let refresh_token = credentials.refresh_token
                .ok_or(AuthError::TokenExpired)?;

            let response = provider_impl.refresh_token(&refresh_token).await?;
            // Update stored credentials
            self.update_credentials(provider, &response)?;

            Ok(response.access_token)
        } else {
            Ok(credentials.access_token)
        }
    }

    /// Start OAuth flow for a provider
    pub async fn authenticate(&self, provider: ProviderId) -> Result<(), AuthError> {
        let provider_impl = self.provider(provider)
            .ok_or(AuthError::NotAuthenticated)?;

        let config = provider_impl.oauth_config();
        let state = pkce::generate_state();
        let verifier = pkce::generate_code_verifier();

        // Start callback server
        let server = CallbackServer::new()?;

        // Build authorization URL
        let auth_url = provider_impl.authorization_url(&state, &verifier);

        // Open browser
        webbrowser::open(&auth_url).map_err(|e| AuthError::Io(
            std::io::Error::new(std::io::ErrorKind::Other, e)
        ))?;

        // Wait for callback
        let code = server.wait_for_code(&state, std::time::Duration::from_secs(300))?;

        // Exchange code for tokens
        let response = provider_impl.exchange_code(&code, &verifier).await?;

        // Store credentials
        self.store_credentials(provider, &response)?;

        Ok(())
    }

    fn store_credentials(&self, provider: ProviderId, response: &TokenResponse) -> Result<(), AuthError> {
        // Implementation: Update auth_accounts.json
        todo!()
    }

    fn update_credentials(&self, provider: ProviderId, response: &TokenResponse) -> Result<(), AuthError> {
        // Implementation: Update existing credentials
        todo!()
    }
}
```

---

## File Structure

```
codex-rs/core/src/
├── provider_auth/
│   ├── mod.rs              # Module exports, ProviderAuth trait
│   ├── pkce.rs             # PKCE code verifier/challenge
│   ├── callback_server.rs  # HTTP callback server
│   ├── manager.rs          # ProviderAuthManager
│   ├── error.rs            # AuthError enum
│   └── storage.rs          # Credential storage
├── providers/
│   ├── mod.rs              # Provider exports
│   ├── openai.rs           # OpenAI implementation (refactor)
│   ├── anthropic.rs        # Anthropic implementation
│   └── google.rs           # Google implementation
└── auth.rs                 # Existing (keep for backward compat)
```

---

## Acceptance Criteria

### Must Pass

1. **Trait Implementation**
   - [ ] `ProviderAuth` trait compiles with all required methods
   - [ ] OpenAI, Anthropic, Google providers implement trait
   - [ ] All providers use PKCE (S256)

2. **Authentication Flow**
   - [ ] Browser opens with correct auth URL and PKCE challenge
   - [ ] Callback server receives code and state
   - [ ] Code exchanges successfully for tokens
   - [ ] Tokens stored in auth_accounts.json v2

3. **Token Management**
   - [ ] `get_token()` returns valid access token
   - [ ] Automatic refresh when expired
   - [ ] Refresh failures properly bubble up as errors

4. **Storage**
   - [ ] Backward compatible with existing auth.json
   - [ ] Multi-provider credentials in auth_accounts.json
   - [ ] Secure file permissions (0o600)

### Tests Required

1. **Unit Tests**
   - [ ] PKCE verifier/challenge generation
   - [ ] Callback URL parsing
   - [ ] Token expiry detection

2. **Integration Tests**
   - [ ] Mock OAuth server for each provider
   - [ ] Full authentication flow
   - [ ] Token refresh flow
   - [ ] Storage roundtrip

---

## Dependencies

### Crate Dependencies

```toml
# codex-rs/core/Cargo.toml additions
[dependencies]
async-trait = "0.1"
sha2 = "0.10"
rand = "0.8"
webbrowser = "0.8"
thiserror = "1.0"
```

---

## Migration Strategy

1. **Phase 1**: Add new provider_auth module alongside existing auth.rs
2. **Phase 2**: Refactor OpenAI to use ProviderAuth trait
3. **Phase 3**: Add Anthropic and Google implementations
4. **Phase 4**: Update TUI to use ProviderAuthManager
5. **Phase 5**: Deprecate direct auth.rs usage (keep for CLI backward compat)

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Breaking existing OpenAI auth | Keep auth.rs functional during transition |
| OAuth endpoints change | Store URLs as constants, easy to update |
| Token format differences | Use serde_json::Value for provider-specific data |
| Callback port conflicts | Let OS assign available port (default) |

---

## Estimated Task Breakdown

| Task | Hours | Notes |
|------|-------|-------|
| Core trait + error types | 4 | Foundation |
| PKCE + callback server | 4 | Shared infrastructure |
| OpenAI refactor | 6 | Maintain backward compat |
| Anthropic implementation | 6 | New provider |
| Google implementation | 6 | New provider |
| Storage schema update | 4 | auth_accounts v2 |
| ProviderAuthManager | 4 | Orchestration |
| Tests | 6 | Unit + integration |
| **Total** | **40** | |

---

## Next Steps

1. Create `codex-rs/core/src/provider_auth/` module structure
2. Implement `ProviderAuth` trait and shared PKCE
3. Refactor OpenAI to use trait (validate backward compat)
4. Implement Anthropic provider
5. Implement Google provider
6. Update storage schema
7. Write tests
8. **Checkpoint 2**: Validate authentication works for all providers

---

## References

- SPEC-KIT-953: Master SPEC
- SPEC-KIT-953-A: Claude Code research (OAuth endpoints)
- SPEC-KIT-953-B: Gemini CLI research (OAuth endpoints)
- SPEC-KIT-953-C: codex-rs auth analysis
- OAuth 2.0 RFC 6749: https://tools.ietf.org/html/rfc6749
- PKCE RFC 7636: https://tools.ietf.org/html/rfc7636
- Anthropic Console: https://console.anthropic.com/
- Google OAuth: https://developers.google.com/identity/protocols/oauth2

---

## Change Log

| Date | Author | Change |
|------|--------|--------|
| 2025-11-19 | Claude | Initial SPEC with trait design and implementations |
