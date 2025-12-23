# Session Handoff — SYNC-028 + SPEC-KIT-926

**Last updated:** 2025-12-23
**Status:** MAINT-11/13 Complete, Pivoting to SYNC-028 (TUI v2)
**Priority:** SYNC-028 → SPEC-KIT-926

---

## Session Summary (2025-12-23 - Session 3)

### Completed This Session

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| MAINT-11 Phase 10 | ✅ | `407cda75d` | pro_overlay.rs wired, mod.rs 18,040 LOC |
| MAINT-13 Closure | ✅ | `de76ac53f` | 2 inheritance tests added, marked done |
| SPEC-926 Review | ✅ | — | Analyzed against DirectProcessExecutor |
| User Decision | ✅ | — | Prioritize SYNC-028 before SPEC-926 |

### Key Decisions Made

| Question | Decision |
|----------|----------|
| UI approach for SPEC-926 | Prioritize TUI v2 (SYNC-028) first |
| SPEC-926 Phase 5 (Consensus) | Remove from spec |
| Include SPEC-920 in scope | No, focus on 926 after 028 |

---

## Next Session: SYNC-028 (TUI v2 Scaffold)

### Continuation Prompt

```
Continue SYNC-028 (TUI v2 scaffold) **ultrathink**

## Context
- User prioritized TUI v2 scaffold before SPEC-926 progress visibility
- TUI v2 will provide better primitives for status bar and progress display
- SPEC-926 follows as Phase 2 (targeting TUI v2 architecture)

## SYNC-028 Task
PRD: docs/SYNC-028-tui2-scaffold/PRD.md

Goal: Bring in upstream tui2 (viewport-based TUI) as optional frontend behind features.tui2

Key deliverables:
1. Add tui2 crate to workspace (from upstream source)
2. Add features.tui2 feature flag to config_types.rs
3. Add CLI flag --tui2 for opt-in
4. Ensure builds cleanly without breaking TUI1
5. Basic launch/exit test

## Blocking Question
Upstream tui2 source not found in repo. Need to:
1. Locate upstream zip/source for tui2
2. Or scaffold minimal tui2 crate structure

## Dependency Check
SYNC-019 (Feature Registry) - may need to check if required first

## Files to Read First
1. docs/SYNC-028-tui2-scaffold/PRD.md
2. docs/SYNC-019-features-registry/PRD.md
3. codex-rs/core/src/config_types.rs
4. codex-rs/tui/src/cli.rs
```

---

## SPEC-KIT-926 Analysis (For After SYNC-028)

### Tmux References to Remove

The spec references obsolete tmux observability (replaced by DirectProcessExecutor in SPEC-936):

| Line | Content | Action |
|------|---------|--------|
| 65 | `Observable: tmux attach -t agents-gemini` | Remove |
| 167-168 | "Show observable tmux session" | Update to DirectProcessExecutor |
| 272 | `tmux_session: Option<String>` | Remove field |
| 793-794 | Example with tmux attach | Update example |

### DirectProcessExecutor Capabilities

From `codex-rs/core/src/async_agent_executor.rs` (SPEC-936):

```rust
pub struct AgentOutput {
    pub stdout: String,      // Streaming capture
    pub stderr: String,      // Error capture
    pub exit_code: i32,      // Reliable completion
    pub duration: Duration,  // Real-time tracking
    pub timed_out: bool,     // Timeout detection
}
```

**Benefits over tmux:**
- <10ms spawn latency (vs 6500ms tmux)
- Streaming stdout/stderr via tokio
- No temp files (vs 4 files per agent)
- Exit codes for completion (no polling)

### Current TUI Message Pattern

From `quality_gate_handler.rs`:
```rust
widget.history_push(crate::history_cell::PlainHistoryCell::new(
    Role::System,
    format!("message"),
));
```

### Phase Updates for SPEC-926

| Phase | Status | Notes |
|-------|--------|-------|
| Phase 1: Status Bar | Keep | Target TUI v2 primitives |
| Phase 2: Agent Visibility | Keep | Use DirectProcessExecutor streaming |
| Phase 3: Sequential Tracker | Keep | No changes |
| Phase 4: Pipeline Preview | Keep | No changes |
| Phase 5: Consensus Visibility | **REMOVE** | Per user decision |
| Phase 6: Pipeline Progress | Keep | No changes |
| Phase 7: Error Context | Keep | No changes |

---

## MAINT-11 Final Status

| Phase | Module | LOC | Status |
|-------|--------|-----|--------|
| 1 | `command_render.rs` | 303 | ✅ |
| 2 | `agent_status.rs` | 123 | ✅ |
| 3 | `submit_helpers.rs` | 302 | ✅ |
| 4 | Dead code cleanup | -2,094 | ✅ |
| 5 | `input_helpers.rs` | 175 | ✅ |
| 6 | Browser/chrome removal | -2,094 | ✅ |
| 7 | `review_handlers.rs` | 462 | ✅ |
| 8 | `session_handlers.rs` | 619 | ✅ |
| 9 | `undo_snapshots.rs` | 497 | ✅ |
| 10 | `pro_overlay.rs` | 619 | ✅ |
| 11 | `validation_config.rs` | ~200 | Future |
| 12 | `model_presets.rs` | ~120 | Future |

**mod.rs trajectory:** 23,413 → 18,040 LOC (5,373 LOC reduced, 23% decrease)

---

## Open Questions for Next Session

1. **Upstream tui2 source**: Where is the upstream tui2 code?
2. **SYNC-019 dependency**: Is Feature Registry required before SYNC-028?
3. **TUI1 fallback**: Should SPEC-926 have inline-only path while TUI v2 matures?

---

## Commits This Session

```
de76ac53f fix(core): MAINT-13 add config inheritance tests and mark done
407cda75d refactor(tui): extract pro_overlay.rs from ChatWidget (MAINT-11 Phase 10)
```

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `docs/SYNC-028-tui2-scaffold/PRD.md` | TUI v2 requirements |
| `docs/SYNC-019-features-registry/PRD.md` | Feature registry (dependency) |
| `docs/SPEC-KIT-926-tui-progress-visibility/spec.md` | Progress visibility spec |
| `codex-rs/core/src/async_agent_executor.rs` | DirectProcessExecutor (replaces tmux) |
| `codex-rs/tui/src/chatwidget/mod.rs` | Main TUI widget (18,040 LOC) |
