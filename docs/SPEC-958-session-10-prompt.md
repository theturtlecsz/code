# Continuation Prompt for SPEC-958 Session 10

## Prior Session Summary (Session 9)

### Completed:
1. ✅ Added `rollout_path: Option<PathBuf>` to `SessionConfiguredEvent`
2. ✅ Wired rollout_path from `RolloutRecorder` through session creation
3. ✅ Added 3 new fork_conversation tests using rollout_path API
4. ✅ Updated mcp-server tests with new field
5. ✅ Updated docs/SPEC-958-test-migration.md

### Key Files Modified (Session 9):
- `codex-rs/core/src/protocol.rs:1287-1305` - SessionConfiguredEvent with rollout_path
- `codex-rs/core/src/codex.rs:3248-3250, 3432` - Wire rollout_path
- `codex-rs/core/tests/suite/fork_conversation.rs` - 3 new tests
- `codex-rs/mcp-server/src/outgoing_message.rs:287, 319` - Test updates

---

## Session 10 Goals (Full Scope)

### Phase 3: Fix exec output_schema Test (1 test) - START HERE

**File**: `codex-rs/exec/tests/suite/output_schema.rs`
**Test**: `exec_includes_output_schema_in_request`

Steps:
1. Read the test file and understand current expectations
2. Check how output_schema flows through exec module
3. Compare with working JSON output tests in core
4. Fix test expectations or implementation as needed
5. Remove `#[ignore]` attribute if present

### Phase 2: Add Op::OverrideTurnContext to codex_core (6 tests)

**Reference**: `codex_protocol::protocol::Op::OverrideTurnContext` (lines 125-153)

Steps:
1. Study how OverrideTurnContext works in codex_protocol
2. Add variant to `codex_core::protocol::Op` enum
3. Wire through session handling in codex.rs
4. Restore tests in order:
   - `override_turn_context_does_not_persist_when_config_exists`
   - `override_turn_context_does_not_create_config_file`
   - `overrides_turn_context_but_keeps_cached_prefix_and_key_constant`
   - `per_turn_overrides_keep_cached_prefix_and_key_constant`
   - `send_user_turn_with_no_changes_does_not_send_environment_context`
   - `send_user_turn_with_changes_sends_environment_context`

**Test Files**:
- `codex-rs/core/tests/suite/model_overrides.rs` (2 tests)
- `codex-rs/core/tests/suite/prompt_caching.rs` (4 tests)

### Phase 4: Update compact_resume_fork Tests (2 tests)

**File**: `codex-rs/core/tests/suite/compact_resume_fork.rs`

Steps:
1. Modify `start_test_conversation()` to return rollout_path from SessionConfiguredEvent
2. Update `fetch_conversation_path()` helper to use captured path
3. Update `resume_conversation()` and `fork_conversation()` helpers
4. Remove `#[ignore]` from:
   - `compact_resume_and_fork_preserve_model_history_view`
   - `compact_resume_fork_twice_preserve_model_history_view`
5. Run full test suite to verify

### Phase 5: Investigate SPEC-957 Issues (6 tests)

**Goal**: Understand blockers, identify quick wins

Tests to investigate:
| Test | File | Issue |
|------|------|-------|
| `prompt_tools_are_consistent_across_requests` | prompt_caching.rs | Tools array structure |
| `prefixes_context_and_instructions_once_and_consistently_across_requests` | prompt_caching.rs | Context prefix structure |
| `auto_compact_runs_after_token_limit_hit` | compact.rs | Timeout |
| `auto_compact_stops_after_failed_attempt` | compact.rs | Timeout |
| `auto_compact_allows_multiple_attempts_when_interleaved_with_other_turn_events` | compact.rs | Timeout |
| `test_exec_timeout_returns_partial_output` | exec_stream_events.rs | ExecStream private fields |

Actions:
1. Read each test to understand what it's testing
2. Identify if issue is timeout (async/mock) or structural (API change)
3. Document findings in SPEC-958-test-migration.md
4. Fix any quick wins (<30 min each)

---

## Files to Examine

### Phase 3 (exec output_schema):
```
codex-rs/exec/tests/suite/output_schema.rs
codex-rs/exec/src/lib.rs
codex-rs/core/tests/suite/json_result.rs  # Working reference
```

### Phase 2 (OverrideTurnContext):
```
codex-rs/protocol/src/protocol.rs:125-153  # Reference implementation
codex-rs/core/src/protocol.rs              # Add variant here
codex-rs/core/src/codex.rs                 # Wire through session
codex-rs/core/tests/suite/model_overrides.rs
codex-rs/core/tests/suite/prompt_caching.rs
```

### Phase 4 (compact_resume_fork):
```
codex-rs/core/tests/suite/compact_resume_fork.rs
```

### Phase 5 (SPEC-957):
```
codex-rs/core/tests/suite/prompt_caching.rs
codex-rs/core/tests/suite/compact.rs
codex-rs/core/tests/suite/exec_stream_events.rs
```

---

## Tracking Document

**Update**: `docs/SPEC-958-test-migration.md`

Current statistics to update:
| Category | Total | Working | Remaining |
|----------|-------|---------|-----------|
| JSON Output | 3 | 3 ✅ | 0 |
| Path Query | 5 | 3 ✅ | 2 |
| Per-Turn Context | 6 | 0 | 6 |
| Exec output_schema | 1 | 0 | 1 |
| SPEC-957 Issues | 6 | 0 | 6 |
| **Session 10 Target** | **15** | - | **15** |

---

## Validation Commands

```bash
# After each change
cd codex-rs && cargo check -p codex-core

# After test restoration
cargo test -p codex-core --test all <test_name>

# Full validation before commit
cargo fmt --all -- --check
cargo clippy -p codex-core --all-targets -- -D warnings
cargo test -p codex-core --test all
```

---

## Success Criteria

### Minimum (Phase 3 + Phase 4):
- [ ] exec output_schema test fixed and passing
- [ ] compact_resume_fork tests using rollout_path API
- [ ] 4 tests restored (1 + 2 + quick wins)

### Target (+ Phase 2):
- [ ] Op::OverrideTurnContext added to codex_core
- [ ] 6 per-turn context tests restored
- [ ] 10+ tests restored total

### Stretch (+ Phase 5):
- [ ] SPEC-957 issues investigated and documented
- [ ] Any quick-win SPEC-957 tests fixed
- [ ] Clear action items for remaining blockers

---

## Recommended Order

1. **Phase 3** (~30 min): Small, isolated - builds momentum
2. **Phase 4** (~45 min): Uses Session 9 API, unblocks integration tests
3. **Phase 2** (~2-3 hr): Larger architectural change, core functionality
4. **Phase 5** (~1 hr): Investigation, document findings

---

## Local Memory Context

```bash
~/.claude/hooks/lm-search.sh "SPEC-958" 5
~/.claude/hooks/lm-search.sh "OverrideTurnContext" 5
~/.claude/hooks/lm-search.sh "Op enum protocol" 5
```

---

## Quick Start

```bash
# Load this prompt in new session:
cat ~/code/docs/SPEC-958-session-10-prompt.md

# Or reference it:
# "Continue SPEC-958 Session 10 using docs/SPEC-958-session-10-prompt.md"
```
