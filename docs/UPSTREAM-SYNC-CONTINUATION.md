# Upstream Sync Implementation - Continuation Prompt

**Session Date**: 2025-11-28
**Status**: PRDs Complete, Ready for Implementation

---

## Quick Context

Upstream sync from 2025-11-27 analysis. 18 SYNC tasks tracked in SPEC.md.

**Completed this session**:
- SYNC-001: Implemented (dangerous command detection)
- All 17 remaining tasks have PRDs created

**Pre-approved decisions**:
- Priority: P0 Security First → P1 Core → P2 Features → P3 QoL
- PRDs: Full PRDs in `docs/SYNC-XXX-<slug>/PRD.md`
- Git: Feature branch per task (`feature/sync-XXX`)
- Scope: All 18 tasks have PRDs

---

## PRD Status Matrix

| SYNC ID | Title | PRD | Status | Priority | Est |
|---------|-------|-----|--------|----------|-----|
| SYNC-001 | Dangerous command detection | N/A (impl done) | **Done** | P0 | - |
| SYNC-002 | Process hardening crate | ✅ Created | Backlog | P0 | 1-2h |
| SYNC-003 | Cargo deny configuration | ✅ Created | Backlog | P0 | 30m |
| SYNC-004 | Async utils crate | ✅ Created | Backlog | P1 | 30m |
| SYNC-005 | Keyring store crate | ✅ Created | Backlog | P1 | 1-5h |
| SYNC-006 | Feedback crate | ✅ Created | Backlog | P1 | 1-6h |
| SYNC-007 | API error bridge | ✅ Created | Backlog | P1 | 3-4h |
| SYNC-008 | ASCII animation | ✅ Created | Backlog | P2 | 4-6h |
| SYNC-009 | Footer improvements | ✅ Created | Backlog | P2 | 4-6h |
| SYNC-010 | Auto Drive patterns | ✅ Created | Backlog | P1 | 8h+20-40h |
| SYNC-011 | OpenTelemetry crate | ✅ Created | Backlog | P1 | 8-12h |
| SYNC-012 | TypeScript SDK | ✅ Created | Backlog | P2 | 4-6h |
| SYNC-013 | Shell MCP server | ✅ Created | Backlog | P2 | 2-3h |
| SYNC-014 | Prompt management UI | ✅ Created | Backlog | P2 | 6-10h |
| SYNC-015 | Character encoding | ✅ Created | Backlog | P3 | 2-3h |
| SYNC-016 | Device code auth | ✅ Created | Backlog | P3 | 3-4h |
| SYNC-017 | /review /merge workflows | ✅ Created | Backlog | P2 | 6-8h |
| SYNC-018 | Branch-aware resume | ✅ Created | Backlog | P3 | 2-3h |

---

## Recommended Next Session Actions

### Option A: Continue P0 Security (Fastest Path to Security)
```
1. Implement SYNC-002 (process-hardening)
2. Implement SYNC-003 (cargo deny)
```
**Time**: ~2h | **Value**: Critical security hardening

### Option B: Quick Wins First (Show Progress)
```
1. Implement SYNC-003 (cargo deny) - 30m
2. Implement SYNC-004 (async-utils) - 30m
3. Implement SYNC-015 (character encoding) - 2-3h
4. Implement SYNC-018 (branch-aware resume) - 2-3h
```
**Time**: ~6h | **Value**: 4 tasks completed

### Option C: Research First (SYNC-010)
```
1. Execute SYNC-010 Phase 1 (research)
2. Create pattern applicability matrix
3. Make go/no-go decision on implementation
```
**Time**: 8h | **Value**: Inform future architecture decisions

---

## Implementation Workflow Per Task

```bash
# 1. Read PRD
cat docs/SYNC-XXX-<slug>/PRD.md

# 2. Create branch
git checkout -b feature/sync-XXX

# 3. Implement per PRD requirements

# 4. Validate
cd codex-rs && cargo fmt && cargo clippy && cargo build

# 5. Test
cargo test -p <relevant-crate>

# 6. Update SPEC.md status

# 7. Commit
git add . && git commit -m "feat(scope): SYNC-XXX description"
```

---

## Source Locations Reference

```bash
# P0 Security
~/old/code/codex-rs/process-hardening/      # SYNC-002
~/old/code/codex-rs/deny.toml               # SYNC-003

# P1 Core
~/old/code/codex-rs/async-utils/            # SYNC-004
~/old/code/codex-rs/keyring-store/          # SYNC-005
~/old/code/codex-rs/feedback/               # SYNC-006
~/old/code/codex-rs/core/src/api_bridge.rs  # SYNC-007
~/old/code/codex-rs/otel/                   # SYNC-011

# P1 Research
~/old/code/code-rs/code-auto-drive-core/    # SYNC-010

# P2 Features
~/old/code/sdk/typescript/                  # SYNC-012
~/old/code/shell-tool-mcp/                  # SYNC-013
~/old/code/codex-rs/tui/src/bottom_pane/    # SYNC-014

# P2 UX
~/old/code/codex-rs/tui/src/ascii_animation.rs  # SYNC-008
~/old/code/codex-rs/tui/src/bottom_pane/footer.rs  # SYNC-009
~/old/code/codex-rs/tui/src/slash_command.rs  # SYNC-017

# P3 QoL
~/old/code/codex-rs/exec/src/bash.rs        # SYNC-015 (chardetng)
~/old/code/codex-rs/login/src/              # SYNC-016
~/old/code/codex-rs/tui/src/                # SYNC-018
```

---

## Protected Areas (DO NOT MODIFY)

- `spec_kit/` - Multi-agent orchestration
- `cli_executor/` - Claude/Gemini CLI routing
- `*_native.rs` - Zero-cost quality commands
- Files with `FORK-SPECIFIC` markers

---

## Files to Reference

- `~/code/SPEC.md` - Task tracking (lines 197-245)
- `~/code/docs/UPSTREAM-ANALYSIS-2025-11-27.md` - Security items detail
- `~/code/docs/UPSTREAM-FEATURE-GAP-ANALYSIS.md` - Feature gaps detail
- `~/code/docs/SYNC-XXX-<slug>/PRD.md` - Individual task PRDs

---

## Session Start Checklist

1. ✅ Load `~/.claude/CLEARFRAME.md`
2. ✅ Read this continuation prompt
3. ✅ Check SPEC.md current status
4. ✅ Select implementation option (A, B, or C)
5. ✅ Use TodoWrite to track progress
