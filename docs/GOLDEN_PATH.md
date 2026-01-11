# Golden Path: Memvid-First Workbench (End-to-End)

This is the “don’t think, just do” walkthrough for implementors and dogfooders.
If a feature isn’t exercised here, it’s at risk of being misunderstood or under-tested.

> **Note:** Some commands are introduced by SPEC-KIT-971/973/974/975 and may not exist yet.
> The goal is that the commands and outputs below become the acceptance test fixtures.

---

## 0) One-time setup (per workspace)

### 0.1 Ensure Memvid backend is enabled

`./.speckit/config.toml` (example)

```toml
[memory]
backend = "memvid"           # memvid | local-memory (fallback until SPEC-KIT-979)
workspace_capsule = ".speckit/memvid/workspace.mv2"

[memvid]
features_default = ["lex", "vec", "temporal_track"]
encryption_default = "off"   # off | on (workspace); exports default to encrypted
```

### 0.2 Initialize capsule

```bash
speckit capsule init
speckit capsule status
speckit capsule doctor
speckit capsule stats
```

Expected:
- capsule created at `./.speckit/memvid/workspace.mv2`
- doctor returns `OK` (no lock, version compatible, footer/index readable)
- stats prints size, frame counts, index status, dedup ratio

---

## 1) Start a run (run isolation via branch)

### 1.1 Start run (via TUI or CLI)

TUI:
```
/speckit.run start SPEC-KIT-971
```

CLI (example shape):
```bash
speckit run start SPEC-KIT-971
```

Expected behavior:
- system creates a writable branch `run/<RUN_ID>` from `main`
- all artifacts/events written during this run are tagged with:
  - `spec_id=SPEC-KIT-971`
  - `run_id=<RUN_ID>`
  - `branch_id=run/<RUN_ID>`
  - `stage=<stage>`
  - `commit_hash=<git_sha>` (if available)

Validate:
```bash
speckit capsule status
speckit capsule checkpoints
```

---

## 2) Ingest artifacts + evidence (during stages)

### 2.1 Ingest a spec artifact (manual example)

```bash
speckit ingest docs/SPEC-KIT-971-memvid-capsule-foundation/spec.md   --spec SPEC-KIT-971   --run <RUN_ID>   --stage plan
```

Expected:
- a stable logical URI is returned, e.g.

```
mv2://workspace/<WORKSPACE_ID>/spec/SPEC-KIT-971/run/<RUN_ID>/stage/plan/artifact/docs/SPEC-KIT-971.../spec.md
```

### 2.2 Stage boundary commit creates a checkpoint

This should happen automatically at stage transitions, but manual commit must exist:

```bash
speckit capsule commit --label "after_plan"
speckit capsule checkpoints
```

Expected:
- new checkpoint shows up with:
  - checkpoint_id, label, stage, spec_id, run_id, git sha, timestamp

---

## 3) Retrieval (hybrid lex + vec) with explainability

### 3.1 Search (current branch view)

```bash
speckit memory search "capsule uri invariants" --top-k 8 --explain
```

Expected:
- results show:
  - uri
  - snippet/preview
  - lex_score, vec_score, recency_bias
  - final fused score + “why returned” breakdown

### 3.2 Search “as-of checkpoint” (time-travel)

```bash
speckit memory search "uri invariants" --as-of "after_plan" --top-k 8 --explain
```

Expected:
- results reflect **exactly** what was committed at that checkpoint.

---

## 4) Time-travel UX (TUI)

### 4.1 Timeline + diff

TUI:
```
/speckit.timeline --run <RUN_ID>
/speckit.diff --a after_plan --b after_implement
```

Expected:
- timeline shows stage transitions + checkpoint labels
- diff shows artifacts added/changed between checkpoints

---

## 5) Merge semantics (run → main)

### 5.1 Unlock PASS triggers merge (default: curated)

If Unlock passes, the system should:
- merge `run/<RUN_ID>` into `main` using merge mode `curated` by default
- emit a `BranchMerged` event
- main retrieval now includes the promoted artifacts/cards/edges

Validate:
```bash
speckit capsule checkpoints
speckit memory search "SPEC-KIT-971" --branch main --top-k 3
```

---

## 6) Export/import (audit handoff)

### 6.1 Export encrypted + safe (default)

```bash
speckit capsule export --run <RUN_ID> --out ./exports/SPEC-KIT-971_<RUN_ID>.mv2e
```

Expected:
- export is encrypted by default
- “safe export” applied by default:
  - includes artifacts + evidence + checkpoints + policy snapshots + retrieval events
  - excludes raw LLM I/O unless capture mode is `full`
- emits `CapsuleExported` event into the workspace capsule

### 6.2 Import on another machine (read-only mount)

```bash
speckit capsule import ./exports/SPEC-KIT-971_<RUN_ID>.mv2e --mount-as audit_971
```

Expected:
- runs `speckit capsule doctor` automatically (or `--require-verified`)
- mounts as **read-only** by default
- supports search/time-travel/replay on the imported capsule

---

## 7) Replayable audits (offline-first)

### 7.1 Replay

```bash
speckit replay <RUN_ID> --as-of after_implement --offline
```

Expected:
- produces:
  - `replay_report.md`
  - `replay_report.json`
- “exact” replay means:
  - ✅ retrieval request/response payloads reproduce within epsilon
  - ✅ event timeline reproduces exactly
  - ⚠️ model prompts/responses depend on capture mode:
    - `full` enables full reconstruction
    - default (`summary+hash`) is partial and will show gaps

### 7.2 Compare replays (policy or retrieval config changes)

```bash
speckit replay <RUN_ID> --as-of after_implement --compare-to after_unlock
```

Expected:
- diff includes:
  - changed hit sets / scores
  - changed gate decisions
  - changed policy snapshot refs (if any)

---

## Exit criteria

This Golden Path is “green” when:
- every command exists and runs end-to-end,
- outputs match the described invariants,
- and CI can run a reduced version of this flow against fixtures.
