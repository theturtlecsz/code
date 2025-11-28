# SPEC-959 Session 1: StreamController Per-ID Stream Buffers

## Context

**Previous Session**: SPEC-958 Session 12 (Test Documentation) - COMPLETE
**Primary Focus**: SPEC-959 StreamController refactor
**Secondary**: SPEC-940 Performance Instrumentation (if time permits)
**Mode**: CLEARFRAME Implementation Mode

---

## Session Setup

```bash
# Load context
load ~/.claude/CLEARFRAME.md and docs/SPEC-959-session-1-prompt.md

# Reference files
read docs/SPEC-959-streamcontroller-per-id-buffers/spec.md
read codex-rs/tui/src/streaming/controller.rs
```

---

## SPEC-959 Overview

**Problem**: StreamController uses single buffer per StreamKind, causing content merging when multiple turns stream concurrently.

**Current Architecture**:
```rust
// Single buffer per kind - problematic
states: [StreamState; 2]  // [Answer, Reasoning]
```

**Target Architecture**:
```rust
// Per-ID buffers - enables concurrent streams
answer_streams: HashMap<String, StreamState>
reasoning_streams: HashMap<String, StreamState>
```

**Impact**: Fixes quick sequential message submission, network reordering, session replay issues.

**Unblocks**: 2 remaining ignored TUI tests from SPEC-958.

---

## Implementation Tasks

### Phase 1: Core Refactor (2-3h)

1. **Read and understand StreamController**
   - `codex-rs/tui/src/streaming/controller.rs`
   - Identify all StreamState access patterns
   - Map callers and consumers

2. **Refactor data structure**
   - Replace `states: [StreamState; 2]` with HashMap approach
   - Update `StreamKind` usage to include ID parameter
   - Modify `push_chunk()`, `complete()`, `get_buffer()` signatures

3. **Update all call sites**
   - `chatwidget/mod.rs` - streaming event handlers
   - `history_cell/mod.rs` - buffer consumption
   - Any other StreamController consumers

### Phase 2: Edge Cases (1-2h)

4. **Handle stream lifecycle**
   - Stream creation on first chunk
   - Stream cleanup on completion
   - Memory management for abandoned streams

5. **Concurrent stream behavior**
   - Multiple active streams of same kind
   - Stream isolation guarantees
   - Order preservation within single stream

### Phase 3: Testing (1-2h)

6. **Enable ignored tests**
   - `test_concurrent_answer_streams` (or similar)
   - Remove `#[ignore]` attributes
   - Verify pass

7. **Add regression tests**
   - Concurrent stream isolation
   - Rapid sequential submissions
   - Stream cleanup verification

### Phase 4: Validation (30min)

8. **Full validation harness**
   ```bash
   cd codex-rs
   cargo fmt --all
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo build --workspace --all-features
   cargo test -p codex-tui
   cargo test -p codex-core
   ```

---

## Secondary: SPEC-940 (If Primary Completes)

**Status**: Phase 1 complete (timing.rs infrastructure)
**Remaining**: Phases 2-4 (9-13h)

### Phase 2: Benchmark Harness (3-4h)
- Create benchmark framework using criterion or custom
- Define baseline measurements for key operations
- Integrate with timing.rs macros

### Phase 3: Pre/Post Validation Baselines (3-4h)
- Capture DirectProcessExecutor performance baselines
- Database operation timing
- MCP call latency

### Phase 4: Statistical Reporting (3-5h)
- Aggregate timing data
- Generate performance reports
- CI integration for regression detection

---

## Session End Checklist

### Required
- [ ] SPEC-959 implementation complete
- [ ] All TUI tests passing (current ignored tests enabled)
- [ ] Full validation passes (fmt, clippy, build, tests)

### Optional (User Selected)
- [ ] **Local-memory cleanup**: Prune stale memories, update importance scores
- [ ] **Commit checkpoint**: Create git commit with session progress
- [ ] **SPEC.md update**: Update tracker with detailed session notes

---

## Validation Commands

```bash
# Full validation (required)
cd ~/code/codex-rs
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace --all-features
cargo test -p codex-tui
cargo test -p codex-core

# Check TUI test status
cargo test -p codex-tui -- --list 2>&1 | grep -E "ignore|test"

# Local-memory cleanup (optional)
~/.claude/hooks/lm-prune.sh --dry-run
~/.claude/hooks/lm-maintenance.sh
```

---

## Files to Modify

| File | Action | Priority |
|------|--------|----------|
| `tui/src/streaming/controller.rs` | Core refactor | P0 |
| `tui/src/streaming/mod.rs` | Export updates | P0 |
| `tui/src/chatwidget/mod.rs` | Call site updates | P0 |
| `tui/src/history_cell/mod.rs` | Buffer consumption | P1 |
| `tui/tests/*.rs` | Enable ignored tests | P1 |
| `SPEC.md` | Status update | P2 |

---

## Success Criteria

1. **SPEC-959 Complete**
   - HashMap-based per-ID stream buffers implemented
   - All existing tests pass
   - Previously ignored tests now pass

2. **Code Quality**
   - Zero clippy warnings
   - Proper error handling
   - Clear documentation on new API

3. **Session Artifacts**
   - Git commit with changes
   - SPEC.md updated
   - Local-memory cleaned (if applicable)

---

## Priority Order (If Time Permits)

1. SPEC-959 (Primary) - 4-8h
2. SPEC-940 Phase 2 (Secondary) - 3-4h
3. SYNC-001-003 Security bundle - 4-6h (next session if not reached)

---

## Commands

```bash
# Start session
load ~/.claude/CLEARFRAME.md and docs/SPEC-959-session-1-prompt.md

# Quick status check
cd ~/code/codex-rs && cargo test -p codex-tui -- --list 2>&1 | grep -c ignore
```
