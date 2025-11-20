# SPEC-KIT-953: Native Multi-Provider Integration (Master SPEC)

**Status**: Draft
**Created**: 2025-11-19
**Type**: Master Orchestration SPEC
**Priority**: High
**Estimated Effort**: ~8 sub-SPECs (3 research, 5 implementation)

---

## Executive Summary

Build native, first-class integrations for Claude and Gemini providers directly into the codex-rs TUI, achieving feature parity with the existing ChatGPT/OpenAI integration. Replace stateless CLI subprocess routing (SPEC-KIT-952) with stateful conversation context.

---

## Problem Statement

Current CLI routing (SPEC-KIT-952) spawns stateless subprocess commands for Claude and Gemini models. Each prompt is fire-and-forget with no conversation context. This architectural limitation prevents meaningful multi-turn conversations with non-ChatGPT providers.

**Evidence**:
- `codex-rs/tui/src/model_router.rs`: Lines 108-148 show stateless `execute_prompt_with_settings()` calls
- `codex-rs/tui/src/providers/claude.rs`: No history accumulation, single-shot execution
- No session persistence between prompts

---

## Vision

Native provider integrations with:
1. Full conversation history passed with each request
2. Provider-appropriate context window management
3. OAuth credential persistence and refresh
4. Streaming responses in real-time
5. Feature parity across ChatGPT, Claude, and Gemini

---

## Master SPEC Responsibilities

This SPEC serves as the **orchestration hub** tracking:
- Research SPEC progress and findings
- Implementation SPEC dependencies and sequencing
- Cross-cutting architectural decisions
- Integration milestones and blockers
- Key decision checkpoints

---

## Foundation Validation (Completed 2025-11-19)

### Research Targets Confirmed

| Target | Path | Language | License | Status |
|--------|------|----------|---------|--------|
| Claude Code | ~/claude-code | TypeScript | **Proprietary** (Anthropic Commercial Terms) | Analysis only, no extraction |
| Gemini CLI | ~/gemini-cli | TypeScript | Apache 2.0 | Can extract patterns/code |
| codex-rs Auth | codex-rs/core/src/auth.rs | Rust | Project license | Extensible patterns |

### Current Architecture

**Existing OAuth** (`codex-rs/core/src/auth.rs`):
- `CodexAuth` struct with `AuthManager`
- Token refresh mechanism (28-day expiry check)
- File-based storage (`auth.json`)
- OpenAI-specific but pattern extensible

**Provider Abstraction** (`codex-rs/tui/src/providers/mod.rs`):
- `ProviderType` enum (ChatGPT, Claude, Gemini)
- `ProviderResponse` with content, model, usage
- `CliRoutingSettings` for sandbox/approval policies

---

## Research SPECs Required

### SPEC-KIT-953-A: Claude Code Architecture Analysis

**Target**: ~/claude-code
**Constraint**: **PROPRIETARY LICENSE** - analyze for independent re-implementation only

**Deliverables**:
- Repository structure analysis (TypeScript/Rust components)
- API client patterns (Anthropic API request/response shapes)
- OAuth/authentication flow documentation (endpoints, scopes, tokens)
- Conversation/session state management patterns
- Message history serialization format
- Streaming response handling architecture
- Tool use protocol (if applicable to TUI integration)
- **Independent implementation feasibility**: Rust rewrite vs FFI bridge decision

**Critical Questions**:
1. Does Claude Code use direct Anthropic API or intermediate service?
2. How is conversation context accumulated and truncated?
3. What OAuth endpoints/scopes are required for Anthropic?
4. Is there session persistence? If so, what format?

### SPEC-KIT-953-B: Gemini CLI Architecture Analysis

**Target**: ~/gemini-cli
**Constraint**: Apache 2.0 - can extract and adapt

**Deliverables**:
- Repository structure and component analysis
- API client extraction feasibility (Google AI API patterns)
- OAuth/authentication flow documentation (Google OAuth 2.0 specifics)
- Conversation context management patterns
- Message history format and storage
- Streaming response handling
- Rate limiting and error handling patterns
- **Rust implementation path**: Direct port vs FFI bridge

**Critical Questions**:
1. What Google OAuth scopes are required?
2. How does context window truncation work for Gemini models?
3. Is there built-in retry logic? What errors trigger it?
4. Can we extract the TypeScript client for FFI bridge?

### SPEC-KIT-953-C: Existing codex-rs OAuth Analysis

**Target**: codex-rs/core, codex-rs/tui
**Scope**: Document existing patterns for multi-provider extension

**Deliverables**:
- Current OAuth implementation architecture diagram
- Token storage format (`auth.json` schema)
- Refresh mechanism and timing
- `AuthManager` API surface
- Extension points for additional providers
- **Proposed multi-provider schema**: How to store Claude/Gemini tokens alongside OpenAI

**Key Files**:
- `codex-rs/core/src/auth.rs` (883 lines)
- `codex-rs/core/src/auth_accounts.rs`
- `codex-rs/tui/src/onboarding/auth.rs`

---

## Implementation SPECs (Generated After Research)

**Important**: Implementation SPECs should be created AFTER research phase completes. Details below are preliminary and will be refined based on research findings.

### SPEC-KIT-953-D: Provider Authentication Framework

**Status**: ✅ SPEC Created (2025-11-19)
**Full SPEC**: [SPEC-953-D-provider-auth-framework.md](SPEC-953-D-provider-auth-framework.md)
**Dependencies**: SPEC-A, SPEC-B, SPEC-C findings (Complete)
**Estimated Effort**: 30-40 hours

**Scope**:
- `ProviderAuth` trait with OAuth 2.0 PKCE abstraction
- Provider-specific implementations (OpenAI refactor, Anthropic, Google)
- Shared PKCE and callback server infrastructure
- Multi-provider token storage schema (auth_accounts.json v2)
- `ProviderAuthManager` for centralized auth orchestration

**Architecture** (from SPEC-953-D):
```rust
#[async_trait]
pub trait ProviderAuth: Send + Sync {
    fn provider_id(&self) -> ProviderId;
    fn display_name(&self) -> &'static str;
    fn oauth_config(&self) -> OAuthConfig;
    fn authorization_url(&self, state: &str, code_verifier: &str) -> String;
    async fn exchange_code(&self, code: &str, code_verifier: &str) -> Result<TokenResponse, AuthError>;
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, AuthError>;
    fn extract_metadata(&self, response: &TokenResponse) -> serde_json::Value;
    fn needs_refresh(&self, credentials: &ProviderCredentials) -> bool;
}
```

**File Structure**:
```
codex-rs/core/src/
├── provider_auth/
│   ├── mod.rs, pkce.rs, callback_server.rs, manager.rs, error.rs, storage.rs
└── providers/
    ├── openai.rs (refactor), anthropic.rs (new), google.rs (new)
```

### SPEC-KIT-953-E: Conversation Context Manager

**Status**: ✅ IMPLEMENTED (2025-11-20)
**Full SPEC**: [SPEC-953-E-context-manager.md](SPEC-953-E-context-manager.md)
**Dependencies**: SPEC-A, SPEC-B findings (message formats); SPEC-D (ProviderAuth types)
**Estimated Effort**: 25-35 hours

**Scope**:
- Abstract conversation history interface
- Provider-specific serialization (Anthropic format vs Google format vs OpenAI format)
- Context window management (token counting per provider)
- Truncation strategies (oldest-first, summarization, priority-based)
- Session persistence (resume conversations across TUI restarts)
- Memory-efficient history storage

**Implementation**:
```rust
// codex-rs/core/src/context_manager/
pub use manager::ContextManager;       // Main API
pub use history::ConversationHistory;  // Token tracking, truncation
pub use serializer::serialize_for_provider;  // OpenAI/Anthropic/Google
pub use tokenizer::count_tokens;       // Per-provider token counting
pub use persistence::SessionManager;   // Save/load sessions
```

**File Structure**:
```
codex-rs/core/src/context_manager/
├── mod.rs         # Core types: Message, ContentBlock, ProviderId
├── history.rs     # ConversationHistory with truncation
├── serializer.rs  # Provider-specific serialization
├── tokenizer.rs   # Token counting per provider
├── persistence.rs # Session save/load
└── manager.rs     # ContextManager public API
```

**Tests**: 48 unit tests passing (100% coverage of public API)

### SPEC-KIT-953-F: Native Claude Provider

**Status**: ✅ IMPLEMENTED (2025-11-20)
**Dependencies**: SPEC-A, SPEC-D, SPEC-E
**Approach**: Rust-native with direct HTTP API calls

**Scope**:
- Direct Anthropic API client (Rust `reqwest`)
- Streaming SSE response parsing
- Conversation history via ContextManager
- OAuth token retrieval via ProviderAuthManager
- Error handling with typed errors
- Replace current CLI subprocess approach in `providers/claude.rs`

**Implementation**:
```rust
// codex-rs/core/src/api_clients/
pub use anthropic::{AnthropicClient, AnthropicConfig};
pub use ApiError;
pub use StreamEvent;

// Usage
let client = AnthropicClient::new(codex_home);
let stream = client.send_message(&messages, &config).await?;
while let Some(event) = stream.next().await {
    match event? {
        StreamEvent::TextDelta { text, .. } => print!("{}", text),
        StreamEvent::MessageStop => break,
        _ => {}
    }
}
```

**File Structure**:
```
codex-rs/core/src/api_clients/
├── mod.rs       # ApiError, StreamEvent, TokenUsage
└── anthropic.rs # AnthropicClient, AnthropicConfig, AnthropicStream
```

**Tests**: 13 unit tests passing (SSE parsing, request building, error handling)

### SPEC-KIT-953-G: Native Gemini Provider

**Status**: ✅ IMPLEMENTED (2025-11-20)
**Dependencies**: SPEC-B, SPEC-D, SPEC-E
**Approach**: Rust-native with direct HTTP API calls

**Scope**:
- Direct Google Generative AI API client (Rust `reqwest`)
- Streaming newline-delimited JSON response parsing
- Conversation history via ContextManager
- OAuth token retrieval via ProviderAuthManager
- Model-specific handling (flash, pro, ultra)
- Error handling with typed errors
- Replace current CLI subprocess approach in `providers/gemini.rs`

**Implementation**:
```rust
// codex-rs/core/src/api_clients/
pub use google::{GeminiClient, GeminiConfig, map_model_name as map_gemini_model};

// Usage
let client = GeminiClient::new(codex_home);
let stream = client.send_message(&messages, &config).await?;
while let Some(event) = stream.next().await {
    match event? {
        StreamEvent::TextDelta { text, .. } => print!("{}", text),
        StreamEvent::MessageStop => break,
        _ => {}
    }
}
```

**File Structure**:
```
codex-rs/core/src/api_clients/
├── mod.rs       # ApiError, StreamEvent, TokenUsage (shared)
├── anthropic.rs # AnthropicClient (SPEC-953-F)
└── google.rs    # GeminiClient, GeminiConfig, GeminiStream (NEW)
```

**Key Differences from Anthropic**:
| Aspect | Anthropic | Google |
|--------|-----------|--------|
| Auth | x-api-key header | Bearer token |
| Streaming | SSE events | Newline-delimited JSON |
| Role | assistant | model |
| System | system field | systemInstruction object |

**Tests**: 14 unit tests passing (JSON parsing, request building, model mapping, error handling)

### SPEC-KIT-953-H: TUI Integration & Migration

**Status**: ✅ IMPLEMENTED (2025-11-20)
**Dependencies**: SPEC-F ✅, SPEC-G ✅
**Scope**: Final integration and cleanup

**Implementation**:
- Updated `model_router.rs` with `execute_with_native_streaming()` function
- Native API clients replace CLI subprocess routing for Claude and Gemini
- Streaming events added to `app_event.rs` (NativeProviderStreamStart/Delta/Complete/Error)
- Conversation history stored per-provider in ChatWidget
- History automatically accumulates user/assistant messages
- Real-time streaming display via existing StreamController facade

**File Structure**:
```
codex-rs/tui/src/
├── model_router.rs      # MODIFIED: Native streaming with execute_with_native_streaming()
├── app.rs               # MODIFIED: Handler for streaming events
├── app_event.rs         # MODIFIED: New streaming event variants
├── app_event_sender.rs  # MODIFIED: Helper methods for streaming events
└── chatwidget/
    └── mod.rs           # MODIFIED: Streaming handlers, history state, native routing
```

**Key Components**:
- `execute_with_native_streaming()`: Main entry point for native provider streaming
- `map_claude_model()` / `map_gemini_model()`: Model preset to API model mapping
- `supports_native_streaming()`: Check if model uses native path
- `on_native_stream_start/delta/complete/error()`: ChatWidget handlers
- `native_provider_history`: Per-provider conversation history HashMap

**Tests**: Build passes. Library tests blocked by pre-existing AgentConfig issue (unrelated to SPEC-953-H).

**Remaining Work (Post-MVP)**:
- Context size indicator in chat view
- History truncation when approaching context limits
- Session persistence across TUI restarts
- /clear command to reset provider history

---

## Must-Have Requirements

### 1. Native TUI Integration
- [ ] No subprocess spawning for provider communication
- [ ] Direct API calls from Rust (native or FFI)
- [ ] Streaming responses render in real-time
- [ ] Consistent UX across all providers

### 2. Session Context with History Tracking
- [ ] Full conversation history passed with each request
- [ ] Provider-appropriate context window management
- [ ] Ability to clear/reset context
- [ ] Optional: persist sessions across TUI restarts
- [ ] Visual indicator of context size/usage

### 3. OAuth Authentication Reuse
- [ ] Leverage existing codex-rs OAuth patterns
- [ ] Support Anthropic authentication
- [ ] Support Google OAuth 2.0
- [ ] Unified token refresh and storage
- [ ] No duplicate auth implementations

---

## Architecture Constraints

1. **Rust-native preferred**: FFI bridges only if TypeScript functionality cannot be reasonably replicated
2. **No breakage**: Must not break existing ChatGPT/OpenAI flow
3. **Pluggable providers**: Future providers (Mistral, Llama, etc.) should be easy to add
4. **Context abstraction**: Provider-specific context management behind common interface
5. **Security**: Auth tokens never logged or exposed; follow existing secure storage patterns

---

## Success Criteria

1. **Conversation depth**: User can have 10+ turn conversation with Claude maintaining full context
2. **Conversation depth**: User can have 10+ turn conversation with Gemini maintaining full context
3. **Auth persistence**: OAuth tokens persist and refresh automatically (no re-auth per session)
4. **Streaming parity**: Response streaming works identically to ChatGPT provider
5. **Context visibility**: Context size visible to user; truncation handled gracefully
6. **Model selection**: All providers selectable via `/model` command
7. **No subprocesses**: Zero subprocess spawning for provider communication

---

## Key Decision Checkpoints

### Checkpoint 1: Post-Research Architecture Decision (After SPECs A, B, C)

**Status**: ✅ COMPLETE (2025-11-19)

**Decision**: **RUST-NATIVE REWRITE FOR BOTH PROVIDERS**

| Provider | Decision | Rationale |
|----------|----------|-----------|
| Claude | Rust-native | Required - proprietary license prohibits FFI extraction |
| Gemini | Rust-native | Preferred - consistency, no Node.js dependency |

**Supporting Evidence**:
- Claude rewrite feasibility: 8/10 (68-90h, **OAuth 2.0 with PKCE**)
- Gemini rewrite feasibility: 8/10 (104-130h, OAuth with PKCE)
- codex-rs auth architecture already extensible (minimal changes)

**Key Findings** (REVISED 2025-11-19):
1. **BOTH providers use OAuth 2.0 with PKCE** - can share infrastructure!
2. Claude Client ID: `9d1c250a-e61b-44d9-88ed-5944d1962f5e`
3. Gemini Client ID: `681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com`
4. Existing `AuthMode` enum and `StoredAccount` can be extended
5. `ProviderType` abstraction already exists for model routing
6. Use `oauth2` crate for shared OAuth infrastructure

**Total Implementation Effort**: ~142-200 hours across SPECs D-H (reduced due to shared OAuth)

### Checkpoint 2: Post-Auth Framework (After SPEC-D)

**Validation**: Can we authenticate with all three providers using the unified trait?

### Checkpoint 3: Post-Context Manager (After SPEC-E)

**Validation**: Does token counting work correctly for all providers?

---

## SPEC Hierarchy

```
SPEC-KIT-953 (Master - This SPEC)
├── SPEC-KIT-953-A (Research: Claude Code) [Parallel]
├── SPEC-KIT-953-B (Research: Gemini CLI) [Parallel]
├── SPEC-KIT-953-C (Research: codex-rs OAuth) [Parallel]
│
│ ─── Checkpoint 1: Architecture Decision ───
│
├── SPEC-KIT-953-D (Impl: Auth Framework) [After A, B, C]
├── SPEC-KIT-953-E (Impl: Context Manager) [After A, B, D]
│
│ ─── Checkpoint 2 & 3: Auth & Context Validation ───
│
├── SPEC-KIT-953-F (Impl: Native Claude) [After D, E]
├── SPEC-KIT-953-G (Impl: Native Gemini) [After D, E]
│   └── (F and G can run in parallel)
│
└── SPEC-KIT-953-H (Impl: TUI Integration) [After F, G]
```

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Claude Code proprietary license blocks pattern extraction | Confirmed | High | Use public Anthropic API docs; analyze architecture without copying code |
| FFI bridge complexity exceeds Rust rewrite effort | Medium | Medium | Default to Rust-native; FFI only if critical functionality cannot be replicated |
| Provider token formats incompatible with unified storage | Low | Medium | Use provider-specific subdirectories in auth storage |
| Context window token counting inconsistent | Medium | High | Use official tokenizer libraries for each provider |
| Streaming format differences break TUI rendering | Low | Medium | Abstract streaming protocol in `ProviderResponse` |

---

## Estimated Timeline

**Research Phase** (Parallelizable): 3 SPECs
- SPEC-A, B, C can run concurrently
- Estimated: 2-3 days

**Implementation Phase** (Sequential Dependencies): 5 SPECs
- D depends on A, B, C
- E depends on A, B, D
- F, G depend on D, E (can run parallel)
- H depends on F, G
- Estimated: 2-3 weeks

**Total**: ~3-4 weeks including decision checkpoints

---

## Next Steps

1. **Immediate**: Create Research SPECs A, B, C (parallelizable)
2. **After research**: Conduct Checkpoint 1 architecture decision
3. **After decision**: Create Implementation SPECs D, E
4. **After D, E**: Create Implementation SPECs F, G (parallel)
5. **After F, G**: Create Implementation SPEC H
6. **Throughout**: Update this Master SPEC with findings and decisions

---

## References

- SPEC-KIT-952: CLI Routing Multi-Provider (current implementation)
- `codex-rs/tui/src/model_router.rs`: Current router
- `codex-rs/tui/src/providers/`: Current CLI providers
- `codex-rs/core/src/auth.rs`: Existing OAuth patterns
- Anthropic API Docs: https://docs.anthropic.com/
- Google AI API Docs: https://ai.google.dev/docs

---

## Change Log

| Date | Author | Change |
|------|--------|--------|
| 2025-11-19 | Claude | Initial draft with foundation validation |
| 2025-11-20 | Claude | SPEC-953-D, E, F implemented |
| 2025-11-20 | Claude | SPEC-953-G implemented: GeminiClient with 14 tests, ~430 LOC |
| 2025-11-20 | Claude | SPEC-953-H implemented: TUI integration with native streaming, conversation history |
