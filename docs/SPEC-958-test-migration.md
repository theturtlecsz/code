# SPEC-958 Test Migration Tracking

Status: **Session 8 Complete**
Created: 2025-11-28
Last Updated: 2025-11-28 Session 8

**Session 8 Results**:
- 3 new JSON output tests added and passing
- fork_conversation test NOT restored (requires API change)
- Total fixable tests: 2 of 23 (8.7%) - remaining need API changes or are SPEC-957 scope

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

**Migration Strategy**: Requires API change - `SessionConfiguredEvent` does NOT include `rollout_path`

| Test Name | File | Status | Notes |
|-----------|------|--------|-------|
| `fork_conversation_twice_drops_to_first_message` | core/tests/suite/fork_conversation.rs | STUBBED | Needs API change |
| `compact_resume_and_fork_preserve_model_history_view` | core/tests/suite/compact_resume_fork.rs | IGNORED | Needs Op::GetPath |
| `compact_resume_fork_twice_preserve_model_history_view` | core/tests/suite/compact_resume_fork.rs | IGNORED | Needs Op::GetPath |

**Total**: 3 tests
**Fixable**: 0 (requires adding `rollout_path` to public API)

**Analysis**: The `SessionConfiguredEvent` struct does NOT include `rollout_path`. The `RolloutRecorder.rollout_path` is `pub(crate)` so not accessible from tests. These tests require either:
1. Adding `rollout_path` to `SessionConfiguredEvent`
2. Adding `rollout_path` to `NewConversation` return struct
3. Re-exposing `Op::GetPath` in the core API

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
| `prompt_tools_are_consistent_across_requests` | core/tests/suite/prompt_caching.rs | IGNORED | Tools array structure differs |
| `prefixes_context_and_instructions_once_and_consistently_across_requests` | core/tests/suite/prompt_caching.rs | IGNORED | Context prefix structure differs |
| `auto_compact_runs_after_token_limit_hit` | core/tests/suite/compact.rs | IGNORED | Timeout waiting for TaskComplete |
| `auto_compact_stops_after_failed_attempt` | core/tests/suite/compact.rs | IGNORED | Timeout waiting for Error event |
| `auto_compact_allows_multiple_attempts_when_interleaved_with_other_turn_events` | core/tests/suite/compact.rs | IGNORED | Timeout during interleaved processing |
| `test_exec_timeout_returns_partial_output` | core/tests/suite/exec_stream_events.rs | IGNORED | ExecStream private fields |

**Total**: 6 tests
**Fixable**: 0 (SPEC-957 scope, not SPEC-958)

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

## Summary Statistics

| Category | Total | Fixable Now | Requires API Change | SPEC-957 Scope |
|----------|-------|-------------|---------------------|----------------|
| JSON Output | 2 | 2 ✅ | 0 | 0 |
| Per-Turn Context | 6 | 0 | 6 | 0 |
| Path Query | 3 | 0 | 3 | 0 |
| Rollout/Internal | 4 | 0 | 4 | 0 |
| SPEC-957 Issues | 6 | 0 | 0 | 6 |
| ExecStream | 2 | 0 | 2 | 0 |
| **TOTAL** | **23** | **2** ✅ | **15** | **6** |

**Relocated (complete)**: 5 tests
**Fixed (this session)**: 3 new JSON output tests (config_output_schema_* tests)

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

### Future Work Required
- **SPEC-957**: Address timeout and structure issues separately (6 tests)
- **API Extension for rollout_path**: Add `rollout_path` to `SessionConfiguredEvent` or `NewConversation` (3 tests)
- **API Extension for per-turn overrides**: Consider exposing `Op::OverrideTurnContext` (6 tests)
- **Internal Tests**: Consider moving rollout tests to `core/src/codex/tests.rs` (4 tests)
- **ExecStream API**: Expose `StdoutStream` fields or provide event collection API (2 tests)

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

// session_configured.rollout_path provides the path
```
