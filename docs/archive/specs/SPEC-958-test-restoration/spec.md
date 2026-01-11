# SPEC-958: Comprehensive Workspace Test Restoration

**Status**: In Progress
**Created**: 2025-11-26
**Priority**: P2 - Medium
**Type**: Technical Debt - Test Infrastructure
**Blocks**: CI stability, test coverage metrics

---

## Background

Continuation of SPEC-955 and SPEC-957 test cleanup work. SPEC-957 completed its primary goal (zero warnings). SPEC-955 fixed TUI test deadlocks (6/9 passing, 3 ignored). This SPEC tracks remaining test restoration across the full workspace.

---

## Current State (Post SPEC-955 Session)

### codex-tui
- **test_harness tests**: 6 passed, 3 ignored
- **Ignored tests** (SPEC-955 documented limitations):
  - `test_overlapping_turns_no_interleaving`: send_user_message + streaming causes blocking
  - `test_chatwidget_two_turns_snapshot`: StreamController 2+ concurrent Answer streams
  - `test_three_overlapping_turns_extreme_adversarial`: StreamController 3+ concurrent streams

### codex-core
- **lib**: 638 passed, 10 ignored
- **integration**: 63 passed, 46 ignored
- **Key issues**:
  - Op types API boundary (codex_protocol::Op vs codex_core::Op)
  - Behavior change tests need assertion updates

### Other Crates
- **MCP manager**: 2 passed (restored in SPEC-957)
- **Performance tests**: 6 passed (thresholds adjusted in SPEC-957)

---

## Remaining Work

### Phase 1: TUI Event Loop Refactor (High Priority)
**Goal**: Enable the 3 ignored TUI tests

1. Refactor StreamController to use per-ID stream buffers
   - `HashMap<String, StreamState>` per kind
   - Estimated: 4-8 hours
   - Tracked in: docs/SPEC-955-tui-test-deadlock/spec.md

2. Update event handling to support concurrent user messages + streaming
   - Investigate why send_user_message + AgentMessageDelta causes blocking
   - May require TestHarness event loop simulation

### Phase 2: codex-core Test Restoration

1. **Op Types Investigation** (~2 hours)
   - Analyze codex_protocol::Op vs codex_core::Op API split
   - Decision needed: expose variants or document as intentional boundary
   - Affected tests: ~14 (see SPEC-957 Session 12)

2. **Behavior Change Tests** (~4 hours)
   - Update assertions for tests failing due to behavior drift:
     - `review_input_isolated_from_parent_history`
     - `review_does_not_emit_agent_message_on_structured_output`
     - `review_uses_custom_review_model_from_config`

3. **Ignored Test Audit** (~3 hours)
   - Create inventory by category
   - Define action plan for each category
   - Target: <40 ignored tests (from ~56)

### Phase 3: Clippy Cleanup

1. Fix deprecated `rand::gen_range` -> `random_range` (~12 occurrences)
2. Fix collapsible if statements
3. Fix is_some() pattern matching
4. MCP test server warning

---

## Success Criteria

- [ ] All 9 TUI test_harness tests passing (0 ignored)
- [ ] Op types decision documented
- [ ] ≥3 behavior tests updated and passing
- [ ] 0 clippy warnings in workspace
- [ ] Ignored test inventory complete with action plans
- [ ] cargo test -p codex-core: 0 failures
- [ ] cargo test -p codex-tui: 0 failures
- [ ] Total ignored tests < 40 (from ~56)

---

## Estimated Effort

| Phase | Description | Estimate |
|-------|-------------|----------|
| 1 | TUI Event Loop Refactor | 4-8 hours |
| 2 | codex-core Test Restoration | 6-9 hours |
| 3 | Clippy Cleanup | 2-3 hours |
| **Total** | | **12-20 hours** |

---

## Dependencies

**Blocked By**: None
**Blocks**:
- CI test stability
- Test coverage reporting
- Future feature development (need reliable tests)

---

## References

- `docs/SPEC-KIT-955-tui-test-deadlock/spec.md` - TUI deadlock investigation
- `docs/SPEC-KIT-957-tui-test-harness-fixes/` - Warning cleanup completed
- `codex-rs/tui/src/chatwidget/test_harness.rs` - Test infrastructure
- `codex-rs/core/src/` - Core crate tests
