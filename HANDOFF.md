# P114 Session Handoff
Date: 2025-12-15
Scope: ChatWidget refactor (MAINT-11 Phase 3) + TUI clippy cleanup

---

## 1. Executive Summary

Planner is a Rust workspace building the `code` binary with TUI focused on Spec-Kit workflows.

**P113 Completed**:
- Extracted `agent_status.rs` (~65 LOC, 3 tests) from ChatWidget
- Fixed 49 stage0 clippy warnings via auto-fix
- Added `test-utils` feature to stage0/Cargo.toml
- Updated MAINT-11 and MAINT-15 in SPEC.md

**Commit**: `09f78f6c9 refactor(tui): extract agent_status module + fix stage0 clippy (MAINT-11 Phase 2)`

---

## 2. P114 Primary Task: MAINT-11 Phase 3 (Submit Helpers)

### Goal
Extract submit helper functions from `mod.rs` to a new `submit_helpers.rs` module.

### Current State
- `mod.rs`: 23,151 LOC (reduced from 23,413)
- **Phase 1 (P110)**: `command_render.rs` (~200 LOC, 8 tests)
- **Phase 2 (P113)**: `agent_status.rs` (~65 LOC, 3 tests)

### Target Functions (lines ~15208-15432)

| Function | Lines | Description |
|----------|-------|-------------|
| `submit_text_message` | 5 | Simple text message wrapper |
| `submit_prompt_with_display` | 94 | Display differs from prompt (slash commands) |
| `submit_prompt_with_ace` | 77 | ACE bullet injection (async) |
| `submit_text_message_with_preface` | 18 | Hidden instruction preface |
| `queue_agent_note` | 6 | Queue note for next submission |

**Total**: ~200 LOC (cohesive API surface)

### Key Dependencies
These functions call `self.submit_user_message(...)` which stays in mod.rs.
Pattern: Create methods that take `&mut ChatWidget` or use a trait.

### Extraction Protocol
1. Read target functions: `mod.rs:15208-15432`
2. Identify imports needed (UserMessage, InputItem, ValidateLifecycle, etc.)
3. Create `submit_helpers.rs` with free functions or helper trait
4. Add module declaration after `agent_status` in mod.rs
5. Add `use self::submit_helpers::*` import
6. Remove original functions from mod.rs
7. Test: `cargo test -p codex-tui`
8. Verify: `cargo clippy -p codex-tui`

### Alternative: Trait-based extraction
If free functions don't work (due to `&mut self` patterns), consider:
```rust
// In submit_helpers.rs
pub(crate) trait SubmitHelpers {
    fn submit_text_message(&mut self, text: String);
    fn submit_prompt_with_display(&mut self, display: String, prompt: String);
    // ...
}

impl SubmitHelpers for ChatWidget { ... }
```

---

## 3. P114 Secondary Task: TUI Clippy Auto-fix

### Issue
12 pre-existing clippy warnings in `codex-tui`:
- `uninlined_format_args`
- `redundant_closure`
- `pass_by_ref_vs_value`

### Fix Strategy
```bash
# Auto-fix where possible
cargo clippy --fix -p codex-tui --allow-dirty --allow-staged

# Manual review for remaining
cargo clippy -p codex-tui 2>&1 | grep "warning:"
```

---

## 4. Extracted Modules (Current State)

```
chatwidget/
├── agent_install.rs      (24KB)
├── agent_status.rs       (5KB)  ← P113 NEW
├── agent.rs              (3KB)
├── command_render.rs     (10KB) ← P110
├── diff_handlers.rs      (6KB)
├── exec_tools.rs         (29KB)
├── gh_actions.rs         (10KB)
├── history_render.rs     (5KB)
├── interrupts.rs         (7KB)
├── layout_scroll.rs      (8KB)
├── limits_handlers.rs    (3KB)
├── limits_overlay.rs     (7KB)
├── perf.rs               (6KB)
├── rate_limit_refresh.rs (4KB)
├── submit_helpers.rs     (TBD)  ← P114 TARGET
└── mod.rs                (949KB) ← 23,151 LOC
```

---

## 5. Verification Commands

```bash
# Full test suite
cd codex-rs && cargo test --workspace

# TUI-specific tests
cargo test -p codex-tui

# New module tests
cargo test -p codex-tui --lib -- submit_helpers

# Clippy
cargo clippy -p codex-tui

# Build
~/code/build-fast.sh
```

---

## 6. Key File References

| Component | File | Lines |
|-----------|------|-------|
| ChatWidget Monolith | `tui/src/chatwidget/mod.rs` | 23,151 |
| Submit Functions | `mod.rs:15208-15432` | ~220 |
| P110 Extraction | `command_render.rs` | ~200 |
| P113 Extraction | `agent_status.rs` | ~120 |
| MAINT-11 Tracker | `SPEC.md:186` | - |

---

## 7. Open Items (Not P114 Scope)

| ID | Title | Status | Notes |
|----|-------|--------|-------|
| MAINT-12 | Stage0 HTTP-only | IN PROGRESS | NotebookLM + local-memory without MCP |
| MAINT-13 | Config inheritance | PENDING | Subdirectory project config |
| SPEC-KIT-900 | E2E validation | IN PROGRESS | Full pipeline test |

---

## 8. P114 Checklist

- [ ] Extract submit helper functions to `submit_helpers.rs`
- [ ] Add tests for extracted functions
- [ ] Fix 12 clippy warnings in `codex-tui`
- [ ] Run `cargo test -p codex-tui` — all pass
- [ ] Run `cargo clippy -p codex-tui` — no warnings
- [ ] Update MAINT-11 in SPEC.md with Phase 3 progress
- [ ] Commit with conventional format

---

## 9. Session Summary

| Session | Commit | Key Deliverable |
|---------|--------|-----------------|
| P110 | - | command_render.rs extraction |
| P111 | 424990cc3 | MCP timeout_sec + per-model providers |
| P112 | c521f9c36 | Regression fix + HANDOFF.md |
| P113 | 09f78f6c9 | agent_status.rs + stage0 clippy |
| P114 | — | submit_helpers.rs + tui clippy |

---

_Generated: 2025-12-15 after commit 09f78f6c9_
