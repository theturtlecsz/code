# HANDOFF.md — Session Continuation

**Created:** 2026-01-11
**Last Session:** 2026-01-20 (ARB Pass 2 Complete: D113-D134 Locked + Docs Alignment)
**Next Session:** SPEC-KIT-979 local-memory sunset + ACE Implementation

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
   - Reflex is a routing mode: Implementer(mode=reflex), not a new Stage0 role

===================================================================
CURRENT STATE — Session completed 2026-01-17
===================================================================

COMPLETED THIS SESSION:

1. ✅ SPEC-KIT-977 Policy CLI Commands
   - `code speckit policy list [--json]` - List all policy snapshots
   - `code speckit policy show <id> [--json]` - Show policy details
   - `code speckit policy current [--json]` - Show current active policy
   - `code speckit policy validate [--path]` - Validate model_policy.toml
   - Exported GovernancePolicy from stage0 for CLI usage

2. ✅ SPEC-KIT-977 Policy TUI Commands
   - `/speckit.policy list` - List policy snapshots
   - `/speckit.policy show <id>` - Show policy details
   - `/speckit.policy current` - Show current active policy
   - New policy.rs command file in commands/

3. ✅ SPEC-KIT-971 Merge at Unlock
   - Added `BranchMerged` event type to EventType enum
   - Added `BranchMergedPayload` struct
   - Added `UriIndex::count_on_branch()` and `merge_branch()` helpers
   - Added `CapsuleHandle::merge_branch(from, to, mode, spec_id, run_id)`
   - Added `merge_run_branch_to_main()` in git_integration.rs
   - Wired merge into Unlock stage in pipeline_coordinator.rs
   - 2 determinism tests: URIs resolve on main after merge

===================================================================
TASK FOR NEXT SESSION: SPEC-KIT-975 + 978 Remaining
===================================================================

### Priority 1: SPEC-KIT-975 Event Schema (Unblocks 973, 976)

**Goal:** Define event schema for replay determinism.

**Key deliverables:**
1. Event schema v1 with all event types
2. LLMCall event capture aligned with PolicySnapshot.capture.mode
3. Events query API for time-travel and replay

### Priority 2: SPEC-KIT-978 Remaining Work

**Already complete:**
- [x] JSON schema enforcement in agent_orchestrator.rs
- [x] Reflex config in model_policy.toml
- [x] ReflexConfig struct and load_reflex_config()
- [x] Routing decision module (reflex_router.rs)
- [x] RoutingDecision capsule events
- [x] Health check integration
- [x] ReflexMetricsDb for bakeoff stats
- [x] `code speckit reflex bakeoff` command
- [x] `code speckit reflex check` command

**Remaining:**
- [ ] Bakeoff report writer: JSON/MD to .speckit/eval/reflex-bakeoff-*
- [ ] CI gate: `code speckit reflex check` fails CI if thresholds not met
- [ ] TUI slash commands: `/speckit.reflex health|status|models`

### Optional: Documentation Updates

Consider updating specs to 100% status:
- SPEC-KIT-971: Update to 100% (merge at unlock complete)
- SPEC-KIT-977: Update to 100% (CLI/TUI complete)

===================================================================
FILES CHANGED THIS SESSION (2026-01-17)
===================================================================

| File | Change |
|------|--------|
| cli/src/speckit_cmd.rs | Added policy list/show/current/validate commands |
| stage0/src/lib.rs | Exported GovernancePolicy |
| tui/src/chatwidget/spec_kit/commands/policy.rs | NEW - TUI policy commands |
| tui/src/chatwidget/spec_kit/commands/mod.rs | Added policy module |
| tui/src/chatwidget/spec_kit/command_registry.rs | Registered policy command (45 total) |
| tui/src/chatwidget/spec_kit/git_integration.rs | Added merge_run_branch_to_main() |
| tui/src/chatwidget/spec_kit/pipeline_coordinator.rs | Wired merge at Unlock |
| tui/src/memvid_adapter/capsule.rs | Added merge_branch() method |
| tui/src/memvid_adapter/mod.rs | Exported MergeMode, BranchMergedPayload |
| tui/src/memvid_adapter/types.rs | Added BranchMerged event, BranchMergedPayload |
| tui/src/memvid_adapter/tests.rs | 2 merge determinism tests |

===================================================================
TEST SUMMARY
===================================================================

| Module | Tests | Status |
|--------|-------|--------|
| TUI total | 667 | ✅ All passing |
| merge determinism | 2 | ✅ All passing |
| policy (TUI) | 2 | ✅ All passing |
| command_registry | 16 | ✅ All passing |

Run commands:
```bash
cargo test -p codex-tui --lib
cargo test -p codex-tui --lib "merge_determinism"
cargo test -p codex-tui --lib "policy"
cargo test -p codex-tui --lib "command_registry"
```

===================================================================
KEY CODE PATTERNS IMPLEMENTED
===================================================================

### Merge at Unlock Flow

```
Unlock Stage Complete
    │
    └── pipeline_coordinator.rs:
        ├── create_capsule_checkpoint(spec_id, run_id, Unlock, commit_hash)
        │
        └── if stage == Unlock:
            └── merge_run_branch_to_main(spec_id, run_id, cwd)
                ├── Open CapsuleHandle
                ├── merge_branch(run/RUN_ID, main, Curated, spec_id, run_id)
                │   ├── Copy URI mappings from run to main
                │   ├── Update event branch_ids to main
                │   ├── Create merge checkpoint
                │   ├── Create URI index snapshot
                │   └── Emit BranchMerged event
                └── Return merge_checkpoint_id
```

### BranchMergedPayload Schema

```json
{
  "from_branch": "run/RUN_ID",
  "to_branch": "main",
  "mode": "Curated",
  "merge_checkpoint_id": "merge_20260117...",
  "uris_merged": 5,
  "events_merged": 3,
  "spec_id": "SPEC-XXX",
  "run_id": "run-xxx"
}
```

### Policy CLI Commands

```bash
code speckit policy list [--json]      # List snapshots
code speckit policy show <id> [--json] # Show details
code speckit policy current [--json]   # Current active
code speckit policy validate [--path]  # Validate TOML
```

===================================================================
ARCHITECTURAL NOTES
===================================================================

### Merge Mode Invariant

Per SPEC.md and SPEC-KIT-971:
- Merge modes are `curated` or `full` ONLY
- Never squash, ff, or rebase
- Curated = selective artifact inclusion
- Full = complete artifact preservation

### Event Binding at Merge

BranchMerged events are emitted on main branch after merge:
- stage = "Unlock"
- Includes from_branch, to_branch, mode, counts
- Merge checkpoint has label "merge:run/RUN_ID"

===================================================================
QUICK COMMANDS
===================================================================

```bash
# Build
~/code/build-fast.sh

# Run tests
cargo test -p codex-tui --lib
cargo test -p codex-stage0 --lib

# Policy CLI smoke test
./target/debug/code speckit policy list
./target/debug/code speckit policy validate

# Reflex CLI
./target/debug/code speckit reflex bakeoff
./target/debug/code speckit reflex check
```

===================================================================
DO NOT INCLUDE (Deferred)
===================================================================

- Dead code cleanup (clippy warnings)
- SPEC-KIT-973 Time-travel UI (needs 975)
- SPEC-KIT-976 Logic Mesh (needs 975)
- SPEC-KIT-979 local-memory sunset (needs 975)

===================================================================
OUTPUT EXPECTATION
===================================================================

1. Complete SPEC-KIT-975 Event Schema v1
2. Add remaining SPEC-KIT-978 work (bakeoff reports, CI gate)
3. Update spec status to 100% where complete
4. Commit with spec IDs and decision IDs
5. Update HANDOFF.md with progress
```

---

## Progress Tracker

### Completed This Session (2026-01-20)

| Task | Status | Tests |
|------|--------|-------|
| ARB Pass 2 Complete | ✅ | D113-D134 locked |
| DECISION_REGISTER.md updated | ✅ | D1-D134 now in register |
| E.3/E.4 capability tests | ✅ | 12 tests (6 archival + 6 integrity) |
| ARB enforcement tests | ✅ | 18 tests across D130-D134 |

### Completed Specs

| Spec | Status | Key Deliverables |
|------|--------|------------------|
| SPEC-KIT-971 (core) | ✅ 100% | Capsule foundation, crash recovery, persistence |
| SPEC-KIT-971 (lock) | ✅ | Cross-process single-writer lock |
| SPEC-KIT-971 (checkpoints) | ✅ | Stage boundary checkpoints with git integration |
| SPEC-KIT-971 (CLI) | ✅ | doctor/stats/checkpoints/commit/resolve-uri/init/events/export |
| SPEC-KIT-971 (merge) | ✅ | Merge at Unlock with BranchMerged event |
| SPEC-KIT-972 | ✅ | Hybrid retrieval, eval harness |
| SPEC-KIT-973 | ✅ | Time-travel TUI commands |
| SPEC-KIT-975 | ✅ | Replay timeline determinism, event schema |
| SPEC-KIT-976 | ✅ | Logic Mesh graph foundation |
| SPEC-KIT-977 (core) | ✅ 100% | PolicySnapshot capture, storage, drift detection |
| SPEC-KIT-977 (CLI) | ✅ | policy list/show/current/validate |
| SPEC-KIT-977 (TUI) | ✅ | /speckit.policy commands |
| SPEC-KIT-978 (core) | ✅ | Reflex routing, bakeoff CLI, circuit breaker |
| ARB Pass 2 | ✅ | D113-D134 locked, all H0-H7 decided |

### Phase Gates

| Phase | Gate | Status |
|-------|------|--------|
| 1→2 | 971 URI contract + checkpoint tests | ✅ Passed |
| 2→3 | 972 eval harness + 975 event schema v1 | ✅ Passed |
| 3→4 | 972 parity gates + export verification | ✅ Passed |
| 4→5 | 977 PolicySnapshot + event binding | ✅ Passed |
| 5→6 | 978 Reflex bakeoff complete + 975 event baseline | ✅ Passed |

---

## Commits This Session (2026-01-20)

```
5ce286598 docs: lock ARB decisions D113–D134 in DECISION_REGISTER
11b6837db docs: ARB output consistency cleanup (#12)
71054a8c4 docs: retire ARCHITECT_QUESTIONS decision driver (Pass 2 complete) (#11)
8e088f449 test: ARB Pass 2 enforcement suite (#10)
eea2f6302 feat: ACE Frame JSON Schema v1 (generated) (#9)
```

---

*Generated by Claude Code session 2026-01-20*
