# SPEC-959: StreamController Per-ID Stream Buffers

## Status: COMPLETE (2025-11-28)

### Completion Summary
- **Implementation**: HashMap-based per-ID stream buffers in `controller.rs`
- **Tests**: All 10 test_harness tests pass (0 ignored)
- **Validation**: TUI 391 tests pass, 0 failed

## Problem Statement

The `StreamController` in `tui/src/streaming/controller.rs` maintains **one buffer per StreamKind** (Answer/Reasoning), not per stream ID. When multiple streams of the same kind are active concurrently, their deltas get mixed in the shared buffer, causing merged/corrupted output.

### Current Architecture (Problematic)

```rust
pub(crate) struct StreamController {
    config: Config,
    header: HeaderEmitter,
    states: [StreamState; 2],           // Two StreamState (Answer, Reasoning)
    current_stream: Option<StreamKind>, // Only ONE active stream
    current_stream_id: Option<String>,  // Only ONE stream ID
    finishing_after_drain: bool,
    thinking_placeholder_shown: bool,
}
```

### Observed Behavior

When events arrive interleaved (e.g., Turn 2 delta, Turn 1 delta, Turn 1 final, Turn 2 delta):
```
Expected:
  Turn 1 Assistant: "hello"
  Turn 2 Assistant: "world"

Actual:
  Assistant: "worldhello"  // MERGED!
```

### When This Occurs

1. **Network reordering** - responses from different requests arrive out-of-order
2. **Parallel backend processing** - model processes multiple requests concurrently
3. **Session restore/replay** - events replayed in bulk
4. **Quick sequential submissions** - user submits messages faster than responses complete

## Proposed Solution

Refactor `StreamController` to use per-ID state maps:

```rust
pub(crate) struct StreamController {
    config: Config,
    header: HeaderEmitter,
    // Per-ID state allows concurrent streams
    answer_streams: HashMap<String, StreamState>,
    reasoning_streams: HashMap<String, StreamState>,
    // Track which stream is "active" for UI display purposes
    active_answer_id: Option<String>,
    active_reasoning_id: Option<String>,
    finishing_after_drain: bool,
    thinking_placeholder_shown: bool,
}
```

## Implementation Plan

### Phase 1: Refactor StreamController (~3 hours)

**File**: `tui/src/streaming/controller.rs`

1. Replace `states: [StreamState; 2]` with:
   - `answer_streams: HashMap<String, StreamState>`
   - `reasoning_streams: HashMap<String, StreamState>`

2. Update `begin_with_id()` to:
   - Create new `StreamState` entry if ID doesn't exist
   - Set `active_*_id` for UI tracking
   - NOT flush other streams (they continue independently)

3. Update `push_and_maybe_commit()` to:
   - Route deltas to the correct per-ID buffer
   - Emit to correct history cell based on ID

4. Update `finalize()` to:
   - Finalize specific stream ID
   - Clean up completed streams from HashMap
   - Update `active_*_id` if finalized stream was active

5. Add cleanup mechanism:
   - Remove stale streams after timeout or explicit close
   - Prevent memory leak from abandoned streams

### Phase 2: Update ChatWidget Integration (~2 hours)

**File**: `tui/src/chatwidget/streaming.rs`

1. Update `delta_text()` to pass stream ID consistently
2. Update `finalize()` to specify which stream ID to finalize
3. Update `begin()` calls to include ID

**File**: `tui/src/chatwidget/mod.rs`

1. Update `AgentMessageDelta` handler - already passes ID
2. Update `AgentMessage` handler to finalize correct stream ID
3. Update `AgentReasoning` handler similarly

### Phase 3: Enable Tests (~1 hour)

**File**: `tui/src/chatwidget/test_harness.rs`

1. Remove `#[ignore]` from:
   - `test_three_overlapping_turns_extreme_adversarial`
   - `test_chatwidget_two_turns_snapshot`

2. Verify tests pass with new architecture

### Phase 4: Edge Cases (~2 hours)

1. Handle orphaned streams (started but never finalized)
2. Handle stream ID collisions (shouldn't happen but defensive)
3. Memory management for long sessions with many streams
4. UI behavior when multiple streams are active (show most recent? all?)

## Files to Modify

| File | Changes |
|------|---------|
| `tui/src/streaming/controller.rs` | Core refactor (~200 lines) |
| `tui/src/streaming/mod.rs` | API updates if needed |
| `tui/src/chatwidget/streaming.rs` | Integration updates (~20 lines) |
| `tui/src/chatwidget/mod.rs` | Handler updates (~50 lines) |
| `tui/src/chatwidget/test_harness.rs` | Enable tests (~10 lines) |

## Success Criteria

- [x] All 10 TUI test_harness tests pass (currently 8 pass, 2 ignored) ✅ 10/10 pass
- [x] No content merging when concurrent streams active ✅ HashMap isolation
- [x] No memory leaks from abandoned streams ✅ Cleanup in finalize()
- [x] Existing single-stream behavior unchanged ✅ Verified
- [x] No new clippy warnings ✅ TUI lib compiles clean

## Estimated Effort

**Total: 4-8 hours**

- Phase 1: 3 hours (core refactor)
- Phase 2: 2 hours (integration)
- Phase 3: 1 hour (tests)
- Phase 4: 2 hours (edge cases)

## Related

- **SPEC-958**: Fixed infinite loop in `resort_history_by_order()` that was blocking these tests
- **SPEC-955**: Original investigation that identified both issues

## Test Commands

```bash
# Run all TUI tests including previously ignored
cargo test -p codex-tui --lib chatwidget::test_harness::tests -- --include-ignored --test-threads=1

# Target: 10 passed, 0 ignored
```

## Decision IDs

N/A — Implementation SPEC (COMPLETE 2025-11-28); bug fix with no architectural decision locks required.
