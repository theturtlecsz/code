# PM-004: Detail Return Position Preservation

<!-- REFRESH-BLOCK
query: "PM-004 detail return position preserve"
snapshot: (none - read-only UI fix)
END-REFRESH-BLOCK -->

## Objective

Preserve list selection and scroll position when returning from detail view to list view.

## Spec

* **PM-UX-D12**: Keyboard model
* **PM-UX-D20**: Detail view layout and scrolling
* **Decisions**: D138, D143

## Problem

List scroll position was being reset due to clamping logic in `render_list` that wrote back the clamped value, overwriting the user's scroll position when switching between modes.

## Solution

Fixed `set_scroll()` to store raw scroll value without clamping. Clamping now only happens during render for display purposes, preserving user's scroll position across mode changes.

## Implementation

### Changes

1. **set\_scroll()** (pm\_overlay.rs):
   * Removed clamping to `max_scroll` in setter
   * Raw scroll value now preserved across mode switches
   * Clamping still happens during render (read-only)

2. **render\_list()** (pm\_overlay.rs):
   * Removed `overlay.scroll.set(scroll as u16)` write-back
   * Clamping only affects local `scroll` variable for rendering
   * User's scroll value remains untouched

3. **Tests added** (3 new tests):
   * `test_list_scroll_preserved_after_detail_close` - Verifies scroll preservation
   * `test_list_selection_and_scroll_both_preserved` - Verifies both selection and scroll
   * `test_multiple_detail_open_close_preserves_position` - Verifies across multiple cycles

### Root Cause

**Before**:

```rust
pub(super) fn set_scroll(&self, val: u16) {
    self.scroll.set(val.min(self.max_scroll.get())); // Clamped!
}
```

**Issue**: If `max_scroll` was 0 (not yet calculated or in different mode), scroll gets clamped to 0.

**After**:

```rust
pub(super) fn set_scroll(&self, val: u16) {
    self.scroll.set(val); // Raw value preserved
}
```

**Fix**: Store raw value; clamping only during render for display.

## Behavior

* **Enter detail**: List selection and scroll position saved
* **Scroll in detail**: Detail scroll independent of list scroll
* **Return to list** (Esc): List position exactly as it was
* **Multiple cycles**: Position preserved across multiple detail open/close

## Constraints Met

* ✅ No protocol/CLI/RPC/service changes
* ✅ Read-only / no mutations
* ✅ Only touched pm\_overlay.rs and this brief (2 files, pm\_handlers.rs not needed)
* ✅ LOC delta: \~70 lines (within budget of <= 120)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
cd codex-rs && cargo test -p codex-tui --lib pm_handlers
```

Expected output: All tests pass (41 + 2 = 43 total)

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (41/41)
* [x] `cargo test -p codex-tui --lib pm_handlers` passes (2/2)
* [x] Enter detail from any row, scroll in detail, press Esc → list returns to same row
* [x] List scroll offset preserved (no jump to top)
* [x] Existing detail/list key behaviors unchanged
