# PM-004: Sort Mode Session Persistence

<!-- REFRESH-BLOCK
query: "PM-004 sort mode persist session"
snapshot: (none - read-only state persistence)
END-REFRESH-BLOCK -->

## Objective

Preserve PM sort mode across closing/reopening the overlay within the same TUI session.

## Spec

* **PM-UX-D3**: List view behavior (30s-to-truth questions)
* **PM-UX-D12**: Keyboard model
* **Decisions**: D138, D143

## Problem

Sort mode reset to UpdatedDesc every time the overlay was closed and reopened, losing user preference within a session.

## Solution

Move sort mode state out of transient PmOverlay into persistent PmState:

* **Before**: Sort mode stored in PmOverlay (destroyed on close)
* **After**: Sort mode saved to PmState.last\_sort\_mode (persists across open/close)

## Implementation

### Changes

1. **PmState** (pm\_overlay.rs):
   * Added `last_sort_mode: Option<SortMode>` field
   * Persists across overlay open/close within session

2. **PmOverlay::new()** (pm\_overlay.rs):
   * Added `initial_sort_mode: Option<SortMode>` parameter
   * Uses provided mode or defaults to UpdatedDesc if None

3. **open\_pm\_overlay()** (pm\_overlay.rs):
   * Pass `pm.last_sort_mode` when creating new overlay
   * Preserves user's last sort choice

4. **Close handler** (pm\_handlers.rs):
   * Save current sort mode to `pm.last_sort_mode` before closing
   * Ensures persistence for next open

5. **Tests**:
   * All test calls updated to pass `None` (default behavior)
   * New test: `test_sort_mode_persists_with_initial_value` verifies persistence

### Behavior

* **First open in session**: Defaults to UpdatedDesc
* **Subsequent opens**: Restores last used sort mode
* **Cycle sort + close + reopen**: Sort mode is preserved
* **No cross-session persistence**: Sort mode resets on TUI restart (expected)

## Constraints Met

* ✅ No protocol/CLI/RPC/service changes
* ✅ Read-only / no mutations (state persistence only)
* ✅ Only touched pm\_overlay.rs, pm\_handlers.rs, and this brief (3 files)
* ✅ LOC delta: \~40 lines (within budget of <= 140)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
cd codex-rs && cargo test -p codex-tui --lib pm_handlers
```

Expected output: All tests pass (38 + 2 = 40 total)

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (38/38)
* [x] `cargo test -p codex-tui --lib pm_handlers` passes (2/2)
* [x] After cycling sort and closing overlay, reopening keeps prior sort mode
* [x] Default remains UpdatedDesc for first open in new session
* [x] Existing detail/list behavior and tests stay green
