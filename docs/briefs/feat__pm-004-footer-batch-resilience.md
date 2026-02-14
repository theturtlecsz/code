# PM-004: Footer Batch - Resilience and Clarity

<!-- REFRESH-BLOCK
query: "PM-004 footer batch resilience clarity"
snapshot: (none - read-only UI enhancement batch)
END-REFRESH-BLOCK -->

## Objective

Ship three footer UX improvements in one PR to reduce CI cycles while keeping PM overlay read-only and protocol-safe.

## Spec

* **PM-UX-D3**: List view behavior (30s-to-truth questions)
* **PM-UX-D12**: Keyboard model
* **Decisions**: D138, D143

## Batch Items

### Item A: Footer Overflow Handling with Priority Truncation

**Implementation**: Width tiers with priority-based degradation

* **Priority order**: Row > Showing > Sort > Key hints (most to least important)
* **Width >= 120**: Full footer (all components)
* **Width >= 80**: Medium (compact hints, symbols only)
* **Width >= 50**: Narrow (Row + Sort + minimal hints)
* **Width < 50**: Very narrow (Row only + ellipsis)

### Item B: Footer Key-Hint Compact Mode

**Implementation**: Graceful degradation for narrow widths

* **Width >= 120**: Full hints with labels (`↑↓ nav  ←→ tree  ⏎ detail  s sort  Esc close`)
* **Width >= 80**: Compact hints symbols only (`↑↓ ←→ ⏎ s Esc`)
* **Width >= 50**: Minimal hints critical keys only (`↑↓ ⏎ s Esc`)
* **Width < 50**: No hints (ellipsis indicates truncation)

### Item C: Footer Consistency Polish

**Implementation**: Stable separators across all width tiers

* Consistent `| ` separator pattern (space after pipe)
* No doubled separators (`| |` or `||`)
* No trailing separators
* Clean rendering at all tested widths

## Implementation Details

### Width Tier Logic

```rust
if width >= 120 {
    // Full: Row + Window + Sort + Full hints
} else if width >= 80 {
    // Medium: Row + Window + Sort + Compact hints
} else if width >= 50 {
    // Narrow: Row + Sort + Minimal hints
} else {
    // Very narrow: Row + ellipsis
}
```

### Tests Added (5 new tests)

1. `test_footer_full_width_shows_all_components` - Verifies width >= 120
2. `test_footer_medium_width_compact_hints` - Verifies width >= 80
3. `test_footer_narrow_width_priority_truncation` - Verifies width >= 50
4. `test_footer_very_narrow_width_with_ellipsis` - Verifies width < 50
5. `test_footer_separators_clean_at_all_widths` - Verifies Item C across all widths

## Footer Examples by Width

**Full (>= 120)**:

```
 Row 3/15 | Showing 1-10 of 15 | Sort: Updated  |  ↑↓ nav  ←→ tree  ⏎ detail  s sort  Esc close
```

**Medium (>= 80)**:

```
 Row 3/15 | Showing 1-10 of 15 | Sort: Updated  |  ↑↓ ←→ ⏎ s Esc
```

**Narrow (>= 50)**:

```
 Row 3/15 | Sort: Updated  |  ↑↓ ⏎ s Esc
```

**Very Narrow (< 50)**:

```
 Row 3/15 …
```

## Behavior

* **All widths**: Most important info (Row) always visible
* **Progressive enhancement**: More context added as width increases
* **No corruption**: Clean separators at all widths
* **Ellipsis indicator**: Shows truncation at very narrow widths
* **Empty state**: Always just "No items" (all widths)
* **Detail mode**: No footer (unchanged)

## Constraints Met

* ✅ No protocol/CLI/RPC/service changes
* ✅ Read-only / no mutations
* ✅ Only touched pm\_overlay.rs and this brief (2 files)
* ✅ LOC delta: \~90 in pm\_overlay.rs (within budget of <= 220)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected output: All tests pass (55/55), including 5 new width tier tests.

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (55/55)
* [x] Item A: Priority truncation with ellipsis
* [x] Item B: Key hints degrade gracefully
* [x] Item C: Separators clean at all widths
* [x] Empty state still shows "No items"
* [x] Detail mode unchanged
