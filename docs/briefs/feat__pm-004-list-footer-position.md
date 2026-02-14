# PM-004: List Footer Position Indicator

<!-- REFRESH-BLOCK
query: "PM-004 list footer position"
snapshot: (none - read-only UI enhancement)
END-REFRESH-BLOCK -->

## Objective

Add a compact list footer showing current row position and total visible rows to improve navigation confidence.

## Spec

* **PM-UX-D3**: List view behavior (30s-to-truth questions)
* **PM-UX-D12**: Keyboard model
* **Decisions**: D138, D143

## Implementation

### Changes

1. **List layout** (pm\_overlay.rs):
   * Reserve 1 line at bottom for footer
   * Update list\_area calculation to subtract footer\_height
   * Add footer\_area below list

2. **render\_list\_footer()** function:
   * Display "Row X/Y" where X = selected+1 (1-based), Y = visible\_count
   * Display "No items" when overlay is empty
   * Footer updates dynamically with selection and tree expansion

3. **Display scope**:
   * Renders in list mode only
   * Does NOT render in detail mode (as specified)

4. **Tests added** (4 new tests):
   * `test_list_footer_shows_position` - Verifies basic "Row X/Y" display
   * `test_list_footer_updates_with_selection` - Verifies updates when selection moves
   * `test_list_footer_updates_with_tree_expansion` - Verifies updates when tree expands/collapses
   * `test_list_footer_shows_no_items_when_empty` - Verifies empty state display

### Footer Format

* **Normal**: `Row 3/15` (current row / total visible)
* **Empty**: `No items` (when visible\_count == 0)
* **Display**: Bright text on background, fills remaining width with dim spaces

## Behavior

* **List mode**: Footer always visible at bottom
* **Detail mode**: No footer (detail has its own layout)
* **Selection moves**: Footer updates to show new position
* **Tree expands/collapses**: Footer updates to show new total
* **Empty overlay**: Shows "No items"

## Constraints Met

* ✅ No protocol/CLI/RPC/service changes
* ✅ Read-only / no mutations
* ✅ Only touched pm\_overlay.rs and this brief (2 files)
* ✅ LOC delta: \~110 lines (within budget of <= 120)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected output: All tests pass (45/45), including 4 new footer tests.

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (45/45)
* [x] In list mode, footer shows "Row X/Y"
* [x] Footer updates when selection moves
* [x] Footer updates when tree expands/collapses
* [x] Footer does not render in detail mode
