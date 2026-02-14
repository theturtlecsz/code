# PM-004: Footer Precision v2 - Quality Upgrades

<!-- REFRESH-BLOCK
query: "PM-004 footer precision quality"
snapshot: (none - read-only UI refinement batch)
END-REFRESH-BLOCK -->

## Objective

Ship three footer quality upgrades in one PR: deterministic width-fit, unicode fallback safety, and compact-mode consistency tests.

## Spec

* **PM-UX-D3**: List view behavior (30s-to-truth questions)
* **PM-UX-D12**: Keyboard model
* **Decisions**: D138, D143

## Batch Items

### Item A: Deterministic Width-Fit Logic

**Implementation**: Explicit budgeting before composing spans

* Pre-compute text components (`row_text`, `window_text`, `sort_text`)
* Budget estimates documented in comments for each tier
* Ensures footer content never exceeds width at tested breakpoints

**Width budgets**:

* Full (>=120): \~109 chars (15 row + 2 + 20 window + 2 + 15 sort + 5 + 50 hints)
* Medium (>=80): \~79 chars (15 + 2 + 20 + 2 + 15 + 5 + 20)
* Narrow (>=50): \~52 chars (15 + 2 + 15 + 5 + 15)
* Very narrow (<50): \~16 chars (15 + 1)

### Item B: ASCII Fallback for Unicode Safety

**Implementation**: Const toggle for terminals without unicode support

* `const USE_ASCII_HINTS: bool = false` (default: unicode)
* Unicode mode: ↑↓ (`\u{2191}\u{2193}`), ←→ (`\u{2190}\u{2192}`), ⏎ (`\u{23ce}`)
* ASCII mode: `^v`, `<>`, `Enter`
* Test verifies both modes work correctly

### Item C: Width-Matrix Boundary Tests

**Implementation**: Comprehensive boundary width testing

* Tests all tier boundaries: 49/50, 79/80, 119/120
* Validates stable priority truncation at each tier
* Verifies no malformed separators (`| |` or `||`)
* Documents expected behavior at each transition

## Changes

1. **USE\_ASCII\_HINTS const**: Toggleable unicode/ASCII mode
2. **Pre-computed text variables**: For deterministic length budgeting
3. **Width budget comments**: Document estimated lengths per tier
4. **2 new comprehensive tests**:
   * `test_footer_boundary_width_matrix` - Tests all 6 boundary widths
   * `test_footer_ascii_fallback_mode` - Verifies unicode/ASCII toggle

## Testing Matrix (Item C)

| Width | Expected Components                 | Priority Truncation  |
| ----- | ----------------------------------- | -------------------- |
| 120   | Row + Window + Sort + Full hints    | None (all visible)   |
| 119   | Row + Window + Sort + Full hints    | None                 |
| 80    | Row + Window + Sort + Compact hints | Hint labels dropped  |
| 79    | Row + Window + Sort + Compact hints | Hint labels dropped  |
| 50    | Row + Sort + Minimal hints          | Window dropped       |
| 49    | Row + ellipsis                      | Sort + hints dropped |

## Behavior

* **Width >= 120**: Full footer with all components
* **Width 80-119**: Compact hints (symbols only)
* **Width 50-79**: Minimal hints (critical only)
* **Width < 50**: Row + ellipsis only
* **ASCII mode**: Set `USE_ASCII_HINTS = true` for compatibility
* **Empty state**: Always "No items" (all widths)
* **Detail mode**: No footer (unchanged)

## Constraints Met

* ✅ No protocol/CLI/RPC/service changes
* ✅ Read-only / no mutations
* ✅ Only touched pm\_overlay.rs and this brief (2 files)
* ✅ LOC delta: \~100 in pm\_overlay.rs (within budget of <= 220)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected output: All tests pass (57/57), including boundary matrix tests.

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (57/57)
* [x] Item A: Deterministic width budgeting implemented
* [x] Item B: ASCII fallback mode available
* [x] Item C: Boundary widths tested (49/50/79/80/119/120)
* [x] Priority order: Row > Showing > Sort > hints
* [x] No separator corruption at any width
