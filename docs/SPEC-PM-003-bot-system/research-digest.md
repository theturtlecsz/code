# SPEC-PM-003 External Research Digest (Gemini + ChatGPT)

**Date**: 2026-02-08  
**Status**: Informational (non-authoritative)

This document captures a review-friendly digest of external deep-research outputs for `SPEC-PM-003` that were generated outside the repo (Gemini Deep Research + ChatGPT Deep Research) and then pasted into this workspace.

It is **not** a locked decision register and must not override:

1. `codex-rs/SPEC.md` (invariants + doc precedence)
2. `docs/DECISIONS.md` (locked decisions)
3. `docs/SPEC-PM-001-project-management/PRD.md`
4. `docs/SPEC-PM-002-bot-runner/spec.md`
5. `docs/SPEC-PM-003-bot-system/spec.md`

## 1. High-Signal Alignment (Matches Current SPEC-PM-003)

Both research outputs strongly agree with the current design direction already captured in `SPEC-PM-003`:

- **Ephemeral CLI runner baseline** (a “Phoenix process”): one run = one process invocation (`code speckit pm bot run ...`), persists artifacts to capsule, exits.
- **No always-on daemon posture** (D38) and **Tier‑1 parity by construction** (D113/D133): the TUI should spawn the same CLI command used in headless/CI.
- **Three-surface truth model**: capsule (`mv2://`) as SoR; OverlayDb and local-memory are consultative/ephemeral surfaces.
- **Bot taxonomy**:
  - `NeedsResearch`: NotebookLM is a hard dependency; missing NotebookLM → **BLOCKED** (no silent downgrade).
  - `NeedsReview`: safe mutation via bot-owned worktrees and patch bundles; default read-only.
- **Safety posture**: explicit write enablement only; “no silent destructive actions” in headless; allowlists for tools.
- **Artifacts-first**: outputs should be structured and persisted as immutable capsule artifacts; filesystem + Linear are projections.

Net: the research mostly **confirms** the architecture already in `docs/SPEC-PM-003-bot-system/spec.md`.

## 2. Net-New Proposals Worth Considering

These items are useful ideas not currently spelled out in detail in `SPEC-PM-003` (or need explicit confirmation against policy/decisions):

### 2.1 NDJSON / Event Stream Protocol

Proposal: standardize on **NDJSON** (“newline-delimited JSON”) events on stdout to represent progress:

- `{"type":"step_start",...}`
- `{"type":"progress",...}`
- `{"type":"artifact",...}`
- `{"type":"result",...}`

This is compatible with a “TUI as viewer” model, but would require explicitly defining:

- how the stream relates to the `SPEC-PM-002` **final JSON result schema** (e.g., streaming mode vs final-only),
- which events are considered Tier‑1 stable (and must have parity across surfaces).

### 2.2 “Pre-Flight Maieutics” Manifest

Proposal: satisfy “headless never prompts” by shifting all interactive questioning earlier:

- run maieutic dialogue in TUI (or explicit CLI command),
- persist `MaieuticContext.json` / “answers manifest” to capsule,
- runner consumes the manifest; if missing/incomplete → `NEEDS_INPUT` exit code.

This idea fits D133 well but must be reconciled with `SPEC-PM-002` semantics (PM bots are manual, optional automation; `/speckit.auto` is still the primary maieutic gate for execution).

### 2.3 Permission Escalation Flags as “Bot sudo”

Proposal: represent capabilities explicitly as flags (e.g., `--allow-net`, `--allow-write`), validated against policy.

We already have `--write-mode` in `SPEC-PM-002`; adding an explicit network capability flag could tighten safety for research runs (and may be required for headless CI posture).

### 2.4 Artifact Lifecycle / Retention Policy

Proposal: formalize retention/compression/cleanup policy for large evidence artifacts and worktrees.

This is **not currently locked**. If adopted, it should be designed as:

- capsule-first (truth preserved),
- projection cleanup as best-effort,
- never deleting artifacts needed for replay determinism without an explicit policy/versioned decision.

## 3. Potentially Unverified Claims / Hallucination Risk

Some statements in the pasted research read as *assertions of existing code/modules* rather than proposals. Treat these as **unverified** until validated in-repo:

- Specific module names like `evidence_cleanup.rs` and specific retention numbers (“30 days compress”, “180 days purge”).
- “Memvid-First Workbench initiative in Q1 2026” as a named program milestone.
- Blanket claims about which exact models are used for “Reflex vs Deep loops” in PM bots (Codex has reflex routing for Implementer stages; PM bot routing may be separate and should be policy-driven).

If we want any of these, we should convert them into explicit roadmap items/decisions rather than assuming they already exist.

## 4. Concrete Follow-Ups (for Planning Architect Review)

If we want to tighten `SPEC-PM-003` beyond the current draft, recommended next “lockable” decisions:

1. **Progress output contract**:
   - final JSON only vs optional NDJSON streaming mode,
   - event schema versioning and parity requirements.
2. **Capability model**:
   - how network/tool permissions are represented (flags vs config),
   - allowlist location + versioning.
3. **Context snapshot policy**:
   - which inputs must be snapshotted for replay,
   - how local-memory inputs become evidence packs.
4. **Lifecycle cleanup policy**:
   - how/when to garbage collect worktrees and non-essential projections,
   - what “safe export” includes/excludes by capture mode.

## 5. NotebookLM (“spec-kit-dev”) Note

This digest is intended to be added as a NotebookLM source so future product development discussions can cite it without re-pasting large research outputs.

