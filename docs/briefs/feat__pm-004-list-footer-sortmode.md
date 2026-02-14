# PM-004: List Footer Sort Mode Display

<!-- REFRESH-BLOCK
query: "PM-004 list footer sort mode"
snapshot: (none - read-only UI enhancement)
END-REFRESH-BLOCK -->

## Objective

Extend the list footer to include current sort mode so navigation context and sort context are visible in one place.

## Spec

* **PM-UX-D3**: List view behavior (30s-to-truth questions)
* **PM-UX-D12**: Keyboard model
* **Decisions**: D138, D143

## Implementation

### Changes

1. **render\_list\_footer()** (pm\_overlay.rs):
   * Extended to show both row position AND sort mode
   * Format: `Row X/Y | Sort: Updated|State|ID`
   * Empty state shows only "No items" (no sort mode)

2. **Display format**:
   * **Normal**: `Row 3/15 | Sort: Updated` (position | sort mode)
   * **Empty**: `No items` (no sort info when empty)

3. **Updated tests**:
   * `test_list_footer_shows_position` - Now verifies sort mode display
   * `test_list_footer_shows_no_items_when_empty` - Verifies empty state has no sort
   * `test_list_footer_updates_with_sort_mode_cycle` - NEW: Verifies footer shows each sort mode

### Footer Components

| Component    | Display              | Color  | Updates When                  |
| ------------ | -------------------- | ------ | ----------------------------- |
| Row position | `Row 3/15`           | Bright | Selection moves, tree changes |
| Separator    | `\|`                 | Dim    | Always                        |
| Sort label   | `Sort: `             | Dim    | Always                        |
| Sort mode    | `Updated\|State\|ID` | Accent | Sort cycles (s key)           |

## Behavior

* **List mode**: Footer shows both position and sort mode
* **Detail mode**: No footer (unchanged)
* **Selection changes**: Position updates, sort mode unchanged
* **Sort cycles**: Sort mode updates, position unchanged
* **Tree expands/collapses**: Position total updates, sort mode unchanged
* **Empty overlay**: Shows "No items" only (no sort info)

## Constraints Met

* ✅ No protocol/CLI/RPC/service changes
* ✅ Read-only / no mutations
* ✅ Only touched pm\_overlay.rs and this brief (2 files)
* ✅ LOC delta: \~25 lines (well within budget of <= 100)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected output: All tests pass (46/46), including updated footer tests.

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (46/46)
* [x] List footer shows both row position and sort mode
* [x] Footer updates when sort mode cycles
* [x] Footer updates when selection/tree changes
* [x] Detail mode unchanged (no footer)
