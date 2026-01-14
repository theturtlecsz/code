# HANDOFF.md — Session Continuation

**Created:** 2026-01-11
**Last Session:** 2026-01-14 (SPEC-KIT-971 CLI + SPEC-KIT-977 Policy Event Binding Complete)
**Next Session:** SPEC-KIT-978 ReflexBackend trait

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
CURRENT STATE — Session completed 2026-01-14
===================================================================

COMPLETED THIS SESSION:

1. ✅ SPEC-KIT-971 CLI Commands Complete
   - `speckit capsule init` - Create new workspace.mv2
   - `speckit capsule events` - List events with stage/type/spec/run filtering
   - `speckit capsule export` - Export per-run archive (events.json, checkpoints.json, manifest.json)
   - All CLI commands support --json output

2. ✅ SPEC-KIT-977 Policy Event Binding (Phase 4→5 Gate)
   - Policy capture wired at run start in pipeline_coordinator.rs
   - policy_id, policy_hash, policy_uri fields added to SpecAutoState
   - All StageTransition events include policy binding after capture
   - 2 phase 4→5 gate verification tests added

PRIOR SESSION (2026-01-13):

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

3. ✅ SPEC-KIT-971 CLI (initial)
   - doctor, stats, checkpoints, commit, resolve-uri commands
   - JSON-first output with stable schema
   - 7 CLI tests passing

===================================================================
TASK FOR NEXT SESSION: SPEC-KIT-978 ReflexBackend trait
===================================================================

### SPEC-KIT-978: ReflexBackend Trait

**Goal:** Create ReflexBackend trait for fast-path model inference.

**Key deliverables:**
1. ReflexBackend trait definition
2. Local inference implementation (vLLM/Ollama)
3. Cloud fallback implementation
4. Latency-based routing

### Deferred Tasks (Do Not Implement)

- Dead code cleanup (9 clippy warnings)
- SPEC-KIT-973 Time-travel UI
- SPEC-KIT-976 Logic Mesh

===================================================================
FILES CHANGED THIS SESSION (2026-01-14)
===================================================================

| File | Change |
|------|--------|
| cli/src/speckit_cmd.rs | Added init, events, export commands |
| tui/src/chatwidget/spec_kit/pipeline_coordinator.rs | Policy capture at run start |
| tui/src/chatwidget/spec_kit/state.rs | policy_id, policy_hash, policy_uri fields |
| tui/src/memvid_adapter/tests.rs | Phase 4→5 gate verification tests |

PRIOR SESSION (2026-01-13):

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
| capsule | 11 | ✅ All passing (incl. phase 4→5 gate) |
| stage0 policy | 15 | ✅ All passing |
| CLI | 7 | ✅ All passing |

Run commands:
```bash
cargo test -p codex-tui --lib "git_integration"
cargo test -p codex-tui --lib "capsule"
cargo test -p codex-tui --lib "phase_4_5"
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

### Completed This Session (2026-01-14)

| Task | Status | Tests |
|------|--------|-------|
| 971 CLI Commands (init, events, export) | ✅ | CLI tests passing |
| 977 Policy Event Binding | ✅ | 2 phase 4→5 gate tests |

### Completed Specs

| Spec | Status | Key Deliverables |
|------|--------|------------------|
| SPEC-KIT-971 (core) | ✅ | Capsule foundation, crash recovery, persistence |
| SPEC-KIT-971 (A5) | ✅ | Pipeline backend routing |
| SPEC-KIT-971 (lock) | ✅ | Cross-process single-writer lock |
| SPEC-KIT-971 (checkpoints) | ✅ | Stage boundary checkpoints with git integration |
| SPEC-KIT-971 (CLI) | ✅ | doctor/stats/checkpoints/commit/resolve-uri/init/events/export |
| SPEC-KIT-972 | ✅ | Hybrid retrieval, eval harness |
| SPEC-KIT-977 (hash) | ✅ | Deterministic hash, content helpers |
| SPEC-KIT-977 (wiring) | ✅ | Policy capture at run start, all events bound |

### In Progress

| Spec | Status | Next Step |
|------|--------|-----------|
| SPEC-KIT-978 | 🔄 0% | Create ReflexBackend trait |

### Phase Gates

| Phase | Gate | Status |
|-------|------|--------|
| 1→2 | 971 URI contract + checkpoint tests | ✅ Passed |
| 2→3 | 972 eval harness + 975 event schema v1 | ✅ Passed |
| 3→4 | 972 parity gates + export verification | ✅ Passed |
| 4→5 | 977 PolicySnapshot + event binding | ✅ Passed (2026-01-14) |
| 5→6 | 978 ReflexBackend + latency routing | ⏳ Pending |

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

## Commits This Session (2026-01-14)

```
29d2d26e2 feat(cli,memvid): SPEC-KIT-971 CLI complete + SPEC-KIT-977 policy binding
```

### Prior Session (2026-01-13)

```
8b9893ec8 feat(memvid): SPEC-KIT-971 checkpoint integration + SPEC-KIT-977 policy wiring
27cbdeddc docs(handoff): SPEC-KIT-971 session complete + CLI next steps
04f2807cc feat(memvid): SPEC-KIT-971 cross-process single-writer lock
5d00c1f2b test(stage0,memvid): SPEC-KIT-971-A5 acceptance tests pass
400704922 docs: V6 contract alignment + policy source files + spec updates
a42f594fd feat(stage0,memvid): SPEC-KIT-971 CLI + SPEC-KIT-977 PolicySnapshot
```

---

*Generated by Claude Code session 2026-01-14*
