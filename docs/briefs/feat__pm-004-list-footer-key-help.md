# PM-004: List Footer Key Help Hints

<!-- REFRESH-BLOCK
query: "PM-004 list footer key help"
snapshot: (none - read-only UI enhancement)
END-REFRESH-BLOCK -->

## Objective

Add compact key-help hints to the list footer (↑↓, ←→, Enter, s, Esc) while preserving current row/window/sort display.

## Spec

* **PM-UX-D12**: Keyboard model
* **Decisions**: D138, D143

## Implementation

### Changes

1. **render\_list\_footer()** (pm\_overlay.rs):
   * Added compact key hints to footer
   * Format: `Row X/Y | Showing A-B of Y | Sort: Mode | ↑↓ nav  ←→ tree  ⏎ detail  s sort  Esc close`
   * Uses Unicode arrow symbols for compactness
   * Accent color for keys, dim for action labels

2. **Tests added** (2 new tests):
   * `test_list_footer_shows_key_hints` - Verifies all 5 key hints present
   * `test_list_footer_preserves_context_with_hints` - Verifies row/window/sort preserved with hints

### Footer Format

**Full footer**:

```
 Row 3/15 | Showing 1-10 of 15 | Sort: Updated  |  ↑↓ nav  ←→ tree  ⏎ detail  s sort  Esc close
```

**Components**:

| Section      | Example              | Color      | Purpose             |
| ------------ | -------------------- | ---------- | ------------------- |
| Row position | `Row 3/15`           | Bright     | Current row / total |
| Window range | `Showing 1-10 of 15` | Dim        | Visible window      |
| Sort mode    | `Sort: Updated`      | Accent     | Current sort        |
| Key hints    | `↑↓ nav  ←→ tree...` | Accent+Dim | Quick reference     |

**Key symbols used**:

* ↑↓ (`\u{2191}\u{2193}`) - Navigate up/down
* ←→ (`\u{2190}\u{2192}`) - Expand/collapse tree
* ⏎ (`\u{23ce}`) - Open detail view
* `s` - Cycle sort mode
* `Esc` - Close overlay

## Behavior

* **List mode**: Footer shows context + hints
* **Detail mode**: No footer (unchanged)
* **Empty**: Shows "No items" only (no hints)
* **All contexts preserved**: Row, window, sort all remain visible

## Constraints Met

* ✅ No protocol/CLI/RPC/service changes
* ✅ Read-only / no mutations
* ✅ Only touched pm\_overlay.rs and this brief (2 files)
* ✅ LOC delta: \~60 lines (within budget of <= 110)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected output: All tests pass (50/50), including key hint tests.

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (50/50)
* [x] Footer includes compact key hints
* [x] Existing footer context (Row, Showing, Sort) remains
* [x] Detail mode unchanged
* [x] Empty state still shows "No items"
