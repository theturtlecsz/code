# HANDOFF.md — Session Continuation

**Created:** 2026-01-11
**Last Session:** 2026-01-13 (SPEC-KIT-971 Checkpoint Integration + SPEC-KIT-977 Wiring Complete)
**Next Session:** CLI Commands + Policy Event Binding

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

1. ✅ SPEC-KIT-971 Checkpoint Integration with Pipeline Stage Commits
   - StageCommitResult struct returns commit hash from auto_commit
   - get_head_commit_hash() function for commit retrieval
   - create_capsule_checkpoint() function wired after git auto-commit
   - 5 git_integration tests passing
   - Checkpoints record spec_id, run_id, stage, commit_hash

2. ✅ SPEC-KIT-977 PolicySnapshot Wiring
   - Deterministic hash (excludes policy_id, created_at, hash)
   - content_matches() and content_changed() helpers
   - put_policy() for global URI: mv2://<workspace>/policy/<policy_id>
   - CurrentPolicyInfo tracking in CapsuleHandle
   - StageTransition events include policy_id/hash
   - 15 policy tests passing

3. ✅ SPEC-KIT-971 CLI (speckit capsule subcommands)
   - doctor, stats, checkpoints, commit, resolve-uri commands
   - JSON-first output with stable schema
   - 7 CLI tests passing

===================================================================
TASK FOR NEXT SESSION: CLI + 977 (Parallel Tracks)
===================================================================

### TRACK 1: Complete CLI Commands

**Location:** `cli/src/speckit_cmd.rs` (already has capsule subcommand structure)

Remaining CLI work:
1. `speckit capsule init` - Create new workspace.mv2
2. `speckit capsule events` - List events with filtering
3. `speckit capsule export` - Export to per-run capsule

### TRACK 2: Policy Event Binding (Phase 4→5 Gate)

**Goal:** Every event emitted after policy capture includes policy_id/hash.

**Locations to wire:**
- `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` - capture policy at run start
- `tui/src/memvid_adapter/capsule.rs` - emit_policy_snapshot_ref_with_info already exists
- `tui/src/memvid_adapter/policy_capture.rs` - capture_and_store_policy exists

**Implementation pattern:**
```rust
// At run start (in pipeline_coordinator.rs handle_spec_auto_run)
let policy_info = capture_and_store_policy(&capsule, &stage0_config).await?;

// All subsequent events get policy binding
capsule.emit_stage_transition_with_policy(
    spec_id, run_id, stage, commit_hash, policy_info
)?;
```

**Tests needed:**
1. All events after policy capture include policy_id
2. Policy unchanged across stages in same run
3. Phase 4→5 gate verification test

===================================================================
FILES CHANGED THIS SESSION (2026-01-13)
===================================================================

| File | Change |
|------|--------|
| tui/src/chatwidget/spec_kit/git_integration.rs | StageCommitResult, create_capsule_checkpoint, 3 tests |
| tui/src/chatwidget/spec_kit/pipeline_coordinator.rs | Wired checkpoint creation after git commit |
| tui/src/memvid_adapter/capsule.rs | CurrentPolicyInfo, put_policy, list_events, policy in StageTransition |
| tui/src/memvid_adapter/mod.rs | Export new types |
| tui/src/memvid_adapter/policy_capture.rs | Uses put_policy() |
| stage0/src/policy.rs | Deterministic hash, content_matches, content_changed |
| cli/src/speckit_cmd.rs | Capsule CLI subcommands |

===================================================================
TEST SUMMARY
===================================================================

| Module | Tests | Status |
|--------|-------|--------|
| git_integration | 5 | ✅ All passing |
| capsule | 9 | ✅ All passing |
| stage0 policy | 15 | ✅ All passing |
| CLI | 7 | ✅ All passing |

Run commands:
```bash
cargo test -p codex-tui --lib "git_integration"
cargo test -p codex-tui --lib "capsule"
cargo test -p codex-stage0 "policy"
```

===================================================================
KEY CODE PATTERNS IMPLEMENTED
===================================================================

### Checkpoint Integration Flow

```
auto_commit_stage_artifacts()
    ├── Stage files (git add)
    ├── Commit with message
    ├── Return StageCommitResult { commit_hash, stage }
    │
    └── Pipeline coordinator:
        └── create_capsule_checkpoint(spec_id, run_id, stage, commit_hash, cwd)
            ├── Open CapsuleHandle
            ├── commit_stage(spec_id, run_id, stage_name, commit_hash)
            └── Return CheckpointId
```

### PolicySnapshot Hash (Deterministic)

```rust
// Excluded from hash (runtime values):
// - policy_id (generated at capture time)
// - created_at (timestamp)
// - hash (self-referential)

// Included in hash (content):
// - policy_name
// - policy_version
// - source_files (sorted for determinism)
// - model_config
// - scoring_weights
```

### Global Policy URI

```
mv2://workspace/policy/{policy_id}
    └── Capsule-scoped, globally referenceable
    └── Stored via put_policy() at dedicated path
```

===================================================================
ARCHITECTURAL NOTES
===================================================================

### Event Binding Pattern

All events should include:
- event_type: EventType enum
- spec_id, run_id: Pipeline context
- stage: Optional stage name
- policy_id, policy_hash: From CurrentPolicyInfo
- payload: Event-specific data

### Phase 4→5 Gate Requirements

1. PolicySnapshot captured at run start ✅
2. All events tagged with policy_id (partial - StageTransition done)
3. Policy unchanged verification (content_matches helper exists)
4. Export includes policy metadata

===================================================================
QUICK COMMANDS
===================================================================

```bash
# Build
~/code/build-fast.sh

# Run tests
cargo test -p codex-tui --lib
cargo test -p codex-stage0 --lib

# Specific modules
cargo test -p codex-tui --lib "capsule"
cargo test -p codex-tui --lib "git_integration"
cargo test -p codex-stage0 "policy"

# CLI smoke test
./target/debug/code-tui speckit capsule doctor
./target/debug/code-tui speckit capsule stats
./target/debug/code-tui speckit capsule checkpoints
```

===================================================================
DO NOT INCLUDE (Deferred)
===================================================================

- Dead code cleanup (9 clippy warnings) - defer to later session
- SPEC-KIT-973 Time-travel UI - needs CLI complete first
- SPEC-KIT-976 Logic Mesh - needs 977 policy wiring complete

===================================================================
OUTPUT EXPECTATION
===================================================================

1. Complete remaining CLI commands (init, events, export)
2. Wire policy capture at pipeline run start
3. Ensure all events include policy_id after capture
4. Add phase 4→5 gate verification test
5. Commit with spec IDs and decision IDs
6. Update HANDOFF.md with progress
```

---

## Progress Tracker

### Completed This Session (2026-01-13)

| Task | Status | Tests |
|------|--------|-------|
| 971 Checkpoint Integration | ✅ | 5 passing |
| 977 PolicySnapshot Wiring | ✅ | 15 passing |
| 971 CLI (partial) | ✅ | 7 passing |

### Completed Specs

| Spec | Status | Key Deliverables |
|------|--------|------------------|
| SPEC-KIT-971 (core) | ✅ | Capsule foundation, crash recovery, persistence |
| SPEC-KIT-971 (A5) | ✅ | Pipeline backend routing |
| SPEC-KIT-971 (lock) | ✅ | Cross-process single-writer lock |
| SPEC-KIT-971 (checkpoints) | ✅ | Stage boundary checkpoints with git integration |
| SPEC-KIT-971 (CLI) | 🔄 70% | doctor/stats/checkpoints done, init/events/export pending |
| SPEC-KIT-972 | ✅ | Hybrid retrieval, eval harness |
| SPEC-KIT-977 (hash) | ✅ | Deterministic hash, content helpers |
| SPEC-KIT-977 (wiring) | 🔄 60% | Policy capture + storage done, event binding partial |

### In Progress

| Spec | Status | Next Step |
|------|--------|-----------|
| SPEC-KIT-971 (CLI) | 🔄 70% | Add init, events, export commands |
| SPEC-KIT-977 (wiring) | 🔄 60% | Wire policy capture at run start, bind all events |
| SPEC-KIT-978 | 🔄 0% | Create ReflexBackend trait |

### Phase Gates

| Phase | Gate | Status |
|-------|------|--------|
| 1→2 | 971 URI contract + checkpoint tests | ✅ Passed |
| 2→3 | 972 eval harness + 975 event schema v1 | ✅ Passed |
| 3→4 | 972 parity gates + export verification | ✅ Passed |
| 4→5 | 977 PolicySnapshot + 978 reflex stack | ⏳ 60% Complete |

---

## Architecture Summary

### Checkpoint + Git Integration Flow

```
Pipeline Stage Complete
    │
    ├── auto_commit_stage_artifacts()
    │   ├── git add <stage files>
    │   ├── git commit -m "feat(SPEC-ID): complete Stage stage"
    │   └── Return StageCommitResult { commit_hash, stage }
    │
    └── create_capsule_checkpoint()
        ├── CapsuleHandle::open(config)
        ├── handle.commit_stage(spec_id, run_id, stage, commit_hash)
        │   ├── Create CheckpointMetadata
        │   ├── Emit StageTransition event (with policy_id if set)
        │   └── Persist to capsule
        └── Return CheckpointId
```

### Policy Capture + Binding Flow

```
Pipeline Run Start
    │
    └── capture_and_store_policy(&capsule, &config)
        ├── PolicySnapshot::capture(files, config)
        ├── capsule.put_policy(snapshot)  // Global URI
        └── capsule.set_current_policy(policy_id, hash)

All Subsequent Events
    │
    └── event.policy_id = capsule.current_policy.id
        event.policy_hash = capsule.current_policy.hash
```

---

## Key Files Reference

| File | Purpose |
|------|---------|
| tui/src/chatwidget/spec_kit/git_integration.rs | Git auto-commit + capsule checkpoint |
| tui/src/chatwidget/spec_kit/pipeline_coordinator.rs | Pipeline orchestration |
| tui/src/memvid_adapter/capsule.rs | CapsuleHandle, checkpoints, events |
| tui/src/memvid_adapter/policy_capture.rs | Policy capture utilities |
| stage0/src/policy.rs | PolicySnapshot struct, deterministic hash |
| cli/src/speckit_cmd.rs | CLI subcommands |

---

## Commits This Session

```
27cbdeddc docs(handoff): SPEC-KIT-971 session complete + CLI next steps
04f2807cc feat(memvid): SPEC-KIT-971 cross-process single-writer lock
5d00c1f2b test(stage0,memvid): SPEC-KIT-971-A5 acceptance tests pass
400704922 docs: V6 contract alignment + policy source files + spec updates
a42f594fd feat(stage0,memvid): SPEC-KIT-971 CLI + SPEC-KIT-977 PolicySnapshot
```

---

*Generated by Claude Code session 2026-01-13*
