# P6-SYNC Continuation Prompt: Device Code Auth Integration & Token Refresh UI

**Session Date**: 2025-11-29
**Previous Session Commits**:
- `e3f9bafea` - feat(tui): Add TokenMetrics UI infrastructure (P6-SYNC Phase 6)
- `7a64b5f76` - feat(tui): Wire TokenMetricsWidget to status bar (P6-SYNC Phase 6 UI)
- `5d0d1aced` - feat(login): Add Device Code OAuth flow (P6-SYNC Phase 5)

---

## What Was Completed

### Phase 6: Token Metrics UI (DONE ✅)
- `TokenMetricsWidget` with full/compact rendering
- Context utilization warnings (>80% yellow, >90% red)
- Per-model context windows (OpenAI/Claude/Gemini)
- Wired to status bar during spec-kit pipeline runs
- 7 tests passing

### Phase 5: Device Code OAuth (DONE ✅)
- `DeviceCodeAuth` trait with RFC 8628 device authorization grant
- **OpenAI** device code implementation (`device_code_openai.rs`)
- **Google** device code implementation (`device_code_google.rs`)
- **Anthropic** device code implementation (`device_code_anthropic.rs`)
- Token storage persistence (`~/.codex/device_tokens.json`)
- 23 tests passing

---

## Remaining Work (Prioritized)

### 1. INVESTIGATE: Device Code Auth vs CLI Wrapping (~30 min) **CRITICAL**

**Current Architecture** (`model_router.rs`):
```
User -> /model claude-sonnet-4.5 -> ModelRouter
  -> ProviderType::Claude -> ClaudeProvider
    -> Spawns `claude` CLI subprocess
    -> CLI handles its own OAuth
    -> CLI calls Anthropic API
```

**Device Code Auth Could Enable**:
```
User -> /auth anthropic -> DeviceCodeAuth::start_device_authorization()
  -> User enters code at claude.ai/oauth/device
  -> Poll for token -> Store in device_tokens.json
  -> (Optional) Native API client uses stored token
```

**Key Question**: Device code auth handles **authentication only**. You still need either:
- **Option A**: Keep CLI wrapping (CLI does its own auth, our device code is unused for Claude)
- **Option B**: Build native API clients that use our device code tokens (significant work)
- **Option C**: Hybrid - use device code auth to prime the CLI's token cache (if possible)

**Investigation Tasks**:
1. Check if Claude CLI stores tokens in a readable format (`~/.claude/` or similar)
2. Determine if we can write tokens to CLI's expected location
3. Evaluate if native `AnthropicClient` in `codex-core/src/api_clients/` could use device code tokens

### 2. Token Refresh UI in Status Bar (~45 min)

Add OAuth token status to footer when device code tokens exist:

```rust
// In chat_composer.rs footer rendering:
if let Some(token_status) = &self.device_token_status {
    match token_status.anthropic {
        TokenStatus::Valid => { /* green check */ },
        TokenStatus::NeedsRefresh => { /* yellow warning + auto-refresh */ },
        TokenStatus::Expired => { /* red X + "run /auth anthropic" hint */ },
        TokenStatus::NotAuthenticated => { /* dim "not authed" */ },
    }
}
```

**Implementation**:
1. Add `device_token_status: Option<TokenStatusSummary>` to `ChatComposer`
2. Periodic check in main loop (every 60s) via `DeviceCodeTokenStorage::status_summary()`
3. Auto-refresh logic when `NeedsRefresh` detected
4. Footer renders compact status indicators (✓ / ⚡ / ✗)

### 3. CLI Integration (~1h)

Based on investigation results, implement ONE of:

**If keeping CLI wrapping (Option A)**:
- Add `/auth` command that displays device code flow status
- `/auth status` - shows token status for all providers
- `/auth login <provider>` - runs device code flow
- `/auth logout <provider>` - removes stored tokens
- Note: This is informational only if CLI handles its own auth

**If native API integration (Option B)**:
- Modify `AnthropicClient`, `GeminiClient` to accept tokens from device code storage
- Update `model_router.rs` to use native clients with our tokens instead of CLI subprocess
- Significant refactoring required

**If hybrid (Option C)**:
- Write tokens to CLI's expected format
- Verify CLI picks up externally-written tokens

---

## Startup Verification

```bash
# 1. Verify Phase 6 UI compiles
cd ~/code/codex-rs && cargo check -p codex-tui

# 2. Verify device code auth tests
cargo test -p codex-login --lib

# 3. Check current model router
grep -n "CliRouting\|uses_cli_routing" ~/code/codex-rs/tui/src/model_router.rs | head -10

# 4. Find where Claude CLI stores tokens
ls -la ~/.claude/ 2>/dev/null || echo "No ~/.claude directory"
ls -la ~/.config/claude/ 2>/dev/null || echo "No ~/.config/claude directory"
```

---

## Decision Points

Before implementing CLI integration, answer:

1. **Does Claude CLI expose its token storage?**
   - If yes → Option C (hybrid) is viable
   - If no → Must choose A (informational) or B (full native)

2. **Is native API the goal for this fork?**
   - If yes → Option B, but requires significant client work
   - If no → Option A is sufficient, device code auth becomes "bonus feature"

3. **What about Gemini?**
   - Same analysis needed for `gemini` CLI
   - Check `~/.config/gcloud/` for existing tokens

---

## File Locations

```
# Device Code Auth (completed)
codex-rs/login/src/device_code.rs           # Trait + types
codex-rs/login/src/device_code_openai.rs    # OpenAI impl
codex-rs/login/src/device_code_google.rs    # Google impl
codex-rs/login/src/device_code_anthropic.rs # Anthropic impl
codex-rs/login/src/device_code_storage.rs   # Token persistence

# Token Metrics UI (completed)
codex-rs/tui/src/token_metrics_widget.rs    # Widget
codex-rs/tui/src/bottom_pane/chat_composer.rs # Footer rendering (lines 1888-1924)

# CLI Routing (to investigate)
codex-rs/tui/src/model_router.rs            # Router logic
codex-rs/tui/src/providers/claude.rs        # Claude CLI wrapper
codex-rs/tui/src/providers/gemini.rs        # Gemini CLI wrapper
codex-rs/core/src/api_clients/              # Native clients (if needed)
```

---

## Success Criteria

1. ✅ Device code OAuth working for OpenAI, Google, Anthropic
2. ⏳ Clear decision on CLI vs native API strategy
3. ⏳ Token refresh UI in status bar
4. ⏳ `/auth` command or native integration (based on decision)
5. ⏳ All tests passing, clippy clean

---

## Notes

- User requested **no integration tests** (unit tests sufficient)
- User requested **full token refresh UI** in status bar
- CLI wrapping investigation is **critical path** - determines scope of remaining work
- If CLI stores tokens in standard location, hybrid approach could give best of both worlds
