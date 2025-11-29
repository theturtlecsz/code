# P6-SYNC Continuation Prompt: Interactive Device Code Login

**Session Date**: 2025-11-29
**Previous Session Commits**:
- `3fc9749c5` - feat(tui): Add Token Refresh UI and /auth command (P6-SYNC Phase 5 UI)
- `5d0d1aced` - feat(login): Add Device Code OAuth flow (P6-SYNC Phase 5)
- `7a64b5f76` - feat(tui): Wire TokenMetricsWidget to status bar (P6-SYNC Phase 6 UI)

---

## What Was Completed

### P6-SYNC Phase 5 + 6 (DONE)
- Device Code OAuth implementations (OpenAI, Google, Anthropic) - 23 tests passing
- Token Metrics UI widget with context warnings
- Token Refresh UI in footer (O/C/G✓⚡✗· indicators)
- `/auth` command (status, login, logout subcommands)
- Token status check on startup

---

## Remaining Work (Prioritized)

### 1. PRIMARY: Interactive Device Code Login (~2-3h)

**Current State**: `/auth login <provider>` shows instructions but doesn't run actual flow.

**Implementation Tasks**:

#### 1.1 Create Device Code Login UI Component
```rust
// New file: tui/src/device_code_login_view.rs
// Similar pattern to LoginAddAccountView

pub struct DeviceCodeLoginView {
    provider: DeviceCodeProvider,
    state: DeviceCodeLoginState,
    user_code: String,
    verification_uri: String,
    expires_at: Instant,
    poll_interval: Duration,
}

enum DeviceCodeLoginState {
    Starting,           // Requesting device code
    WaitingForUser,     // Displaying code, polling
    Success,            // Token received
    Error(String),      // Failed
    Expired,            // Device code expired
}
```

#### 1.2 Implement Poll Loop with TUI Updates
```rust
// In handle_auth_command login branch:
// 1. Start device authorization
// 2. Show view with user_code and verification_uri
// 3. Open browser automatically (optional)
// 4. Poll in background, update UI on each attempt
// 5. On success: store token, update footer, show confirmation
// 6. On error/expiry: show retry option
```

#### 1.3 Wire to Bottom Pane
- Add `show_device_code_login(view)` method to BottomPane
- Handle escape to cancel, enter to retry

#### 1.4 Test Coverage
- Unit tests for state transitions
- Mock server tests for poll responses

**Files to Create/Modify**:
```
codex-rs/tui/src/device_code_login_view.rs  # NEW
codex-rs/tui/src/bottom_pane/mod.rs         # Add view handler
codex-rs/tui/src/chatwidget/mod.rs          # Update handle_auth_command
codex-rs/login/src/device_code.rs           # Add poll_until_complete helper
```

---

### 2. OPTIONAL A: CLI Token Sync (Hybrid) (~1h)

**Goal**: Write device code tokens to CLI's expected format so CLI tools can use them.

**Claude CLI Format** (`~/.claude/.credentials.json`):
```json
{
  "claudeAiOauth": {
    "accessToken": "sk-ant-oat01-...",
    "refreshToken": "sk-ant-ort01-...",
    "expiresAt": 1764419119047,  // milliseconds
    "scopes": ["user:inference", "user:profile", "user:sessions:claude_code"]
  }
}
```

**Implementation**:
```rust
// In device_code_storage.rs:
pub fn sync_to_cli_format(&self, provider: DeviceCodeProvider) -> Result<(), StorageError> {
    match provider {
        DeviceCodeProvider::Anthropic => self.write_claude_cli_format(),
        DeviceCodeProvider::Google => self.write_gcloud_adc_format(),
        DeviceCodeProvider::OpenAI => Ok(()), // Native format already compatible
    }
}
```

**Risks**:
- CLI format could change between versions
- May interfere with CLI's own token management
- Need to handle timestamp format differences (ms vs s)

---

### 3. OPTIONAL B: Native API Fallback (~1.5h)

**Goal**: Use device code tokens with native API clients if CLI is unavailable.

**Current Architecture**:
```
User -> /model claude-sonnet -> ModelRouter
  -> ProviderType::Claude -> ClaudeProvider (CLI subprocess)
```

**Proposed Fallback**:
```
User -> /model claude-sonnet -> ModelRouter
  -> ProviderType::Claude -> ClaudeProvider (CLI)
    -> IF CLI unavailable OR auth failed:
       -> DeviceCodeTokenStorage.get_token(Anthropic)
       -> AnthropicClient.with_token(token)
       -> Native API call
```

**Implementation**:
1. Add fallback path in `model_router.rs`
2. Modify `AnthropicClient` to accept external token
3. Same for `GeminiClient`
4. Add configuration option to enable/disable fallback

---

### 4. DOCUMENTATION FIX (Quick)

**CLAUDE.md line 25** incorrectly states:
> "Gemini CLI routing disabled (see Known Limitations)"

**Reality**: Gemini CLI routing IS enabled and working (see `model_router.rs:119`).

Fix:
```markdown
- **Multi-provider model support (SPEC-KIT-952):** Claude and Gemini models route through native CLI with streaming support.
```

---

## Startup Verification

```bash
# 1. Verify P6-SYNC Phase 5 UI compiles
cd ~/code/codex-rs && cargo check -p codex-tui

# 2. Run device code auth tests
cargo test -p codex-login --lib

# 3. Verify CLI routing for all providers
grep -n "execute_.*_prompt\|is_available" ~/code/codex-rs/tui/src/model_router.rs | head -15

# 4. Check current /auth command behavior
# Launch TUI and run: /auth status
```

---

## Success Criteria

### PRIMARY (Must Complete)
1. [ ] `/auth login openai` runs interactive device code flow
2. [ ] `/auth login claude` runs interactive device code flow
3. [ ] `/auth login google` runs interactive device code flow
4. [ ] User code and verification URL displayed in TUI
5. [ ] Background polling with progress indication
6. [ ] Token stored on success, footer updates
7. [ ] Proper error handling (expiry, cancellation, network errors)

### OPTIONAL A - CLI Token Sync
1. [ ] Tokens written to `~/.claude/.credentials.json` for Anthropic
2. [ ] Tokens written to ADC format for Google (if applicable)
3. [ ] Version/format detection for safety

### OPTIONAL B - Native API Fallback
1. [ ] Fallback triggers when CLI unavailable
2. [ ] Native client uses stored device code token
3. [ ] Configuration option to enable/disable

### Documentation
1. [ ] Fix CLAUDE.md Gemini routing statement
2. [ ] Add P6-SYNC completion status to CLAUDE.md

---

## Implementation Order

1. **Start with OpenAI** - Simplest, well-documented device code flow
2. **Then Anthropic** - Our tokens already match their format closely
3. **Finally Google** - More complex OAuth with multiple scopes
4. **Optional items** - Only after primary flow works

---

## File Locations Reference

```
# Device Code Auth (existing)
codex-rs/login/src/device_code.rs           # Trait + types
codex-rs/login/src/device_code_openai.rs    # OpenAI impl
codex-rs/login/src/device_code_google.rs    # Google impl
codex-rs/login/src/device_code_anthropic.rs # Anthropic impl
codex-rs/login/src/device_code_storage.rs   # Token persistence

# Token UI (existing)
codex-rs/tui/src/token_metrics_widget.rs    # Widget
codex-rs/tui/src/bottom_pane/chat_composer.rs # Footer rendering

# CLI Routing (existing)
codex-rs/tui/src/model_router.rs            # Router logic
codex-rs/tui/src/providers/claude.rs        # Claude CLI wrapper
codex-rs/tui/src/providers/gemini.rs        # Gemini CLI wrapper
codex-rs/core/src/api_clients/              # Native clients

# New files (to create)
codex-rs/tui/src/device_code_login_view.rs  # Interactive login UI
```

---

## Notes

- User requested **NO periodic background token refresh** - manual `/auth status` only
- Gemini CLI routing IS working (CLAUDE.md documentation is wrong)
- Token format differences: our storage uses seconds, Claude CLI uses milliseconds
- Test with actual device code flows before marking complete
