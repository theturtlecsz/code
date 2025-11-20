# PRD: Multi-Provider OAuth Architecture

**SPEC-ID**: SPEC-KIT-947
**Status**: Draft
**Created**: 2025-11-19
**Author**: User Request
**Priority**: P1 - HIGH

---

## Executive Summary

Enable automatic multi-provider OAuth authentication switching in the TUI, with `/model` command as the central controller for both model selection AND authentication provider. Users should seamlessly switch between ChatGPT, Claude, and Gemini models without manual authentication changes.

**Current State**:
- SPEC-KIT-946 expanded `/model` command to show all 13 models (Gemini, Claude, GPT-5.1)
- AuthMode enum only supports ApiKey and ChatGPT OAuth
- Users with Claude/Gemini OAuth subscriptions cannot use those models in chat
- Error when selecting Claude Opus: "The 'claude-opus-4.1' model is not supported when using Codex with a ChatGPT account"

**Proposed Solution**:
- Expand AuthMode enum to support Claude and Gemini OAuth
- Implement OAuth 2.0 flows for both providers
- Auto-switch authentication when user selects a model
- Support multiple simultaneous OAuth sessions

---

## Problem Statement

### Current State

**What Exists Today**:
- Single OAuth provider support (ChatGPT only)
- Manual authentication switching not supported
- 13 models displayed in `/model` command (SPEC-KIT-946)
- Provider detection logic exists but doesn't trigger auth switching

**Pain Points**:
1. Users with multiple OAuth subscriptions (ChatGPT Plus, Claude Pro, Gemini Advanced) cannot use all their models
2. Selecting a Claude/Gemini model results in error instead of prompting for login
3. No way to maintain multiple OAuth sessions simultaneously
4. Manual workarounds required (changing auth config files)

**Impact**:
- Users cannot utilize their paid subscriptions to multiple AI providers
- Poor user experience - error messages instead of helpful guidance
- Feature parity gap - `/model` command shows models that can't be used

### Root Cause

```rust
// protocol/src/mcp_protocol.rs:98
pub enum AuthMode {
    ApiKey,
    ChatGPT,  // ← Only OAuth provider!
    // Missing: Claude, Gemini
}
```

The authentication system was designed for single-provider OAuth, limiting multi-provider support.

---

## Goals & Success Criteria

### Primary Goals

**G1**: Seamless multi-provider OAuth switching
**How to Measure**: User selects any model from `/model` → authentication automatically switches → no errors

**G2**: Support multiple simultaneous OAuth sessions
**How to Measure**: User logged into ChatGPT + Claude + Gemini simultaneously, can switch between them instantly

**G3**: Maintain quality and performance
**Metric**: 100% test pass rate, <500ms auth switch latency, no regressions

### Secondary Goals

- Documentation complete (OAuth setup guides, troubleshooting)
- Clear user feedback during auth operations
- Token refresh works automatically in background

### Non-Goals

**What We Won't Do**:
- Enterprise SSO integration (Future enhancement)
- Custom OAuth providers (Stick to ChatGPT, Claude, Gemini)
- Multi-user authentication (Single-user TUI focus)

**Why These Are Non-Goals**: Focus on core multi-provider functionality first. Enterprise features can be added later based on user demand.

---

## Scope

### Included Features

**FR1**: Expand AuthMode enum to include Claude and Gemini variants

**FR2**: Implement Claude OAuth 2.0 flow
- Authorization URL: https://auth.anthropic.com/authorize
- Token endpoint: https://auth.anthropic.com/oauth/token
- PKCE support (code challenge/verifier)

**FR3**: Implement Gemini OAuth 2.0 flow
- Authorization URL: https://accounts.google.com/o/oauth2/v2/auth
- Token endpoint: https://oauth2.googleapis.com/token
- Scopes: https://www.googleapis.com/auth/generative-language

**FR4**: Multi-provider AuthManager
- Store tokens for all providers simultaneously
- Switch active provider dynamically
- Background token refresh for inactive providers

**FR5**: Auto-switch on model selection
- Detect provider from selected model
- Switch AuthManager to appropriate provider
- Show helpful modal if not logged in

**FR6**: Enhanced `/login` command
- `--provider=<chatgpt|claude|gemini>` flag
- `--status` flag to show all logged-in providers

**FR7**: Model picker UI enhancements
- Show auth status for each model (✅ logged in, ⚠️ login required)
- Indicate currently active model
- Helper text for login-required models

---

## Assumptions & Constraints

### Assumptions

**A1**: OAuth client credentials for Claude/Gemini will be obtained
- This may require developer program enrollment or partnership
- Alternatively, users provide their own credentials

**A2**: Required dependencies are available
- OAuth 2.0 client library (oauth2 crate or similar)
- PKCE support in OAuth library
- Token storage mechanism exists

**A3**: No breaking changes in provider OAuth APIs
- Claude and Gemini OAuth endpoints remain stable
- Token format/lifetime expectations are met

### Technical Constraints

**C1**: Must maintain backward compatibility
- Existing ChatGPT OAuth users continue working
- Config migration from `using_chatgpt_auth` boolean
- Legacy auth.json format supported during migration

**C2**: Security requirements
- Token encryption in storage (investigate platform keychain integration)
- PKCE flow for all OAuth providers (security best practice)
- Secure token refresh mechanism

**C3**: UX constraints
- Auth switch must be fast (<500ms target)
- Clear error messages when login required
- No breaking changes to existing `/model` command behavior

### Resource Constraints

**Time**: 19-29 hours estimated (3-4 days full-time)
**Complexity**: MEDIUM RISK - well-scoped but security-critical

---

## Detailed Requirements

### FR1: Expand AuthMode Enum

**File**: `protocol/src/mcp_protocol.rs` (line 98)

**Current**:
```rust
pub enum AuthMode {
    ApiKey,
    ChatGPT,
}
```

**Required**:
```rust
pub enum AuthMode {
    ApiKey,
    ChatGPT,
    Claude,   // ← NEW: Anthropic OAuth
    Gemini,   // ← NEW: Google OAuth
}
```

**Acceptance Criteria**:
- [ ] AuthMode enum includes Claude and Gemini variants
- [ ] Serialization/deserialization works (serde, TypeScript bindings)
- [ ] All existing code using AuthMode compiles without errors
- [ ] Tests updated for new variants

---

### FR2: OAuth Flow Implementation

**Files**: `core/src/auth.rs`, `login/src/server.rs`

**Requirements**:

**Claude OAuth 2.0**:
- Authorization URL: `https://auth.anthropic.com/authorize`
- Token endpoint: `https://auth.anthropic.com/oauth/token`
- PKCE support (code_challenge, code_verifier)
- Scopes: TBD (research required)

**Gemini OAuth 2.0**:
- Authorization URL: `https://accounts.google.com/o/oauth2/v2/auth`
- Token endpoint: `https://oauth2.googleapis.com/token`
- Scopes: `https://www.googleapis.com/auth/generative-language`
- PKCE support

**Token Storage**:
```json
{
  "chatgpt": {
    "access_token": "...",
    "refresh_token": "...",
    "expires_at": "..."
  },
  "claude": {
    "access_token": "...",
    "refresh_token": "...",
    "expires_at": "..."
  },
  "gemini": {
    "access_token": "...",
    "refresh_token": "...",
    "expires_at": "..."
  }
}
```

**Acceptance Criteria**:
- [ ] `/login --provider=claude` initiates Claude OAuth flow
- [ ] `/login --provider=gemini` initiates Gemini OAuth flow
- [ ] Tokens stored separately per provider in `~/.code/auth.json`
- [ ] Token refresh works for all providers
- [ ] Multiple simultaneous OAuth sessions supported
- [ ] PKCE flow implemented for security
- [ ] Token encryption (if feasible)

---

### FR3: AuthManager Multi-Provider Support

**File**: `core/src/auth.rs` (AuthManager struct, line 770)

**Current Limitation**:
```rust
pub struct AuthManager {
    codex_home: PathBuf,
    originator: String,
    inner: RwLock<CachedAuth>,  // ← Single auth session
}
```

**Required Changes**:
- Support multiple active OAuth sessions simultaneously
- New method: `switch_provider(provider: AuthMode) -> Result<()>`
- Load tokens for ALL logged-in providers at startup
- Switch active provider dynamically without re-login

**New Methods**:
```rust
impl AuthManager {
    /// Switch active OAuth provider
    pub fn switch_provider(&self, provider: AuthMode) -> Result<()>;

    /// Check if logged into a specific provider
    pub fn is_logged_in(&self, provider: AuthMode) -> bool;

    /// Get all logged-in providers
    pub fn logged_in_providers(&self) -> Vec<AuthMode>;

    /// Refresh tokens for all providers (background task)
    pub fn refresh_all_tokens(&self) -> Result<()>;
}
```

**Acceptance Criteria**:
- [ ] AuthManager can hold tokens for ChatGPT + Claude + Gemini simultaneously
- [ ] `switch_provider()` changes active provider instantly (<500ms)
- [ ] `preferred_auth_method()` returns currently active provider
- [ ] Token refresh works for inactive providers (background refresh)
- [ ] Thread-safe provider switching (proper locking)

---

### FR4: Model Selection Auto-Switches Auth

**File**: `tui/src/chatwidget/mod.rs` (`apply_model_selection`, line ~9847)

**Current** (from SPEC-KIT-946):
```rust
// Detects provider but doesn't switch auth
if let Some((provider, auth_method)) = Self::infer_provider_for_model(&self.config.model) {
    self.config.model_provider_id = provider.to_string();
    // ← Missing: Actually switch AuthManager to this provider!
}
```

**Required**:
```rust
if let Some((provider, auth_method)) = Self::infer_provider_for_model(&self.config.model) {
    // Update provider
    self.config.model_provider_id = provider.to_string();

    // **NEW: Switch authentication provider**
    if let Err(e) = self.auth_manager.switch_provider(auth_method) {
        // Handle not logged in to this provider
        self.show_auth_required_modal(auth_method, &self.config.model);
        return;
    }

    // Update active auth mode
    self.config.active_auth_mode = auth_method;

    // Save config
    self.save_config();

    // Show confirmation
    self.show_confirmation(&format!(
        "Switched to {} ({})",
        self.config.model,
        auth_method_display(auth_method)
    ));
}
```

**Acceptance Criteria**:
- [ ] Selecting a Claude model switches to Claude OAuth automatically
- [ ] Selecting a Gemini model switches to Gemini OAuth automatically
- [ ] Selecting a GPT model switches to ChatGPT OAuth automatically
- [ ] If not logged in → show friendly modal: "Claude Opus requires Claude login. Run `/login --provider=claude`"
- [ ] No errors or authentication failures
- [ ] Confirmation message shown: "Switched to Claude Opus 4.1 (Claude OAuth)"
- [ ] Config persisted automatically

---

### FR5: Config Persistence

**File**: `core/src/config.rs`

**Current**:
```rust
pub struct Config {
    // ...
    pub using_chatgpt_auth: bool,  // ← OLD: Boolean flag
    // ...
}
```

**Required**:
```rust
pub struct Config {
    // ...
    pub active_auth_provider: Option<AuthMode>,  // ← NEW: Track active OAuth
    // ...
}
```

**Config file** (`~/.code/config.toml`):
```toml
model = "claude-opus-4.1"
active_auth_provider = "claude"  # ← NEW: Tracks active OAuth
```

**Migration**:
```rust
// Old config with using_chatgpt_auth = true
// → Migrate to active_auth_provider = Some(AuthMode::ChatGPT)

// Old config with using_chatgpt_auth = false
// → Migrate to active_auth_provider = Some(AuthMode::ApiKey)
```

**Acceptance Criteria**:
- [ ] Config persists active auth provider
- [ ] TUI restores correct OAuth session on restart
- [ ] Switching models updates `active_auth_provider` in config
- [ ] Config migration from `using_chatgpt_auth` boolean works seamlessly
- [ ] No data loss during migration

---

### FR6: `/login` Command Enhancement

**File**: `tui/src/chatwidget/mod.rs` (command handling)

**Required Commands**:
```bash
# New provider flag
/login --provider=chatgpt  # ← Default (existing)
/login --provider=claude   # ← NEW
/login --provider=gemini   # ← NEW

# Show all logged-in providers
/login --status
# Output:
#   ✅ ChatGPT: logged in (expires in 7 days)
#   ✅ Claude: logged in (expires in 14 days)
#   ❌ Gemini: not logged in
```

**Acceptance Criteria**:
- [ ] `/login` without args defaults to ChatGPT (backward compatible)
- [ ] `/login --provider=<name>` initiates OAuth for that provider
- [ ] `/login --status` shows all provider login states with expiry times
- [ ] Login flow works in TUI (opens browser, returns to TUI on success)
- [ ] Error handling for invalid provider names
- [ ] Help text documents new flags

---

### FR7: Model Picker UI Enhancement

**File**: `tui/src/bottom_pane/model_selector_view.rs` (or similar)

**Required Display**:
```
Gemini 3 Pro (LMArena #1) — $2/$12 ✅ Logged in
Claude Opus 4.1 (ultra premium) — $15/$75 ⚠️  Login required
GPT-5.1 Medium (default) — $1.25/$10 ✅ Active
```

**Legend**:
- `✅` = Logged in to this provider
- `⚠️` = Login required (not logged in)
- Bold or highlighted = Currently active model

**Acceptance Criteria**:
- [ ] Models for logged-in providers show ✅ indicator
- [ ] Models requiring login show ⚠️ indicator
- [ ] Currently active model highlighted differently (bold, color, arrow)
- [ ] Selecting a "login required" model shows helper: "Run `/login --provider=claude` first"
- [ ] Status updates in real-time when user logs in/out

---

## Architecture & Implementation

### Component 1: Protocol Layer (2-3 hours)

**Files**:
- `protocol/src/mcp_protocol.rs`

**Tasks**:
1. Expand AuthMode enum (Claude, Gemini)
2. Update serialization tests
3. Regenerate TypeScript bindings (if applicable)
4. Update documentation

**Estimated LOC**: +3

---

### Component 2: Authentication Layer (8-12 hours)

**Files**:
- `core/src/auth.rs`
- `login/src/server.rs`

**Tasks**:
1. Implement Claude OAuth 2.0 flow
   - Research Claude OAuth endpoints and scopes
   - Authorization URL handling
   - Token exchange with PKCE
2. Implement Gemini OAuth 2.0 flow
   - Authorization URL handling
   - Token exchange with PKCE
   - Scope configuration
3. Multi-provider token storage
   - Update auth.json schema
   - Migration from old format
4. Token refresh for all providers
5. Error handling and retry logic

**Estimated LOC**: +300-400

---

### Component 3: AuthManager Refactor (4-6 hours)

**Files**:
- `core/src/auth.rs`

**Tasks**:
1. Support multiple simultaneous OAuth sessions
2. Implement `switch_provider()` method
3. Load all provider tokens at startup
4. Background token refresh mechanism
5. Provider availability checking
6. Thread-safe state management

**Estimated LOC**: +150-200

---

### Component 4: TUI Integration (3-5 hours)

**Files**:
- `tui/src/chatwidget/mod.rs`
- `tui/src/bottom_pane/model_selector_view.rs`

**Tasks**:
1. Update `apply_model_selection()` to switch auth
2. Enhance `/model` picker with auth status
3. Add `--provider` flag to `/login` command
4. Implement `/login --status` command
5. Config persistence integration
6. UI indicators for auth status

**Estimated LOC**: +130-200

---

### Component 5: Error Handling & UX (2-3 hours)

**Files**:
- Various (modals, error messages)

**Tasks**:
1. "Not logged in" modal with helpful instructions
2. Auth switch confirmation messages
3. Login expiry warnings
4. Graceful fallback when provider unavailable
5. Error message improvements

**Estimated LOC**: +50-80

---

**Total Estimated Effort**: 19-29 hours (3-4 days full-time)
**Total Estimated LOC**: ~630-880 LOC

---

## User Stories

### US1: Multi-Provider Developer (Priority: P1)

**As a**: Developer with ChatGPT Plus, Claude Pro, and Gemini Advanced subscriptions
**I want**: To switch between models seamlessly via `/model` command
**So that**: I can use the best model for each task without manual auth changes

**Today They Work**: Manually edit config files to change auth provider, then restart TUI

**What Frustrates Them**:
- Can't use all their paid subscriptions
- Error messages instead of helpful guidance
- Manual configuration changes required

**What They Want**:
- Select any model from `/model` → just works
- Automatic auth switching behind the scenes

**Acceptance Test**:
```
GIVEN I'm logged into ChatGPT, Claude, and Gemini
WHEN I select "Claude Opus 4.1" from /model
THEN authentication automatically switches to Claude
AND my next chat message uses Claude Opus
AND I see confirmation: "Switched to Claude Opus 4.1 (Claude OAuth)"
```

---

### US2: First-Time Claude User (Priority: P1)

**As a**: User trying Claude for the first time
**I want**: Clear guidance when I select a Claude model without being logged in
**So that**: I know exactly what to do to start using Claude

**Acceptance Test**:
```
GIVEN I'm only logged into ChatGPT
WHEN I select "Claude Opus 4.1" from /model
THEN I see a modal: "Claude Opus requires Claude login. Run /login --provider=claude"
AND the model selection is NOT applied
AND I remain on my current model
```

---

### US3: Multi-Session User (Priority: P2)

**As a**: Power user with multiple AI subscriptions
**I want**: To check which providers I'm logged into
**So that**: I can manage my authentication state

**Acceptance Test**:
```
GIVEN I run /login --status
THEN I see:
  ✅ ChatGPT: logged in (expires in 7 days)
  ✅ Claude: logged in (expires in 14 days)
  ❌ Gemini: not logged in
```

---

## User Flows

### Primary User Flow: Seamless Model Switching

**Flow**: Using Multi-Provider OAuth

**Step 1**: User selects "Claude Opus 4.1" from `/model` command
**System Response**: Detects Claude provider, checks auth status

**Step 2**: System switches to Claude OAuth session
**System Response**: AuthManager activates Claude tokens

**Step 3**: User sends chat message
**System Response**: Message sent using Claude Opus with Claude OAuth

**Happy Path Outcome**: User seamlessly uses Claude without manual auth changes

---

### Error Handling Flow: Not Logged In

**Error Condition**: User selects model for provider they're not logged into

**System Handling**:
1. Show modal: "Requires [Provider] login"
2. Provide command: `/login --provider=<name>`
3. Do NOT switch model (stay on current)
4. Log event for debugging

**User Recovery**: Run `/login --provider=<name>`, then retry model selection

---

### Secondary Flow: Login to Multiple Providers

**Flow**: Setting up multi-provider auth

**Step 1**: User runs `/login --provider=chatgpt`
**System Response**: Opens browser for ChatGPT OAuth

**Step 2**: User completes OAuth flow
**System Response**: Stores ChatGPT tokens in auth.json

**Step 3**: User runs `/login --provider=claude`
**System Response**: Opens browser for Claude OAuth

**Step 4**: User completes OAuth flow
**System Response**: Stores Claude tokens alongside ChatGPT

**Outcome**: User now has both providers available, can switch freely

---

## Technical Dependencies

### Library Dependencies

**Required**:
- OAuth 2.0 client library (e.g., `oauth2` crate)
- PKCE support (code challenge generation)
- HTTP client for token requests (e.g., `reqwest`)

**Optional**:
- Platform keychain integration (macOS Keychain, Linux Secret Service)
  - For secure token storage
  - Falls back to encrypted file storage

---

### Service Dependencies

**OAuth Client Credentials**:
- ChatGPT OAuth: Already configured ✅
- Claude OAuth: Need client_id, client_secret (registration required)
- Gemini OAuth: Need client_id, client_secret (Google Cloud Console)

**How to Obtain**:
1. Claude: Register at Anthropic Developer Portal (TBD)
2. Gemini: Create OAuth 2.0 credentials in Google Cloud Console

**Approval Requirement**:
- May require partnership with Anthropic for official OAuth app
- Google Cloud project approval for OAuth consent screen

---

### Data & Migration

**Auth Storage Migration**:

**Old Format** (`auth.json`):
```json
{
  "access_token": "...",
  "refresh_token": "...",
  "expires_at": "..."
}
```

**New Format** (`auth.json`):
```json
{
  "chatgpt": {
    "access_token": "...",
    "refresh_token": "...",
    "expires_at": "..."
  },
  "claude": {},
  "gemini": {}
}
```

**Migration Logic**:
```rust
// Detect old format (single object, no provider keys)
// Convert to new format with "chatgpt" key
// Preserve existing tokens
```

**Migration Requirement**: Automatic on first run with new code

---

## Risks & Mitigation

### Risk 1: OAuth flow bugs (Impact: High)

**Mitigation Strategy**:
- Reuse existing ChatGPT OAuth patterns (proven implementation)
- Thorough testing of all OAuth flows
- Unit tests for token refresh logic
- Integration tests with mock OAuth servers

**Owner**: Code
**Status**: Mitigatable

---

### Risk 2: OAuth credentials unavailable (Impact: High)

**Mitigation Strategy**:
- Research Claude/Gemini OAuth requirements early
- Alternative: Allow users to provide their own client credentials
- Document credential setup process
- Graceful degradation if credentials missing

**Owner**: Code
**Status**: Mitigatable

---

### Risk 3: Token storage security (Impact: Medium)

**Mitigation Strategy**:
- Investigate platform keychain integration
- Encrypt tokens in auth.json as fallback
- Document security considerations
- Follow OAuth security best practices

**Owner**: Code
**Status**: Mitigatable

---

### Risk 4: Provider API changes (Impact: Low)

**Mitigation Strategy**:
- Monitor provider OAuth documentation
- Implement defensive error handling
- Log OAuth failures for debugging
- Clear error messages for users

**Owner**: Code
**Status**: Acceptable

---

## Success Metrics

### Completion Criteria

**Criterion 1**: All tests pass (unit, integration, E2E)

**Criterion 2**: Documentation complete
- OAuth setup guides for each provider
- Troubleshooting section in docs
- `/login` command help text updated

**Criterion 3**: Code review approved
- Security review for OAuth implementation
- Architecture review for multi-provider pattern
- UX review for error messages and flows

---

### KPIs

**KPI 1**: Feature adoption
**Target**: 80% of users with multiple subscriptions use multi-provider auth within 30 days

**KPI 2**: User satisfaction
**Target**: ≥4/5 on "ease of model switching" rating
**Measurement**: Post-feature user feedback survey

**KPI 3**: Auth errors
**Target**: <1% authentication failures after successful login
**Measurement**: Telemetry from TUI error logs

---

## Testing Requirements

### Test Coverage

**Target**: ≥80% code coverage for new auth code

**Test Scenarios**:
1. OAuth flow completion (ChatGPT, Claude, Gemini)
2. Token refresh (active and inactive providers)
3. Provider switching (<500ms latency)
4. Error handling (network failures, invalid tokens, expired tokens)
5. Config migration (old → new format)
6. Multi-session management

### Critical Paths

**Main User Workflows**:
1. Login to multiple providers → switch models → verify correct auth used
2. Select model when not logged in → see helpful modal
3. Token expires → auto-refresh → no user disruption
4. TUI restart → restore last used provider

### Load Testing

**Not applicable** - OAuth flows are user-initiated, not high-throughput

---

## Approval & Review

### Stakeholders

**Owner**: Code (implementation)
**Reviewers**: Security review (OAuth implementation)

### Review Process

**Standard Code Review**:
1. Architecture review (multi-provider pattern)
2. Security review (token storage, PKCE implementation)
3. UX review (error messages, modals, help text)
4. Testing review (coverage, test quality)

### Security Audit

**Required**: Yes (OAuth implementation, token storage)

**Audit Scope**:
- PKCE implementation correctness
- Token storage security (encryption, file permissions)
- OAuth flow security (CSRF protection, state parameter)
- Error message information leakage

---

## Consensus & Validation

### Multi-Agent Consensus

**Native PRD generation** - No multi-agent consensus for SPEC creation

**Agreement**: N/A

**Disagreements**: N/A

---

## Open Questions

### Q1: OAuth Credentials Acquisition (Priority: HIGH, Blocker: YES)

**Question**: How to obtain official OAuth client credentials for Claude and Gemini?

**What Needs Clarification**:
- Anthropic: Developer program enrollment process, partnership requirements
- Google: OAuth consent screen approval process, API quota limits

**How to Resolve**:
1. Research Anthropic Developer Portal
2. Create Google Cloud project and OAuth credentials
3. Document setup process
4. Alternative: Allow user-provided credentials

---

### Q2: Token Encryption Strategy (Priority: MEDIUM, Blocker: NO)

**Question**: Should tokens be encrypted in auth.json? Which encryption method?

**Options**:
1. Platform keychain (macOS Keychain, Linux Secret Service, Windows Credential Manager)
2. File encryption with master key (derived from user password or system key)
3. No encryption (rely on file permissions only)

**How to Resolve**:
- Security review during implementation
- Evaluate platform keychain libraries
- Define encryption standard for fork

---

### Q3: Background Token Refresh Strategy (Priority: MEDIUM, Blocker: NO)

**Question**: When and how often to refresh tokens for inactive providers?

**Options**:
1. Refresh all tokens daily (background task)
2. Refresh only when switching to provider (lazy refresh)
3. Refresh on expiry warning (e.g., <24 hours remaining)

**How to Resolve**:
- Implement during Component 3 (AuthManager)
- Test with real OAuth tokens
- Monitor for excessive refresh API calls

---

### Q4: Fallback Behavior (Priority: LOW, Blocker: NO)

**Question**: What happens when all providers are unavailable (network down, OAuth endpoints down)?

**Options**:
1. Show "All providers unavailable" message, disable chat
2. Queue requests for retry when connectivity restored
3. Allow API key fallback (if configured)

**How to Resolve**:
- Define during error handling implementation
- Test offline scenarios
- Document expected behavior

---

## Appendix: Decision Log

### Major Decisions Made

**Decision 1**: Created SPEC for multi-provider OAuth architecture (2025-11-19)

**Decision 2**: `/model` command is central controller for both model AND auth provider
- Rationale: Single source of truth, seamless UX
- Alternative considered: Separate `/auth` command (rejected as extra step)

**Decision 3**: Support multiple simultaneous OAuth sessions
- Rationale: Power users have multiple subscriptions, should work seamlessly
- Alternative considered: Single active session (rejected as limiting)

**Decision 4**: Auto-switch auth on model selection
- Rationale: Best UX, no manual auth management
- Alternative considered: Manual switching (rejected as poor UX)

---

## Next Steps

### For New Session

1. **Create SPEC-KIT-947**: ✅ Manual creation (this document)
2. **Research OAuth details**:
   - Claude OAuth endpoints and scopes
   - Gemini OAuth endpoints and scopes
   - Obtain or document how to obtain client credentials
3. **Phase 1 Implementation**: Start with AuthMode enum expansion
4. **Iterative testing**: Test each OAuth flow as implemented

---

## Related Documents

- **SPEC-KIT-946**: Model Command Expansion (dependency - provides 13 models and provider detection)
- **SPEC.md**: Task tracker (to be updated with SPEC-KIT-947 entry)
- **Protocol Layer**: `protocol/src/mcp_protocol.rs` (AuthMode enum)
- **Auth Layer**: `core/src/auth.rs` (AuthManager)
- **TUI Integration**: `tui/src/chatwidget/mod.rs` (model selection)

---

**END OF PRD**
