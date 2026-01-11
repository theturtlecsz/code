# TUI Blocking UX Audit

**Session:** S31
**Date:** 2025-12-26
**Scope:** All blocking patterns in `codex-rs/tui/src/` that freeze the interface

---

## Executive Summary

The TUI has significant blocking UX issues, primarily in the spec-kit pipeline. The root cause is synchronous execution of Stage0 and consensus operations that can take 15-30+ seconds. The async infrastructure (`Stage0PendingOperation`, `spawn_stage0_async`) exists but is not wired up.

**Critical Finding:** `pipeline_coordinator.rs:285` blocks TUI thread with `handle.join()` during Stage0 execution, freezing the UI for the entire duration (~15-30s for Tier2 queries).

---

## Blocking Pattern Categories

### 1. Thread Join Operations (HIGH PRIORITY)

| Location | Line | Pattern | Impact | Duration |
|----------|------|---------|--------|----------|
| `pipeline_coordinator.rs` | 285 | `handle.join()` | **CRITICAL** - Stage0 blocks TUI | 15-30s |
| `stage0_adapters.rs` | 495 | `handle.join()` | Tier2 HTTP in isolated thread | 5-15s |
| `resume/discovery.rs` | 210 | `handle.join()` | Resume session fetch | 1-5s |

**Root cause:** `handle_spec_auto` spawns Stage0 in a thread then immediately joins, blocking the TUI event loop.

### 2. block_on Patterns (MEDIUM PRIORITY)

These bridge sync/async boundaries but can cause nested runtime panics if misused.

| Location | Lines | Usage | Risk |
|----------|-------|-------|------|
| `consensus_coordinator.rs` | 28-42 | `block_on_sync` helper | Safe - uses `block_in_place` |
| `consensus_db.rs` | 211, 284, 377, 488, 886 | Database operations | Moderate - file I/O |
| `agent_orchestrator.rs` | 1080, 1139, 1383 | MCP calls | Moderate - network |
| `quality_gate_handler.rs` | 1851 | Agent validation | Moderate - network |
| `stage0_integration.rs` | 403, 828 | Stage0 internal | Safe - in separate thread |
| `agent_install.rs` | 553 | Agent installation | Low - one-time |
| `rate_limit_refresh.rs` | 35 | Rate limit refresh | Low - fast HTTP |
| `app.rs` | 3018 | Theme operations | Low - file I/O |

**Pattern:** Most use the `block_on_sync` helper which handles nested runtime correctly via `block_in_place`.

### 3. Channel recv() Operations (MEDIUM PRIORITY)

| Location | Lines | Type | Blocking? | Impact |
|----------|-------|------|-----------|--------|
| `agent_install.rs` | 451, 597 | `std::sync::mpsc::recv()` | **YES** | Blocks during agent install |
| `file_search.rs` | 177 | `std::sync::mpsc::recv()` | YES | Blocks search thread (not TUI) |
| `app.rs` | 933 | `std::sync::mpsc::recv()` | YES | Writer thread (not TUI) |
| Various | Many | `try_recv()` | NO | Non-blocking polling |
| Various | Many | `.recv().await` | NO | Async - yields to runtime |

**Safe patterns:** Most use `try_recv()` or async `.recv().await` which don't block.

### 4. thread::sleep Operations (LOW PRIORITY)

| Location | Lines | Duration | Purpose | Impact |
|----------|-------|----------|---------|--------|
| `quality_gate_handler.rs` | 106 | 200ms | Debounce | Low - brief |
| `chatwidget/mod.rs` | 5702 | 120ms | Animation delay | Low - brief |
| `file_search.rs` | 123, 129 | Variable | Search debounce | Low - background thread |
| `onboarding/onboarding_screen.rs` | 129, 131 | 150-200ms | Animation | Low - onboarding only |
| `terminal_info.rs` | 74, 172 | 10ms | Terminal query | Low - startup only |
| `app.rs` | 368, 630, 651, etc. | 5-10ms | Event loop throttle | Low - expected |
| `bottom_pane/mod.rs` | 896, 907, 1094, etc. | 120ms | UI debounce | Low - brief |
| `chat_composer.rs` | 282, 2470 | Variable | Paste/input | Low - user-initiated |

**Assessment:** Most sleeps are brief (<200ms) and in background threads or for debouncing.

### 5. Command::output() Subprocess Calls (MEDIUM PRIORITY)

| Location | Lines | Commands | Impact |
|----------|-------|----------|--------|
| `chatwidget/mod.rs` | 936-12832 | git status/diff | Variable - depends on repo size |
| `gh_actions.rs` | 29 | `gh auth token` | Low - fast |
| `quality_gate_handler.rs` | 1616, 1624 | Test execution | High - can be slow |
| `git_integration.rs` | 145, 179 | git commands | Low-Medium |
| `native_guardrail.rs` | 200 | Guardrail scripts | Medium - external |
| `stage0_integration.rs` | 554, 572, 848 | Local memory CLI | Medium - HTTP round-trip |
| `routing.rs` | 23, 43 | Routing check | Low |
| `stage0_seeding.rs` | 460 | Seeding | Low |
| `local_memory_cli.rs` | 89, 128 | `lm` CLI calls | Medium - HTTP |
| `cli_executor.rs` | 192 | Generic CLI | Variable |
| `get_git_diff.rs` | 71, 91 | git diff | Low-Medium |
| `lib.rs` | 655 | Config check | Low |

**Pattern:** Git operations are generally fast (<1s). Test execution can be slow (10s+).

### 6. reqwest::blocking HTTP (HIGH PRIORITY - IN ISOLATED THREAD)

| Location | Lines | Usage | Impact |
|----------|-------|-------|--------|
| `stage0_integration.rs` | 645 | Tier2 health check | Isolated in thread |
| `stage0_adapters.rs` | 21, 173, 203, 349, 380 | Tier2 HTTP client | Isolated in thread |
| `local_memory_cli.rs` | 20, 24 | Local memory HTTP | Can block TUI |

**S30 Fix:** Tier2 HTTP now runs in isolated `std::thread` to avoid nested runtime conflict.

### 7. Synchronous File I/O (LOW PRIORITY)

| Location | Pattern | Count | Impact |
|----------|---------|-------|--------|
| Various | `std::fs::read_to_string` | 50+ | Low - typically fast |
| Various | `std::fs::read_dir` | 20+ | Low - directory listing |

**Assessment:** File I/O is generally fast (<10ms) and acceptable.

---

## Priority Remediation

### P0: Stage0 Blocking (CRITICAL)

**File:** `pipeline_coordinator.rs:274-309`

**Current:**
```rust
let handle = std::thread::spawn(move || {
    run_stage0_for_spec(...)
});
let result = match handle.join() {  // <-- BLOCKS TUI 15-30s
    Ok(r) => r,
    Err(e) => { ... }
};
```

**Required:** Wire up existing async infrastructure:
- `spawn_stage0_async()` in `stage0_integration.rs:70`
- `Stage0PendingOperation` struct with progress/result channels
- Add `Stage0Pending { status: String }` phase to `SpecAutoPhase` enum
- Poll in `on_commit_tick()` for progress updates

### P1: Consensus Blocking (MEDIUM)

**File:** `pipeline_coordinator.rs:1145, 1518`

Uses `block_on_sync` which is safe but still blocks during consensus HTTP calls.

**Future:** Consider async consensus with progress polling.

### P2: Quality Gate Blocking (MEDIUM)

**File:** `quality_gate_handler.rs:1851`

Uses `block_on` for validation calls. Could benefit from async handling.

---

## Existing Async Infrastructure (Ready to Wire)

### stage0_integration.rs

```rust
// Lines 50-64
pub struct Stage0PendingOperation {
    pub progress_rx: mpsc::Receiver<Stage0Progress>,
    pub result_rx: mpsc::Receiver<Stage0ExecutionResult>,
    pub spec_id: String,
    pub spec_content: String,
    pub config: Stage0ExecutionConfig,
}

// Lines 66-101
pub fn spawn_stage0_async(...) -> Stage0PendingOperation {
    // Already implemented - spawns thread with progress channels
}
```

### Stage0Progress Enum (Lines 22-45)

```rust
pub enum Stage0Progress {
    Starting,
    CheckingLocalMemory,
    LoadingConfig,
    CheckingTier2Health,
    CompilingContext,
    QueryingTier2,
    Tier2Complete(u64),
    Finished { success: bool, tier2_used: bool, duration_ms: u64 },
}
```

**Status:** Fully implemented, NOT wired up to TUI.

---

## Blocking vs Non-Blocking Summary

| Category | Blocking Instances | Non-Blocking | Priority |
|----------|-------------------|--------------|----------|
| Thread join | 3 | - | HIGH |
| block_on | 30+ | - | MEDIUM |
| Channel recv | 3 | 25+ (try_recv, async) | LOW |
| thread::sleep | 15+ | - | LOW |
| Command::output | 40+ | - | MEDIUM |
| reqwest::blocking | 8 | - | ISOLATED |
| File I/O | 70+ | - | LOW |

---

## Recommendations

1. **S32 Implementation:** Wire `spawn_stage0_async` to pipeline_coordinator
2. **Add Stage0Pending phase** to SpecAutoPhase enum
3. **Poll progress** in `on_commit_tick()` for UI updates
4. **Future:** Consider async consensus for quality gates
5. **Testing:** Manual validation - TUI should remain responsive during Stage0

---

## Test Plan

After S32 implementation:
1. Start `/speckit.auto SPEC-DOGFOOD-001`
2. During Stage0 execution (Tier2 query), TUI should remain responsive
3. Status bar should show progress updates (Checking Tier2, Querying, etc.)
4. User can scroll history while Stage0 runs
5. Ctrl+C should cancel Stage0 gracefully
