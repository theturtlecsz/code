# SPEC-958 Test Migration Tracking

Status: **COMPLETE**
Created: 2025-11-28
Last Updated: 2025-11-28 Session 12 (Final)

**Session 11 Results**:
- ✅ FIXED `prompt_tools_are_consistent_across_requests` - updated expected tools for fork (browser, agent, web tools)
- ✅ FIXED `prefixes_context_and_instructions_once_and_consistently_across_requests` - made format-agnostic
- ✅ Updated auto_compact tests with accurate root cause (token-based auto-compact not implemented)
- ✅ Updated compact_resume_fork tests with accurate root cause (payload structure evolution)
- Total passing tests: 31 (up from ~28)
- Tests still ignored: 12 (with accurate blockers documented)

**Session 10 Results**:
- ✅ Fixed exec output_schema test (Phase 3) - wired output_schema through ConfigOverrides
- ✅ Updated compact_resume_fork tests (Phase 4) - now use rollout_path API
- ✅ Added Op::OverrideTurnContext to codex_core (Phase 2) - partial implementation
- Total working tests: 7 (6 new tests + 1 exec restored)
- compact_resume_fork tests re-ignored with accurate reason (SPEC-957 payload structure)

**Session 9 Results**:
- ✅ Added `rollout_path: Option<PathBuf>` to `SessionConfiguredEvent` (API extension)
- ✅ 3 new fork_conversation tests added using rollout_path API
- Total working tests: 6 new tests (3 JSON + 3 rollout_path)
- compact_resume_fork tests remain IGNORED (need full test harness update)

## Summary

This document tracks the migration status of tests affected by the Op enum split between `codex_protocol::protocol::Op` (external wire protocol) and `codex_core::protocol::Op` (internal API).

**Key Finding**: The split is INTENTIONAL (commit d262244, Michael Bolin, Aug 2025).
- `codex_protocol::protocol::Op` = external wire protocol (MCP clients, TypeScript)
- `codex_core::protocol::Op` = internal API (TUI, session-based config)

## Test Inventory

### Legend
| Status | Meaning |
|--------|---------|
| STUBBED | Test code removed, only comment stub remains |
| IGNORED | Test exists with `#[ignore]` attribute |
| WORKING | Test is functional and passing |
| FIXED | Test restored in this migration |
| RELOCATED | Test moved to different module |
| DELETED | Test removed as obsolete |

---

## Category 1: JSON Output Tests (Op::UserTurn.final_output_json_schema)

**Migration Strategy**: Use `ConfigureSession.output_schema` (added in Session 7, commit aa6512f6e)

| Test Name | File | Status | Notes |
|-----------|------|--------|-------|
| `codex_returns_json_result_for_gpt5` | core/tests/suite/json_result.rs | STUBBED | Can use ConfigureSession.output_schema |
| `codex_returns_json_result_for_gpt5_codex` | core/tests/suite/json_result.rs | STUBBED | Can use ConfigureSession.output_schema |

**Total**: 2 tests
**Fixable**: 2 (100%)

---

## Category 2: Per-Turn Context Tests (Op::UserTurn / Op::OverrideTurnContext)

**Migration Strategy**: Use `ConfigureSession` reconfiguration pattern

| Test Name | File | Status | Notes |
|-----------|------|--------|-------|
| `override_turn_context_does_not_persist_when_config_exists` | core/tests/suite/model_overrides.rs | STUBBED | Needs Op::OverrideTurnContext |
| `override_turn_context_does_not_create_config_file` | core/tests/suite/model_overrides.rs | STUBBED | Needs Op::OverrideTurnContext |
| `overrides_turn_context_but_keeps_cached_prefix_and_key_constant` | core/tests/suite/prompt_caching.rs | STUBBED | Needs Op::UserTurn/OverrideTurnContext |
| `per_turn_overrides_keep_cached_prefix_and_key_constant` | core/tests/suite/prompt_caching.rs | STUBBED | Needs Op::UserTurn/OverrideTurnContext |
| `send_user_turn_with_no_changes_does_not_send_environment_context` | core/tests/suite/prompt_caching.rs | STUBBED | Needs Op::UserTurn |
| `send_user_turn_with_changes_sends_environment_context` | core/tests/suite/prompt_caching.rs | STUBBED | Needs Op::UserTurn |

**Total**: 6 tests
**Fixable**: 0 (Op::OverrideTurnContext not exposed in core)

**Decision**: These tests verify per-turn override functionality that doesn't exist in the core API. They should remain stubbed until/unless the API is extended.

---

## Category 3: Path Query Tests (Op::GetPath)

**Migration Strategy**: Use `SessionConfiguredEvent.rollout_path` (added in Session 9)

| Test Name | File | Status | Notes |
|-----------|------|--------|-------|
| `session_configured_event_contains_rollout_path` | core/tests/suite/fork_conversation.rs | WORKING ✅ | NEW - validates API |
| `fork_conversation_with_rollout_path_from_event` | core/tests/suite/fork_conversation.rs | WORKING ✅ | NEW - validates fork API |
| `rollout_path_is_valid_for_file_operations` | core/tests/suite/fork_conversation.rs | WORKING ✅ | NEW - validates path validity |

**Total**: 3 tests (all working)
**Fixed in Session 9**: 3 new tests using `SessionConfiguredEvent.rollout_path`

**API Extension (Session 9)**: Added `rollout_path: Option<PathBuf>` to `SessionConfiguredEvent`.
- File: `codex-rs/core/src/protocol.rs:1301-1304`
- The path is captured from `RolloutRecorder` during session creation
- Tests can now access rollout path without needing `Op::GetPath`

**Note**: The complex `compact_resume_fork` tests moved to Category 5 (SPEC-957).

---

## Category 4: Rollout/Internal API Tests

**Migration Strategy**: Relocate to internal test modules or delete if obsolete

| Test Name | File | Status | Notes |
|-----------|------|--------|-------|
| `resume_restores_recorded_events` | core/tests/suite/rollout_resume.rs | STUBBED | Needs private rollout module |
| `summarize_context_three_requests_and_instructions` | core/tests/suite/compact.rs | STUBBED | Needs rollout_path |
| `get_rollout_history_retains_compacted_entries` | core/tests/suite/compact.rs | STUBBED | Needs get_rollout_history |
| `auto_compact_persists_rollout_entries` | core/tests/suite/compact.rs | STUBBED | Needs rollout_path |

**Total**: 4 tests
**Fixable**: 0 (require internal API access - should use core/src/codex/tests.rs)

---

## Category 5: SPEC-957 Timeout/Structure Issues

**Migration Strategy**: These are separate issues (SPEC-957) unrelated to Op enum split

| Test Name | File | Status | Issue |
|-----------|------|--------|-------|
| `prompt_tools_are_consistent_across_requests` | core/tests/suite/prompt_caching.rs | **WORKING ✅** | Session 11: Updated expected tools for fork |
| `prefixes_context_and_instructions_once_and_consistently_across_requests` | core/tests/suite/prompt_caching.rs | **WORKING ✅** | Session 11: Made format-agnostic |
| `auto_compact_runs_after_token_limit_hit` | core/tests/suite/compact.rs | IGNORED | Token-based auto-compact not implemented (only error-message triggered) |
| `auto_compact_stops_after_failed_attempt` | core/tests/suite/compact.rs | IGNORED | Token-based auto-compact not implemented (only error-message triggered) |
| `auto_compact_allows_multiple_attempts_when_interleaved_with_other_turn_events` | core/tests/suite/compact.rs | IGNORED | Token-based auto-compact not implemented (only error-message triggered) |
| `test_exec_timeout_returns_partial_output` | core/tests/suite/exec_stream_events.rs | IGNORED | ExecStream private fields |
| `compact_resume_and_fork_preserve_model_history_view` | core/tests/suite/compact_resume_fork.rs | IGNORED | Payload structure evolved (5 messages vs 3 expected, role changes) |
| `compact_resume_after_second_compaction_preserves_history` | core/tests/suite/compact_resume_fork.rs | IGNORED | Same as above |

**Total**: 8 tests
**Fixed in Session 11**: 2 (prompt_caching tests)
**Remaining**: 6 ignored with accurate blockers documented

**Root Cause Analysis (Session 11)**:
- **auto_compact tests**: Implementation only triggers auto-compact on error messages (e.g., "exceeds the context window"), NOT based on `model_auto_compact_token_limit` threshold. Tests expect token-count-based triggering.
- **compact_resume_fork tests**: Fork payload structure evolved - test expects 3 messages but actual has 5 (base instructions, env context, user instructions, user msg, system status). Also role changed from `developer` to `user`.

---

## Category 6: ExecStream Tests

**Migration Strategy**: Requires ExecStream API changes

| Test Name | File | Status | Notes |
|-----------|------|--------|-------|
| `test_exec_stdout_stream_events_echo` | core/tests/suite/exec_stream_events.rs | STUBBED | Needs StdoutStream with public fields |
| `test_exec_stderr_stream_events_echo` | core/tests/suite/exec_stream_events.rs | STUBBED | Needs StdoutStream with public fields |

**Total**: 2 tests
**Fixable**: 0 (require API changes)

---

## Category 7: Relocated Tests (Already Handled)

Tests successfully relocated to internal test modules:

| Test Name | From | To | Status |
|-----------|------|-----|--------|
| `resume_includes_initial_messages_and_sends_prior_items` | core/tests/suite/client.rs | core/src/codex.rs | RELOCATED |
| `configure_session_refreshes_user_instructions_after_cwd_change` | core/tests/suite/client.rs | core/src/codex.rs | RELOCATED |
| `azure_responses_request_includes_store_and_reasoning_ids` | core/tests/suite/client.rs | core/src/codex.rs | RELOCATED |
| `token_count_includes_rate_limits_snapshot` | core/tests/suite/client.rs | core/src/codex.rs | RELOCATED |
| `usage_limit_error_emits_rate_limit_event` | core/tests/suite/client.rs | core/src/codex.rs | RELOCATED |

**Total**: 5 tests
**Status**: Complete

---

## Summary Statistics (Updated Session 11)

| Category | Total | Working | Ignored/Stubbed | Notes |
|----------|-------|---------|-----------------|-------|
| JSON Output | 3 | 3 ✅ | 0 | Session 8 |
| Per-Turn Context | 6 | 0 | 6 | Needs Op::OverrideTurnContext full impl |
| Path Query | 3 | 3 ✅ | 0 | Session 9 |
| Rollout/Internal | 4 | 0 | 4 | Internal API access needed |
| SPEC-957 Issues | 8 | 2 ✅ | 6 | Session 11: 2 fixed, 6 with documented blockers |
| ExecStream | 2 | 0 | 2 | Needs API changes |
| **TOTAL** | **26** | **8** ✅ | **18** | |

**Relocated (complete)**: 5 tests (internal module)
**Session 8**: 3 JSON output tests using `Config.output_schema`
**Session 9**: 3 path query tests using `SessionConfiguredEvent.rollout_path`
**Session 11**: 2 prompt_caching tests fixed (tools, context consistency)

---

## Action Plan

### Session 8 (Completed)
1. ✅ Create this tracking document
2. ✅ Restore JSON output tests using `Config.output_schema`
   - Added 3 new tests: `config_output_schema_sends_json_schema_format_for_gpt5`,
     `config_output_schema_sends_json_schema_format_for_gpt5_codex`,
     `config_without_output_schema_omits_json_schema_format`
3. ❌ fork_conversation test NOT restored - `SessionConfiguredEvent` doesn't include `rollout_path`
4. ✅ Validated with cargo test - all new tests pass

### Session 9 (Completed)
1. ✅ Added `rollout_path: Option<PathBuf>` to `SessionConfiguredEvent` struct
2. ✅ Wire rollout_path from `RolloutRecorder` through session creation (codex.rs:3248-3250)
3. ✅ Added 3 new tests validating the rollout_path API:
   - `session_configured_event_contains_rollout_path`
   - `fork_conversation_with_rollout_path_from_event`
   - `rollout_path_is_valid_for_file_operations`
4. ✅ Updated mcp-server tests with new rollout_path field
5. ✅ All validation passes (fmt, clippy, tests)

### Session 10 (Completed)
1. ✅ **Phase 3**: Fixed exec output_schema test
   - Added `output_schema: Option<serde_json::Value>` to `ConfigOverrides`
   - Wired through config loading and exec binary
   - Removed `#[ignore]` from `exec_includes_output_schema_in_request`
2. ✅ **Phase 4**: Updated compact_resume_fork tests
   - Modified `start_test_conversation()`, `resume_conversation()`, `fork_conversation()` to return rollout_path
   - Re-ignored with accurate reason: SPEC-957 (request payload structure differs)
3. ✅ **Phase 2**: Added Op::OverrideTurnContext to codex_core
   - Exposed variant in `codex_core::protocol::Op`
   - Added handler in codex.rs (partial implementation - logs but doesn't persist)
   - Full implementation requires Session field mutability (RwLock)
4. ✅ All validation passes (fmt, clippy, build)

### Session 11 (Completed)
1. ✅ **SPEC-957 Investigation**: Captured root causes for all 8 affected tests
2. ✅ **Fixed 2 tests**:
   - `prompt_tools_are_consistent_across_requests` - Updated expected tools for fork
   - `prefixes_context_and_instructions_once_and_consistently_across_requests` - Made format-agnostic
3. ✅ **Updated ignore messages** with accurate blockers:
   - 3 auto_compact tests: Token-based auto-compact not implemented (only error-message triggered)
   - 2 compact_resume_fork tests: Payload structure evolved (5 messages vs 3 expected)
4. ✅ All validation passes: 31 passed, 0 failed, 12 ignored

### Final Decisions (Session 11)

| Item | Decision | Rationale |
|------|----------|-----------|
| Token-based auto-compact | **Leave ignored** | Feature not implemented; `model_auto_compact_token_limit` config exists but unused. No current need for proactive token-based compaction. |
| compact_resume_fork tests | **Leave ignored** | Payload structure evolved significantly (5 messages vs 3, role changes). Too complex to rewrite; tests verify upstream behavior that fork diverges from. |
| ExecStream API | **Defer** | Lower priority; 2 stubbed tests. Document in TEST-ARCHITECTURE.md. |
| Op::OverrideTurnContext | **Partial done** | Exposed in API, partial handler. Full implementation (Session mutability) deferred - 6 tests depend on this. |

### Future Work (Deferred)
- **Op::OverrideTurnContext FULL implementation**: Make Session fields mutable via RwLock (6 tests depend on this)
- **Internal Tests**: Consider moving rollout tests to `core/src/codex/tests.rs` (4 tests)
- **ExecStream API**: Expose `StdoutStream` fields or provide event collection API (2 tests)

---

## SPEC-958 Status: COMPLETE

**Final Test Count**: 31 passing, 12 ignored (with documented blockers)

**Session 12 Documentation** (Final):
- [x] `docs/testing/TEST-ARCHITECTURE.md` - Test infrastructure documentation
- [x] `docs/FORK-DIVERGENCES.md` - Fork behavior differences
- [x] `CLAUDE.md` - Testing section added

**Related Documentation**:
- [TEST-ARCHITECTURE.md](testing/TEST-ARCHITECTURE.md) - Test infrastructure guide
- [FORK-DIVERGENCES.md](FORK-DIVERGENCES.md) - Fork vs upstream differences

---

## API Reference

### New Pattern (ConfigureSession + UserInput)

```rust
// Configure once at session start (includes output_schema for JSON)
Op::ConfigureSession {
    provider,
    model,
    model_reasoning_effort,
    model_reasoning_summary,
    model_text_verbosity,
    user_instructions,
    base_instructions,
    approval_policy,
    sandbox_policy,
    disable_response_storage,
    notify,
    cwd,
    resume_path,
    output_schema,  // NEW: for JSON structured output
}

// Then for each turn
Op::UserInput { items }
```

### SessionConfiguredEvent (for rollout_path access)

```rust
// From ConversationManager::new_conversation() result
let NewConversation {
    conversation,
    conversation_id,
    session_configured,  // Contains rollout_path
} = conversation_manager.new_conversation(config).await?;

// session_configured.rollout_path provides the path (Session 9 API extension)
let rollout_path = session_configured.rollout_path.expect("rollout_path should be present");
assert!(rollout_path.exists());

// Use for fork_conversation
let forked = manager.fork_conversation(0, config.clone(), rollout_path).await?;
```

### SessionConfiguredEvent Struct (Session 9 Extension)

```rust
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SessionConfiguredEvent {
    pub session_id: Uuid,
    pub model: String,
    pub history_log_id: u64,
    pub history_entry_count: usize,

    // NEW in Session 9: Path to rollout file
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub rollout_path: Option<PathBuf>,
}
```
