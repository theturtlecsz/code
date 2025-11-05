# Phase 1 Progress - Dead Code Cleanup

**Started**: 2025-10-28
**Status**: In Progress - Day 1-2

---

## Summary

**Before**: 92 warnings in codex-tui
**After cargo fix**: 86 warnings (-6 fixed automatically)
**Remaining**: 86 warnings to address

---

## Spec-Kit Specific Unused Functions Found

### file_modifier.rs
- `restore_from_backup()` - Line 431 - **UNUSED**

### guardrail.rs
- `read_latest_spec_ops_telemetry()` - Line 339 - **UNUSED**
- `collect_guardrail_outcome()` - Line 405 - **UNUSED**

### quality.rs
- `find_majority_answer()` - Line 89 - **UNUSED**
- `find_dissent()` - Line 95 - **UNUSED**

### quality_gate_handler.rs
- `build_quality_gate_prompt()` - Line 990 - **UNUSED** (private function)

---

## Decision: Mark as #[allow(dead_code)] for Now

These functions appear to be:
1. **Utility functions** that may be used in future features
2. **Public API** functions that external code might call
3. **Test helpers** that aren't currently used

**Recommendation**: Add `#[allow(dead_code)]` annotations rather than delete, since they may be intentional API surface or future use.

**Alternative**: If confirmed truly dead, can delete in Phase 1 Day 3-4 investigation.

---

## Actions Taken

1. ✅ Ran `cargo fix --lib -p codex-tui --allow-dirty`
2. ✅ Reduced warnings from 92 → 86
3. ✅ Identified 6 spec_kit unused functions

---

## Next Steps

1. Investigate if these functions are dead code or future API
2. Either mark with `#[allow(dead_code)]` or remove
3. Move to Phase 1 Day 3-4 (cargo-udeps analysis)
