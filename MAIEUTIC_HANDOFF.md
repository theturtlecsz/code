# Session Handoff: Mandatory Maieutic Elicitation Step

**PR**: `feat: enforce mandatory maieutic elicitation step (fast path)`
**Branch**: `docs/lock-arb-d113-d134`
**Date**: 2026-01-20

---

## Restart Prompt

```
Continue implementing mandatory maieutic elicitation step (D130/D131).

Context:
- PR: "feat: enforce mandatory maieutic elicitation step (fast path)"
- Core types module created: codex-rs/tui/src/chatwidget/spec_kit/maieutic.rs (~400 lines)
- Plan file: ~/.claude/plans/cozy-sauteeing-codd.md

Completed:
1. ✅ Created maieutic.rs with MaieuticSpec, DelegationBounds, ElicitationMode, questions, persistence

Remaining tasks (in order):
2. Add maieutic fields to SpecAutoState in state.rs
3. Add write_maieutic_spec to evidence.rs
4. Add maieutic gate to pipeline_coordinator.rs (after line ~148, after constitution gate)
5. Create maieutic_modal.rs for TUI modal (pattern: prd_builder_modal.rs)
6. Add MaieuticSubmitted/Cancelled events to app_event.rs
7. Update mod.rs files (spec_kit/mod.rs, bottom_pane/mod.rs)
8. Wire event handling in chatwidget/mod.rs
9. Write maieutic tests
10. Run validation (cargo test, clippy, fmt)

Key decisions:
- D130: Maieutic step mandatory before automation (fast path allowed)
- D131: Persistence follows capture_mode (none = in-memory only)
- Insert gate AFTER constitution gate, BEFORE Stage0 execution

Read the plan file and maieutic.rs first, then continue from step 2.
```

---

## What Was Accomplished

### 1. Plan Created
- **Plan file**: `~/.claude/plans/cozy-sauteeing-codd.md`
- Comprehensive implementation plan for maieutic gate

### 2. Core Types Module Created
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/maieutic.rs` (NEW, ~400 lines)
- Contains:
  - `MaieuticSpec` struct - Pre-flight interview output with serde
  - `DelegationBounds` struct - What runs automatically
  - `ElicitationMode` enum - Interactive vs PreSupplied
  - `MaieuticQuestion` / `MaieuticOption` types for modal
  - `default_fast_path_questions()` - 5 structured questions
  - `persist_maieutic_spec()` - D131 capture-mode-aware persistence
  - `has_maieutic_completed()` - Check if already done
  - Unit tests

---

## What Remains

### Files to Modify

#### 1. `codex-rs/tui/src/chatwidget/spec_kit/state.rs`
Add to `SpecAutoState`:
```rust
pub maieutic_completed: bool,
pub maieutic_spec: Option<super::maieutic::MaieuticSpec>,
pub maieutic_skip_reason: Option<String>,
```

#### 2. `codex-rs/tui/src/chatwidget/spec_kit/evidence.rs`
Add to `EvidenceRepository` trait:
```rust
fn write_maieutic_spec(&self, spec_id: &str, maieutic: &MaieuticSpec) -> Result<PathBuf>;
```

#### 3. `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`
Insert after line ~148 (after constitution gate, before Stage0):
```rust
// P93/D130: Mandatory Maieutic Elicitation Gate
if !run_maieutic_gate(widget, &spec_id, capture_mode) {
    return;  // Modal opened, pipeline paused
}
```

Add functions:
- `run_maieutic_gate()`
- `resume_pipeline_after_maieutic()`

#### 4. `codex-rs/tui/src/app_event.rs`
Add events around line ~637:
```rust
/// Maieutic elicitation completed (D130)
MaieuticSubmitted { spec_id: String, maieutic_spec: MaieuticSpec },

/// Maieutic elicitation cancelled (D130)
MaieuticCancelled { spec_id: String },
```

### Files to Create

#### 5. `codex-rs/tui/src/bottom_pane/maieutic_modal.rs` (NEW)
TUI modal following `prd_builder_modal.rs` pattern:
- 5 questions with option-based answers (A-D)
- Keyboard handling (A-D select, Esc cancel, Enter submit custom)
- Emits MaieuticSubmitted/Cancelled events
- ~300 lines expected

### Files to Update

#### 6. Module Exports
- `codex-rs/tui/src/chatwidget/spec_kit/mod.rs` - Add `pub mod maieutic;`
- `codex-rs/tui/src/bottom_pane/mod.rs` - Add `pub(crate) mod maieutic_modal;`

#### 7. Event Handling
- `codex-rs/tui/src/chatwidget/mod.rs` - Handle MaieuticSubmitted/Cancelled

#### 8. Bottom Pane Integration
- `codex-rs/tui/src/bottom_pane/mod.rs` - Add `show_maieutic_modal()` method

### Tests to Write
- `test_maieutic_required_before_execute`
- `test_capture_none_does_not_persist_maieutic`
- `test_capture_prompts_persists_maieutic`
- `test_maieutic_completion_resumes_pipeline`
- `test_maieutic_cancellation_aborts_pipeline`

---

## Key Architecture Points

### Pipeline Flow
```
Re-entry guard → Config validation → Evidence check → Constitution gate → MAIEUTIC GATE → Stage0 → Stages...
```

### Insertion Point
`pipeline_coordinator.rs:handle_spec_auto()` line ~148-149, after:
```rust
if !run_constitution_readiness_gate(widget) {
    return;
}
// INSERT MAIEUTIC GATE HERE
```

### Capture Mode Logic (D131)
```rust
match capture_mode {
    LLMCaptureMode::None => Ok(None),  // In-memory only
    LLMCaptureMode::PromptsOnly | LLMCaptureMode::FullIo => {
        // Write to docs/{spec_id}/evidence/maieutic_spec_{timestamp}.json
    }
}
```

### Fast-Path Questions (5 questions, 30-90 seconds)
1. **Goal** - Primary objective
2. **Constraints** - Non-negotiables (multi-select)
3. **Acceptance** - How to verify success
4. **Risks** - Concerns
5. **Delegation** - What runs automatically

---

## Reference Files

| File | Purpose |
|------|---------|
| `codex-rs/tui/src/chatwidget/spec_kit/maieutic.rs` | Core types (CREATED) |
| `codex-rs/tui/src/bottom_pane/prd_builder_modal.rs` | Modal pattern to follow |
| `codex-rs/tui/src/chatwidget/spec_kit/prd_builder_handler.rs` | Handler pattern |
| `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs:145-148` | Gate insertion point |
| `codex-rs/tui/src/app_event.rs:609-637` | Event definition pattern |
| `~/.claude/plans/cozy-sauteeing-codd.md` | Full implementation plan |

---

## Validation Commands
```bash
cd codex-rs
cargo test -p codex-tui -- maieutic
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

---

## Out of Scope (Later PR - D133)
- `--maieutic <path>` CLI flag for headless
- `--maieutic-answers <json>` inline answers
- Ship milestone gating (D132)
