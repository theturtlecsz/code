# P6-SYNC Continuation: UI Integration + Phase 5 Device Code Auth

_Generated: 2025-11-29_
_Previous Session: Phase 6 TokenMetricsWidget infrastructure complete_

## Session Context

### Just Completed (Phase 6 Infrastructure)
- **TokenMetricsWidget** created: `codex-rs/tui/src/token_metrics_widget.rs`
- **Model context windows**: Covers OpenAI, Claude, Gemini (128k-1M)
- **Pricing integration**: Uses existing `cost_tracker.rs` ModelPricing
- **Per-stage tracking**: `stage_metrics` HashMap in SpecAutoState
- **New methods**: `record_stage_tokens()`, `context_window()`, `context_utilization()`
- **7 tests passing**, clippy clean

### Remaining Work (This Session)
1. **Commit Phase 6** (~2 min)
2. **Wire TokenMetricsWidget to UI** (~30 min)
3. **Phase 5: Device Code Auth** - OpenAI + Google in parallel (~2-3h)

---

## Step 1: Startup Verification

```bash
cd ~/code/codex-rs

# Verify Phase 6 compiles
cargo check -p codex-tui

# Verify tests pass
cargo test -p codex-tui --lib -- token_metrics

# Verify widget exists
grep -n "pub struct TokenMetricsWidget" tui/src/token_metrics_widget.rs

# Check git status
git status
git diff --stat
```

---

## Step 2: Commit Phase 6

```bash
git add -A && git commit -m "$(cat <<'EOF'
feat(tui): Add TokenMetrics UI infrastructure (P6-SYNC Phase 6)

- TokenMetricsWidget with full/compact rendering modes
- Context utilization warnings (>80% yellow, >90% red)
- Per-model context window lookup (OpenAI/Claude/Gemini)
- Cost estimation using existing ModelPricing
- Per-stage token breakdown tracking in SpecAutoState
- New methods: record_stage_tokens, context_window, context_utilization

Also fixes pre-existing clippy errors in codex-spec-kit retry module.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Step 3: Wire TokenMetricsWidget to ChatWidget Status Bar

### Goal
Display token metrics in the TUI status bar during spec-kit pipeline runs.

### 3.1 Find Status Bar Rendering
Location: Search in `codex-rs/tui/src/chatwidget/` or `bottom_pane/`

```bash
# Find where status/footer is rendered
grep -rn "status\|footer\|render" tui/src/bottom_pane/
grep -rn "spec_auto_state" tui/src/chatwidget/mod.rs
```

### 3.2 Integration Pattern
When `spec_auto_state` is active and has metrics, render the widget:

```rust
use crate::token_metrics_widget::{TokenMetricsWidget, model_context_window};

// In status bar render method:
if let Some(state) = &self.spec_auto_state {
    let model_id = state.current_model.as_deref().unwrap_or("unknown");
    let widget = TokenMetricsWidget::from_session_metrics(
        &state.session_metrics,
        model_context_window(model_id),
        model_id,
    );

    // Render warning if context is stressed
    if widget.is_critical() {
        // Show red warning in status
    } else if widget.is_warning() {
        // Show yellow warning
    }

    // Render widget in appropriate area
    widget.render(metrics_area, buf);
}
```

### 3.3 Wire Model Tracking
Ensure `set_current_model()` is called when model changes:

```bash
# Find where model is selected/changed
grep -rn "model\|provider" tui/src/chatwidget/spec_kit/handler.rs | head -20
```

### Acceptance Criteria (UI Integration)
- [ ] TokenMetricsWidget renders in status bar during spec-kit runs
- [ ] Shows token counts (input/output)
- [ ] Shows context utilization with color coding
- [ ] Shows cost estimation
- [ ] Warning appears when >80% context used
- [ ] Critical warning appears when >90% context used

---

## Step 4: Phase 5 - Device Code Auth

### Goal
OAuth device code flow for OpenAI + Google, enabling headless/SSH authentication.

### 4.1 Create Device Code Trait
Location: `codex-rs/login/src/device_code.rs` (NEW)

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Device code response from OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: u64,
}

/// Token received after successful authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// Errors during device code flow
#[derive(Debug, thiserror::Error)]
pub enum DeviceCodeError {
    #[error("Authorization pending - user hasn't completed login")]
    AuthorizationPending,
    #[error("Slow down - polling too fast")]
    SlowDown,
    #[error("Access denied by user")]
    AccessDenied,
    #[error("Device code expired")]
    ExpiredToken,
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Provider-agnostic device code authentication
#[async_trait]
pub trait DeviceCodeAuth: Send + Sync {
    fn provider_name(&self) -> &'static str;
    async fn request_device_code(&self) -> Result<DeviceCodeResponse, DeviceCodeError>;
    async fn poll_for_token(&self, device_code: &str) -> Result<AuthToken, DeviceCodeError>;
    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, DeviceCodeError>;
    fn poll_interval(&self) -> Duration { Duration::from_secs(5) }
}
```

### 4.2 OpenAI Implementation
Location: `codex-rs/login/src/providers/openai.rs` (NEW)

```rust
const OPENAI_DEVICE_AUTH_URL: &str = "https://auth.openai.com/oauth/device/code";
const OPENAI_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const OPENAI_CLIENT_ID: &str = "app-codex-cli";

pub struct OpenAIDeviceAuth {
    client: reqwest::Client,
    client_id: String,
}

#[async_trait]
impl DeviceCodeAuth for OpenAIDeviceAuth {
    fn provider_name(&self) -> &'static str { "OpenAI" }

    async fn request_device_code(&self) -> Result<DeviceCodeResponse, DeviceCodeError> {
        // POST to device auth URL with client_id and scope
    }

    async fn poll_for_token(&self, device_code: &str) -> Result<AuthToken, DeviceCodeError> {
        // POST to token URL with grant_type=device_code
        // Handle authorization_pending, slow_down, access_denied, expired_token
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, DeviceCodeError> {
        // POST to token URL with grant_type=refresh_token
    }
}
```

### 4.3 Google Implementation
Location: `codex-rs/login/src/providers/google.rs` (NEW)

```rust
const GOOGLE_DEVICE_AUTH_URL: &str = "https://oauth2.googleapis.com/device/code";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

pub struct GoogleDeviceAuth {
    client: reqwest::Client,
    client_id: String,
    client_secret: Option<String>,
}

#[async_trait]
impl DeviceCodeAuth for GoogleDeviceAuth {
    fn provider_name(&self) -> &'static str { "Google" }
    // Similar implementation, Google requires client_secret for some flows
}
```

### 4.4 Provider Registry
Location: `codex-rs/login/src/providers/mod.rs` (NEW)

```rust
mod openai;
mod google;

pub use openai::OpenAIDeviceAuth;
pub use google::GoogleDeviceAuth;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthProvider {
    OpenAI,
    Google,
}

impl AuthProvider {
    pub fn from_model_id(model_id: &str) -> Option<Self> {
        if model_id.contains("gpt") || model_id.contains("o1") || model_id.contains("o3") {
            Some(Self::OpenAI)
        } else if model_id.contains("gemini") {
            Some(Self::Google)
        } else {
            None  // Claude uses API key, no device code
        }
    }
}

pub fn get_auth_handler(provider: AuthProvider) -> Box<dyn DeviceCodeAuth> {
    match provider {
        AuthProvider::OpenAI => Box::new(OpenAIDeviceAuth::new()),
        AuthProvider::Google => Box::new(GoogleDeviceAuth::new(
            std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
            std::env::var("GOOGLE_CLIENT_SECRET").ok(),
        )),
    }
}
```

### 4.5 Token Storage
Location: `codex-rs/login/src/token_store.rs` (NEW)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenStore {
    tokens: HashMap<String, StoredToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub provider: String,
}

impl TokenStore {
    pub fn load(path: &PathBuf) -> Result<Self, std::io::Error>;
    pub fn save(&self, path: &PathBuf) -> Result<(), std::io::Error>;
    pub fn store_token(&mut self, provider: AuthProvider, token: AuthToken);
    pub fn get_token(&self, provider: AuthProvider) -> Option<&StoredToken>;
    pub fn is_expired(&self, provider: AuthProvider) -> bool;
    pub fn clear(&mut self, provider: AuthProvider);
}
```

### 4.6 CLI Integration
Update `/model` command handler to trigger device code flow when needed:

```rust
async fn handle_model_auth(model_id: &str) -> Result<(), Error> {
    let provider = AuthProvider::from_model_id(model_id);

    if let Some(provider) = provider {
        let store_path = get_token_store_path();
        let mut store = TokenStore::load(&store_path)?;

        // Already authenticated?
        if !store.is_expired(provider) {
            return Ok(());
        }

        // Try refresh first
        if let Some(token) = store.get_token(provider) {
            if let Some(refresh) = &token.refresh_token {
                let handler = get_auth_handler(provider);
                if let Ok(new_token) = handler.refresh_token(refresh).await {
                    store.store_token(provider, new_token);
                    store.save(&store_path)?;
                    return Ok(());
                }
            }
        }

        // Device code flow
        let handler = get_auth_handler(provider);
        let device_code = handler.request_device_code().await?;

        println!("\n🔐 Authentication required for {}", handler.provider_name());
        println!("   Visit: {}", device_code.verification_uri);
        println!("   Enter code: {}", device_code.user_code);
        println!("\n   Waiting for authorization...\n");

        // Poll for token
        let interval = handler.poll_interval();
        let deadline = std::time::Instant::now()
            + std::time::Duration::from_secs(device_code.expires_in);

        while std::time::Instant::now() < deadline {
            tokio::time::sleep(interval).await;

            match handler.poll_for_token(&device_code.device_code).await {
                Ok(token) => {
                    store.store_token(provider, token);
                    store.save(&store_path)?;
                    println!("   ✅ Authenticated successfully!\n");
                    return Ok(());
                }
                Err(DeviceCodeError::AuthorizationPending) => continue,
                Err(DeviceCodeError::SlowDown) => {
                    tokio::time::sleep(interval).await;
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }

        Err(DeviceCodeError::ExpiredToken.into())
    } else {
        Ok(())  // No device code auth needed (e.g., Claude uses API key)
    }
}
```

### Acceptance Criteria (Phase 5)
- [ ] DeviceCodeAuth trait defined with all methods
- [ ] OpenAI device code implementation complete
- [ ] Google device code implementation complete
- [ ] Provider registry with model-to-provider mapping
- [ ] Token storage with persistence to disk
- [ ] Token refresh on expiry (before falling back to device code)
- [ ] CLI integration - prompts user with URL and code
- [ ] Polling loop with timeout handling
- [ ] Tests for each provider (can be unit tests with mocked responses)
- [ ] Error handling for all OAuth error codes

---

## Files to Create/Modify

### Phase 6 UI Integration (3 files)
- `codex-rs/tui/src/chatwidget/mod.rs` or `bottom_pane/` - Wire widget
- `codex-rs/tui/src/chatwidget/spec_kit/handler.rs` - Call `set_current_model()`
- Integration point TBD after exploration

### Phase 5 Device Code Auth (7 files)
- `codex-rs/login/src/device_code.rs` (NEW) - Trait + types
- `codex-rs/login/src/providers/mod.rs` (NEW) - Registry
- `codex-rs/login/src/providers/openai.rs` (NEW) - OpenAI impl
- `codex-rs/login/src/providers/google.rs` (NEW) - Google impl
- `codex-rs/login/src/token_store.rs` (NEW) - Token persistence
- `codex-rs/login/src/lib.rs` - Exports
- `codex-rs/tui/src/` - CLI integration point

---

## Testing Commands

```bash
# Phase 6 UI
cargo test -p codex-tui -- token_metrics
cargo check -p codex-tui

# Phase 5
cargo test -p codex-login -- device_code
cargo test -p codex-login -- providers
cargo test -p codex-login -- token_store

# Full validation
cd ~/code/codex-rs
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
```

---

## Commit Strategy

### After UI Integration
```bash
git add -A && git commit -m "$(cat <<'EOF'
feat(tui): Wire TokenMetrics to status bar display

- Render token counts and context utilization in status bar
- Color-coded warnings at 80%/90% context usage
- Cost estimation visible during pipeline runs

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

### After Phase 5
```bash
git add -A && git commit -m "$(cat <<'EOF'
feat(login): Add device code OAuth for OpenAI and Google

- DeviceCodeAuth trait for provider-agnostic flow
- OpenAI device code implementation
- Google device code implementation
- TokenStore for secure token persistence
- Auto-refresh on token expiry
- CLI integration for /model command
- Enables headless/SSH authentication

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Priority Order

1. **Commit Phase 6** (2 min) - Lock in completed work
2. **UI Integration** (30 min) - Wire widget to status bar
3. **Phase 5 Trait** (20 min) - Define DeviceCodeAuth + types
4. **Phase 5 OpenAI** (45 min) - Implement + test
5. **Phase 5 Google** (45 min) - Implement + test
6. **Phase 5 Storage** (20 min) - Token persistence
7. **Phase 5 CLI** (30 min) - Wire to /model command
8. **Final validation** (15 min) - Full test suite

**Total estimated: ~3.5h**

---

## Quick Start Command

```bash
load ~/code/docs/NEXT-SESSION-P6-SYNC-PHASE-5-UI-COMPLETE.md
```
