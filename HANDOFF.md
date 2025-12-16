# P116 Session Handoff
Date: 2025-12-16
Scope: ChatWidget refactor (MAINT-11 Phase 5) - Input helpers extraction

---

## 1. Executive Summary

Planner is a Rust workspace building the `code` binary with TUI focused on Spec-Kit workflows.

**P115 Completed**:
- Fixed 8 dead_code warnings (removed unused code, annotated planned features)
- Verified `/speckit.new` code path intact (SPEC-KIT-970/971)
- Included 4 clippy auto-fixes from MAINT-12 staging
- mod.rs: 22,911 → 22,906 LOC (5 LOC removed)
- **Key finding**: Slash-command routing already well-modularized in `slash_command.rs` + `app.rs`

**Commit**: `e82064d50 refactor(tui): fix dead_code warnings + clippy cleanup (MAINT-11 Phase 4)`

---

## 2. P116 Primary Task: MAINT-11 Phase 5 (Input Helpers Extraction)

### Goal
Extract input handling logic from `mod.rs` to new module(s).

### Current State
- `mod.rs`: 22,906 LOC
- **Phase 1 (P110)**: `command_render.rs` (~200 LOC, 8 tests)
- **Phase 2 (P113)**: `agent_status.rs` (~65 LOC, 3 tests)
- **Phase 3 (P114)**: `submit_helpers.rs` (~300 LOC, 4 tests)
- **Phase 4 (P115)**: Dead code cleanup (8 warnings → 0)

### Target Functions (to identify)
Search for input handling patterns:
```bash
cd codex-rs

# Find paste/input handling
grep -n "handle_paste\|handle_input\|handle_key" tui/src/chatwidget/mod.rs | head -20

# Find compose field operations
grep -n "compose\|bottom_pane.*input\|insert_str" tui/src/chatwidget/mod.rs | head -20

# Find keyboard event handling
grep -n "KeyCode::\|KeyEvent\|on_key" tui/src/chatwidget/mod.rs | head -20
```

### Extraction Candidates
1. **Paste handling** (`handle_paste` + image detection logic) - ~100 LOC
2. **Keyboard event routing** (key dispatch logic) - ~200 LOC
3. **Compose field helpers** (text manipulation) - ~100 LOC

### Extraction Protocol
1. Search mod.rs for cohesive input handling functions
2. Identify dependencies and shared state
3. Create `input_handlers.rs` or similar
4. Add module declaration in mod.rs
5. Test: `cargo test -p codex-tui`
6. Verify: `cargo clippy -p codex-tui`

---

## 3. Architecture Context

### Slash-Command Flow (Verified P115)
```
User types "/speckit.new <desc>"
    ↓
slash_command.rs:400 → process_slash_command_message()
    ↓
app.rs:1943 → AppEvent::DispatchCommand match arm
    ↓
command_registry.rs → SpecKitNewCommand::execute()
    ↓
special.rs:117 → widget.show_prd_builder_with_context()
```

### /speckit.new Code Path (All Intact)
| Component | Location | Status |
|-----------|----------|--------|
| Command registration | `command_registry.rs:154` | ✅ |
| Command execution | `special.rs:84-122` | ✅ |
| ChatWidget wrapper | `mod.rs:5654-5665` | ✅ |
| BottomPane method | `bottom_pane/mod.rs:648-663` | ✅ |
| Modal constructor | `prd_builder_modal.rs:49-66` | ✅ |

---

## 4. Extracted Modules (Current State)

```
chatwidget/
├── agent_install.rs      (24KB)
├── agent_status.rs       (5KB)  ← P113
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
├── submit_helpers.rs     (11KB) ← P114
└── mod.rs                (935KB) ← 22,906 LOC
```

---

## 5. Verification Commands

```bash
# Full test suite
cd codex-rs && cargo test --workspace

# TUI-specific tests
cargo test -p codex-tui

# Clippy (should show 0 warnings after P115)
cargo clippy -p codex-tui

# Build
~/code/build-fast.sh
```

---

## 6. Dead Code Status (P115 Resolved)

| Item | Resolution |
|------|------------|
| `show_prd_builder` (2 locations) | Removed (dead fallback) |
| `PrdBuilderModal::new` | Removed (~96 LOC) |
| `build_stage0_context_prefix` | Removed (trivial wrapper) |
| `allow_multiple` field | `#[allow(dead_code)]` - planned multi-select |
| `with_max_snippet_chars/lines` | `#[allow(dead_code)]` - API completeness |
| `find_missing_instruction_files` | `#[allow(dead_code)]` - error helper |
| `UpgradeResolution::Command` | `#[allow(dead_code)]` - future automation |

---

## 7. Key File References

| Component | File | Lines |
|-----------|------|-------|
| ChatWidget Monolith | `tui/src/chatwidget/mod.rs` | 22,906 |
| Slash Command Parsing | `tui/src/slash_command.rs` | 786 |
| Command Dispatch | `tui/src/app.rs:1943-2300` | ~350 |
| MAINT-11 Tracker | `SPEC.md:186` | - |

---

## 8. P116 Checklist

- [ ] Search mod.rs for input handling patterns
- [ ] Identify cohesive function groups for extraction
- [ ] Create `input_handlers.rs` or similar module
- [ ] Move handle_paste and related functions
- [ ] Add tests for extracted functions
- [ ] Run `cargo test -p codex-tui` — all pass
- [ ] Run `cargo clippy -p codex-tui` — no warnings
- [ ] Update MAINT-11 in SPEC.md with Phase 5 progress
- [ ] Commit with conventional format

---

## 9. Session Summary

| Session | Commit | Key Deliverable |
|---------|--------|-----------------|
| P110 | - | command_render.rs extraction |
| P111 | 424990cc3 | MCP timeout_sec + per-model providers |
| P112 | c521f9c36 | Regression fix + HANDOFF.md |
| P113 | 09f78f6c9 | agent_status.rs + stage0 clippy |
| P114 | 83ae857d1 | submit_helpers.rs + core clippy |
| P115 | e82064d50 | dead_code cleanup (8→0 warnings) |
| P116 | — | input_handlers extraction |

---

## 10. MAINT-11 Progress Summary

| Phase | Session | LOC Extracted | Total mod.rs |
|-------|---------|---------------|--------------|
| 1 | P110 | ~200 | 23,213 |
| 2 | P113 | ~65 | 23,151 |
| 3 | P114 | ~300 | 22,911 |
| 4 | P115 | ~5 (removed) | 22,906 |
| 5 | P116 | ~400 (target) | ~22,500 |

**Cumulative**: 23,413 → 22,906 = ~507 LOC extracted/removed

---

_Generated: 2025-12-16 after commit e82064d50_
