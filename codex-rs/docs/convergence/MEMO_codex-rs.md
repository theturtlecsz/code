# Update Memo — codex-rs (Golden Path Orchestrator)

_Last updated: 2025-12-23_

## Naming reminder

- Repo/workspace: **codex-rs** (typically checked out at `~/code`)
- User CLI: **`code`**

## What changed (high level)

- `/speckit.auto` remains the **single golden path** UX.
- Stage0 Tier2 (NotebookLM) is **enabled by default**, but must **fail closed** unless configured.
- System outputs (spec-tracker / Stage0 artifacts) are stored in local-memory as **pointer memories** only.

## Non-goals

- Do not make `lm ask` the golden path.
- Do not implement local-memory policy logic ad-hoc inside codex-rs; instead conform to localmemory-policy invariants.

## Required behaviors

### 1) Tier2 enabled-by-default, fail-closed

Stage0 should attempt Tier2 by default.
Tier2 should run only when:
- notebooklm-mcp service is reachable AND ready
- the current domain/spec maps to a notebook id/url

If not true:
- proceed with Tier1 only
- emit diagnostics with actionable next steps (doctor output)

### 2) “Doctor” helpers in the `code` CLI

Add a single command surface that checks:
- local-memory daemon health (REST)
- domain resolution (domain-map)
- notebooklm service health/ready
- notebook mapping exists for this domain/spec
- (optional) ability to run a 1-question smoke test against the notebook

Suggested command names (pick one and standardize):
- `code doctor` (preferred)
- `code stage0 doctor`

### 3) System pointer memories for Stage0 outputs

When Stage0 runs, it may write a system pointer memory:
- domain: `spec-tracker`
- tags: `system:true`, `spec:<id>`, `stage:0`, `artifact:*`
- content: pointers to `TASK_BRIEF.md` and `DIVINE_TRUTH.md` plus 2–5 bullet summary

This enables traceability without polluting normal recall.

### 4) Exclusion compliance

Stage0 Tier1 memory retrieval must exclude system artifacts by default.
That means either:
- exclude domain `spec-tracker`, OR
- exclude tag `system:true`
(or both)

## Tests (acceptance-level)

- Stage0 runs successfully when notebooklm service is down (Tier1 only).
- Stage0 runs Tier2 when service is ready and notebook mapped.
- System pointer memory is written with required tags.
- Retrieval used for Tier1 does not include system memories unless explicitly enabled (debug).

## References

- Convergence matrix: `CONVERGENCE_MATRIX.yaml`
