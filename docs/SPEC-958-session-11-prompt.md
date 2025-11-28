# SPEC-958 Session 11: SPEC-957 Investigation & Fixes

## Session Context

**Continuation of**: SPEC-958 Test Migration (Sessions 8-10)
**Primary Focus**: SPEC-957 root cause analysis + fix what's feasible
**Scope**: Investigation → Implementation where straightforward → Document blockers

## Prior Session Summary

### Session 10 Completed
- ✅ Phase 3: Fixed exec output_schema test (wired ConfigOverrides.output_schema)
- ✅ Phase 4: Updated compact_resume_fork test harness (rollout_path API)
- ✅ Phase 2: Added Op::OverrideTurnContext (partial - logs but doesn't persist)

### Current Test Status
| Category | Working | Ignored/Stubbed | Blockers |
|----------|---------|-----------------|----------|
| JSON Output | 3 ✅ | 0 | - |
| Path Query | 3 ✅ | 2 | SPEC-957 |
| Exec | 1 ✅ | 0 | - |
| SPEC-957 Issues | 0 | 6 | Investigation needed |
| Per-Turn Context | 0 | 6 | Needs OverrideTurnContext full impl |
| ExecStream | 0 | 2 | API exposure needed |
| **TOTAL** | **7** | **16** | |

---

## Phase 1: SPEC-957 Investigation (Priority)

### Tests to Investigate

**File: `core/tests/suite/compact_resume_fork.rs`**
```
1. compact_resume_and_fork_preserve_model_history_view
   Ignore reason: "SPEC-957: Request payload structure differs (role/content format)"

2. compact_resume_after_second_compaction_preserves_history
   Ignore reason: "SPEC-957: Request payload structure differs (role/content format)"
```

**File: `core/tests/suite/prompt_caching.rs`**
```
3. prompt_tools_are_consistent_across_requests
   Ignore reason: "SPEC-957: tools array structure differs from expected"

4. prefixes_context_and_instructions_once_and_consistently_across_requests
   Ignore reason: "SPEC-957: Context prefix structure differs from expected"
```

**File: `core/tests/suite/compact.rs`**
```
5. auto_compact_runs_after_token_limit_hit
   Ignore reason: "SPEC-957: Timeout waiting for TaskComplete event"

6. auto_compact_stops_after_failed_attempt
   Ignore reason: "SPEC-957: Timeout waiting for Error event"

7. auto_compact_allows_multiple_attempts_when_interleaved_with_other_turn_events
   Ignore reason: "SPEC-957: Timeout during interleaved processing"
```

**File: `core/tests/suite/exec_stream_events.rs`**
```
8. test_exec_timeout_returns_partial_output
   Ignore reason: "SPEC-957: ExecStream private fields"
```

### Investigation Tasks

1. **Payload Structure Analysis** (compact_resume_fork tests)
   - Run tests with `--include-ignored --nocapture`
   - Capture actual vs expected payloads
   - Identify: role changes (developer→user), content format changes, extra/missing fields
   - **Action**: Update expected payloads to match current implementation

2. **Tools Array Analysis** (prompt_caching test 3)
   - Compare expected tools array structure with actual
   - Check if tool definitions changed or ordering differs
   - **Action**: Update expectations or fix generation

3. **Context Prefix Analysis** (prompt_caching test 4)
   - Compare expected prefix structure with actual
   - Identify format changes in environment context, instructions
   - **Action**: Update expectations or fix generation

4. **Timeout Root Cause** (compact tests 5-7)
   - Add tracing/debug output
   - Check if events are emitted but not received
   - Check mock server response timing
   - **Action**: Fix timing issues or update test timeouts

---

## Phase 2: Update compact_resume_fork Expected Payloads

Based on Phase 1 findings, update test expectations in:
- `core/tests/suite/compact_resume_fork.rs`

**Approach**:
1. Capture current actual payloads from test runs
2. Verify the current behavior is correct (not a regression)
3. Update `expected` JSON structures to match
4. Remove `#[ignore]` attributes
5. Validate tests pass

---

## Phase 3: ExecStream API Exposure

**Tests Blocked**:
```
core/tests/suite/exec_stream_events.rs:
- test_exec_stdout_stream_events_echo (STUBBED)
- test_exec_stderr_stream_events_echo (STUBBED)
```

**Current Issue**: `StdoutStream` fields are private

**Tasks**:
1. Read `exec_stream_events.rs` to understand test requirements
2. Identify minimal API exposure needed
3. Options:
   - Make specific fields `pub(crate)` for testing
   - Add getter methods
   - Create test-only event collection API
4. Implement chosen approach
5. Restore test code

---

## Phase 4: Fix Remaining Feasible Tests

Based on investigation findings, fix any additional tests that have straightforward solutions.

---

## Reference Files

**Tracking Document**: `docs/SPEC-958-test-migration.md`
**Test Files**:
- `codex-rs/core/tests/suite/compact_resume_fork.rs`
- `codex-rs/core/tests/suite/prompt_caching.rs`
- `codex-rs/core/tests/suite/compact.rs`
- `codex-rs/core/tests/suite/exec_stream_events.rs`

**Key Implementation Files**:
- `codex-rs/core/src/codex.rs` - Session, turn context
- `codex-rs/core/src/protocol.rs` - Op enum, EventMsg
- `codex-rs/core/src/codex/compact.rs` - Compaction logic

---

## Success Criteria

### Must Have
- [ ] Root cause documented for all SPEC-957 tests
- [ ] compact_resume_fork tests updated and passing (or documented blockers)
- [ ] At least 3 additional tests restored to working state

### Should Have
- [ ] ExecStream API exposed, stub tests restored
- [ ] Prompt caching tests updated
- [ ] Auto-compact timeout issues resolved

### Nice to Have
- [ ] All SPEC-957 tests passing
- [ ] SPEC-958-test-migration.md fully updated

---

## Commands Reference

```bash
# Run specific test with output
cargo test -p codex-core --test all TEST_NAME -- --include-ignored --nocapture

# Run all ignored tests
cargo test -p codex-core --test all -- --ignored --nocapture

# Validation
cargo fmt --all -- --check
cargo clippy -p codex-core --all-targets -- -D warnings
cargo test -p codex-core --test all

# Check test count
cargo test -p codex-core --test all -- --list 2>&1 | grep -c "test"
```

---

## Session Start Checklist

1. Load CLEARFRAME.md
2. Load this prompt (docs/SPEC-958-session-11-prompt.md)
3. Review docs/SPEC-958-test-migration.md for context
4. Begin Phase 1 investigation
