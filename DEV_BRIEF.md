# DEV\_BRIEF.md

> **Tier-1 Truth Anchor** — Read first every session. Keep this short and current.

**Last Updated**: 2026-02-18

## Current Focus (P0)

**Epoch 1 Program:** Consultant-First Packet-Driven Autonomy
**Active Phase:** Trust Foundation (Days 1-30)

Priority order:

1. **PM-006 Packet Persistence** — durable `.speckit/packet.yaml` (restore on restart)
2. **PM-007 Recap Enforcement** — "Explain before Act/Merge"
3. **PM-005 Gatekeeper** — Class 2 boundary + Class E emergency protocol
   P0 Maintenance:

* **SPEC-P0-TUI2-QUARANTINE** — reduce split-brain confusion (non-default build, scaffold banner, guardrails)

Truth anchor for "what's in progress / next":

* `codex-rs/SPEC.md` (canonical tracker)
* `docs/PROGRAM.md` (30/60/90 DAG)
* `docs/VISION.md` + `docs/adr/ADR-005..ADR-012` (product contract)

## Session Workflow (Non-Negotiable)

* Work happens on feature branches (never commit directly to main).
* **Per-branch context goes in**: `docs/briefs/<branch>.md`
  * Required on feature branches (pre-commit hard blocks if missing).
  * Create/refresh via: `code speckit brief refresh --query "<keywords>"`

## Hard Constraints / Guardrails

* `tui` is primary UI; `tui2` is scaffold/reference only (no "rewrite track").
* One primary merge train; research/review threads do not merge.
* Unattended mode performs **no merges**.
* Class 2 changes only at milestone boundaries; Class E only for real emergencies.
* Local-memory access: CLI/REST only (no MCP).

## Open Questions (keep <=5)

* (fill in when blocked)

## Verification (minimum)

```bash
python3 scripts/doc_lint.py
cd codex-rs && cargo fmt --all -- --check
cd codex-rs && cargo clippy --workspace --all-targets --all-features -- -D warnings
```
