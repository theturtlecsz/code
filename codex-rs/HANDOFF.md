# HANDOFF.md — Session Continuation

**Created:** 2026-01-11
**Last Session:** 2026-01-13 (SPEC-KIT-971 Cross-Process Locking Complete)
**Next Session:** Implement CLI Commands (checkpoints, stats, doctor)

---

## Continuation Prompt (Next Session)

```markdown
ROLE
You are an implementor working in the Codex-RS / Spec-Kit repo.

NON-NEGOTIABLES (read first)
1) SPEC.md is the primary source of truth.
2) Doc precedence order is mandatory:
   SPEC.md → docs/PROGRAM_2026Q1_ACTIVE.md → docs/DECISION_REGISTER.md
3) Invariants you MUST NOT violate:
   - Stage0 core has no Memvid dependency (adapter boundary enforced)
   - Logical mv2:// URIs are immutable; physical IDs are never treated as stable keys
   - LocalMemoryClient trait is the interface; MemvidMemoryAdapter is the implementation
   - Single-writer capsule model: cross-process lock + in-process writer queue
   - Hybrid = lex + vec (required, not optional)
   - Merge modes are `curated` or `full` only (never squash/ff/rebase)
   - Lock file path: <capsule_path>.lock (e.g., workspace.mv2.lock)

===================================================================
CURRENT STATE — Session completed 2026-01-13
===================================================================

COMPLETED THIS SESSION:
1. ✅ SPEC-KIT-971-A5 Pipeline Integration
   - 3 acceptance tests passing
   - UnifiedMemoryClient enum dispatch pattern
   - Backend routing in run_speckit_auto_pipeline()
   - 607 tests passing

2. ✅ SPEC-KIT-971 Cross-Process Single-Writer Lock
   - lock.rs module with LockMetadata, CapsuleLock, LockError
   - Atomic lock file creation (O_CREAT|O_EXCL) + fs2 advisory lock
   - Stale lock detection (process existence check on same host)
   - CapsuleOpenOptions for write_lock + context
   - open_read_only() for non-locking access
   - Doctor shows detailed lock info + recovery steps
   - 8 new tests, all passing

===================================================================
TASK FOR NEXT SESSION: CLI Commands
===================================================================

Implement `speckit capsule` CLI subcommands:

### 1. `speckit capsule checkpoints` (Priority 1)
```bash
# JSON-first output (user preference)
speckit capsule checkpoints                    # JSON output
speckit capsule checkpoints --format table     # Human-readable table
speckit capsule checkpoints --branch run/abc   # Filter by branch
speckit capsule checkpoints --label v1.0       # Find by label
```

Implementation location: `tui/src/chatwidget/spec_kit/commands/capsule.rs`

Output JSON schema:
```json
{
  "checkpoints": [
    {
      "id": "SPEC-971_plan_20260113120000",
      "label": "stage:plan",
      "stage": "plan",
      "spec_id": "SPEC-971",
      "run_id": "run-abc",
      "commit_hash": "abc123",
      "timestamp": "2026-01-13T12:00:00Z",
      "is_manual": false
    }
  ],
  "total": 5,
  "branch": "main"
}
```

### 2. `speckit capsule stats` (Priority 2)
```bash
speckit capsule stats                          # JSON output
speckit capsule stats --format table           # Human-readable
```

Uses existing `CapsuleHandle::stats()` method.

### 3. `speckit capsule doctor` (Priority 3)
```bash
speckit capsule doctor                         # JSON output
speckit capsule doctor --format table          # Human-readable
```

Uses existing `CapsuleHandle::doctor()` method. Already shows lock details.

### 4. `speckit capsule init` (Priority 4)
```bash
speckit capsule init                           # Create workspace.mv2
speckit capsule init --path custom.mv2         # Custom path
```

===================================================================
WHERE TO IMPLEMENT
===================================================================

1. Create new command module:
   `tui/src/chatwidget/spec_kit/commands/capsule.rs`

2. Register in command_registry.rs:
   - CapsuleCheckpointsCommand
   - CapsuleStatsCommand
   - CapsuleDoctorCommand (may already exist)
   - CapsuleInitCommand

3. Wire up in chatwidget/spec_kit/mod.rs

===================================================================
EXISTING CODE TO USE
===================================================================

The capsule functionality is already implemented:
```rust
// tui/src/memvid_adapter/capsule.rs
impl CapsuleHandle {
    pub fn list_checkpoints(&self) -> Vec<CheckpointMetadata>
    pub fn list_checkpoints_filtered(&self, branch: Option<&BranchId>) -> Vec<CheckpointMetadata>
    pub fn get_checkpoint_by_label(&self, label: &str) -> Option<CheckpointMetadata>
    pub fn stats(&self) -> CapsuleStats
    pub fn doctor(path: &Path) -> Vec<DiagnosticResult>
}

// Lock info available via:
pub use lock::{LockMetadata, is_locked, lock_path_for};
```

===================================================================
TEST COMMANDS
===================================================================

```bash
# Build
~/code/build-fast.sh

# Run specific tests
cargo test -p codex-tui --lib -- memvid_adapter::tests
cargo test -p codex-tui --lib -- capsule

# Full suite
cargo test -p codex-tui --lib

# Verify CLI works (after implementation)
./target/debug/code-tui --help
```

===================================================================
AFTER CLI: NEXT PRIORITY
===================================================================

SPEC-KIT-977 PolicySnapshot Integration
- PolicySnapshot struct exists in stage0/src/policy.rs
- Need to integrate into capsule events
- Tag all events with policy_id
- Phase 4→5 gate requirement

===================================================================
OUTPUT EXPECTATION
===================================================================

- Implement CLI commands with JSON-first output
- Add --format table option for human-readable output
- Register commands in command_registry.rs
- Add tests for CLI output parsing
- Commit with spec ID and decision IDs
- Update progress tracker
```

---

## Progress Tracker

### Completed This Session (2026-01-13)

| Task | Status | Commits | Tests |
|------|--------|---------|-------|
| 971-A5 Pipeline Integration | ✅ | 5d00c1f2b | 3 acceptance tests |
| 971 Cross-Process Lock | ✅ | 04f2807cc | 8 lock tests |

### Completed Specs

| Spec | Status | Commits | Key Deliverables |
|------|--------|---------|------------------|
| SPEC-KIT-971 (core) | ✅ | 41c640977+ | Capsule foundation, crash recovery |
| SPEC-KIT-971 (A5) | ✅ | 5d00c1f2b | Pipeline backend routing |
| SPEC-KIT-971 (lock) | ✅ | 04f2807cc | Cross-process single-writer lock |
| SPEC-KIT-972 | ✅ | 01a263d4a+ | Hybrid retrieval, eval harness |

### In Progress

| Spec | Status | Next Step |
|------|--------|-----------|
| SPEC-KIT-971 (CLI) | 🔄 10% | Implement checkpoints/stats/doctor CLI |
| SPEC-KIT-977 | 🔄 40% | Integrate PolicySnapshot into capsule events |
| SPEC-KIT-978 | 🔄 0% | Create ReflexBackend trait |

### Blocked / Waiting

| Spec | Blocker | Unblocks |
|------|---------|----------|
| SPEC-KIT-973 | Needs 971 CLI | Time-Travel UI |
| SPEC-KIT-975 (full) | Needs 977 | 976 Logic Mesh |

### Phase Gates

| Phase | Gate | Status |
|-------|------|--------|
| 1→2 | 971 URI contract + checkpoint tests | ✅ Passed |
| 2→3 | 972 eval harness + 975 event schema v1 | ✅ Passed |
| 3→4 | 972 parity gates + export verification | ✅ Passed |
| 4→5 | 977 PolicySnapshot + 978 reflex stack | ⏳ Pending |

---

## Architecture Notes

### Cross-Process Lock Flow (IMPLEMENTED)

```
CapsuleHandle::open(config)
    │
    ├── If write_lock=true (default):
    │   ├── Create <capsule_path>.lock atomically (O_CREAT|O_EXCL)
    │   ├── Write LockMetadata JSON (pid, host, user, started_at, context)
    │   ├── Acquire fs2 advisory lock
    │   └── Store CapsuleLock in handle
    │
    ├── If lock exists:
    │   ├── Read LockMetadata from JSON
    │   ├── Check if stale (process not running on same host)
    │   │   ├── Stale → Clean up and retry
    │   │   └── Active → Return CapsuleError::LockedByWriter(metadata)
    │
    └── On Drop:
        └── CapsuleLock::drop() releases lock + removes file
```

### LockMetadata Schema

```json
{
  "pid": 12345,
  "host": "hostname",
  "user": "username",
  "started_at": "2026-01-13T01:00:00Z",
  "spec_id": "SPEC-KIT-971",
  "run_id": "run-abc",
  "branch": "main",
  "schema_version": 1
}
```

### Adapter Boundary (enforced)

```
Stage0 Core (no Memvid dependency)
    │
    └── LocalMemoryClient trait
            │
            ▼
    UnifiedMemoryClient (enum dispatch)
            │
            ├── Memvid(MemvidMemoryAdapter)
            │       ├── CapsuleHandle
            │       └── CapsuleLock (cross-process)
            │
            └── LocalMemory(LocalMemoryCliAdapter)
                    └── `lm` CLI commands
```

---

## Files Changed This Session (2026-01-13)

| File | Change |
|------|--------|
| tui/src/memvid_adapter/lock.rs | NEW: LockMetadata, CapsuleLock, lock_path_for |
| tui/src/memvid_adapter/capsule.rs | CapsuleOpenOptions, open_with_options, LockedByWriter |
| tui/src/memvid_adapter/mod.rs | Export lock types |
| tui/src/memvid_adapter/tests.rs | 8 new cross-process lock tests |
| tui/Cargo.toml | Added hostname, whoami dependencies |
| tui/src/chatwidget/spec_kit/stage0_integration.rs | Backend routing, 3 acceptance tests |
| tui/src/chatwidget/spec_kit/command_registry.rs | Fixed command count 42→43 |

---

## Test Summary

| Package | Tests | Status |
|---------|-------|--------|
| codex-tui (all) | 607 | ✅ Passing |
| memvid_adapter | 27 | ✅ Passing |
| lock module | 4 | ✅ Passing |
| 971-A5 acceptance | 3 | ✅ Passing |
| cross-process lock | 8 | ✅ Passing |

---

## Quick Reference

### Build & Test
```bash
~/code/build-fast.sh              # Fast build
cargo test -p codex-tui --lib     # Full TUI tests (607)
cargo test -p codex-tui --lib -- memvid_adapter::tests  # Memvid tests (27)
cargo test -p codex-tui --lib -- lock  # Lock tests (4)
cargo test -p codex-stage0 --lib  # Stage0 tests
```

### Key Paths
```
codex-rs/tui/src/memvid_adapter/lock.rs       # Cross-process lock (NEW)
codex-rs/tui/src/memvid_adapter/capsule.rs    # CapsuleHandle + diagnostics
codex-rs/tui/src/memvid_adapter/adapter.rs    # UnifiedMemoryClient
codex-rs/stage0/src/policy.rs                 # PolicySnapshot
codex-rs/SPEC.md                              # Root docs contract
```

### Commits This Session
```
5d00c1f2b  test(stage0,memvid): SPEC-KIT-971-A5 acceptance tests pass
04f2807cc  feat(memvid): SPEC-KIT-971 cross-process single-writer lock
```

---

*Generated by Claude Code session 2026-01-13*
