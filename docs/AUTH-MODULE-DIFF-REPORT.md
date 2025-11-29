# Auth Module Diff Report

**Generated**: 2025-11-29
**Purpose**: Unblock SYNC-016 (Device Code Auth)
**Status**: GO - Migration path identified

---

## Executive Summary

Investigation reveals **4 of 5 originally listed blockers don't exist in upstream**. The actual migration is simpler than anticipated:

| Original Blocker | Status | Action |
|-----------------|--------|--------|
| `AuthCredentialsStoreMode` enum | NOT IN UPSTREAM | Remove from blockers |
| `save_auth` helper | NOT IN UPSTREAM | Remove from blockers |
| `cli_auth_credentials_store_mode` field | NOT IN UPSTREAM | Remove from blockers |
| `ensure_workspace_allowed` function | NOT IN UPSTREAM | Remove from blockers |
| `CODEX_API_KEY_ENV_VAR` constant | **CONFIRMED MISSING** | Port required |

**Real Migration Scope**: ~300 lines of code across 2 files.

---

## Module Structure Comparison

### Login Crate (`login/src/`)

| Component | Fork | Upstream | Gap |
|-----------|------|----------|-----|
| `lib.rs` | 22 lines | 25 lines | Missing device_code exports |
| `server.rs` | 20,714 bytes | 21,347 bytes | Minor diffs |
| `pkce.rs` | 750 bytes | 750 bytes | Identical |
| `device_code_auth.rs` | **MISSING** | 11,115 bytes (359 lines) | **BLOCKING** |

### Core Auth (`core/src/auth.rs`)

| Component | Fork | Upstream | Gap |
|-----------|------|----------|-----|
| File size | 30,532 bytes | 42,930 bytes | +12KB in upstream |
| `CODEX_API_KEY_ENV_VAR` | Missing | Line 296 | Port required |
| `read_code_api_key_from_env()` | Missing | Lines 304-309 | Port required |
| `RefreshTokenError` type | Missing | Lines 44-79 | Port required |
| `RefreshTokenErrorKind` enum | Missing | Lines 38-41 | Port required |
| `classify_refresh_failure()` | Missing | Lines 600-649 | Port required |
| OAuth error types | Missing | Lines 583-598 | Port required |
| `adopt_rotated_refresh_token_from_disk()` | Missing | Lines 120-140 | Port required |

---

## API Differences

### Public Exports: Fork

```rust
// core/src/auth.rs
pub struct CodexAuth
pub const OPENAI_API_KEY_ENV_VAR: &str = "OPENAI_API_KEY"
pub const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann"
pub struct AuthDotJson
pub struct AuthManager
pub fn get_auth_file(codex_home: &Path) -> PathBuf
pub fn logout(codex_home: &Path) -> std::io::Result<bool>
pub fn login_with_api_key(codex_home: &Path, api_key: &str) -> std::io::Result<()>
pub fn activate_account(codex_home: &Path, account_id: &str) -> std::io::Result<()>
pub fn try_read_auth_json(auth_file: &Path) -> std::io::Result<AuthDotJson>
pub fn write_auth_json(auth_file: &Path, auth_dot_json: &AuthDotJson) -> std::io::Result<()>
```

### Public Exports: Upstream (Additional)

```rust
// core/src/auth.rs - items missing from fork
pub const CODEX_API_KEY_ENV_VAR: &str = "CODEX_API_KEY"
pub enum RefreshTokenErrorKind { Permanent, Transient }
pub struct RefreshTokenError { kind, message }
pub fn read_code_api_key_from_env() -> Option<String>
```

### Login Crate Exports: Upstream (Additional)

```rust
// login/src/lib.rs - items missing from fork
pub use device_code_auth::{run_device_code_login, DeviceCodeSession}
```

---

## Naming Convention Differences

The upstream uses "code" prefix while fork uses "codex":

| Fork | Upstream |
|------|----------|
| `codex_home` | `code_home` |
| `from_codex_home()` | `from_code_home()` |
| `codex_protocol::mcp_protocol::AuthMode` | `code_app_server_protocol::AuthMode` |
| `resolve_codex_path_for_read()` | `resolve_code_path_for_read()` |
| `codex_core::auth` | `code_core::auth` |

**Note**: Fork naming is correct for this project. Porting requires s/code_/codex_/ substitution.

---

## Missing Types Detail

### RefreshTokenError (Upstream lines 44-79)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshTokenErrorKind {
    Permanent,  // Auth revoked, invalid grant
    Transient,  // Network error, server unavailable
}

#[derive(Debug, Clone)]
pub struct RefreshTokenError {
    pub kind: RefreshTokenErrorKind,
    pub message: String,
}

impl RefreshTokenError {
    pub fn permanent(message: impl Into<String>) -> Self;
    pub fn transient(message: impl Into<String>) -> Self;
    pub fn is_permanent(&self) -> bool;
    pub fn is_refresh_token_reused(&self) -> bool;
}
```

**Why it matters**: Enables automatic retry for transient failures, immediate fail for permanent errors.

### OAuth Error Classification (Upstream lines 583-649)

```rust
#[derive(Deserialize)]
struct OAuthErrorBody { error: Option<String>, error_description: Option<String> }

#[derive(Deserialize)]
struct OpenAiErrorWrapper { error: Option<OpenAiErrorData> }

#[derive(Deserialize)]
struct OpenAiErrorData { code: Option<String>, message: Option<String> }

fn classify_refresh_failure(status: StatusCode, body: &str) -> RefreshTokenError;
```

**Why it matters**: Proper error classification for OAuth responses, handles refresh_token_reused scenario.

---

## device_code_auth.rs Overview

The missing module provides headless authentication:

```rust
pub struct DeviceCodeSession {
    client: reqwest::Client,
    opts: ServerOptions,
    api_base_url: String,
    base_url: String,
    device_auth_id: String,
    user_code: String,
    interval: u64,
}

impl DeviceCodeSession {
    pub async fn start(opts: ServerOptions) -> std::io::Result<Self>;
    pub fn authorize_url(&self) -> String;
    pub fn user_code(&self) -> &str;
    pub async fn wait_for_tokens(self) -> std::io::Result<()>;
}

pub async fn run_device_code_login(opts: ServerOptions) -> std::io::Result<()>;
```

### Flow
1. Request user code from `/api/accounts/deviceauth/usercode`
2. Display code and URL to user
3. Poll `/api/accounts/deviceauth/token` every N seconds
4. Exchange code for tokens on success
5. Persist tokens via `persist_tokens_async()`

### External Dependencies
- `code_browser::global` - Cloudflare challenge fallback
- `code_core::default_client` - HTTP client factory
- `crate::server::{persist_tokens_async, exchange_code_for_tokens, ServerOptions}`
- `crate::pkce::PkceCodes`

---

## Migration Path

### Phase 1: Core Auth Enhancements (~150 lines)

**File**: `codex-rs/core/src/auth.rs`

1. Add constants:
```rust
pub const CODEX_API_KEY_ENV_VAR: &str = "CODEX_API_KEY";

pub fn read_codex_api_key_from_env() -> Option<String> {
    std::env::var(CODEX_API_KEY_ENV_VAR)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
```

2. Add error types:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshTokenErrorKind { Permanent, Transient }

#[derive(Debug, Clone)]
pub struct RefreshTokenError { pub kind: RefreshTokenErrorKind, pub message: String }
```

3. Add error classification (port `classify_refresh_failure()`)

4. Update `CodexAuth::refresh_token()` to use `RefreshTokenError`

5. Add `adopt_rotated_refresh_token_from_disk()` method

**Risk**: Low - additive changes, no breaking API changes.

### Phase 2: Device Code Auth Port (~180 lines)

**Files**:
- New: `codex-rs/login/src/device_code_auth.rs`
- Modified: `codex-rs/login/src/lib.rs`

1. Copy `device_code_auth.rs` from upstream

2. Apply substitutions:
   - `code_core` → `codex_core`
   - `code_browser` → `codex_browser`
   - `code_home` → `codex_home`

3. Update `lib.rs`:
```rust
mod device_code_auth;
pub use device_code_auth::{run_device_code_login, DeviceCodeSession};
```

4. Verify `codex_browser` crate exists (check `browser/` crate in fork)

**Risk**: Medium - requires `codex_browser` crate integration.

### Phase 3: CLI Integration

**File**: `codex-rs/cli/src/` (login command)

Add `--device-code` flag to invoke `run_device_code_login()`.

**Risk**: Low - optional feature addition.

---

## Verification Checklist

### Pre-Port
- [ ] Verify `codex_browser` crate has `global::get_or_create_browser_manager()`
- [ ] Verify `ServerOptions` struct is compatible
- [ ] Verify `persist_tokens_async()` and `exchange_code_for_tokens()` are exported

### Post-Port
- [ ] `cargo build -p codex-login` succeeds
- [ ] `cargo test -p codex-login` passes
- [ ] Device code flow works against staging auth server
- [ ] Cloudflare fallback path exercised

---

## Decision

**GO** - Migration is feasible with estimated 2-3 hours effort.

### Rationale
1. 4 of 5 original blockers don't exist
2. Actual scope is ~300 lines across 2 files
3. Changes are additive (no breaking changes)
4. Clear dependency chain
5. Good test coverage opportunity

### Next Steps
1. Port Phase 1 (core auth enhancements)
2. Verify browser crate compatibility
3. Port Phase 2 (device_code_auth.rs)
4. Add CLI flag
5. Update SYNC-016 status to READY

---

## Appendix: File Locations

| Component | Fork Path | Upstream Path |
|-----------|-----------|---------------|
| Core auth | `codex-rs/core/src/auth.rs` | `~/old/code/code-rs/core/src/auth.rs` |
| Login lib | `codex-rs/login/src/lib.rs` | `~/old/code/code-rs/login/src/lib.rs` |
| Device code | N/A | `~/old/code/code-rs/login/src/device_code_auth.rs` |
| Server | `codex-rs/login/src/server.rs` | `~/old/code/code-rs/login/src/server.rs` |
| Browser | `codex-rs/browser/` | `~/old/code/code-rs/browser/` |
