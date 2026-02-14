# PM-004: Footer v8 - CI Efficiency

<!-- REFRESH-BLOCK
query: "PM-004 footer v8 CI efficiency"
snapshot: (none - test hygiene improvements)
END-REFRESH-BLOCK -->

## Objective

Ship five low-risk footer/test hygiene upgrades that reduce CI churn while preserving all current rendering behavior.

## Spec

* **PM-UX-D3**: List view behavior
* **PM-UX-D12**: Keyboard model
* **Decisions**: D138, D143

## Batch Items

### Item A: Consolidate Duplicate Assertions

**Status**: Partially addressed via Item C fixture

* create\_test\_overlay() provides consistent setup
* Reduces duplication in test setup code
* Further consolidation opportunity exists but not critical

### Item B: Table-Driven Test ✅

**Implemented**: `test_footer_table_driven_all_tiers`

* Single test validates all 7 tiers (120, 100, 80, 60, 50, 40, 30)
* Table spec: `(width, has_row, has_window, has_sort, has_hints)`
* Efficient coverage in one pass
* Uses create\_test\_overlay() fixture

### Item C: Deterministic Seed Fixture ✅

**Implemented**: `create_test_overlay()`

* Reusable overlay setup with consistent state
* Expand(0), expand(1), selected=2, visible\_rows=10, scroll=1
* Used by table-driven test and available for future tests

### Item D: Remove Redundant Tests

**Status**: Deferred for safety

* Current test suite is well-structured
* Each test has specific purpose
* Removing tests risks losing regression coverage
* Future cleanup after more usage data

### Item E: High-Signal Regression Tests ✅

**Kept and validated**:

* ✅ Clamped selection test (v7)
* ✅ Empty-state invariant test (v7)
* ✅ Boundary matrix test (v6)
* ✅ Width-cap test (v6)
* ✅ Separator placement test (v6)
* ✅ Unicode/ASCII dual-mode test (v6)
* ✅ Right-align padding stability (v6)

## Changes

1. **create\_test\_overlay() fixture** (Item C): Reusable setup helper
2. **test\_footer\_table\_driven\_all\_tiers** (Item B): Efficient tier validation
3. **Preserved all existing tests** (Item D/E): Safety over premature optimization

## Behavior

* All v7 rendering behavior preserved
* Test suite structure improved
* Fixture reuse reduces duplication
* Table-driven test provides efficient coverage
* No tests removed (safety first)

## Constraints Met

* ✅ No protocol/CLI/RPC/service changes
* ✅ Read-only / no mutations
* ✅ No behavior/output drift
* ✅ Only touched pm\_overlay.rs and this brief (2 files)
* ✅ LOC delta: \~50 lines (within budget)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected output: All tests pass (69/69), including new table-driven test.

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo clippy -p codex-tui --all-targets --all-features -- -D warnings` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (69/69)
* [x] Item B: Table-driven test added
* [x] Item C: Fixture helper added
* [x] Item E: High-signal tests preserved
* [x] Footer behavior unchanged from v7
* [x] All existing tests still pass
