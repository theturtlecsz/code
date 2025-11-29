# Next Session: Upstream Sync Continuation

**Date**: 2025-11-29
**Primary Focus**: Comprehensive Upstream Sync (SYNC-001 through SYNC-018)
**Estimated Effort**: Full day session (8-12h)
**Mode**: IMPLEMENTATION (direct execution, minimal prose)

---

## Session Context

### Previous Session Accomplishments (2025-11-28)
- **SYNC-002 COMPLETE**: Process Hardening Crate
  - 173 LOC + 73 LOC tests (246 total)
  - Platforms: Linux, macOS, FreeBSD/OpenBSD, Windows stub
  - TUI integration: `pre_main_hardening()` in main.rs:17-19
  - 4 unit tests passing
- **SYNC-003 COMPLETE**: Cargo Deny Configuration
  - 288 LOC deny.toml with fork-specific additions
  - 28 workspace crates → Apache-2.0 license
  - All checks passing: advisories, licenses, bans, sources
- **Commits**: 7 commits total (latest: 80e66d4)

### Current State
- **Build**: Passes (`cargo build --workspace`)
- **Tests**: codex-core 31 passing, 12 ignored (documented)
- **Cargo Deny**: All checks passing
- **Completed**: SYNC-002, SYNC-003
- **Remaining**: 15 sync tasks (SYNC-001, SYNC-004 through SYNC-018)

---

## Priority Execution Order

### Phase 1: P0 Security (IMMEDIATE)

| SYNC | Task | Est. | Status | Decision Point |
|------|------|------|--------|----------------|
| **SYNC-001** | Dangerous Command Detection | 2-3h | Backlog | None - straightforward port |

**SYNC-001 Details**:
- Source: `~/old/code/codex-rs/core/src/command_safety/is_dangerous_command.rs`
- Integration: `safety.rs` approval flow
- Tests: Port existing test cases
- No fork conflicts expected

### Phase 2: P1 Core Functionality

| SYNC | Task | Est. | Status | Decision Point |
|------|------|------|--------|----------------|
| **SYNC-004** | Async Utils Crate | 30min | Backlog | None - 90 LOC standalone |
| **SYNC-005** | Keyring Store | 1-8h | Backlog | **Depth**: Crate only (1h) vs full auth integration (4-8h) |
| **SYNC-007** | API Error Bridge | 3-4h | Backlog | Adapt to fork's error types |

**SYNC-005 Decision**: At implementation time, choose:
- [ ] **Scaffold only**: Create crate with platform stubs, defer integration
- [ ] **Full integration**: Wire into login flow, migrate credentials

### Phase 3: P2 User Experience

| SYNC | Task | Est. | Status | Decision Point |
|------|------|------|--------|----------------|
| **SYNC-006** | Feedback Crate | 1-6h | Backlog | **Depth**: Ring buffer only (1h) vs Sentry integration (4-6h) |
| **SYNC-008** | ASCII Animation | 4-6h | Backlog | TUI widget integration |
| **SYNC-009** | Footer Improvements | 4-6h | Backlog | Adapt to bottom_pane_view.rs |
| **SYNC-015** | Character Encoding | 2-3h | Backlog | UTF-8 detection utilities |

**SYNC-006 Decision**: At implementation time, choose:
- [ ] **Ring buffer only**: Logging infrastructure without external services
- [ ] **Full Sentry**: Requires Sentry account setup, env vars

### Phase 4: P3 Extensions (Backlog for Future)

| SYNC | Task | Priority | Notes |
|------|------|----------|-------|
| SYNC-010 | Auto Drive Patterns | Medium | Agent retry/recovery |
| SYNC-011 | OpenTelemetry | Low | Observability (large scope) |
| SYNC-012 | TypeScript SDK | Low | VS Code integration |
| SYNC-013 | Shell MCP Server | Medium | MCP protocol expansion |
| SYNC-014 | Prompt Management | Low | UI feature |
| SYNC-016 | Device Code Auth | Medium | Headless auth fallback |
| SYNC-017 | Review/Merge Workflows | Medium | /review, /merge commands |
| SYNC-018 | Branch-Aware Resume | Low | Session filtering |

---

## Execution Checklist

### Session Start
```
1. [ ] Load context: `load ~/.claude/CLEARFRAME.md`
2. [ ] Query local-memory for SYNC context:
       - mcp__local-memory__search(query="SYNC upstream security", limit=5)
       - mcp__local-memory__search(query="SYNC-002 SYNC-003 milestone", limit=3)
3. [ ] Verify build: `cd ~/code/codex-rs && cargo build -p codex-tui`
4. [ ] Verify deny: `cargo deny check`
```

### Per-SYNC Workflow
```
For each SYNC-XXX:
1. [ ] Check upstream source: `ls ~/old/code/codex-rs/<path>`
2. [ ] Read existing PRD if exists: `docs/SYNC-XXX-*/PRD.md`
3. [ ] Create/scaffold crate or module
4. [ ] Add to workspace if new crate
5. [ ] Port implementation
6. [ ] Add tests
7. [ ] Run validation: `cargo build && cargo clippy && cargo test`
8. [ ] Update PRD status to Done
9. [ ] Update UPSTREAM-ANALYSIS table
10. [ ] Store milestone in local-memory (importance ≥8)
11. [ ] Commit with conventional format
```

### Session End
```
1. [ ] Run full validation: `cargo deny check && cargo clippy --workspace`
2. [ ] Update this file with next session context
3. [ ] Store session summary in local-memory
```

---

## Quick Reference

### Upstream Source Paths
```bash
~/old/code/codex-rs/core/src/command_safety/   # SYNC-001
~/old/code/codex-rs/async-utils/               # SYNC-004
~/old/code/codex-rs/keyring-store/             # SYNC-005
~/old/code/codex-rs/feedback/                  # SYNC-006
~/old/code/codex-rs/core/src/api_error_bridge/ # SYNC-007
~/old/code/codex-rs/tui/src/ascii_animation/   # SYNC-008
~/old/code/codex-rs/tui/src/bottom_pane_view/  # SYNC-009
```

### Build Commands
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
cargo test -p codex-process-hardening

# Deny checks
cargo deny check
```

### Local Memory Commands
```bash
# Search (CLI - fast)
~/.claude/hooks/lm-search.sh "SYNC upstream" 10

# Store (MCP - validation)
mcp__local-memory__store_memory(
  content="...",
  domain="infrastructure",
  tags=["sync:SYNC-XXX", "type:milestone"],
  importance=8
)
```

---

## Rejected Items (Reference Only)

These upstream items conflict with fork architecture:

| Item | Reason | Alternative |
|------|--------|-------------|
| codex-api crate | Fork has SPEC-952 CLI routing | Keep api_clients/ |
| compact_remote | Fork has compact.rs | Review for bug fixes only |
| app-server crates | Conflicts with DirectProcessExecutor | Keep fork's execution model |

---

## Effort Summary

| Phase | Items | Est. Hours | Cumulative |
|-------|-------|------------|------------|
| P0 Security | 1 | 2-3h | 2-3h |
| P1 Core | 3 | 4.5-12.5h | 6.5-15.5h |
| P2 UX | 4 | 11-18h | 17.5-33.5h |
| P3 Extensions | 8 | (backlog) | - |

**Recommended Session Goal**: Complete P0 + P1 (~6-15h depending on integration depth)

---

## Notes for Claude

1. **Use IMPLEMENTATION mode** - Direct execution, code quality over prose
2. **Decision points marked** - Ask user at SYNC-005/006 for integration depth
3. **Commit incrementally** - One SYNC per commit, conventional format
4. **Store milestones** - Local-memory for each completed SYNC (importance ≥8)
5. **Track with TodoWrite** - Update todo list as work progresses
6. **P3 is backlog** - Document but don't implement unless time permits
