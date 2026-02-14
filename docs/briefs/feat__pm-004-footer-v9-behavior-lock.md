# PM-004: Footer v9 - Behavior Lock Tests

<!-- REFRESH-BLOCK
query: "PM-004 footer v9 behavior locks"
snapshot: (none - test hardening only)
END-REFRESH-BLOCK -->

## Objective

Add five behavior-lock tests to freeze current footer semantics and prevent accidental UX drift in future batches.

## Spec

* **PM-UX-D3**: List view behavior
* **PM-UX-D12**: Keyboard model
* **Decisions**: D138, D143

## Batch Items

### Item A: Golden Matrix Exact Strings ✅

**Test**: `test_footer_golden_matrix_exact_strings`

* Asserts exact expected strings at all 7 widths
* Locks Row format: " Row X/Y "
* Locks Sort presence per tier
* Locks Window text variations (Show vs Showing)

### Item B: Separator Policy Lock ✅

**Test**: `test_footer_separator_policy_lock`

* Wide (>=80): Spaced separator " | "
* Narrow (<80): Compact separator "|"
* Explicit assertions per tier group

### Item C: Show vs Showing Breakpoint ✅

**Test**: `test_footer_show_vs_showing_breakpoint_lock`

* > \=100: "Show" (abbreviated)
* 80-99: "Showing" (full)
* Negative assertions (Show NOT present at 80, Showing NOT at 100+)

### Item D: Right-Alignment Lock ✅

**Test**: `test_footer_right_alignment_lock`

* Width 120: Right-aligned hints (ends with "close")
* Width <120: Linear flow (no multi-space padding)
* Explicit positive and negative assertions

### Item E: Empty State Negative Lock ✅

**Test**: `test_footer_empty_state_negative_lock`

* Tests all 7 widths with empty overlay
* Positive: "No items" present
* Negative: Row/Sort/Show/Hints ALL absent
* Comprehensive invariant enforcement

## Changes

Added 5 behavior-lock tests (+210 lines):

1. test\_footer\_golden\_matrix\_exact\_strings
2. test\_footer\_separator\_policy\_lock
3. test\_footer\_show\_vs\_showing\_breakpoint\_lock
4. test\_footer\_right\_alignment\_lock
5. test\_footer\_empty\_state\_negative\_lock

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected: All tests pass (74/74), including 5 new behavior locks.

## Constraints Met

* ✅ No protocol/CLI/RPC changes
* ✅ No rendering behavior changes
* ✅ Tests/harness only
* ✅ Only touched pm\_overlay.rs and brief
* ✅ LOC: \~210 test lines (within budget)
