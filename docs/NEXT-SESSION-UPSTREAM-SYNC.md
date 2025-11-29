# Next Session: Upstream Sync Continuation

**Date**: 2025-11-29 (updated)
**Primary Focus**: Complete P2 UX + Document P3 Backlog
**Estimated Effort**: 12-18h across sessions
**Mode**: IMPLEMENTATION (direct execution, minimal prose)

---

## Completed SYNC Tasks

| SYNC | Task | Status | Session | Notes |
|------|------|--------|---------|-------|
| SYNC-001 | Dangerous Command Detection | ✅ Done | 2025-11-29 | ~100 LOC, 9 tests, tree-sitter-bash |
| SYNC-002 | Process Hardening Crate | ✅ Done | 2025-11-28 | 246 LOC, Linux/macOS/FreeBSD |
| SYNC-003 | Cargo Deny Configuration | ✅ Done | 2025-11-28 | 288 LOC deny.toml |
| SYNC-004 | Async Utils Crate | ✅ Done | 2025-11-29 | ~90 LOC, OrCancelExt trait |
| SYNC-005 | Keyring Store (scaffold) | ✅ Done | 2025-11-29 | ~220 LOC, Linux/macOS only |
| SYNC-007 | API Error Bridge | ⏭️ N/A | 2025-11-29 | Fork already has equivalent |

**Total Completed**: 5 SYNC tasks, ~950 LOC added

---

## Next Session: P2 UX Items

### Priority Order

| Order | SYNC | Task | Est. | Decision |
|-------|------|------|------|----------|
| 1 | SYNC-006 | Feedback Crate | 1-2h | **Ring buffer only** (no Sentry) |
| 2 | SYNC-015 | Character Encoding | 2-3h | UTF-8 detection utilities |
| 3 | SYNC-008 | ASCII Animation | 2-4h | TUI widget integration |
| 4 | SYNC-009 | Footer Improvements | 4-8h | Complex - 11K LOC bottom_pane |

### SYNC-006: Feedback Crate (Ring Buffer Only)

**Source**: `~/old/code/codex-rs/feedback/src/lib.rs` (299 LOC)

**Scope**: Ring buffer logging infrastructure WITHOUT Sentry integration

**Files to Create**:
```
codex-rs/feedback/
├── Cargo.toml
└── src/
    └── lib.rs
```

**Integration**:
- Add to workspace Cargo.toml
- Wire into TUI for log capture (optional)

### SYNC-015: Character Encoding Utilities

**Source**: Check `~/old/code/codex-rs/` for encoding-related modules

**Scope**: UTF-8 detection and validation utilities

### SYNC-008: ASCII Animation Module

**Source**: `~/old/code/codex-rs/tui/src/ascii_animation.rs` (111 LOC)

**Scope**: Loading/spinner animations for TUI

**Integration Point**: `codex-rs/tui/src/`

### SYNC-009: Footer/Bottom Pane Improvements

**Source**: `~/old/code/codex-rs/tui/src/bottom_pane/` (~11K LOC total)

**Scope**: Extract useful patterns - context percentage, mode indicators

**Warning**: Large module - selective extraction recommended

---

## P3 Backlog (Document Only)

| SYNC | Task | Priority | Upstream Source | Notes |
|------|------|----------|-----------------|-------|
| SYNC-010 | Auto Drive Patterns | Medium | codex-rs/core/ | Agent retry/recovery patterns |
| SYNC-011 | OpenTelemetry | Low | codex-rs/telemetry/ | Large scope, defer |
| SYNC-012 | TypeScript SDK | Low | protocol-ts/ | VS Code integration |
| SYNC-013 | Shell MCP Server | Medium | mcp-server/ | MCP protocol expansion |
| SYNC-014 | Prompt Management | Low | tui/prompts/ | UI feature |
| SYNC-016 | Device Code Auth | Medium | login/ | Headless auth fallback |
| SYNC-017 | Review/Merge Workflows | Medium | tui/commands/ | /review, /merge commands |
| SYNC-018 | Branch-Aware Resume | Low | core/session/ | Session filtering by branch |

---

## Execution Checklist

### Session Start
```
1. [ ] Load context: load ~/.claude/CLEARFRAME.md and load docs/NEXT-SESSION-UPSTREAM-SYNC.md
2. [ ] Query local-memory:
       ~/.claude/hooks/lm-search.sh "SYNC upstream milestone" 5
3. [ ] Verify build: cd ~/code/codex-rs && cargo build -p codex-tui
4. [ ] Verify deny: cargo deny check
```

### Per-SYNC Workflow
```
For each SYNC-XXX:
1. [ ] Check upstream source exists
2. [ ] Create crate/module structure
3. [ ] Add to workspace if new crate
4. [ ] Port implementation (adapt to fork patterns)
5. [ ] Add tests
6. [ ] Run validation: cargo build && cargo clippy && cargo test -p <crate>
7. [ ] Store milestone in local-memory (importance ≥8)
8. [ ] Commit: feat(sync): <description> (SYNC-XXX)
```

### Session End
```
1. [ ] Run full validation: cargo deny check && cargo clippy --workspace
2. [ ] Update this file with completion status
3. [ ] Store session summary in local-memory
4. [ ] Create continuation prompt if work remains
```

---

## Upstream Source Paths

```bash
~/old/code/codex-rs/feedback/                    # SYNC-006
~/old/code/codex-rs/tui/src/ascii_animation.rs   # SYNC-008
~/old/code/codex-rs/tui/src/bottom_pane/         # SYNC-009
# SYNC-015: Search for encoding utilities
```

---

## Build Commands

```bash
cd ~/code/codex-rs

# Full build
cargo build --workspace

# Single crate
cargo build -p codex-<name>

# Clippy
cargo clippy --workspace --all-targets -- -D warnings

# Tests
cargo test -p codex-core
cargo test -p codex-<new-crate>

# Deny checks
cargo deny check
```

---

## Rejected Items (Reference)

| Item | Reason | Alternative |
|------|--------|-------------|
| codex-api crate | Fork has SPEC-952 CLI routing | Keep api_clients/ |
| compact_remote | Fork has compact.rs | Review for bug fixes only |
| app-server crates | Conflicts with DirectProcessExecutor | Keep fork's execution model |
| Windows support | User decision | Linux/macOS only |

---

## Notes for Claude

1. **IMPLEMENTATION mode** - Direct execution, minimal prose
2. **Ring buffer only for SYNC-006** - No Sentry integration
3. **SYNC-009 is large** - Extract selectively, don't port entire 11K LOC
4. **Commit incrementally** - One SYNC per commit
5. **Store milestones** - Local-memory for each completed SYNC
6. **Track with TodoWrite** - Update todo list as work progresses
7. **Linux/macOS only** - Skip Windows-specific code
