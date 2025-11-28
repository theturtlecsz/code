# Next Session: Upstream Sync Implementation

**Date**: 2025-11-28
**Primary Focus**: Upstream Sync (SYNC-002 through SYNC-018)
**Estimated Effort**: Extended session (8h+)

---

## Session Context

### Previous Session Accomplishments (2025-11-28)
- **SPEC-940 COMPLETE**: Performance instrumentation framework
  - BenchmarkHarness with statistical analysis (mean, stddev, percentiles)
  - Baseline measurements: spawn 0.92ms, sqlite 0.04ms, config 0.05ms
  - ComparisonReport with Welch's t-test regression detection
  - 23 tests, ~850 LOC total
- **Cleanup**: Removed 77 stale session/prompt files (~1MB)
- **Commits**: 6 commits (bd8dc0d → 2bafa12)

### Current State
- **Build**: Passes (`cargo build --workspace`)
- **Tests**: codex-core 31 passing, 12 ignored (documented blockers)
- **Active SPECs**: None in progress
- **Pending**: 17 upstream sync tasks (SYNC-002 to SYNC-018)

---

## Upstream Sync Priority Matrix

### P0: Security-Critical (Start Here)

| SYNC | Feature | Status | Notes |
|------|---------|--------|-------|
| **SYNC-002** | Process Hardening | **Scaffold Ready** | 170 LOC staged, needs workspace integration + tests |
| SYNC-003 | Cargo Deny | Backlog | Vulnerability scanning config |
| SYNC-005 | Keyring Store | Backlog | Secure credential storage |

### P1: Core Functionality

| SYNC | Feature | Status | Notes |
|------|---------|--------|-------|
| SYNC-004 | Async Utils | Backlog | Cancellation tokens, utilities |
| SYNC-007 | API Error Bridge | Backlog | Rate limit handling |
| SYNC-010 | Auto Drive Patterns | Backlog | Agent retry/recovery |

### P2: User Experience

| SYNC | Feature | Status | Notes |
|------|---------|--------|-------|
| SYNC-006 | Feedback Crate | Backlog | User feedback collection |
| SYNC-008 | ASCII Animation | Backlog | Loading indicators |
| SYNC-009 | Footer Improvements | Backlog | Context visibility |
| SYNC-015 | Character Encoding | Backlog | UTF-8 detection |

### P3: Integrations & Extensions

| SYNC | Feature | Status | Notes |
|------|---------|--------|-------|
| SYNC-011 | OpenTelemetry | Backlog | Observability |
| SYNC-012 | TypeScript SDK | Backlog | VS Code integration |
| SYNC-013 | Shell MCP Server | Backlog | MCP protocol support |
| SYNC-014 | Prompt Management | Backlog | Prompt reuse UI |
| SYNC-016 | Device Code Auth | Backlog | Headless auth fallback |
| SYNC-017 | Review/Merge Workflows | Backlog | /review, /merge commands |
| SYNC-018 | Branch-Aware Resume | Backlog | Session filtering |

---

## Phase 0: Complete SYNC-002 (Process Hardening)

Already have implementation staged. Steps:

### Checklist
- [ ] Add `process-hardening` to workspace `Cargo.toml` members
- [ ] Verify build: `cargo build -p codex-process-hardening`
- [ ] Run clippy: `cargo clippy -p codex-process-hardening`
- [ ] Add unit tests for each platform (Linux, macOS, BSD)
- [ ] Integrate into TUI startup (call `pre_main_hardening()`)
- [ ] Create integration test verifying RLIMIT_CORE = 0
- [ ] Update SYNC-002 status to Done

### Files
```
codex-rs/process-hardening/Cargo.toml (exists)
codex-rs/process-hardening/src/lib.rs (exists, ~170 LOC)
codex-rs/Cargo.toml (add to members)
codex-rs/tui/src/main.rs or lib.rs (add startup call)
```

---

## Phase 1: SYNC-003 Cargo Deny

### Overview
Add cargo-deny configuration for dependency vulnerability scanning.

### Deliverables
- [ ] `.cargo/deny.toml` configuration
- [ ] CI integration (GitHub Actions step)
- [ ] Initial audit run, fix any vulnerabilities
- [ ] Document in CONTRIBUTING.md

---

## Phase 2: SYNC-005 Keyring Store

### Overview
Secure credential storage using system keyring.

### Deliverables
- [ ] `codex-keyring` crate
- [ ] Platform abstraction (Linux: libsecret, macOS: Keychain, Windows: Credential Manager)
- [ ] Migration from plaintext config to keyring
- [ ] Fallback for headless environments

---

## Quick Reference Commands

```bash
# Build check
cd ~/code/codex-rs && cargo build --workspace

# Clippy
cargo clippy --workspace --all-targets -- -D warnings

# Run core tests
cargo test -p codex-core

# Run specific SYNC's tests (once implemented)
cargo test -p codex-process-hardening
```

---

## Local Memory Queries

At session start, retrieve context:
```
mcp__local-memory__search(query="SYNC upstream process-hardening", limit=5)
mcp__local-memory__search(query="SPEC-940 benchmark performance", limit=3)
```

---

## Notes for Claude

1. **Start with SYNC-002** - Already has 170 LOC implementation
2. **Follow PRD specs** - Each SYNC-* has detailed PRD in `docs/SYNC-*/PRD.md`
3. **Commit incrementally** - One SYNC per commit when possible
4. **Update SPEC.md** - Mark completed tasks
5. **Store in local-memory** - Architecture decisions and patterns (importance ≥8)
