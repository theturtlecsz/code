# PM-004: Onboarding Command Parity Fix

<!-- REFRESH-BLOCK
query: "PM-004 onboarding command parity"
snapshot: (none - read-only UI fix)
END-REFRESH-BLOCK -->

## Objective

Fix empty-state onboarding copy to reference only real PM commands and add a test guard for command validity.

## Spec

* **PM-UX-D24**: Onboarding wizard flow
* **PM-UX-D13**: Command parity with actual slash-command surface
* **Decisions**: D113, D138, D143

## Problem

The empty-state onboarding panel incorrectly referenced `/pm create feature "User Authentication"`, which is not a valid PM command. This violates PM-UX-D13 command parity requirements.

## Valid PM Commands

From PM-UX-D13 specification:

* **Interactive**: `/pm open`
* **Bot commands**: `/pm bot run --id <ID> --kind <kind>`, `/pm bot status`, `/pm bot runs`, etc.
* **Service commands**: `/pm service doctor`, `/pm service status`

**NO** `/pm create` command exists.

## Changes

### pm\_overlay.rs

1. **Updated onboarding "Next steps" section**:
   * Removed: `/pm create feature "User Authentication"` (invalid)
   * Added: `/pm service doctor` (check service status)
   * Added: `/pm open` (view PM overlay)
   * Added: `/pm bot run --id <ID> --kind research` (run research on work item)

2. **Added test guard** (`test_onboarding_only_references_valid_pm_commands`):
   * Asserts onboarding text does NOT contain `/pm create`
   * Asserts onboarding text contains at least one valid PM command
   * Prevents future regressions

### Brief

* This document

## Testing

All existing tests pass, plus new command validity guard:

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected: 32/32 tests pass (including new `test_onboarding_only_references_valid_pm_commands`)

## Constraints Met

* ✅ No protocol/RPC/CLI changes
* ✅ Read-only / no mutations
* ✅ Only touched pm\_overlay.rs and this brief (2 files)
* ✅ LOC delta: \~20 lines (within budget of <= 60)

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo test -p codex-tui --lib pm_overlay` passes (32/32 tests)
* [x] Onboarding references only valid PM commands
* [x] Test guard prevents `/pm create` regression
