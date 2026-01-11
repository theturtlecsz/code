# Memvid Environment (Ops Runbook)
**Last Updated:** 2026-01-10

This runbook documents the **Memvid-first** operational posture for Codex-RS/Spec‑Kit.

> During the rollout window, `local-memory` remains available as a fallback backend until SPEC-KIT-979 parity gates pass. See `docs/LOCAL-MEMORY-ENVIRONMENT.md`.

---

## Capsule Locations
### Workspace capsule (default)
- **Path:** `./.speckit/memvid/workspace.mv2`  
  - Gitignored by default.  
  - Contains workspace-wide memory + evidence frames + indexes + checkpoint timeline.

### Run capsule export (audit handoff)
- **Path:** `./docs/specs/<SPEC_ID>/runs/<RUN_ID>.mv2e`
- Always exported **encrypted** by default (password prompt or env var).

---

## Feature Flags (build profiles)
Memvid capabilities are compiled behind cargo feature gates to avoid bloat:
- **Default build**: `lex` + `vec` + `temporal_track` + `encryption` + (optionally) `pdf_extract` + `docx_extract`
- **Power user build**: add `clip` (images) and `whisper` (audio)

See: `SPEC-KIT-980` for ingestion feature gating details.

---

## Operator Commands
> Commands listed below are **implementation targets** for the 2026‑Q1 program. If a command is marked (planned), it will land with the referenced spec.

### Capsule lifecycle
- `speckit capsule init` *(planned: SPEC-KIT-971)*  
  Creates the workspace capsule if missing.

- `speckit capsule doctor` *(planned: SPEC-KIT-971)*  
  Validates capsule health and prints repair steps.

- `speckit capsule stats` *(planned: SPEC-KIT-971)*  
  Prints capsule size, frame counts, dedup ratio, index status.

- `speckit capsule checkpoints --last 50` *(planned: SPEC-KIT-971)*  
  Lists checkpoints (stage boundaries + manual commits).

### Retrieval sanity checks
- `speckit memory search "<query>" --backend memvid --explain` *(planned: SPEC-KIT-972)*  
  Runs hybrid retrieval and shows score breakdown (lex/vec/recency/tags).

### Time travel / branching
- `/speckit.timeline` *(planned: SPEC-KIT-973)*
- `/speckit.diff <A> <B>` *(planned: SPEC-KIT-973)*
- `/speckit.branch <checkpoint> --name <branch>` *(planned: SPEC-KIT-973)*

### Export/import
- `speckit capsule export --spec <SPEC_ID> --run <RUN_ID> --encrypt` *(planned: SPEC-KIT-974)*
- `speckit capsule import <PATH_TO_MV2E>` *(planned: SPEC-KIT-974)*

### Replay audits
- `/speckit.replay <RUN_ID> --as-of <checkpoint>` *(planned: SPEC-KIT-975)*

---

## Backups and Restore
### Backups
Because capsules are single files, backup is simple:
- Stop active writers (or ensure writer queue is idle)
- Copy the file:
  - `cp ./.speckit/memvid/workspace.mv2 ./.speckit/memvid/backups/workspace.$(date +%F_%H%M).mv2`

For encrypted exports (`.mv2e`), treat them as sensitive artifacts even when encrypted.

### Restore
- Replace the workspace capsule with a known-good backup.
- Run `speckit capsule doctor` to verify indexes and time index.

---

## Troubleshooting
### “Capsule is locked”
Expected if another Spec‑Kit process is running.
- Confirm only one writer exists (single-writer policy)
- If no writer is running, use `speckit capsule doctor` output to identify stale locks and recovery steps.

### “Version mismatch”
- Pin memvid crate versions in `Cargo.lock` during Q1.
- Prefer “read old, write new” migrations with a backup copy before upgrading.

### Corruption / partial writes
- This should be rare due to append-only frames.
- Always run `speckit capsule doctor` first.
- If doctor cannot recover, restore from the last backup.

---

## SGLang Local Reflex (Operator Notes)
Local reflex is the only “daemon-like” component we accept:
- Purpose: sub-second Rust compile/test fix loops
- Stack: SGLang OpenAI-compatible server on the RTX 5090

See: `SPEC-KIT-978` for exact launch config + bakeoff gates.
