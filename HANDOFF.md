# P121 Session Handoff
Date: 2025-12-16
Scope: ChatWidget refactor (MAINT-11) - Post agents terminal extraction

---

## 1. Executive Summary

Planner is a Rust workspace building the `code` binary with TUI focused on Spec-Kit workflows.

**P120 Completed**:
- Extracted agents terminal overlay from mod.rs into `agents_terminal.rs` (759 LOC)
- 3 types extracted: `AgentTerminalEntry`, `AgentsTerminalState`, `AgentsTerminalFocus`
- 8 functions extracted including render_agents_terminal_overlay (~425 LOC)
- mod.rs: 19,792 → 19,073 LOC (-719 LOC)
- Cumulative MAINT-11: -4,340 LOC (-18.5%)
- All TUI tests pass (541/544, 3 pre-existing spec-kit failures), clippy clean

**Commit**: (to be created)

---

## 2. P121 Tasks (Recommended)

### Task A: MAINT-11 Phase 10 - History Handlers (Primary)
Extract history cell management from `mod.rs` into `history_handlers.rs`.

**Target**: ~600 LOC extraction

### Task B: Investigation for Event Handlers (Secondary)
Analyze event handler code for Phase 11 planning (~1,000 LOC potential).

### Task C: Fix Pre-existing Spec-Kit Test Failures (Optional)
3 environment-related test failures in `consensus::gr001_tests`.

---

## 3. Current State

- `mod.rs`: 19,073 LOC
- **Phase 9 (P120)**: `agents_terminal.rs` (759 LOC, -719 mod.rs reduction)

### Investigation Steps for History Handlers
```bash
cd codex-rs

# Find history-related functions
grep -n "fn history_\|fn.*history" tui/src/chatwidget/mod.rs | head -30

# Find history cell management
grep -n "history\.push\|history\.replace\|history_cells" tui/src/chatwidget/mod.rs | head -20
```

---

## 4. Architecture Context

### ChatWidget Module Structure (Post-P120)
```
chatwidget/
├── agent_install.rs      (24KB)
├── agent_status.rs       (4KB)  ← P113
├── agent.rs              (4KB)
├── agents_terminal.rs    (31KB) ← P120 (NEW)
├── command_render.rs     (10KB) ← P110
├── diff_handlers.rs      (7KB)
├── exec_tools.rs         (29KB)
├── gh_actions.rs         (10KB)
├── history_render.rs     (5KB)
├── input_helpers.rs      (6KB)  ← P116
├── interrupts.rs         (7KB)
├── layout_scroll.rs      (8KB)
├── limits_handlers.rs    (4KB)
├── limits_overlay.rs     (7KB)
├── perf.rs               (6KB)
├── rate_limit_refresh.rs (4KB)
├── review_handlers.rs    (17KB) ← P118
├── session_handlers.rs   (23KB) ← P119
├── submit_helpers.rs     (11KB) ← P114
└── mod.rs                (781KB) ← 19,073 LOC
```

See full diagram: `docs/architecture/chatwidget-structure.md`

---

## 5. Verification Commands

```bash
# Full test suite
cd codex-rs && cargo test --workspace

# TUI-specific tests
cargo test -p codex-tui

# Clippy (should show 0 warnings)
cargo clippy -p codex-tui -- -D warnings

# Build
~/code/build-fast.sh
```

---

## 6. P120 Extraction Summary

| Component | LOC |
|-----------|-----|
| Types extracted | 3 (AgentTerminalEntry, AgentsTerminalState, AgentsTerminalFocus) |
| Functions extracted | 8 |
| agents_terminal.rs total | 759 |
| mod.rs reduction | -719 |

Extracted functions:
- `update_agents_terminal_state()` - Sync agent info to terminal state
- `enter_agents_terminal_mode()` - Activate agents overlay
- `exit_agents_terminal_mode()` - Deactivate agents overlay
- `toggle_agents_hud()` - Toggle agents overlay on/off (Ctrl+A)
- `record_current_agent_scroll()` - Save scroll position for agent
- `restore_selected_agent_scroll()` - Restore scroll position
- `navigate_agents_terminal_selection()` - Navigate agent list
- `render_agents_terminal_overlay()` - Render split-pane overlay (~425 LOC)

---

## 7. Key File References

| Component | File | Lines |
|-----------|------|-------|
| ChatWidget Monolith | `tui/src/chatwidget/mod.rs` | 19,073 |
| Agents Terminal | `tui/src/chatwidget/agents_terminal.rs` | 759 |
| Session Handlers | `tui/src/chatwidget/session_handlers.rs` | 624 |
| Review Handlers | `tui/src/chatwidget/review_handlers.rs` | ~580 |
| Architecture Diagram | `docs/architecture/chatwidget-structure.md` | - |
| MAINT-11 Plan | `docs/MAINT-11-EXTRACTION-PLAN.md` | - |
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
| P118 | 5584713cb | review_handlers.rs extraction |
| P119 | 7ce0d4111 | session_handlers.rs extraction |
| P120 | — | agents_terminal.rs extraction |
| P121 | — | next: history_handlers.rs |

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
| 7 | P118 | -408 extracted | 20,350 |
| 8 | P119 | -558 extracted | 19,792 |
| 9 | P120 | -719 extracted | 19,073 |

**Cumulative**: 23,413 → 19,073 = **-4,340 LOC** (-18.5%)

---

## 10. Remaining Extraction Candidates

| Target | Est. LOC | Complexity | Session |
|--------|----------|------------|---------|
| History handlers | ~600 | Medium | **P121** |
| Event handlers | ~1,000 | High | P122+ |
| Config handlers | ~400 | Medium | P123+ |

---

## 11. P121 Expected Deliverables

| Category | Deliverable | Status |
|----------|-------------|--------|
| **Extraction** | history_handlers.rs (~600 LOC) | Pending |
| **Extraction** | mod.rs → ~18,500 LOC | Pending |
| **Testing** | All TUI tests pass | Pending |
| **Testing** | Clippy clean | Pending |
| **Docs** | MAINT-11 plan updated | Pending |
| **Docs** | HANDOFF.md for P122 | Pending |

---

## 12. Notes

- Agents terminal extraction was larger than initial ~300 LOC estimate (759 LOC actual) due to the large render function (~425 LOC).
- 3 pre-existing test failures in spec_kit consensus tests (environment variable related, not from extraction).
- The extraction pattern is now well-established: investigate → create module → extract types → extract functions → update imports → verify.

---

_Generated: 2025-12-16 after P120_
