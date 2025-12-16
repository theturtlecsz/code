# P118 Session Handoff
Date: 2025-12-16
Scope: ChatWidget refactor (MAINT-11) - Post browser code removal

---

## 1. Executive Summary

Planner is a Rust workspace building the `code` binary with TUI focused on Spec-Kit workflows.

**P117 Completed**:
- Removed browser/chrome integration dead code from upstream
- Deleted `chrome_selection_view.rs` (229 LOC)
- Removed browser handlers, events, fields, screenshot rendering
- Cleaned up unused imports and dependencies
- mod.rs: 22,852 → 20,758 LOC (-2,094 LOC, -9.2%)

**Commit**: `15aa783a7 refactor(tui): remove browser/chrome dead code (MAINT-11 Phase 6)`

---

## 2. P118 Primary Task: MAINT-11 Phase 7 (Continued Extraction)

### Goal
Continue extracting cohesive functionality from `mod.rs` to reduce its 20,758 LOC.

### Current State
- `mod.rs`: 20,758 LOC
- **Phase 1 (P110)**: `command_render.rs` (~200 LOC, 8 tests)
- **Phase 2 (P113)**: `agent_status.rs` (~65 LOC, 3 tests)
- **Phase 3 (P114)**: `submit_helpers.rs` (~300 LOC, 4 tests)
- **Phase 4 (P115)**: Dead code cleanup (8 warnings → 0)
- **Phase 5 (P116)**: `input_helpers.rs` (175 LOC, 5 tests)
- **Phase 6 (P117)**: Browser/chrome dead code removal (-2,094 LOC)

### Extraction Candidates for Next Phase
Search for cohesive function groups:
```bash
cd codex-rs

# Find handler functions that could be grouped
grep -n "pub(crate) fn handle_" tui/src/chatwidget/mod.rs | head -30

# Find free functions at module level
grep -n "^fn " tui/src/chatwidget/mod.rs | head -20

# Find session-related functions
grep -n "session\|Session" tui/src/chatwidget/mod.rs | head -20
```

### Potential Extraction Targets
1. **Session handling** - session-related methods
2. **Agent terminal handling** - agents terminal mode methods
3. **History management** - history cell management
4. **Review/merge handlers** - PR review functionality

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
├── input_helpers.rs      (6KB)  ← P116
├── interrupts.rs         (7KB)
├── layout_scroll.rs      (8KB)
├── limits_handlers.rs    (3KB)
├── limits_overlay.rs     (7KB)
├── perf.rs               (6KB)
├── rate_limit_refresh.rs (4KB)
├── submit_helpers.rs     (11KB) ← P114
└── mod.rs                (845KB) ← 20,758 LOC (P117)
```

---

## 5. Verification Commands

```bash
# Full test suite
cd codex-rs && cargo test --workspace

# TUI-specific tests
cargo test -p codex-tui

# Clippy (should show 0 warnings)
cargo clippy -p codex-tui

# Build
~/code/build-fast.sh
```

---

## 6. P117 Removed Code Summary

| Component | LOC Removed |
|-----------|-------------|
| `chrome_selection_view.rs` | 229 |
| Browser handlers in `mod.rs` | ~1,500 |
| Screenshot rendering methods | ~170 |
| Unused imports/fields | ~195 |
| **Total** | ~2,094 |

Removed items:
- `handle_browser_command()`
- `handle_chrome_command()`
- `show_chrome_options()`
- `handle_chrome_launch_option()`
- `toggle_browser_hud()`
- `render_screenshot_highlevel()`
- `render_screenshot_placeholder()`
- `ChromeLaunchOption`, `ShowChromeOptions` events
- `browser_is_external`, `browser_hud_expanded` fields
- `cached_image_protocol`, `cached_picker` fields
- `BG_SHOT_IN_FLIGHT`, `BG_SHOT_LAST_START_MS` statics
- `codex_browser` dependency

---

## 7. Key File References

| Component | File | Lines |
|-----------|------|-------|
| ChatWidget Monolith | `tui/src/chatwidget/mod.rs` | 20,758 |
| Input Helpers | `tui/src/chatwidget/input_helpers.rs` | 175 |
| Slash Command Parsing | `tui/src/slash_command.rs` | 786 |
| Command Dispatch | `tui/src/app.rs:1943-2300` | ~350 |
| MAINT-11 Tracker | `SPEC.md:186` | - |

---

## 8. Session Summary

| Session | Commit | Key Deliverable |
|---------|--------|-----------------|
| P110 | - | command_render.rs extraction |
| P111 | 424990cc3 | MCP timeout_sec + per-model providers |
| P112 | c521f9c36 | Regression fix + HANDOFF.md |
| P113 | 09f78f6c9 | agent_status.rs + stage0 clippy |
| P114 | 83ae857d1 | submit_helpers.rs + core clippy |
| P115 | e82064d50 | dead_code cleanup (8→0 warnings) |
| P116 | d5c58634c | input_helpers.rs extraction |
| P117 | 15aa783a7 | Browser/chrome dead code removal |
| P118 | — | next extraction phase |

---

## 9. MAINT-11 Progress Summary

| Phase | Session | LOC Change | Total mod.rs |
|-------|---------|------------|--------------|
| 1 | P110 | -200 extracted | 23,213 |
| 2 | P113 | -65 extracted | 23,151 |
| 3 | P114 | -300 extracted | 22,911 |
| 4 | P115 | -5 removed | 22,906 |
| 5 | P116 | -54 extracted | 22,852 |
| 6 | P117 | -2,094 removed | 20,758 |

**Cumulative**: 23,413 → 20,758 = **-2,655 LOC** (-11.3%)

---

_Generated: 2025-12-16 after commit 15aa783a7_
