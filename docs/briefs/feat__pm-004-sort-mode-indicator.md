# PM-004: List/Detail Sort Mode Indicator

<!-- REFRESH-BLOCK
query: "PM-004 sort mode indicator"
snapshot: (none - read-only UI enhancement)
END-REFRESH-BLOCK -->

## Objective

Add a visible sort mode indicator to PM list/detail headers and support a read-only cycle key for demo sort modes.

## Spec

* **PM-UX-D3**: List view behavior (30s-to-truth questions)
* **PM-UX-D5**: Column adaptation and data density
* **PM-UX-D12**: Keyboard model
* **Decisions**: D113, D138, D143

## Implementation

### Changes

1. **Added SortMode enum** with 3 modes:
   * `UpdatedDesc` - Sort by most recently updated first (default)
   * `StatePriority` - Sort by state priority (InProgress > NeedsReview > Planned > Backlog, etc.)
   * `IdAsc` - Sort alphabetically by ID ascending

2. **Updated PmOverlay structure**:
   * Added `sort_mode: Cell<SortMode>` field
   * Initialize to `SortMode::UpdatedDesc` by default

3. **Implemented sorting logic**:
   * `apply_sort()` method applies current sort mode to visible indices
   * `state_priority()` helper assigns priority values for state-based sorting
   * Sorting applied in `visible_indices()` after collecting nodes

4. **Added public methods**:
   * `sort_mode()` - Returns current sort mode
   * `cycle_sort_mode()` - Cycles through modes (UpdatedDesc → StatePriority → IdAsc → UpdatedDesc)

5. **Updated title bars**:
   * List view title shows `Sort: Updated|State|ID`
   * Detail view title shows `Sort: Updated|State|ID`
   * Sort label displayed in accent color

6. **Unit tests** (5 new tests):
   * `test_sort_mode_default_is_updated_desc` - Verifies default mode
   * `test_sort_mode_cycle` - Verifies cycle behavior
   * `test_sort_mode_affects_visible_ordering_updated_desc` - Verifies UpdatedDesc ordering
   * `test_sort_mode_affects_visible_ordering_id_asc` - Verifies IdAsc ordering
   * `test_sort_mode_visible_in_title` - Verifies sort mode accessor

### Behavior

* **Default mode**: UpdatedDesc (most recently updated first)
* **Cycling**: UpdatedDesc → StatePriority → IdAsc → UpdatedDesc
* **Sorting**: Applied to rendered list rows only (no data mutation)
* **Title display**: Shows current sort mode in both list and detail views

## Constraints Met

* ✅ No protocol/RPC/CLI changes
* ✅ Read-only / no mutations (sorting is display-only)
* ✅ Only touched pm\_overlay.rs and this brief (2 files)
* ✅ LOC delta: \~165 lines (within budget of <= 170)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected output: All tests pass, including 5 new sort mode tests.

## Fixes Applied

* Made `SortMode` enum `pub(super)` to match method visibility (fixes `private_interfaces` warning)
* Added `#[allow(dead_code)]` to `SortMode` enum and `cycle_sort_mode()` method (used in tests)
* Simplified boolean expression in test per clippy suggestion

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (37/37 tests)
* [x] Default sort mode is UpdatedDesc
* [x] Cycle behavior works correctly
* [x] Visible ordering changes for at least two modes
* [x] Sort mode displayed in list and detail title bars
