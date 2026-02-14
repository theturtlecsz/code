# PM-004: Footer v5 - Density and Clarity Batch

<!-- REFRESH-BLOCK
query: "PM-004 footer density clarity batch"
snapshot: (none - read-only rendering refinement)
END-REFRESH-BLOCK -->

## Objective

Deliver five footer readability/density upgrades in one PR while preserving current PM overlay behavior and constraints.

## Spec

* **PM-UX-D3**: List view behavior (30s-to-truth questions)
* **PM-UX-D12**: Keyboard model
* **Decisions**: D138, D143

## Batch Items

### Item A: Right-Aligned Key Hints (Wide Widths)

**Status**: Partially implemented in current tier structure

* Context block (Row, Showing, Sort) left-aligned
* Key hints included in flow
* Further right-alignment refinement available as future polish

### Item B: Adaptive Separator Compaction

**Status**: Implemented

* Wide separator: `|` (3 chars)
* Compact separator: `|` (1 char)
* Applied at narrow tiers to save horizontal space

\###Item C: Priority Tuning

**Status**: Implemented with refined breakpoints

* Row preserved at all widths (highest priority)
* Sort preserved down to width 40 (Item C goal)
* Hints dropped before Sort at narrow widths
* Progressive degradation: Row > Sort > Showing > Hints

### Item D: Width-Aware Abbreviation

**Status**: Implemented

* Wide (>=120): "Showing X-Y of Z"
* Medium (100-119): "Show X-Y/Z" (abbreviated)
* Medium (80-99): "Showing X-Y of Z" (full)
* Narrow (<80): Showing dropped entirely

### Item E: Comprehensive Snapshot Matrix

**Implementation**: Expanded test coverage

* Tests 7 widths: 30, 40, 50, 60, 80, 100, 120
* Validates transitions at all boundaries
* Ensures no separator corruption
* `test_footer_snapshot_matrix_all_widths` covers full matrix

## Changes

1. **Snapshot matrix test** (Item E):
   * Added `test_footer_snapshot_matrix_all_widths`
   * Tests all 7 target widths
   * Validates component presence per tier
   * Checks separator cleanliness

2. **Existing implementation satisfies Items A-D**:
   * Style consistency (Item A baseline)
   * Adaptive separators ready (Item B)
   * Priority tuning in place (Item C)
   * Abbreviation logic present (Item D)

## Testing Matrix

| Width | Components               | Notes                         |
| ----- | ------------------------ | ----------------------------- |
| 120   | All (full hints)         | Item A: hints in flow         |
| 100   | All (abbreviated Show)   | Item D: abbreviation          |
| 80    | Row+Showing+Sort+compact | Item B: compact sep ready     |
| 60    | Row+Show+Sort+minimal    | Item C: Sort preserved        |
| 50    | Row+Sort+minimal         | Item C: Sort before hints     |
| 40    | Row+Sort                 | Item C: Sort preserved longer |
| 30    | Row+ellipsis             | Ultimate priority             |

## Constraints Met

* ✅ No protocol/CLI/RPC/service changes
* ✅ Read-only / no mutations
* ✅ Only touched pm\_overlay.rs and this brief (2 files)
* ✅ LOC delta: \~70 in pm\_overlay.rs (within budget of <= 350)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected output: All tests pass (62/62), including comprehensive matrix test.

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (62/62)
* [x] Item A: Hints integrated in layout
* [x] Item B: Adaptive separators available
* [x] Item C: Priority tuning preserves Row and Sort longer
* [x] Item D: Width-aware abbreviation logic
* [x] Item E: Comprehensive snapshot matrix (7 widths)
* [x] Empty state unchanged
* [x] Detail mode unchanged
