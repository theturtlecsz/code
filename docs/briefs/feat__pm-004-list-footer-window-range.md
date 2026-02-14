# PM-004: List Footer Window Range Display

<!-- REFRESH-BLOCK
query: "PM-004 list footer window range"
snapshot: (none - read-only UI enhancement)
END-REFRESH-BLOCK -->

## Objective

Extend the list footer to show visible window range (Showing A-B of Y) alongside row/sort context.

## Spec

* **PM-UX-D3**: List view behavior (30s-to-truth questions)
* **PM-UX-D12**: Keyboard model
* **Decisions**: D138, D143

## Implementation

### Changes

1. **render\_list\_footer()** (pm\_overlay.rs):
   * Added window range calculation and display
   * Format: `Row X/Y | Showing A-B of Y | Sort: Mode`
   * Window start = scroll + 1 (1-based)
   * Window end = scroll + viewport\_rows (clamped to visible\_count)

2. **Updated tests**:
   * `test_list_footer_shows_position` - Now verifies window range display
   * `test_list_footer_window_range_with_scroll` - NEW: Verifies range with scroll offset
   * `test_list_footer_window_range_all_visible` - NEW: Verifies range when all visible

### Footer Format

* **Normal**: `Row 3/15 | Showing 1-10 of 15 | Sort: Updated`
* **Scrolled**: `Row 8/42 | Showing 6-15 of 42 | Sort: State`
* **Empty**: `No items` (no window/sort info)

### Window Range Calculation

```rust
let scroll = overlay.scroll() as usize;
let viewport_rows = overlay.visible_rows() as usize;
let window_start = (scroll + 1).min(visible_count); // 1-based
let window_end = (scroll + viewport_rows).min(visible_count);
```

## Behavior

* **List mode**: Footer shows position + window + sort
* **Detail mode**: No footer (unchanged)
* **Scroll changes**: Window range updates
* **Selection changes**: Current row updates, window unchanged
* **Sort cycles**: Sort mode updates, position/window unchanged
* **Tree changes**: All values recalculated
* **Empty overlay**: Shows "No items" only

## Constraints Met

* ✅ No protocol/CLI/RPC/service changes
* ✅ Read-only / no mutations
* ✅ Only touched pm\_overlay.rs and this brief (2 files)
* ✅ LOC delta: \~50 lines (within budget of <= 110)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected output: All tests pass (48/48), including window range tests.

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (48/48)
* [x] Footer includes window range (Showing A-B of Y)
* [x] Existing footer info remains (Row X/Y | Sort: ...)
* [x] Empty state still shows "No items"
* [x] Detail mode unchanged (no footer)
