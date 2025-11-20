# SPEC-KIT-953-C: Existing codex-rs OAuth Analysis

**Parent**: SPEC-KIT-953
**Status**: Research Complete
**Priority**: High (Parallel with A, B)
**Target**: codex-rs/core, codex-rs/tui
**Completed**: 2025-11-19

---

## Purpose

Document existing OAuth patterns to determine extension points for multi-provider support.

---

## Executive Summary

The codex-rs authentication system uses a **two-tiered architecture** with excellent extensibility:

1. **AuthMode Enum** - Defines auth strategies (ApiKey, ChatGPT)
2. **StoredAccount System** - Multi-account storage with switching
3. **ProviderType Abstraction** - Already exists for model routing

**Key Finding**: Architecture is already designed for multi-provider extension. Minimal changes required.

---

## Research Findings

### 1. Current Auth Architecture

**Three-Layer Design**:

```
┌─────────────────────────────────────────────────────────┐
│                    TUI (onboarding)                      │
│   AuthModeWidget → sign in flow → account selection     │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│              Auth Account Layer (codex_core)             │
│  AuthManager → account activation → token management    │
│  (codex-rs/core/src/auth.rs)                            │
│  (codex-rs/core/src/auth_accounts.rs)                   │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│           Persistent Storage Layer                       │
│  auth.json (active account)                             │
│  auth_accounts.json (all accounts)                      │
│  OPENAI_API_KEY env var (fallback)                      │
└─────────────────────────────────────────────────────────┘
```

### 2. CodexAuth Struct

**Location**: `codex-rs/core/src/auth.rs:24-32`

```rust
pub struct CodexAuth {
    pub mode: AuthMode,                              // ApiKey or ChatGPT
    pub(crate) api_key: Option<String>,             // For ApiKey mode
    pub(crate) auth_dot_json: Arc<Mutex<Option<AuthDotJson>>>,
    pub(crate) auth_file: PathBuf,                  // Path to auth.json
    pub(crate) client: reqwest::Client,             // HTTP client for refresh
}
```

**Key Methods**:
- `from_codex_home()` - Load auth from codex home + env vars
- `get_token()` - Returns token based on current mode
- `refresh_token()` - Async token refresh (ChatGPT only)
- `get_token_data()` - Full TokenData with account info
- `get_account_id()` - Extract account ID from JWT

### 3. AuthManager (Caching Layer)

**Location**: `codex-rs/core/src/auth.rs:770-882`

```rust
pub struct AuthManager {
    codex_home: PathBuf,
    originator: String,
    inner: RwLock<CachedAuth>,  // Current CodexAuth
}
```

**Key Methods**:
- `auth()` - Return current CodexAuth (cloned)
- `reload()` - Force reload from disk
- `refresh_token()` - Async refresh + auto-reload
- `logout()` - Delete auth.json + clear cache
- `preferred_auth_method()` - Get preferred AuthMode

**Pattern**: Single source of truth with explicit reload points.

### 4. AuthMode Enum

**Location**: `codex-rs/protocol/src/mcp_protocol.rs:96-101`

```rust
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, TS)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    ApiKey,    // Direct API key, no refresh
    ChatGPT,   // OAuth via OpenAI, refresh via endpoint
}
```

**Mode-Specific Behavior**:

| Aspect | ApiKey | ChatGPT |
|--------|--------|---------|
| Token Source | `api_key` field | `tokens` field |
| Refresh | ❌ None | ✅ Required |
| Account ID | ❌ None | ✅ From JWT |
| Env Fallback | `OPENAI_API_KEY` | ❌ None |

---

## Token Storage Schemas

### auth.json (Active Account)

**Location**: `$CODEX_HOME/auth.json`
**Permissions**: 0600

```json
{
  "OPENAI_API_KEY": null,
  "tokens": {
    "id_token": "header.payload.signature",
    "access_token": "...",
    "refresh_token": "...",
    "account_id": "bc3618e3-489d-4d49-..."
  },
  "last_refresh": "2025-08-06T20:41:36.232376Z"
}
```

### auth_accounts.json (All Accounts)

**Location**: `$CODEX_HOME/auth_accounts.json`
**Permissions**: 0600

```json
{
  "version": 1,
  "active_account_id": "550e8400-e29b-41d4-a716-446655440000",
  "accounts": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "mode": "chatgpt",
      "label": "Personal Account",
      "openai_api_key": null,
      "tokens": {
        "id_token": "...",
        "access_token": "...",
        "refresh_token": "...",
        "account_id": "acct-123"
      },
      "last_refresh": "2025-08-06T20:41:36.232376Z",
      "created_at": "2025-08-01T10:00:00Z",
      "last_used_at": "2025-08-10T15:30:00Z"
    }
  ]
}
```

### StoredAccount Struct

**Location**: `codex-rs/core/src/auth_accounts.rs:14-36`

```rust
pub struct StoredAccount {
    pub id: String,                        // UUID
    pub mode: AuthMode,                   // chatgpt or apikey
    pub label: Option<String>,            // User-friendly name
    pub openai_api_key: Option<String>,  // For ApiKey mode
    pub tokens: Option<TokenData>,        // For ChatGPT mode
    pub last_refresh: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}
```

---

## Token Refresh Mechanism

### Refresh Flow

**Trigger**: 28-day heuristic check in `get_token_data()`

```
1. Check last_refresh timestamp
2. If > 28 days old:
   ├─ POST to https://auth.openai.com/oauth/token
   │  ├─ client_id: "app_EMoamEEZ73f0CkXaXp7hrann"
   │  ├─ grant_type: "refresh_token"
   │  └─ scope: "openai profile email"
   │
   ├─ Update tokens in auth.json
   ├─ Update auth_accounts.json
   └─ Return updated TokenData
3. If recent: return cached tokens
```

### RefreshRequest/Response

```rust
struct RefreshRequest {
    client_id: &'static str,
    grant_type: &'static str,
    refresh_token: String,
    scope: &'static str,
}

struct RefreshResponse {
    id_token: String,
    access_token: Option<String>,
    refresh_token: Option<String>,
}
```

---

## Extension Points

### 1. AuthMode Enum Extension

**Most Direct Path**: Add provider-specific variants

```rust
pub enum AuthMode {
    ApiKey,           // OpenAI API key
    ChatGPT,          // OpenAI OAuth
    // NEW:
    AnthropicApiKey,  // Anthropic API key
    GoogleOAuth,      // Google OAuth (for Gemini)
}
```

**Why This Works**:
- `StoredAccount.mode` already holds the auth type
- Token refresh dispatches on mode
- Account switching operates on StoredAccount

### 2. StoredAccount Extension

**Add Provider Field**:

```rust
pub struct StoredAccount {
    pub id: String,
    pub mode: AuthMode,
    pub provider: Option<String>,  // NEW: "openai", "anthropic", "google"
    pub label: Option<String>,

    // Legacy (backward compat):
    pub openai_api_key: Option<String>,
    pub tokens: Option<TokenData>,

    // Generic (NEW):
    pub api_key: Option<String>,          // Provider-agnostic API key
    pub oauth_tokens: Option<OAuthTokens>, // Provider-agnostic OAuth

    pub last_refresh: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}
```

### 3. ProviderType Already Exists

**Location**: `codex-rs/tui/src/providers/mod.rs:76-134`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    ChatGPT,   // Uses native OAuth
    Claude,    // Currently routes through CLI
    Gemini,    // Currently routes through CLI
}
```

**Mapping**: `ProviderType::from_model_name()` already maps model names to providers.

---

## Proposed Multi-Provider Schema

### auth.json V2

```json
{
  "version": 2,

  // Legacy (backward compat):
  "OPENAI_API_KEY": null,
  "tokens": null,
  "last_refresh": null,

  // NEW:
  "active_provider": "anthropic",
  "providers": {
    "openai": {
      "auth_mode": "chatgpt",
      "api_key": null,
      "tokens": { ... },
      "last_refresh": "2025-08-06T..."
    },
    "anthropic": {
      "auth_mode": "api_key",
      "api_key": "sk-ant-...",
      "tokens": null,
      "last_refresh": null
    },
    "google": {
      "auth_mode": "oauth",
      "api_key": null,
      "tokens": {
        "access_token": "...",
        "refresh_token": "...",
        "expiry_date": 1699999999000
      },
      "last_refresh": "2025-08-06T..."
    }
  }
}
```

### auth_accounts.json V2

```json
{
  "version": 2,
  "active_account_id": "...",
  "accounts": [
    {
      "id": "...",
      "mode": "chatgpt",
      "provider": "openai",
      "label": "Personal OpenAI",
      "openai_api_key": null,
      "tokens": { ... },
      "api_key": null,
      "oauth_tokens": null,
      ...
    },
    {
      "id": "...",
      "mode": "apikey",
      "provider": "anthropic",
      "label": "Claude API Key",
      "openai_api_key": null,
      "tokens": null,
      "api_key": "sk-ant-...",
      "oauth_tokens": null,
      ...
    }
  ]
}
```

---

## Migration & Backward Compatibility

### Strategy

1. **V1 Detection**: Check for `version` field
2. **Auto-Migration**: On first load, copy V1 fields to V2 `providers.openai`
3. **Write-Back**: After any auth operation, write V2 format
4. **Fallback**: If `provider` missing, default to "openai"

### Migration Code Pattern

```rust
fn migrate_auth_json(old: AuthDotJsonV1) -> AuthDotJsonV2 {
    AuthDotJsonV2 {
        version: 2,
        active_provider: "openai".to_string(),
        providers: {
            let mut map = HashMap::new();
            map.insert("openai".to_string(), ProviderAuth {
                auth_mode: if old.tokens.is_some() { "chatgpt" } else { "api_key" },
                api_key: old.openai_api_key,
                tokens: old.tokens,
                last_refresh: old.last_refresh,
            });
            map
        },
        // Keep legacy for old clients
        OPENAI_API_KEY: old.openai_api_key,
        tokens: old.tokens,
        last_refresh: old.last_refresh,
    }
}
```

---

## Implementation Roadmap

### Phase 1: Foundation (SPEC-953-D)

1. **Extend AuthMode enum** - Add `AnthropicApiKey`, `GoogleOAuth`
2. **Add provider field** to StoredAccount
3. **Create upsert functions** - `upsert_anthropic_account()`, `upsert_google_account()`
4. **Update refresh dispatch** - Route by AuthMode

### Phase 2: Account Switching

1. **Extend LoginAccountsView** - Show provider icon/badge
2. **Provider selector** in onboarding
3. **Update account labels** - "Claude API Key (Anthropic)"

### Phase 3: OAuth (Gemini)

1. **Implement Google OAuth flow** with PKCE
2. **Add token refresh** for Google tokens
3. **Integrate with AuthManager**

---

## Proposed Trait Abstraction

```rust
/// Provider-agnostic auth trait
pub trait ProviderAuth: Send + Sync {
    fn provider_name(&self) -> &'static str;
    fn supports_refresh(&self) -> bool;
    async fn refresh_token(&self, refresh: &str) -> Result<RefreshResponse, Error>;
    fn validate_token(&self, token: &str) -> Result<(), Error>;
}

struct OpenAiAuth;
impl ProviderAuth for OpenAiAuth {
    fn provider_name(&self) -> &'static str { "openai" }
    fn supports_refresh(&self) -> bool { true }
    async fn refresh_token(&self, token: &str) -> ... {
        // POST https://auth.openai.com/oauth/token
    }
}

struct AnthropicAuth;
impl ProviderAuth for AnthropicAuth {
    fn provider_name(&self) -> &'static str { "anthropic" }
    fn supports_refresh(&self) -> bool { false }
    async fn refresh_token(&self, _: &str) -> ... {
        Err("API key auth doesn't support refresh")
    }
}

struct GoogleAuth;
impl ProviderAuth for GoogleAuth {
    fn provider_name(&self) -> &'static str { "google" }
    fn supports_refresh(&self) -> bool { true }
    async fn refresh_token(&self, token: &str) -> ... {
        // POST https://oauth2.googleapis.com/token
    }
}
```

---

## Key Files Reference

| File | Lines | Purpose |
|------|-------|---------|
| `codex-rs/core/src/auth.rs` | 883 | Main auth implementation |
| `codex-rs/core/src/auth_accounts.rs` | ~200 | Account management |
| `codex-rs/core/src/token_data.rs` | ~150 | Token structures |
| `codex-rs/protocol/src/mcp_protocol.rs` | ~100 | AuthMode enum |
| `codex-rs/tui/src/onboarding/auth.rs` | ~300 | TUI auth flow |
| `codex-rs/tui/src/bottom_pane/login_accounts_view.rs` | ~200 | Account management UI |
| `codex-rs/tui/src/providers/mod.rs` | ~150 | ProviderType enum |

---

## Deliverables Summary

- [x] Architecture diagram of current OAuth flow
- [x] auth.json schema documentation
- [x] Refresh mechanism and timing analysis
- [x] AuthManager API surface documentation
- [x] Extension point identification
- [x] **Proposed multi-provider schema** (V2 format)

---

## Implications for SPEC-953

### For SPEC-953-D (Auth Framework)

**Clear Extension Path**:
1. Add 2 new AuthMode variants
2. Add `provider` field to StoredAccount
3. Create provider-specific upsert functions
4. Implement ProviderAuth trait

**Effort Estimate**: 20-30 hours

### For SPEC-953-E (Context Manager)

- Context management is separate from auth
- Can share AuthManager across providers
- Token refresh timing differs by provider

### For SPEC-953-F/G (Native Providers)

- Providers can request tokens via unified interface
- Each provider implements its own refresh logic
- Consistent error handling

---

## Architecture Decision Input

**For Checkpoint 1**:

The existing auth architecture is **well-designed for multi-provider extension**:

| Aspect | Current State | Extension Effort |
|--------|---------------|------------------|
| AuthMode | 2 variants | Add 2-3 more |
| StoredAccount | OpenAI-specific fields | Add generic fields |
| AuthManager | Provider-agnostic | No changes |
| Account Switching | Full support | Minor UI updates |
| Token Refresh | OpenAI only | Dispatch by provider |

**Recommendation**: Extend existing patterns rather than rewrite.

---

## References

- `codex-rs/core/src/auth.rs` - Main auth implementation
- `codex-rs/core/src/auth_accounts.rs` - Account storage
- `codex-rs/tui/src/providers/mod.rs` - ProviderType enum
- SPEC-KIT-951: Multi-Provider OAuth Research

---

## Document History

| Date | Status | Notes |
|------|--------|-------|
| 2025-11-19 | Complete | Architecture well-suited for extension |
