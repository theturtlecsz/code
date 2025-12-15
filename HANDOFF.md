# P113 Session Handoff
Date: 2025-12-15
Scope: ChatWidget refactor (MAINT-11) + Stage0 clippy cleanup

---

## 1. Executive Summary

Planner is a Rust workspace building the `code` binary with TUI focused on Spec-Kit workflows.

**P112 Completed**:
- Regression fix in `mcp_consensus_benchmark.rs` (missing struct fields)
- MAINT-15 entry added to SPEC.md
- PRD/GR-001 messaging conflict resolved
- Full test suite verification: 2230 tests pass

**Commit**: `c521f9c36 fix(test): add missing McpServerConfig fields + align PRD with GR-001`

---

## 2. P113 Primary Task: MAINT-11 ChatWidget Refactor

### Current State
- `mod.rs`: 23,215 LOC (967KB) — the "gravity well"
- **Phase 1 done (P110)**: Extracted `command_render.rs` (~200 LOC, 8 tests)
- **Remaining targets**: Input submission, slash-command routing, agent status helpers

### Existing Extracted Modules
```
chatwidget/
├── agent_install.rs      (24KB)
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
└── mod.rs                (967KB) ← TARGET
```

### Extraction Candidates (P113)

1. **Input Submission** (~500-800 LOC estimated)
   - `handle_input_submission()`
   - `process_user_input()`
   - Related validation and preprocessing

2. **Slash-Command Routing** (~300-500 LOC estimated)
   - `route_slash_command()`
   - Command parsing and dispatch logic
   - Autocomplete handlers

3. **Agent Status Helpers** (~200-400 LOC estimated)
   - Status display formatting
   - Progress indicators
   - Agent lifecycle helpers

### Extraction Protocol
1. Identify function boundaries with `grep -n "pub fn\|fn "`
2. Check dependencies (what it calls, what calls it)
3. Create new module with minimal public API
4. Update `mod.rs` to re-export if needed
5. Run tests: `cargo test -p codex-tui`
6. Verify: `cargo clippy -p codex-tui`

---

## 3. P113 Secondary Task: Stage0 Clippy Cleanup

### Issue
49 pre-existing clippy warnings in `stage0` test modules:
- `uninlined_format_args` (most common)
- `redundant_closure_for_method_calls`
- `field_reassign_with_default`

### Files Affected
```
codex-rs/stage0/src/lib.rs          (test module ~line 686+)
codex-rs/stage0/src/tier2.rs        (test code)
codex-rs/stage0/src/librarian/client.rs (test code)
```

### Fix Strategy
```bash
# Auto-fix where possible
cargo clippy --fix -p codex-stage0 --allow-dirty --allow-staged

# Manual review for remaining
cargo clippy -p codex-stage0 2>&1 | grep "error\|warning"
```

---

## 4. Verification Commands

```bash
# Full test suite
cd codex-rs && cargo test --workspace

# Clippy (should be clean after fixes)
cargo clippy --workspace --all-targets -- -D warnings

# Build
~/code/build-fast.sh

# Specific package tests
cargo test -p codex-tui
cargo test -p codex-stage0
```

---

## 5. Key File References

| Component | File | Lines |
|-----------|------|-------|
| ChatWidget Monolith | `tui/src/chatwidget/mod.rs` | 23,215 |
| P110 Extraction | `tui/src/chatwidget/command_render.rs` | ~200 |
| Stage0 Tests | `stage0/src/lib.rs` | 686+ |
| MAINT-11 Tracker | `SPEC.md` | 186 |
| MAINT-15 Tracker | `SPEC.md` | 190 |

---

## 6. Open Items (Not P113 Scope)

| ID | Title | Status | Notes |
|----|-------|--------|-------|
| MAINT-12 | Stage0 HTTP-only | IN PROGRESS | NotebookLM + local-memory without MCP |
| MAINT-13 | Config inheritance | PENDING | Subdirectory project config |
| SPEC-KIT-900 | E2E validation | IN PROGRESS | Full pipeline test |

---

## 7. P113 Checklist

- [ ] Extract input submission module from `mod.rs`
- [ ] Extract slash-command routing module from `mod.rs`
- [ ] Extract agent status helpers module from `mod.rs`
- [ ] Fix 49 clippy warnings in `stage0` test modules
- [ ] Run `cargo test --workspace` — all pass
- [ ] Run `cargo clippy --workspace` — no warnings
- [ ] Update MAINT-11 in SPEC.md with progress
- [ ] Commit with conventional format

---

_Generated: 2025-12-15 after commit c521f9c36_
