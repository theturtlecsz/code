# PM-004: Fix Tree Sort Hierarchy

<!-- REFRESH-BLOCK
query: "PM-004 tree sort hierarchy"
snapshot: (none - read-only UI fix)
END-REFRESH-BLOCK -->

## Objective

Keep sort modes read-only while preserving tree hierarchy (parents stay with descendants).

## Spec

* **PM-UX-D3**: List view behavior (30s-to-truth questions)
* **PM-UX-D5**: Column adaptation and data density
* **PM-UX-D12**: Keyboard model
* **Decisions**: D137, D138, D143

## Problem

The initial sort mode implementation globally reordered the flattened tree, breaking parent-child relationships. A parent could appear after its children, or siblings could be separated from their parent.

## Solution

Changed sorting from global to hierarchical:

* **Before**: Sort entire visible list (breaks tree structure)
* **After**: Sort siblings within each parent scope (preserves hierarchy)

## Implementation

### Changes

1. **Removed global sort** from `visible_indices()`:
   * Deleted `apply_sort()` call that sorted entire visible list
   * Deleted `apply_sort()` method

2. **Added hierarchical sort** to tree collection:
   * New `sort_siblings()` method applies sort mode to sibling groups
   * Modified `collect_subtree()` to sort children before recursing
   * Modified `collect_visible()` to sort root nodes before traversal

3. **Updated tests** for hierarchy verification:
   * `test_sort_mode_affects_visible_ordering_updated_desc` - Now verifies parent-before-child invariant
   * `test_sort_mode_affects_visible_ordering_id_asc` - Now verifies parent-before-child invariant
   * Both tests verify siblings are correctly sorted within their parent scope

### Sort Algorithm

For each node:

1. Add node to visible list
2. If expanded, collect children indices
3. **Sort children** according to current sort mode
4. Recursively add each sorted child

This ensures:

* Parents always appear before their children
* Siblings are sorted relative to each other
* Tree structure is preserved

## Constraints Met

* ✅ No protocol/CLI/RPC/service changes
* ✅ Read-only / no mutations
* ✅ Only touched pm\_overlay.rs and this brief (2 files)
* ✅ LOC delta: \~60 lines (within budget of <= 140)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected output: All tests pass (37/37), including updated hierarchy tests.

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (37/37 tests)
* [x] Sorting no longer globally reorders the flattened tree
* [x] Parent-child adjacency is preserved in all sort modes
* [x] Existing sort mode label/behavior remains visible and functional
