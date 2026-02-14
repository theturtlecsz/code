# PM-004: Sort Cycle Keybinding

<!-- REFRESH-BLOCK
query: "PM-004 sort cycle keybinding"
snapshot: (none - read-only UI enhancement)
END-REFRESH-BLOCK -->

## Objective

Add a list-view keybinding to cycle PM sort mode so the existing sort feature is user-reachable in TUI.

## Spec

* **PM-UX-D3**: List view behavior (30s-to-truth questions)
* **PM-UX-D12**: Keyboard model
* **Decisions**: D113, D138, D143

## Implementation

### Changes

1. **pm\_handlers.rs**:
   * Added `s`/`S` key handler in `handle_list_key()` to cycle sort mode
   * Detail mode ignores `s` key (returns true but no action)
   * Added 2 unit tests:
     * `test_sort_cycle_method_available` - Verifies cycle\_sort\_mode() works
     * `test_s_key_handler_exists_in_list_mode` - Verifies 's' key is handled

2. **pm\_overlay.rs**:
   * Updated list view title to show `s sort` hint
   * Sort label continues to display current mode (Updated|State|ID)

### Behavior

* **List mode**: Press `s` → cycles UpdatedDesc → StatePriority → IdAsc → UpdatedDesc
* **Detail mode**: `s` key ignored (returns true but no sort cycle)
* **Title bar**: Shows `s sort` hint and current `Sort: <mode>` indicator

### Keybinding Summary

| Key          | List Mode       | Detail Mode  |
| ------------ | --------------- | ------------ |
| `s` / `S`    | Cycle sort mode | Ignored      |
| `Up/Dn`      | Navigate        | Scroll       |
| `Left/Right` | Collapse/Expand | Ignored      |
| `Enter`      | Open detail     | -            |
| `Esc`        | Close overlay   | Back to list |

## Constraints Met

* ✅ No protocol/RPC/CLI changes
* ✅ Read-only / no mutations
* ✅ Only touched pm\_handlers.rs, pm\_overlay.rs, and this brief (3 files)
* ✅ LOC delta: \~40 lines (within budget of <= 120)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
cd codex-rs && cargo test -p codex-tui --lib pm_handlers
```

Expected output: All tests pass (37 + 2 = 39 total)

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (37/37)
* [x] `cargo test -p codex-tui --lib pm_handlers` passes (2/2)
* [x] In list mode, pressing 's' cycles sort mode
* [x] In detail mode, 's' does not alter sort mode
* [x] Title/help hint reflects 's sort' action
