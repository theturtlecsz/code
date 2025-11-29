# Next Session: P3 Upstream Sync + Footer Completion

**Date**: 2025-11-29 (created)
**Primary Focus**: Complete P3 Medium-Priority Items + SYNC-009 Footer
**Estimated Effort**: 8-12h across sessions
**Mode**: IMPLEMENTATION (direct execution, minimal prose)

---

## Session Priorities (User-Selected)

### Priority 1: Complete SYNC-009 Footer (2-3h)
Finish the partial footer work - add context percentage display and shortcut overlay.

### Priority 2: P3 Medium Items (All 4 selected)

| Order | SYNC | Task | Est. | Upstream Source |
|-------|------|------|------|-----------------|
| 1 | SYNC-010 | Auto Drive Patterns | 2-4h | codex-rs/core/ |
| 2 | SYNC-013 | Shell MCP Server | 2-3h | mcp-server/ |
| 3 | SYNC-016 | Device Code Auth | 2-3h | login/ |
| 4 | SYNC-017 | Review/Merge Workflows | 3-4h | tui/commands/ |

---

## Detailed Task Specifications

### SYNC-009 Footer Completion

**Already Done**: `tui/src/key_hint.rs` (170 LOC, 6 tests)

**Remaining Work**:
1. Create `tui/src/bottom_pane/footer.rs` from upstream `~/old/code/codex-rs/tui/src/bottom_pane/footer.rs`
2. Key features to port:
   - `FooterMode` enum (CtrlCReminder, ShortcutSummary, ShortcutOverlay, EscHint, ContextOnly)
   - `context_window_line()` - "X% context left" display
   - `shortcut_overlay_lines()` - multi-column keyboard shortcut reference
   - Mode toggle functions
3. Integration: Wire into `chat_composer.rs` footer rendering

**Dependencies**: Uses `key_hint.rs` module (already ported)

### SYNC-010: Auto Drive Patterns

**Source**: `~/old/code/codex-rs/core/` - look for retry, recovery, auto-drive patterns

**Scope**:
- Agent retry/recovery logic
- Automatic task continuation patterns
- Error recovery strategies

**Investigation First**: Search upstream for `auto_drive`, `retry`, `recovery` patterns

### SYNC-013: Shell MCP Server

**Source**: `~/old/code/codex-rs/mcp-server/`

**Scope**:
- Shell operation tools for MCP
- Process execution via MCP protocol
- May overlap with existing fork MCP implementation

**Investigation First**: Compare fork's `mcp-server/` with upstream

### SYNC-016: Device Code Auth

**Source**: `~/old/code/codex-rs/login/`

**Scope**:
- Device code flow for headless authentication
- Fallback when browser-based auth unavailable
- OAuth device authorization grant

**Key Files**: Look for `device_code`, `device_flow` in login/

### SYNC-017: Review/Merge Workflows

**Source**: `~/old/code/codex-rs/tui/` - commands or workflows

**Scope**:
- `/review` command for code review workflows
- `/merge` command for merge operations
- PR/MR integration

**Investigation First**: Search for review/merge command implementations

---

## Execution Checklist

### Session Start
```
1. [ ] Load context:
       load ~/.claude/CLEARFRAME.md
       load docs/NEXT-SESSION-P3-SYNC.md

2. [ ] Query local-memory for context:
       Search: "SYNC upstream" "footer" "key_hint"

3. [ ] Verify build and tests:
       cd ~/code/codex-rs && cargo build -p codex-tui
       cargo deny check
       cargo test -p codex-tui key_hint

4. [ ] Create TodoWrite for session tasks
```

### Per-Task Workflow
```
For each SYNC-XXX:
1. [ ] Investigate upstream source (grep, read key files)
2. [ ] Assess fork overlap (may already have equivalent)
3. [ ] Create/modify files
4. [ ] Add to workspace if new crate
5. [ ] Add tests
6. [ ] Run validation: cargo clippy && cargo test -p <crate>
7. [ ] Store milestone in local-memory (importance ≥8)
8. [ ] Commit: feat(sync): <description> (SYNC-XXX)
```

### Session End
```
1. [ ] Run full validation: cargo deny check && cargo clippy --workspace
2. [ ] Update tracking document with completion status
3. [ ] Store session summary in local-memory
4. [ ] Create continuation prompt if work remains
```

---

## Upstream Source Paths

```bash
# SYNC-009 Footer
~/old/code/codex-rs/tui/src/bottom_pane/footer.rs

# SYNC-010 Auto Drive
~/old/code/codex-rs/core/
# Search: grep -r "auto_drive\|retry\|recovery" ~/old/code/codex-rs/core/

# SYNC-013 Shell MCP
~/old/code/codex-rs/mcp-server/

# SYNC-016 Device Auth
~/old/code/codex-rs/login/
# Search: grep -r "device_code\|device_flow" ~/old/code/codex-rs/login/

# SYNC-017 Review/Merge
~/old/code/codex-rs/tui/
# Search: grep -r "review\|merge" ~/old/code/codex-rs/tui/src/*.rs
```

---

## Build Commands

```bash
cd ~/code/codex-rs

# Quick build
cargo build -p codex-tui

# Full validation
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p codex-tui
cargo deny check

# Specific module tests
cargo test -p codex-tui key_hint
cargo test -p codex-tui footer
```

---

## Notes for Claude

1. **Start with SYNC-009 footer** - builds on key_hint.rs, high-visibility UX improvement
2. **Investigate before implementing** - P3 items need upstream analysis first
3. **Check for fork overlap** - fork may already have equivalents
4. **SYNC-013 and SYNC-016 may be scaffolds** - minimal viable implementation OK
5. **SYNC-017 is complex** - /review and /merge involve git integration
6. **Commit incrementally** - one feature per commit
7. **Store discoveries in local-memory** - architecture insights, patterns found
