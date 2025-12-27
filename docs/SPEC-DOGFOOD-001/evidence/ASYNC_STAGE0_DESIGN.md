# Async Stage0 Design Document

**Session:** S31
**Date:** 2025-12-26
**Status:** ✅ IMPLEMENTED (S31)
**SPEC:** SPEC-DOGFOOD-001 UX Improvement

### Implementation Commits
- `220832042` - feat(stage0): Implement async Stage0 with non-blocking TUI
- `08d892178` - fix(tier2): Reduce prompt to fit NotebookLM 2k char query limit

### Verified Behavior
- TUI remains responsive during Stage0 execution
- Progress polling via `on_commit_tick()`
- 5-minute timeout with graceful fallback
- Thread disconnection handling

### Remaining Gaps (Tracked in SPEC-TIER2-SOURCES)
- Tier2 source-based architecture (query limit workaround)
- System pointer storage validation

---

## Problem Statement

The TUI freezes for 15-30 seconds during Stage0 execution because:
1. `handle_spec_auto` spawns Stage0 in a thread then immediately `.join()`s
2. During `.join()`, no events are processed - TUI appears frozen
3. User cannot see progress, scroll history, or cancel

**Root cause:** `pipeline_coordinator.rs:285` - blocking thread join

---

## Design Goals

1. **Responsiveness:** TUI remains interactive during Stage0
2. **Progress Feedback:** User sees real-time status updates
3. **Cancellation:** User can abort Stage0 with Ctrl+C
4. **Minimal Changes:** Reuse existing `Stage0PendingOperation` infrastructure
5. **State Machine Integrity:** Fit within existing `SpecAutoPhase` pattern

---

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────┐
│                        TUI Event Loop                         │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ on_commit_tick()                                        │  │
│  │   ├── poll_stage0_pending() ←───────────────────────┐  │  │
│  │   │     └── try_recv() progress/result              │  │  │
│  │   │           │                                      │  │  │
│  │   │           ├── Progress → update status bar      │  │  │
│  │   │           │                                      │  │  │
│  │   │           └── Result → continue pipeline        │  │  │
│  │   │                                                  │  │  │
│  │   └── Other tick handlers...                        │  │  │
│  └─────────────────────────────────────────────────────│──┘  │
│                                                         │     │
│  ┌─────────────────────────────────────────────────────│──┐  │
│  │ handle_spec_auto()                                   │  │  │
│  │   ├── spawn_stage0_async() ─────────────────────┐   │  │  │
│  │   │     └── returns Stage0PendingOperation      │   │  │  │
│  │   │                                              │   │  │  │
│  │   └── set phase = Stage0Pending { ... }         │   │  │  │
│  │         └── return immediately (non-blocking)    │   │  │  │
│  └──────────────────────────────────────────────────│──│──┘  │
└────────────────────────────────────────────────────│──│──────┘
                                                      │  │
                    ┌─────────────────────────────────┘  │
                    │                                     │
┌───────────────────▼───────────────────────────────────▼──────┐
│                     Background Thread                         │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ run_stage0_for_spec()                                    │ │
│  │   ├── Send(Progress::Starting)                          │ │
│  │   ├── Send(Progress::CheckingLocalMemory)               │ │
│  │   ├── Send(Progress::LoadingConfig)                     │ │
│  │   ├── Send(Progress::CheckingTier2Health)               │ │
│  │   ├── Send(Progress::QueryingTier2)                     │ │
│  │   ├── ... Tier2 HTTP (5-15s) ...                        │ │
│  │   ├── Send(Progress::Tier2Complete(duration_ms))        │ │
│  │   └── Send(result) via result_tx                        │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Phase 1: Add Stage0Pending to SpecAutoPhase

**File:** `tui/src/chatwidget/spec_kit/state.rs`

```rust
// Add to SpecAutoPhase enum (around line 276)
pub enum SpecAutoPhase {
    /// Stage0 executing in background (SPEC-DOGFOOD-001 S32)
    Stage0Pending {
        /// Current progress status for display
        status: String,
        /// When Stage0 started (for timeout detection)
        started_at: std::time::Instant,
    },
    Guardrail,
    ExecutingAgents { ... },
    CheckingConsensus,
    // ... rest of phases
}
```

### Phase 2: Add Pending Operation to ChatWidget

**File:** `tui/src/chatwidget/mod.rs`

```rust
// Add field to ChatWidget struct
pub struct ChatWidget {
    // ... existing fields ...

    /// Pending Stage0 operation for async execution
    /// When Some, poll in on_commit_tick for progress/completion
    stage0_pending: Option<Stage0PendingOperation>,
}
```

### Phase 3: Modify handle_spec_auto

**File:** `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`

**Current (blocking):**
```rust
let handle = std::thread::spawn(move || {
    run_stage0_for_spec(...)
});
let result = match handle.join() {  // BLOCKS
    Ok(r) => r,
    Err(e) => { ... }
};
// Continue with result...
```

**New (async):**
```rust
// Spawn async and return immediately
let pending = spawn_stage0_async(
    config_clone,
    spec_id.clone(),
    spec_content.clone(),
    cwd_clone,
    stage0_config_clone,
);

// Store pending operation for polling
widget.stage0_pending = Some(pending);

// Set phase to Stage0Pending
if let Some(ref mut state) = widget.spec_auto_state {
    state.phase = SpecAutoPhase::Stage0Pending {
        status: "Starting Stage0...".to_string(),
        started_at: std::time::Instant::now(),
    };
}

// Return immediately - on_commit_tick will poll for completion
return;
```

### Phase 4: Add Polling in on_commit_tick

**File:** `tui/src/chatwidget/mod.rs`

Add to `on_commit_tick()`:

```rust
fn on_commit_tick(&mut self) {
    // Poll Stage0 pending operation
    if let Some(ref pending) = self.stage0_pending {
        // Check for progress updates (non-blocking)
        while let Ok(progress) = pending.progress_rx.try_recv() {
            match progress {
                Stage0Progress::Starting => {
                    self.update_stage0_status("Starting...");
                }
                Stage0Progress::CheckingLocalMemory => {
                    self.update_stage0_status("Checking local-memory...");
                }
                Stage0Progress::LoadingConfig => {
                    self.update_stage0_status("Loading config...");
                }
                Stage0Progress::CheckingTier2Health => {
                    self.update_stage0_status("Checking Tier2 health...");
                }
                Stage0Progress::QueryingTier2 => {
                    self.update_stage0_status("Querying NotebookLM...");
                }
                Stage0Progress::Tier2Complete(ms) => {
                    self.update_stage0_status(&format!("Tier2 complete ({}ms)", ms));
                }
                Stage0Progress::Finished { success, tier2_used, duration_ms } => {
                    self.update_stage0_status(&format!(
                        "Stage0 finished: success={}, tier2={}, {}ms",
                        success, tier2_used, duration_ms
                    ));
                }
            }
        }

        // Check for final result (non-blocking)
        if let Ok(result) = pending.result_rx.try_recv() {
            // Take ownership and clear pending
            let pending = self.stage0_pending.take().unwrap();

            // Continue pipeline with result
            self.continue_pipeline_after_stage0(result, pending.spec_id, pending.spec_content);
        }
    }

    // ... rest of tick handlers
}
```

### Phase 5: Add Status Update Helper

**File:** `tui/src/chatwidget/mod.rs`

```rust
fn update_stage0_status(&mut self, status: &str) {
    if let Some(ref mut state) = self.spec_auto_state {
        if let SpecAutoPhase::Stage0Pending { status: ref mut s, .. } = state.phase {
            *s = status.to_string();
        }
    }
    // Trigger redraw for status bar
    self.mark_dirty();
}

fn continue_pipeline_after_stage0(
    &mut self,
    result: Stage0ExecutionResult,
    spec_id: String,
    spec_content: String,
) {
    // Process result (same as current blocking code)
    if let Some(ref stage0_result) = result.result {
        // Store Stage0 result in state
        if let Some(ref mut state) = self.spec_auto_state {
            state.stage0_result = Some(stage0_result.clone());
            state.stage0_skip_reason = result.skip_reason.clone();
        }

        // Log success
        self.history_push(history_cell::new_info_event(format!(
            "Stage 0: {} ({}ms, Tier2: {}, Cache: {})",
            if result.tier2_used { "Tier2 query" } else { "Local only" },
            result.duration_ms,
            result.tier2_used,
            result.cache_hit,
        )));
    }

    // Transition to Guardrail phase
    if let Some(ref mut state) = self.spec_auto_state {
        state.phase = SpecAutoPhase::Guardrail;
    }

    // Continue with guardrail execution
    self.continue_spec_auto_pipeline();
}
```

### Phase 6: Update Status Bar Display

**File:** `tui/src/chatwidget/mod.rs` (in status bar rendering)

```rust
// In status bar rendering, show Stage0 progress
if let Some(ref state) = self.spec_auto_state {
    match &state.phase {
        SpecAutoPhase::Stage0Pending { status, started_at } => {
            let elapsed = started_at.elapsed().as_secs();
            let display = format!("[Stage0: {} ({:}s)]", status, elapsed);
            // Render display in status bar
        }
        // ... other phases
    }
}
```

---

## Files to Modify (S32)

| File | Changes |
|------|---------|
| `state.rs` | Add `Stage0Pending` variant to `SpecAutoPhase` |
| `mod.rs` (chatwidget) | Add `stage0_pending` field, polling in `on_commit_tick`, helper methods |
| `pipeline_coordinator.rs` | Use `spawn_stage0_async` instead of blocking spawn+join |
| Status bar renderer | Display Stage0 progress status |

---

## Existing Infrastructure (No Changes Needed)

| Component | Location | Status |
|-----------|----------|--------|
| `Stage0PendingOperation` | `stage0_integration.rs:50-64` | Ready |
| `spawn_stage0_async()` | `stage0_integration.rs:66-101` | Ready |
| `Stage0Progress` enum | `stage0_integration.rs:22-45` | Ready |
| Progress sender integration | `run_stage0_for_spec` | Ready - accepts `Option<Sender>` |

---

## Timeout Handling

**Recommendation:** Add timeout detection in `on_commit_tick`:

```rust
if let SpecAutoPhase::Stage0Pending { started_at, .. } = state.phase {
    let elapsed = started_at.elapsed();
    if elapsed > Duration::from_secs(120) {
        // Stage0 timeout - cancel and continue with fallback
        self.stage0_pending = None;
        self.history_push(history_cell::new_warning_event(
            "Stage0 timeout (120s) - continuing with fallback"
        ));
        state.stage0_skip_reason = Some("Timeout".to_string());
        state.phase = SpecAutoPhase::Guardrail;
        self.continue_spec_auto_pipeline();
    }
}
```

---

## Cancellation Support

**Recommendation:** Handle Ctrl+C during Stage0:

```rust
// In keyboard handler for Ctrl+C
if self.stage0_pending.is_some() {
    self.stage0_pending = None;  // Drop channels, thread will see disconnected
    self.history_push(history_cell::new_warning_event("Stage0 cancelled"));
    if let Some(ref mut state) = self.spec_auto_state {
        state.stage0_skip_reason = Some("User cancelled".to_string());
        state.phase = SpecAutoPhase::Guardrail;
    }
    // Continue without Stage0 context
}
```

---

## Testing Plan

### Manual Testing (S32)

1. **Progress Display:**
   - Start `/speckit.auto SPEC-DOGFOOD-001`
   - Verify status bar shows Stage0 progress updates
   - Verify elapsed time counter increments

2. **Responsiveness:**
   - During Stage0, scroll history up/down
   - Verify TUI responds immediately
   - Verify no freeze during Tier2 query

3. **Completion:**
   - Wait for Stage0 to complete
   - Verify pipeline continues to Guardrail phase
   - Verify Stage0 result is used for context injection

4. **Cancellation:**
   - Start `/speckit.auto`
   - Press Ctrl+C during Stage0
   - Verify Stage0 is cancelled gracefully
   - Verify pipeline continues without Stage0 context

5. **Timeout:**
   - (Optional) Test with very slow Tier2
   - Verify 120s timeout triggers fallback

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Race condition in polling | Low | Medium | Use try_recv(), not blocking recv() |
| Channel disconnection | Low | Low | Handle Err(TryRecvError::Disconnected) |
| State machine corruption | Medium | High | Careful phase transitions with logging |
| Progress messages out of order | Low | Low | Display only latest status |
| Thread panic | Low | Medium | Already handled in existing code |

---

## Dependencies

- No new crates required
- Uses existing `std::sync::mpsc` channels
- Uses existing `std::thread::spawn`

---

## Estimated Effort

| Task | Time |
|------|------|
| Add Stage0Pending to enum | 10 min |
| Add stage0_pending field | 5 min |
| Modify handle_spec_auto | 20 min |
| Add polling in on_commit_tick | 30 min |
| Add helper methods | 20 min |
| Update status bar | 15 min |
| Testing | 30 min |
| **Total** | ~2.5 hours |

---

## Session 32 Deliverables

1. [ ] Implement Phase 1-6 as described
2. [ ] Manual testing of all scenarios
3. [ ] Update HANDOFF.md with results
4. [ ] Mark UX3 acceptance criteria as PASS
