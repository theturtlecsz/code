# PM-004: Close Remaining Feature Gaps

**Branch:** `feat/pm-004-close-remaining-gaps`
**Status:** In Progress
**Date:** 2026-02-14

## Objective

Complete the remaining PM-004 feature IDs (D10, D18, D22) that were deferred from the initial batches A/B/C implementation.

## Features

### 1. Write-Mode Double Confirmation (PM-UX-D10)

**Current State:**

* Write mode toggle exists (w key)
* Visual warning shows \[w:ON] in orange
* No confirmation modal before executing write-mode runs

**Implementation:**

* Add `PmWriteConfirmation` state to `ChatWidget`
* Show modal when F8/F9 pressed with write mode enabled
* Modal: "Write mode enabled - will modify files. Continue? \[Y/N]"
* Y: Execute run with write\_mode=worktree
* N/Esc: Cancel, return to detail view
* If write mode OFF: Execute immediately (no modal)

**Files:**

* `pm_overlay.rs`: Add modal rendering, update `execute_pm_run()`
* `pm_handlers.rs`: Add Y/N/Esc handlers when modal active
* `mod.rs`: Add modal state to `ChatWidget`

### 2. Filter Input UI (PM-UX-D18)

**Current State:**

* Filter infrastructure exists (`set_filter()`, ancestor-preserving logic)
* No keyboard trigger or input UI

**Implementation:**

* `/` key in list mode: Enter filter input mode
* Show input prompt at bottom: "Filter: \[text]\_"
* Enter: Apply filter
* Esc: Cancel/clear filter
* Backspace/chars: Edit filter text
* Visual feedback: Show "Filtered: N items" in footer

**Files:**

* `pm_overlay.rs`: Add filter input state, rendering
* `pm_handlers.rs`: Add `/` handler, filter input handlers
* Update `render_list_footer()` to show filter status

### 3. F10 Cancel Active Run (PM-UX-D22)

**Current State:**

* F10 shows placeholder "not yet implemented" toast

**Implementation:**

* F10 in detail view: Cancel active run for current work item
* RPC: `bot.cancel` with workspace + work\_item\_id + run\_id
* Requires active run (extract from node.latest\_run)
* Degraded mode: Disabled
* Success: Toast "Cancelled run {run\_id}"
* Error: Show error in chat history

**Files:**

* `pm_overlay.rs`: Add `get_active_run_id()` method
* `pm_handlers.rs`: Update F10 handler to call cancel
* `mod.rs`: Add `execute_pm_cancel()` method

## Constraints

* LOC budget: \~200 LOC total (D10: \~90, D18: \~80, D10: \~30)
* No protocol changes (use existing RPCs)
* Preserve all existing F5-F9, F11 behavior
* Maintain degraded mode safety

## Verification

```bash
cd codex-rs
cargo fmt --all -- --check
cargo clippy -p codex-tui --all-targets --all-features -- -D warnings
cargo test -p codex-tui --lib pm_overlay
cargo test -p codex-tui --lib pm_handlers
```

## Success Criteria

* [x] Write confirmation modal shows when write mode enabled
* [x] Filter input mode works with `/` key
* [x] F10 cancels active runs
* [x] All existing tests pass (61 PM overlay, 2 PM handlers)
* [x] Degraded mode safety preserved

## Implementation Summary

**Total LOC:** \~200 (within budget)

**Features Implemented:**

1. Write-mode confirmation modal with Y/N/Esc handling
2. Filter input mode with `/` trigger and char/backspace/enter/esc handlers
3. F10 cancel active run with validation and error handling

**Verification Results:**

* ✅ `cargo fmt --all -- --check` - PASS
* ✅ `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` - PASS
* ✅ `cargo test -p codex-tui --lib pm_overlay` - 61 tests PASS
* ✅ `cargo test -p codex-tui --lib pm_handlers` - 2 tests PASS
