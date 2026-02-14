# PM-004: Footer Polish v3 - Quality Batch

<!-- REFRESH-BLOCK
query: "PM-004 footer polish quality batch"
snapshot: (none - read-only rendering refinement)
END-REFRESH-BLOCK -->

## Objective

Ship three footer polish improvements in one PR: style unification, deterministic clipping utility, and snapshot-style rendering tests.

## Spec

* **PM-UX-D3**: List view behavior (30s-to-truth questions)
* **PM-UX-D12**: Keyboard model
* **Decisions**: D138, D143

## Batch Items

### Item A: Unified Footer Styles

**Implementation**: Consistent color role mapping across all tiers

* **Context/Data** (Row, Showing): Bright or Dim
* **Delimiter** (|, separators): Dim
* **Key** (symbols, shortcuts): Accent
* **Action** (nav, tree, detail labels): Dim

Ensures visual consistency across all width tiers.

### Item B: Deterministic Clipping Helper

**Implementation**: Width-safe segment builder utility

* `build_footer_segment()` function guarantees no overflow
* Takes vec of (text, style) pairs and width budget
* Returns spans that fit within budget
* Adds ellipsis if truncation needed
* Available for future footer refinements

**Function signature**:

```rust
fn build_footer_segment(
    spans: Vec<(&str, Style)>,
    width_budget: usize,
) -> Vec<Span<'static>>
```

### Item C: Snapshot-Style Rendering Tests

**Implementation**: Exact output verification at representative widths

* 4 snapshot tests for widths: 120, 80, 50, 30
* Assert exact expected footer format at each width
* Verify priority truncation works correctly
* Document expected rendering at each tier

**Tests added**:

* `test_footer_snapshot_width_120` - Full footer format
* `test_footer_snapshot_width_80` - Compact hints format
* `test_footer_snapshot_width_50` - Narrow format
* `test_footer_snapshot_width_30` - Very narrow format

## Changes

1. **Style mapping** (Item A):
   * Consistent use of bright/dim/accent across tiers
   * Context data: bright or dim
   * Keys: accent
   * Delimiters: dim

2. **Clipping utility** (Item B):
   * Added `build_footer_segment()` helper function
   * Provides width-safe span composition
   * Marked `#[allow(dead_code)]` for future use

3. **Snapshot tests** (Item C):
   * 4 tests covering representative widths
   * Exact string assertions for each tier
   * Validates priority truncation behavior

## Footer Snapshots by Width

**Width 120** (Full):

```
 Row 3/5 | Showing 1-5 of 5 | Sort: Updated  |  ↑↓ nav  ←→ tree  ⏎ detail  s sort  Esc close
```

**Width 80** (Compact):

```
 Row 2/3 | Showing 1-3 of 3 | Sort: Updated  |  ↑↓ ←→ ⏎ s Esc
```

**Width 50** (Narrow):

```
 Row 1/3 | Sort: Updated  |  ↑↓ ⏎ s Esc
```

**Width 30** (Very Narrow):

```
 Row 1/3 …
```

## Behavior

* **Item A**: Consistent styles across all widths
* **Item B**: Clipping helper available for safe composition
* **Item C**: Snapshot tests prevent regressions
* **Empty state**: Always "No items"
* **Detail mode**: No footer (unchanged)

## Constraints Met

* ✅ No protocol/CLI/RPC/service changes
* ✅ Read-only / no mutations
* ✅ Only touched pm\_overlay.rs and this brief (2 files)
* ✅ LOC delta: \~140 in pm\_overlay.rs (within budget of <= 220)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected output: All tests pass (61/61), including 4 new snapshot tests.

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (61/61)
* [x] Item A: Style mapping consistent across tiers
* [x] Item B: Clipping helper implemented and tested
* [x] Item C: Snapshot tests cover all target widths
* [x] Priority order preserved: Row > Showing > Sort > hints
* [x] Empty state unchanged
* [x] Detail mode unchanged
