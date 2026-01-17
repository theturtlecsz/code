# SPEC-KIT-971 — Memvid Capsule Foundation + Single-Writer Adapter
**Date:** 2026-01-17 (Updated)
**Status:** COMPLETE (100%)
**Owner (role):** Platform Eng

## Summary
Introduce Memvid as an in-process backend behind Stage0 memory traits with a single-writer capsule coordinator, checkpoint commits aligned to Spec-Kit stages, and crash-safe reopen/search.

## Decision IDs implemented

**Implemented by this spec:** D1, D2, D3, D4, D6, D7, D18, D20, D45, D53, D70

**Referenced (must remain consistent):** D8, D22, D52

**Explicitly out of scope:** D79

---

## Non-Negotiables

These requirements are non-negotiable and must be enforced mechanically:

- **Memvid capsule is system-of-record** - local-memory is fallback only (until SPEC-KIT-979 parity gates pass)
- **Single-writer capsule model is enforced** - Cross-process lock + writer queue
- **Stage boundary commits create checkpoints** - Automatic on stage transitions
- **Manual commits also create checkpoints** - User-triggered via CLI/TUI
- **Run isolation via branches** - Every run writes to `run/<RUN_ID>` branch; merges at Unlock
- **URI stability** - Only `mv2://` logical URIs appear in graphs/events; no physical IDs

---

## Goals
- Deliver the listed deliverables with tests and safe rollout.
- Keep Stage0 core abstracted (Memvid is an adapter).

## Non-Goals
- Hosted multi-tenant memory service.
- Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

---

## CLI Contract

### TUI Slash Commands (Interactive)

| Command | Description |
|---------|-------------|
| `/speckit.capsule doctor` | Verify capsule health, lock status, last-good checkpoint |
| `/speckit.capsule stats` | Size, frame counts, index status, dedup ratio |
| `/speckit.capsule checkpoints` | List checkpoints with ID, label, stage, time |
| `/speckit.capsule commit --label <LABEL>` | Create manual checkpoint |
| `/speckit.capsule resolve-uri <mv2://...>` | Resolve URI to physical location |

### Headless CLI (Implemented)

| Command | Description | Status |
|---------|-------------|--------|
| `code speckit capsule init [--force] [--json]` | Initialize new workspace capsule | DONE |
| `code speckit capsule doctor [--json]` | Health check with actionable recovery steps | DONE |
| `code speckit capsule stats [--json]` | Stats with optional JSON output | DONE |
| `code speckit capsule checkpoints [--json]` | List checkpoints, JSON for automation | DONE |
| `code speckit capsule events [--stage <S>] [--type <T>] [--limit N] [--json]` | List events with filtering | DONE |
| `code speckit capsule commit --label <LABEL> [--force] [--json]` | Create checkpoint (--force allows duplicate labels) | DONE |
| `code speckit capsule resolve-uri <URI> [--as-of <checkpoint>] [--out <path>] [--json]` | Time-travel URI resolution | DONE |
| `code speckit capsule export --spec <ID> --run <ID> [--out <path>] [--json]` | Export run archive | DONE |

### Output Requirements

- Every command supports `--json` and emits stable machine-readable JSON
- Human output is acceptable, but JSON is the contract for tests + automation
- Exit codes: 0 = success, 1 = user error, 2 = system error

---

## Single-Writer Lock (Cross-Process)

### Lock Mechanism

On capsule open for writing, create and hold an OS-level exclusive lock:

- **Implementation**: `flock`/`fcntl` style lock via lockfile
- **Lockfile path**: `<capsule_path>.lock` (e.g., `.speckit/memvid/workspace.mv2.lock`)

### Lockfile Contents (JSON)

Matches `LockMetadata` struct in `tui/src/memvid_adapter/lock.rs`:

```json
{
  "pid": 12345,
  "host": "workstation",
  "user": "developer",
  "started_at": "2026-01-16T10:30:00Z",
  "schema_version": 1,
  "spec_id": "SPEC-KIT-971",
  "run_id": "abc123",
  "branch": "run/abc123"
}
```

**Notes:**
- `schema_version` is required (defaults to 1 for forward compatibility)
- `spec_id`, `run_id`, `branch` are optional (omitted if null)

### Lock Failure Behavior

If lock is held by another process:
- Open MUST fail with `LockedByWriter` error
- Error includes lockfile metadata and instructions (who/when, how to recover)
- `capsule doctor` detects stale locks and provides recovery steps

### Writer Queue (In-Process)

- All writes go through a bounded writer queue
- Writer queue order is FIFO
- Flush occurs on: stage commit, manual commit, process shutdown (best effort)

---

## Run Branch Contract

### Branch Naming

Every `/speckit.auto` run creates and writes to branch: `run/<RUN_ID>`

**Note**: RUN_ID should NOT include redundant `run_` prefix (normalize to avoid `run/run_*`)

### Branch Usage

- Branch name is stable and used everywhere: events, checkpoint metadata, URI resolution context
- `--branch` parameter is optional; defaults to current branch

### Merge Contract (Unlock)

- Unlock merges `run/<RUN_ID>` into `main` using merge mode: `curated` or `full` only
- Merge outputs: a merge event in the capsule + a merge checkpoint

---

## Pipeline Integration (Required)

Stage0 integration MUST honor `Stage0Config.memory_backend`:

| Backend | Behavior |
|---------|----------|
| `memvid` | Do NOT require local-memory daemon health at startup; use Memvid adapter |
| `local-memory` | Keep current behavior (require daemon health) |

### Fallback Logic

If `memvid` open fails:
1. Check if fallback is enabled in config
2. Check if local-memory daemon is healthy
3. If both true: activate fallback and log warning
4. If either false: fail with clear error

**Acceptance test**: Pipeline Stage0 can run with local-memory daemon offline when memvid backend is selected and capsule opens.

---

## Deliverables

### Core Infrastructure (DONE)
- [x] New `MemvidMemoryAdapter` implementing existing Stage0 memory traits
- [x] Capsule path conventions: `./.speckit/memvid/workspace.mv2`
- [x] Canonical URI scheme: stable `mv2://...` URIs
- [x] Config switch: `memory_backend = memvid | local-memory`
- [x] Cross-process single-writer lock (`LockMetadata` + flock)
- [x] Wire `memory_backend` into pipeline coordinator

### CLI Commands (DONE)
- [x] `speckit capsule init` - Initialize workspace capsule
- [x] `speckit capsule doctor` - Health diagnostics
- [x] `speckit capsule stats` - Size and index stats
- [x] `speckit capsule checkpoints` - List checkpoints
- [x] `speckit capsule events` - List events with filtering
- [x] `speckit capsule commit --label <LABEL> [--force]` - Manual checkpoint with label uniqueness
- [x] `speckit capsule resolve-uri <URI> [--as-of <checkpoint>]` - Time-travel resolution
- [x] `speckit capsule export` - Per-run archive export

### Checkpoint & Branching (DONE)
- [x] Checkpoint API: stage boundary commit + manual commit
- [x] Branch isolation: `run/<RUN_ID>` branch per run
- [x] Time-travel URI resolution: `resolve_uri(uri, branch, as_of=checkpoint)`
- [x] UriIndexSnapshot: checkpoint-scoped URI index persistence
- [x] Label uniqueness enforcement with `--force` override

### Merge at Unlock (DONE)
- [x] `CapsuleHandle::merge_branch(from, to, mode, spec_id, run_id)`
- [x] `BranchMerged` event type and `BranchMergedPayload`
- [x] `UriIndex::merge_branch()` - Copy URI mappings from run to main
- [x] Merge checkpoint created on main branch
- [x] Wired into Unlock stage in `pipeline_coordinator.rs`
- [x] Determinism test: URIs on run branch resolve on main after merge

### Event Plumbing (DONE)
- [x] Event track: `RunEventEnvelope` with `StageTransition` + `PolicySnapshotRef`
- [x] Routing decision events: `RoutingDecisionPayload` for reflex decisions
- [x] `BranchMerged` events emitted at Unlock

---

## Remaining Work

### CLI Polish (Future)
- [ ] `capsule checkpoints --branch <B>` - List checkpoints for specific branch
- [ ] `capsule branches` - List all branches with metadata
- [ ] `capsule merge --from <branch> --to main --mode <curated|full>` - Manual branch merge CLI

### MergeMode Semantics (Future)
- [ ] `curated` mode: UI for selective artifact inclusion during merge
- [ ] `full` mode is default; `curated` requires user interaction
- [ ] Branch state persistence after capsule reopen (current state derived from events)

---

## URI Invariants (Normative)

URIs are the addressing primitive for:
- replay/events (`SPEC-KIT-975`)
- Cards/Edges graph endpoints (`SPEC-KIT-976`)
- export/import bundles (`SPEC-KIT-974`)

If URIs drift, **replay breaks** and **graph edges orphan**. This section is intentionally strict.

### Definitions

- **Logical URI**: the stable identifier we expose externally (examples start with `mv2://...`)
- **Physical frame address**: Memvid's internal frame/fragment identity (may change with append-only writes)
- **Resolution**: mapping `(logical_uri, branch, as_of)` -> the correct physical record/revision

### Invariants

1. **Logical URIs are immutable.** Once a URI is returned to the caller, it MUST remain valid across reopen, time-travel queries, branch merges/promotions, and export/import.
2. **Logical URIs are stable keys, not "frame IDs".** A URI may have multiple revisions over time.
3. **All cross-object references use logical URIs.** Events (`source_uris`) and graph edges (`from_uri`, `to_uri`) use logical URIs.
4. **Promotion/merge writes MUST preserve the same logical URI.** Do not mint new URIs for the same conceptual object during merge.
5. **Alias map is the emergency escape hatch only.** If we must change a URI due to a bug, record `old_uri -> new_uri` in `uri_aliases` track.

---

## Acceptance Criteria (Testable)

### 971-A1: Lock Test
- Start process A -> open capsule for write -> succeeds
- Start process B -> open capsule for write -> fails with `LockedByWriter` and includes PID/host/run_id

### 971-A2: Persistence Test
- Put artifact -> close handle -> reopen handle -> `resolve_uri` returns identical bytes
- Search returns the artifact after reopen

### 971-A3: Checkpoint Test
- `commit_stage(spec_id, run_id, stage)` creates checkpoint with stable URI
- Metadata includes `spec_id`, `run_id`, `stage`, `git_commit` (if present)
- `list_checkpoints()` returns it deterministically ordered

### 971-A4: Stage Boundary Integration Test
- `/speckit.auto` completes Plan->Tasks (or any stage) and produces at least one stage checkpoint in capsule

### 971-A5: Pipeline Backend Test
- Stage0 runs with `memory_backend=memvid` and local-memory daemon absent
- Succeeds when capsule exists/opens

### Existing Acceptance Criteria
- `speckit capsule doctor` detects: missing, locked, corrupted, version mismatch
- Crash recovery: simulate crash mid-write; capsule reopens; last committed checkpoint readable
- Local-memory fallback: if capsule missing/corrupt, falls back and records evidence
- All Memvid types stay behind adapter boundary
- Every `put` returns a `mv2://` URI; URIs stable after reopen
- At least one `StageTransition` event on stage commit

---

## Dependencies
- Memvid crate(s) pinned behind adapter boundary
- Decision Register: `docs/DECISION_REGISTER.md`
- Architecture: `docs/MEMVID_FIRST_WORKBENCH.md`

## Rollout / Rollback
- Roll out behind config flags with dual-backend fallback
- Roll back by switching `memory_backend` back to `local-memory`

## Risks & Mitigations
- **Single-file corruption** -> enforce single-writer + lockfile; commit barriers; `capsule doctor`
- **Memvid API churn** -> pin versions; wrap behind traits; contract tests
- **Single-file contention** -> single-writer + lock + writer queue
- **Retrieval regressions** -> eval harness + A/B parity gates
