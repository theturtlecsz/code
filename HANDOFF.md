# HANDOFF: SPECKIT-TASK-0001 Area-Scoped Feature IDs

**Generated:** 2026-02-03
**Audience:** Next session
**Scope:** `codex-rs` / Area-scoped feature IDs + docs layout

***

## TL;DR (Current State)

**Implementation: \~95% Complete**

* All code changes done
* Compilation passes (with expected deprecation warnings)
* Tests need to be run and verified

**Build Status:** `cargo check -p codex-tui` PASSES

***

## Resume Prompt

Copy this to start the next session:

```
Continue SPECKIT-TASK-0001: Area-scoped feature IDs

## Current State
Implementation is ~95% complete. All code changes done, compilation passes.

## Immediate Next Steps
1. Run the test suite:
   cd codex-rs
   cargo test -p codex-tui spec_id_generator
   cargo test -p codex-tui spec_id_generator_integration

2. Fix any test failures (if needed)

3. Final verification:
   cargo fmt --all -- --check
   cargo clippy -p codex-tui -- -D warnings

4. If all passes, git diff --stat and summarize for commit

## Key Implementation Summary
- /speckit.new <AREA> <description> [--deep] syntax
- Default areas: CORE, CLI, TUI, STAGE0, SPECKIT
- ID format: AREA-FEAT-#### (4-digit zero-padded)
- tasks/ subdirectory created in each feature directory
- Legacy SPEC-* and MAINT-* formats still accepted by guardrails
- Deprecated generate_next_spec_id() for backward compat (warnings expected)

## Stashed Work
Previous PLATFORM-TASK-0001 stash can be dropped - this supersedes it:
git stash drop  # if stash contains "PLATFORM-TASK-0001"

## Reference Files
- Plan: ~/.claude/plans/buzzing-crunching-acorn.md
- Handoff: /home/thetu/code/HANDOFF.md
```

***

## What Was Implemented

### 1. spec\_id\_generator.rs (Complete)

New functions:

* `validate_area(area)` - Validates `^[A-Z][A-Z0-9]*$` format
* `get_available_areas(cwd)` - Returns DEFAULT\_AREAS + discovered from docs/
* `generate_next_feature_id(cwd, area)` - Generates `AREA-FEAT-####`
* `generate_feature_directory_name(cwd, area, desc)` - Full dir name
* `DEFAULT_AREAS` constant: `["CORE", "CLI", "TUI", "STAGE0", "SPECKIT"]`

Legacy functions marked `#[deprecated]`:

* `generate_next_spec_id(cwd)` - Still works for backward compat
* `generate_spec_directory_name(cwd, desc)` - Still works

### 2. Command Parsing (special.rs)

* `/speckit.new <AREA> <description> [--deep]` syntax
* Missing AREA → error with available areas list
* Invalid AREA → format validation error
* `parse_new_spec_args()` function extracts area/description/deep

### 3. Modal/Event Pipeline

| File                     | Change                                     |
| ------------------------ | ------------------------------------------ |
| `spec_intake_modal.rs`   | Added `area: Option<String>` field         |
| `app_event.rs`           | Added `area` to `SpecIntakeSubmitted`      |
| `chatwidget/mod.rs`      | Updated `show_spec_intake_modal` signature |
| `bottom_pane/mod.rs`     | Updated `show_spec_intake_modal` signature |
| `spec_intake_handler.rs` | Threads area to generator                  |
| `app.rs`                 | Updated event handler                      |

### 4. Additional Features

| File                        | Change                                   |
| --------------------------- | ---------------------------------------- |
| `intake_core.rs`            | Creates `tasks/` subdirectory            |
| `native_guardrail.rs`       | Accepts both legacy and `AREA-FEAT-####` |
| `project_intake_handler.rs` | Defaults to "CORE" area                  |

### 5. Tests

* Unit tests in `spec_id_generator.rs` - All new functions tested
* Integration tests rewritten in `spec_id_generator_integration.rs`

***

## Files Modified

```
codex-rs/tui/src/chatwidget/spec_kit/spec_id_generator.rs
codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs
codex-rs/tui/src/bottom_pane/spec_intake_modal.rs
codex-rs/tui/src/app_event.rs
codex-rs/tui/src/chatwidget/mod.rs
codex-rs/tui/src/bottom_pane/mod.rs
codex-rs/tui/src/chatwidget/spec_kit/spec_intake_handler.rs
codex-rs/tui/src/chatwidget/spec_kit/intake_core.rs
codex-rs/tui/src/chatwidget/spec_kit/native_guardrail.rs
codex-rs/tui/src/chatwidget/spec_kit/project_intake_handler.rs
codex-rs/tui/src/app.rs
codex-rs/tui/tests/spec_id_generator_integration.rs
```

***

## Acceptance Criteria Status

* [x] `/speckit.new` without AREA → error with available areas list
* [x] `/speckit.new CORE "Add feature"` → creates `docs/CORE-FEAT-0001-add-feature/`
* [x] Feature directory contains `tasks/` subdirectory
* [x] Legacy `SPEC-KIT-*` directories pass guardrail validation
* [x] New `AREA-FEAT-####` directories pass guardrail validation
* [ ] All tests pass (needs verification)

***

## Stashed Work

Previous work (PLATFORM-TASK-0001) is stashed and can be dropped:

```bash
git stash list  # Shows "PLATFORM-TASK-0001: SPECKIT-FEAT-#### migration (uncommitted)"
git stash drop  # Safe to drop - SPECKIT-TASK-0001 supersedes it
```

***

## Remaining Verification Commands

```bash
cd codex-rs
cargo test -p codex-tui spec_id_generator
cargo test -p codex-tui spec_id_generator_integration
cargo fmt --all -- --check
cargo clippy -p codex-tui -- -D warnings
```

***

*Generated by Claude Code session 2026-02-03*

***

***

# Previous Handoffs (Archive)

## MAINT-930 Post-Fix Clippy Cleanup

**Generated:** 2026-02-01
**Status:** COMPLETE

... (previous content truncated for brevity)
