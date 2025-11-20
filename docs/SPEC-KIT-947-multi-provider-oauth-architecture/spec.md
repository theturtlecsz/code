**SPEC-ID**: SPEC-KIT-947
**Feature**: Multi-Provider OAuth Architecture - Master Validation & Testing
**Status**: Blocked
**Created**: 2025-11-19
**Branch**: TBD
**Owner**: Code
**Priority**: P1 - HIGH
**Type**: MASTER VALIDATION SPEC

**Context**: Master validation and testing spec for multi-provider OAuth architecture. This SPEC depends on research (SPEC-KIT-951) and implementation (SPEC-KIT-952) and serves as the final validation gate before production deployment.

**Objective**: Comprehensive end-to-end testing and validation of multi-provider OAuth switching (ChatGPT, Claude, Gemini) with /model command as central controller.

**Current Pain**: SPEC-KIT-946 expanded /model command to show all 13 models, but AuthMode enum only supports ApiKey and ChatGPT OAuth. Users with Claude/Gemini OAuth subscriptions cannot use those models - selecting Claude Opus produces error: "The 'claude-opus-4.1' model is not supported when using Codex with a ChatGPT account"

---

## Background & Problem Statement

### Current State (2025-11-19)

**What Exists**:
- SPEC-KIT-946 expanded /model command to show all 13 models (Gemini, Claude, GPT-5.1)
- AuthMode enum only supports `ApiKey` and `ChatGPT` OAuth
- Provider detection logic exists (`infer_provider_for_model()`) but doesn't trigger auth switching
- Single OAuth session support only

**Root Cause**:
```rust
// protocol/src/mcp_protocol.rs:98
pub enum AuthMode {
    ApiKey,
    ChatGPT,  // ← Only OAuth provider!
    // Missing: Claude, Gemini
}
```

**User Need**:
- Users have OAuth subscriptions to ChatGPT Plus, Claude Pro, and Gemini Advanced
- Want to select ANY model from /model command
- System should automatically switch to appropriate OAuth session
- NO manual authentication switching required

---

## User Stories

### US1: Multi-Provider Developer (P1)

**As a** developer with ChatGPT Plus, Claude Pro, and Gemini Advanced subscriptions
**I want** to switch between models seamlessly via /model command
**So that** I can use the best model for each task without manual auth changes

**Why this is P1**: Core UX enhancement, blocks multi-provider usage, affects users with multiple paid subscriptions

**How to verify independently**:
1. Login to ChatGPT, Claude, and Gemini
2. Select "Claude Opus 4.1" from /model
3. Verify authentication automatically switches to Claude
4. Send chat message → verify response from Claude Opus

**Scenario**: When developer logged into multiple providers
**Context**: User has active ChatGPT, Claude, and Gemini OAuth sessions
**Action**: User selects "Claude Opus 4.1" from /model command
**Outcome**: Authentication automatically switches to Claude OAuth, next message uses Claude Opus, confirmation shown: "Switched to Claude Opus 4.1 (Claude OAuth)"

**Error Handling**:
**Error Condition**: User selects Claude model but not logged into Claude
**Error Handling**: Show modal: "Claude Opus requires Claude login. Run /login --provider=claude", do NOT switch model (stay on current)

---

### US2: First-Time Claude User (P1)

**As a** user trying Claude for the first time
**I want** clear guidance when I select a Claude model without being logged in
**So that** I know exactly what to do to start using Claude

**Why this is P1**: Prevents user confusion, provides clear path forward, critical for onboarding

**Verification**:
1. Ensure only logged into ChatGPT (not Claude)
2. Select "Claude Opus 4.1" from /model
3. Verify modal shows: "Claude Opus requires Claude login. Run /login --provider=claude"
4. Verify model selection NOT applied (remain on current model)

---

### US3: Login Status Visibility (P2)

**As a** power user with multiple AI subscriptions
**I want** to check which providers I'm logged into
**So that** I can manage my authentication state

**Why P2, not P1**: Nice to have for power users, not critical for basic functionality

**Verification**: Run `/login --status`, verify output shows all providers with status:
```
✅ ChatGPT: logged in (expires in 7 days)
✅ Claude: logged in (expires in 14 days)
❌ Gemini: not logged in
```

---

## Edge Cases

### EC1: Network Failure During Auth Switch
**Boundary Condition**: Network connectivity lost during provider switch
**Handling**: Show error: "Failed to switch to Claude OAuth: network error", remain on current provider, log for debugging

### EC2: Expired Refresh Token
**Concurrent Access**: Refresh token expired or revoked
**Handling**: Show modal: "Claude OAuth session expired. Please run /login --provider=claude", clear stored tokens, prevent further API calls with invalid tokens

### EC3: Multiple Simultaneous Model Switches
**Error Recovery**: User rapidly switches between models (ChatGPT → Claude → Gemini in <1s)
**Handling**: Queue auth switches, process sequentially, show final confirmation only, prevent race conditions with proper locking

### EC4: Provider API Down
**Performance Limit**: OAuth provider endpoint unavailable (500 error, timeout)
**Handling**: Show error: "Claude OAuth unavailable, please try again later", allow fallback to other logged-in providers, log incident for monitoring

---

## Requirements

### Functional Requirements

#### FR1: Expand AuthMode Enum

**File**: `protocol/src/mcp_protocol.rs:98`

**Requirement**: Add Claude and Gemini variants to AuthMode enum

**Acceptance Criteria**:
- AuthMode enum includes `Claude` and `Gemini` variants
- Serialization/deserialization works (serde, TypeScript bindings if applicable)
- All existing code using AuthMode compiles without errors

---

#### FR2: Implement OAuth 2.0 Flows

**Files**: `core/src/auth.rs`, `login/src/server.rs`

**Requirement**: Implement Claude and Gemini OAuth 2.0 flows with PKCE

**Claude OAuth**:
- Authorization URL: https://auth.anthropic.com/authorize
- Token endpoint: https://auth.anthropic.com/oauth/token
- PKCE support (code_challenge, code_verifier)

**Gemini OAuth**:
- Authorization URL: https://accounts.google.com/o/oauth2/v2/auth
- Token endpoint: https://oauth2.googleapis.com/token
- Scopes: https://www.googleapis.com/auth/generative-language

**Token Storage** (`~/.code/auth.json`):
```json
{
  "chatgpt": { "access_token": "...", "refresh_token": "...", "expires_at": "..." },
  "claude": { ... },
  "gemini": { ... }
}
```

**Acceptance Criteria**:
- `/login --provider=claude` initiates Claude OAuth flow
- `/login --provider=gemini` initiates Gemini OAuth flow
- Tokens stored separately per provider
- Token refresh works for all providers
- Multiple simultaneous OAuth sessions supported

---

#### FR3: Multi-Provider AuthManager

**File**: `core/src/auth.rs:770` (AuthManager struct)

**Requirement**: Support multiple active OAuth sessions with dynamic switching

**New Methods**:
```rust
impl AuthManager {
    pub fn switch_provider(&self, provider: AuthMode) -> Result<()>;
    pub fn is_logged_in(&self, provider: AuthMode) -> bool;
    pub fn logged_in_providers(&self) -> Vec<AuthMode>;
    pub fn refresh_all_tokens(&self) -> Result<()>;
}
```

**Acceptance Criteria**:
- AuthManager holds tokens for ChatGPT + Claude + Gemini simultaneously
- `switch_provider()` changes active provider in <500ms
- Token refresh works for inactive providers (background refresh)
- Thread-safe provider switching

---

#### FR4: Auto-Switch on Model Selection

**File**: `tui/src/chatwidget/mod.rs:~9847` (apply_model_selection)

**Requirement**: Automatically switch authentication when user selects a model

**Implementation**:
```rust
if let Some((provider, auth_method)) = Self::infer_provider_for_model(&self.config.model) {
    self.config.model_provider_id = provider.to_string();

    // NEW: Switch authentication provider
    if let Err(e) = self.auth_manager.switch_provider(auth_method) {
        self.show_auth_required_modal(auth_method, &self.config.model);
        return;
    }

    self.config.active_auth_mode = auth_method;
    self.save_config();
    self.show_confirmation(&format!("Switched to {} ({})", self.config.model, auth_method_display(auth_method)));
}
```

**Acceptance Criteria**:
- Selecting Claude model → switches to Claude OAuth
- Selecting Gemini model → switches to Gemini OAuth
- Selecting GPT model → switches to ChatGPT OAuth
- Not logged in → show modal with `/login` command
- No errors or authentication failures
- Confirmation message shown

---

#### FR5: Config Persistence

**File**: `core/src/config.rs`

**Requirement**: Replace `using_chatgpt_auth: bool` with `active_auth_provider: Option<AuthMode>`

**Config Migration**:
- Old: `using_chatgpt_auth = true` → New: `active_auth_provider = Some(AuthMode::ChatGPT)`
- Old: `using_chatgpt_auth = false` → New: `active_auth_provider = Some(AuthMode::ApiKey)`

**Acceptance Criteria**:
- Config persists active auth provider in `~/.code/config.toml`
- TUI restores correct OAuth session on restart
- Config migration works seamlessly, no data loss

---

#### FR6: Enhanced /login Command

**File**: `tui/src/chatwidget/mod.rs` (command handling)

**Requirement**: Add `--provider` and `--status` flags

**Commands**:
```bash
/login --provider=chatgpt  # Default (backward compatible)
/login --provider=claude   # NEW
/login --provider=gemini   # NEW
/login --status            # Show all providers
```

**Acceptance Criteria**:
- `/login` without args defaults to ChatGPT
- `/login --provider=<name>` initiates OAuth for that provider
- `/login --status` shows all provider login states with expiry times
- Login flow works in TUI (opens browser, returns on success)

---

#### FR7: Model Picker UI

**File**: `tui/src/bottom_pane/model_selector_view.rs` (or similar)

**Requirement**: Show auth status for each model in picker

**Display**:
```
Gemini 3 Pro (LMArena #1) — $2/$12 ✅ Logged in
Claude Opus 4.1 (ultra premium) — $15/$75 ⚠️  Login required
GPT-5.1 Medium (default) — $1.25/$10 ✅ Active
```

**Acceptance Criteria**:
- Logged-in providers show ✅ indicator
- Login-required show ⚠️ indicator
- Currently active model highlighted
- Selecting login-required shows helper message

---

### Non-Functional Requirements

#### NFR1: Performance
**Metric**: Auth switch latency <500ms
**Verification**: Measure time from model selection to auth switch complete, use performance benchmarks

#### NFR2: Security
**Requirement**: Token storage encryption (platform keychain or file encryption)
**Verification**: Security audit of token storage, PKCE implementation, OAuth flow

#### NFR3: Reliability
**Requirement**: Token refresh works automatically, no manual intervention required
**Uptime Target**: 99% successful auth switches (network conditions permitting)
**Monitoring**: Telemetry logs for auth failures, token refresh errors

---

## Architecture Changes

### Component 1: Protocol Layer (2-3 hours)
- Expand AuthMode enum
- Update serialization tests
- Regenerate TypeScript bindings
- **Estimated LOC**: +3

### Component 2: Authentication Layer (8-12 hours)
- Implement Claude OAuth flow
- Implement Gemini OAuth flow
- Multi-provider token storage
- Token refresh for all providers
- **Estimated LOC**: +300-400

### Component 3: AuthManager Refactor (4-6 hours)
- Multi-session support
- `switch_provider()` method
- Background token refresh
- **Estimated LOC**: +150-200

### Component 4: TUI Integration (3-5 hours)
- Update `apply_model_selection()`
- Enhance /model picker
- `/login` enhancements
- **Estimated LOC**: +130-200

### Component 5: Error Handling & UX (2-3 hours)
- Modals, confirmations, error messages
- **Estimated LOC**: +50-80

**Total Estimated**: 19-29 hours (3-4 days full-time), ~630-880 LOC

---

## Success Criteria

### Measurable Outcomes

**MO1**: User selects any model from /model → authentication automatically switches → no errors (100% success rate for logged-in providers)

**MO2**: Multiple simultaneous OAuth sessions work (ChatGPT + Claude + Gemini all logged in at once)

**MO3**: Config persists active auth provider across TUI restarts (100% config persistence)

**MO4**: Token refresh works for all providers (0% manual token refresh required)

**MO5**: All tests pass (≥80% code coverage for new auth code)

---

## Testing Requirements

### Test Coverage

**Target**: ≥80% code coverage for new authentication code

**Test Scenarios**:
1. OAuth flow completion (ChatGPT, Claude, Gemini)
2. Token refresh (active and inactive providers)
3. Provider switching latency (<500ms)
4. Error handling (network failures, invalid tokens, expired tokens)
5. Config migration (old → new format)
6. Multi-session management
7. Concurrent model switches (race conditions)

### Critical Paths

**Main Workflows**:
1. Login to multiple providers → switch models → verify correct auth
2. Select model when not logged in → see modal
3. Token expires → auto-refresh → no disruption
4. TUI restart → restore last used provider

---

## Dependencies

### Upstream Dependencies (BLOCKERS)

**SPEC-KIT-951**: ⚠️ RESEARCH PHASE (P0 - CRITICAL)
- OAuth credential acquisition research
- OAuth flow specifications
- Security architecture validation
- Token management strategy
- Provider requirements documentation
- **Status**: Must complete before SPEC-KIT-952 begins

**SPEC-KIT-952**: ⚠️ IMPLEMENTATION (P1 - HIGH, will be created)
- Multi-provider AuthManager implementation
- OAuth 2.0 flows (Claude, Gemini, ChatGPT)
- Token storage and refresh
- TUI integration (model selection auto-switch)
- Config persistence
- **Status**: Waits for SPEC-KIT-951 GO decision

**SPEC-KIT-946**: ✅ COMPLETE
- Provides 13-model presets
- Provider detection logic (`infer_provider_for_model()`)
- Model picker infrastructure

### Relationship to Other SPECs

This is a **MASTER VALIDATION SPEC** that depends on:
1. **SPEC-KIT-951** (Research) → validates feasibility, documents OAuth requirements
2. **SPEC-KIT-952** (Implementation) → builds the actual multi-provider OAuth system
3. **SPEC-KIT-947** (This spec) → validates end-to-end functionality before production

**Workflow**:
```
SPEC-951 (Research) → GO/NO-GO decision
    ↓ GO
SPEC-952 (Implementation) → Code complete
    ↓
SPEC-947 (Validation) → Production ready
```

---

## Open Questions & Decisions

### Q1: OAuth Credentials Acquisition (Priority: HIGH, Blocker: YES)

**Question**: How to obtain official OAuth client credentials for Claude and Gemini?

**Answer/Decision**: Research required
- Investigate Anthropic Developer Portal for Claude OAuth app registration
- Create Google Cloud project for Gemini OAuth credentials
- Alternative: Allow users to provide their own credentials
- Document credential setup process

---

### Q2: Token Encryption Strategy (Priority: MEDIUM, Blocker: NO)

**Question**: Should tokens be encrypted in auth.json? Which encryption method?

**Answer/Decision**: Define during implementation
- Evaluate platform keychain libraries (macOS Keychain, Linux Secret Service)
- Fallback to file encryption with derived key
- Security review required

---

### Q3: Background Token Refresh (Priority: MEDIUM, Blocker: NO)

**Question**: When and how often to refresh tokens for inactive providers?

**Answer/Decision**: Define during Component 3 implementation
- Options: Daily background refresh, lazy refresh on switch, expiry-based refresh
- Test with real OAuth tokens
- Monitor for excessive API calls

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-11-19 | Initial SPEC created from user prompt |

---

## Notes

Created via manual SPEC generation from comprehensive user requirements. This SPEC builds on SPEC-KIT-946 (model command expansion) to enable true multi-provider OAuth support.

**Next Steps**:
- Run `/speckit.clarify SPEC-KIT-947` to resolve open questions
- Run `/speckit.plan SPEC-KIT-947` to generate detailed implementation plan
- Or run `/speckit.auto SPEC-KIT-947` for full automated pipeline

**Related Files**:
- See `docs/SPEC-KIT-947-multi-provider-oauth-architecture/PRD.md` for comprehensive requirements
- See `docs/SPEC-KIT-946-model-command-expansion/` for dependency SPEC
