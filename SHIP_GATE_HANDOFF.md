# HANDOFF: capture=none Private Scratch Mode + Ship Hard-Fail Gating

**PR Title:** `feat: capture=none private scratch mode + ship hard-fail gating`
**Date:** 2026-01-20
**Session Context:** Implementation ~70% complete, test files need updates

## What Was Done

### 1. Core Implementation (COMPLETE)

| File | Change | Status |
|------|--------|--------|
| `codex-rs/tui/src/chatwidget/spec_kit/state.rs` | Added `capture_mode: LLMCaptureMode` field to `SpecAutoState`, updated `new()` and `with_quality_gates()` constructors | Done |
| `codex-rs/tui/src/chatwidget/spec_kit/ship_gate.rs` | NEW FILE - Ship gate validation logic with `ShipGateResult` enum, `validate_ship_gate()`, `has_ace_milestone_frame()` stub, tests | Done |
| `codex-rs/tui/src/chatwidget/spec_kit/command_handlers.rs` | Added `halt_spec_auto_no_resume()` helper for capture=none failures | Done |
| `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | Integrated ship gate before Unlock stage, load capture_mode at pipeline start | Done |
| `codex-rs/tui/src/chatwidget/spec_kit/mod.rs` | Registered `ship_gate` module | Done |
| `codex-rs/tui/src/chatwidget/spec_kit/ace_reflector.rs` | Added `persist_ace_frame()` stub following D131 pattern | Done |
| `codex-rs/tui/src/chatwidget/mod.rs:13819` | Fixed test to pass capture_mode | Done |
| `codex-rs/tui/src/chatwidget/spec_kit/context.rs:327` | Fixed test to pass capture_mode | Done |

### 2. What Remains (TEST FILES NEED CAPTURE_MODE ARGUMENT)

The `SpecAutoState::new()` and `SpecAutoState::with_quality_gates()` constructors now require a `capture_mode: LLMCaptureMode` argument. Test files need updating:

**Files to update with `LLMCaptureMode::PromptsOnly` argument:**

```
tui/tests/state_tests.rs           # ~15 call sites
tui/tests/spec_auto_e2e.rs         # ~25 call sites
tui/tests/common/integration_harness.rs  # 1 call site
```

**Pattern to apply:**
```rust
// Before:
let state = SpecAutoState::new(
    "SPEC-ID".to_string(),
    "goal".to_string(),
    SpecStage::Plan,
    None,
    PipelineConfig::defaults(),
);

// After:
let state = SpecAutoState::new(
    "SPEC-ID".to_string(),
    "goal".to_string(),
    SpecStage::Plan,
    None,
    PipelineConfig::defaults(),
    codex_tui::LLMCaptureMode::PromptsOnly,  // NEW
);
```

**Need to export LLMCaptureMode from lib.rs:**
Add to `codex-rs/tui/src/lib.rs`:
```rust
pub use memvid_adapter::LLMCaptureMode;
```

### 3. Key Design Decisions

- **Ship gate location:** Before Unlock stage (fail-fast, in `advance_spec_auto()`)
- **capture_mode storage:** In SpecAutoState (loaded from GovernancePolicy at pipeline start)
- **ACE frames:** Stub returns true until ACE persistence is implemented
- **No override pathway:** Hard fail is enforced, no bypass

### 4. Files Modified (Full List)

```
codex-rs/tui/src/
├── chatwidget/
│   ├── mod.rs                          # Test fix
│   └── spec_kit/
│       ├── ace_reflector.rs           # persist_ace_frame() stub
│       ├── command_handlers.rs        # halt_spec_auto_no_resume()
│       ├── context.rs                 # Test fix
│       ├── mod.rs                     # Module registration
│       ├── pipeline_coordinator.rs    # Ship gate integration
│       ├── ship_gate.rs               # NEW - validation logic
│       └── state.rs                   # capture_mode field
└── lib.rs                             # (NEEDS LLMCaptureMode export)
```

## Restart Prompt

Copy this prompt to continue:

---

**Continue PR: capture=none private scratch mode + ship hard-fail gating**

Implementation is ~70% complete. Core logic is done, test files need updates.

**Tasks remaining:**
1. Export `LLMCaptureMode` from `codex-rs/tui/src/lib.rs`:
   ```rust
   pub use memvid_adapter::LLMCaptureMode;
   ```

2. Update test files to pass `LLMCaptureMode::PromptsOnly` as last argument:
   - `tui/tests/state_tests.rs` (~15 calls)
   - `tui/tests/spec_auto_e2e.rs` (~25 calls)
   - `tui/tests/common/integration_harness.rs` (1 call)

3. Run validation:
   ```bash
   cargo test -p codex-tui ship_gate
   cargo test -p codex-tui state_tests
   cargo build -p codex-tui
   ```

**Context:**
- See `/home/thetu/code/SHIP_GATE_HANDOFF.md` for full details
- Plan file: `/home/thetu/.claude/plans/curious-jumping-hinton.md`
- Locked contracts: D131 (capture mode persistence), D132 (ship hard-fail gating)

---

## Validation Commands

```bash
# Build
cargo build -p codex-tui

# Run ship_gate tests
cargo test -p codex-tui ship_gate

# Run all relevant tests
cargo test -p codex-tui state_tests
cargo test -p codex-tui spec_auto_e2e

# Clippy (may have pre-existing errors in stage0)
cargo clippy -p codex-tui
```

## Git Status Notes

These files have unstaged changes from this PR:
- `codex-rs/tui/src/chatwidget/spec_kit/*.rs` (multiple)
- `codex-rs/tui/src/chatwidget/mod.rs`
- `codex-rs/tui/src/lib.rs` (will need change)
- `codex-rs/tui/tests/*.rs` (will need changes)
