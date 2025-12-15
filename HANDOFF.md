# P115 Session Handoff
Date: 2025-12-15
Scope: ChatWidget refactor (MAINT-11 Phase 4) + dead code cleanup

---

## 1. Executive Summary

Planner is a Rust workspace building the `code` binary with TUI focused on Spec-Kit workflows.

**P114 Completed**:
- Extracted `submit_helpers.rs` (~300 LOC, 4 tests) from ChatWidget
- Fixed 6 manual + auto clippy errors in codex-core (unwrap/expect removal, eprintln replacement)
- mod.rs: 23,151 → 22,911 LOC (~240 LOC extracted)
- Updated MAINT-11 Phase 3 in SPEC.md

**Commit**: `83ae857d1 refactor(tui): extract submit_helpers module + fix core clippy (MAINT-11 Phase 3)`

---

## 2. P115 Primary Task: MAINT-11 Phase 4 (Slash-Command Routing)

### Goal
Extract slash-command routing/dispatch logic from `mod.rs` to a new module.

### Current State
- `mod.rs`: 22,911 LOC
- **Phase 1 (P110)**: `command_render.rs` (~200 LOC, 8 tests)
- **Phase 2 (P113)**: `agent_status.rs` (~65 LOC, 3 tests)
- **Phase 3 (P114)**: `submit_helpers.rs` (~300 LOC, 4 tests)

### Target Functions (to identify)
Search for slash-command routing patterns:
```bash
# Find command dispatch logic
grep -n "handle_slash_command\|dispatch_command\|slash_command" tui/src/chatwidget/mod.rs | head -20

# Find command matching patterns
grep -n "starts_with(\"/\"\|match.*command" tui/src/chatwidget/mod.rs | head -20
```

### Estimated Extraction
- ~400 LOC of command routing logic
- Pattern matching for /speckit.*, /help, /new, etc.
- Command validation and argument parsing

### Extraction Protocol
1. Search mod.rs for slash-command handling patterns
2. Identify cohesive function groups for extraction
3. Create `command_routing.rs` or `slash_commands.rs`
4. Add module declaration in mod.rs
5. Test: `cargo test -p codex-tui`
6. Verify: `cargo clippy -p codex-tui`

---

## 3. P115 Secondary Task: Dead Code Cleanup

### Issue
8 dead_code warnings in `codex-tui`:

| Location | Item | Type |
|----------|------|------|
| `bottom_pane/mod.rs:649` | `show_prd_builder` | method |
| `bottom_pane/prd_builder_modal.rs:69` | `PrdBuilderModal::new` | function |
| `bottom_pane/vision_builder_modal.rs:25` | `allow_multiple` field | field |
| `chatwidget/mod.rs:5654` | `show_prd_builder` | method |
| `chatwidget/spec_kit/code_index.rs:127` | `with_max_snippet_chars` | method |
| `chatwidget/spec_kit/code_index.rs:133` | `with_max_context_lines` | method |
| `chatwidget/spec_kit/stage0_integration.rs:396` | `build_stage0_context_prefix` | function |
| `updates.rs:119` | `UpgradeResolution::Command` | variant |

### Fix Strategy
For each item:
1. Check if it's planned for future use (search for TODO/FIXME comments)
2. If unused and no planned use → remove
3. If planned use → add `#[allow(dead_code)]` with comment explaining future use
4. Verify with `cargo clippy -p codex-tui`

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
├── submit_helpers.rs     (11KB) ← P114 NEW
└── mod.rs                (936KB) ← 22,911 LOC
```

---

## 5. Verification Commands

```bash
# Full test suite
cd codex-rs && cargo test --workspace

# TUI-specific tests
cargo test -p codex-tui

# Clippy (should show only 8 dead_code warnings before cleanup)
cargo clippy -p codex-tui

# Build
~/code/build-fast.sh
```

---

## 6. Unstaged Files (MAINT-12 Related)

These 6 files were modified in prior sessions for MAINT-12 (Stage0 HTTP-only):
- `tui/src/chatwidget/spec_kit/code_index.rs`
- `tui/src/chatwidget/spec_kit/commands/intel.rs`
- `tui/src/chatwidget/spec_kit/commands/special.rs`
- `tui/src/chatwidget/spec_kit/project_native.rs`
- `tui/src/chatwidget/spec_kit/stage0_integration.rs`
- `tui/src/stage0_adapters.rs`

**Not P115 scope** - review and commit separately if MAINT-12 work is complete.

---

## 7. Key File References

| Component | File | Lines |
|-----------|------|-------|
| ChatWidget Monolith | `tui/src/chatwidget/mod.rs` | 22,911 |
| P114 Extraction | `submit_helpers.rs` | ~300 |
| MAINT-11 Tracker | `SPEC.md:186` | - |
| Dead Code Locations | See Section 3 | 8 items |

---

## 8. P115 Checklist

- [ ] Identify slash-command routing functions in mod.rs
- [ ] Extract to `command_routing.rs` or similar
- [ ] Add tests for extracted functions
- [ ] Remove/annotate 8 dead_code items
- [ ] Run `cargo test -p codex-tui` — all pass
- [ ] Run `cargo clippy -p codex-tui` — no warnings
- [ ] Update MAINT-11 in SPEC.md with Phase 4 progress
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
| P115 | — | command routing + dead code cleanup |

---

_Generated: 2025-12-15 after commit 83ae857d1_
