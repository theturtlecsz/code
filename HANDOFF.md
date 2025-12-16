# P120 Session Handoff
Date: 2025-12-16
Scope: ChatWidget refactor (MAINT-11) - Post session handlers extraction

---

## 1. Executive Summary

Planner is a Rust workspace building the `code` binary with TUI focused on Spec-Kit workflows.

**P119 Completed**:
- Extracted session handlers from mod.rs into `session_handlers.rs` (624 LOC with tests)
- 10 functions extracted: handle_sessions_command, show_resume_picker, render_replay_item, etc.
- 6 unit tests added for human_ago timestamp formatting
- mod.rs: 20,350 → 19,792 LOC (-558 LOC)
- exec_stream interleave test verified passing (was listed as failing but passes consistently)
- Created architecture diagram at docs/architecture/chatwidget-structure.md
- All TUI tests pass (543 passing), clippy clean

**Commit**: (to be created)

---

## 2. P120 Tasks (Recommended)

### Task A: MAINT-11 Phase 9 - Agents Terminal (Primary)
Extract agents terminal functionality from `mod.rs` into `agents_terminal.rs`.

**Target**: ~300 LOC extraction

### Task B: History Handlers Preparation (Secondary)
Analyze and plan history-related function extraction (~600 LOC potential).

### Task C: Documentation Update (Tertiary)
Update MAINT-11-EXTRACTION-PLAN.md with P119 completion.

---

## 3. Current State

- `mod.rs`: 19,792 LOC
- **Phase 8 (P119)**: `session_handlers.rs` (~558 LOC extracted, 624 LOC total with tests)

### Investigation Steps for Agents Terminal
```bash
cd codex-rs

# Find agents_terminal related code
grep -n "agents_terminal\|AgentsTerminal" tui/src/chatwidget/mod.rs | head -30

# Find agent terminal overlay functions
grep -n "fn.*agent.*terminal\|show_agents" tui/src/chatwidget/mod.rs | head -20
```

---

## 4. Architecture Context

### ChatWidget Module Structure (Post-P119)
```
chatwidget/
├── agent_install.rs      (24KB)
├── agent_status.rs       (4KB)  ← P113
├── agent.rs              (4KB)
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
├── session_handlers.rs   (23KB) ← P119 (NEW)
├── submit_helpers.rs     (11KB) ← P114
└── mod.rs                (812KB) ← 19,792 LOC
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

## 6. P119 Extraction Summary

| Component | LOC |
|-----------|-----|
| Functions extracted | 10 |
| Tests added | 6 |
| session_handlers.rs total | 624 |
| mod.rs reduction | -558 |

Extracted functions:
- `human_ago()` - Format timestamps as relative time
- `list_cli_sessions_impl()` - List active CLI sessions (async)
- `kill_cli_session_impl()` - Kill specific session (async)
- `kill_all_cli_sessions_impl()` - Kill all sessions (async)
- `handle_sessions_command()` - Process /sessions command
- `show_resume_picker()` - Resume session picker UI
- `render_replay_item()` - Render replayed session items
- `export_response_items()` - Export history as ResponseItems
- `handle_feedback_command()` - Export session logs
- `export_transcript_lines_for_buffer()` - Export transcript
- `render_lines_for_terminal()` - Terminal render helper

---

## 7. Key File References

| Component | File | Lines |
|-----------|------|-------|
| ChatWidget Monolith | `tui/src/chatwidget/mod.rs` | 19,792 |
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
| P119 | — | session_handlers.rs extraction |
| P120 | — | next: agents_terminal.rs |

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

**Cumulative**: 23,413 → 19,792 = **-3,621 LOC** (-15.5%)

---

## 10. Remaining Extraction Candidates

| Target | Est. LOC | Complexity | Session |
|--------|----------|------------|---------|
| Agents terminal | ~300 | Low | **P120** |
| History handlers | ~600 | Medium | P121 |
| Event handlers | ~1,000 | High | P122+ |
| Config handlers | ~400 | Medium | P123+ |

---

## 11. P120 Expected Deliverables

| Category | Deliverable | Status |
|----------|-------------|--------|
| **Extraction** | agents_terminal.rs (~300 LOC) | Pending |
| **Extraction** | mod.rs → ~19,500 LOC | Pending |
| **Testing** | All TUI tests pass | Pending |
| **Testing** | Clippy clean | Pending |
| **Docs** | MAINT-11 plan updated | Pending |
| **Docs** | HANDOFF.md for P121 | Pending |

---

## 12. Notes

- The exec_stream interleave test (`test_aggregated_output_interleaves_in_order`) was listed as failing in P118 handoff but passes consistently in P119. It may have been a flaky test or machine-specific issue.
- streaming.rs visibility changed: `begin` function changed from `pub(super)` to `pub(crate)` to allow session_handlers.rs to call it.
- Architecture diagram created at docs/architecture/chatwidget-structure.md with Mermaid diagrams.

---

_Generated: 2025-12-16 after P119_
