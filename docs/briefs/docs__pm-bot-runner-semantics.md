# Session Brief — docs/pm-bot-runner-semantics

## Goal

Update `SPEC-PM-001` / `SPEC-PM-002` docs to reflect confirmed bot-runner semantics (NotebookLM hard requirement for `NeedsResearch`, validator tool+write access, background service spawned by TUI, and web-research allowances).

## Scope / Constraints

- Docs-only (no Rust changes).
- Tier‑1 parity + headless never prompts remain locked constraints (D133).
- Linux-only expectations.

## Plan

- Update `docs/SPEC-PM-001-project-management/PRD.md` to reference the pinned `SPEC-PM-002` semantics.
- Update `docs/SPEC-PM-002-bot-runner/spec.md` to capture the confirmed execution model + tool/write boundaries.
- Run doc gates + pre-commit, then PR + merge.

## Open Questions

- Should the in-depth bot automation system be tracked as its own dedicated SPEC/project (separate from PM semantics)?

## Verification

- `python3 scripts/doc_lint.py`
- `python3 scripts/check_doc_links.py`
- `bash .githooks/pre-commit`

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->
## Product Knowledge (auto)

- Query: `SPEC-PM-002 needsresearch needsreview notebooklm hard fail validator bot service worktree branch`
- Domain: `codex-product`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260207T030318Z/artifact/briefs/docs__pm-bot-runner-semantics/20260207T030318Z.md`
- Capsule checkpoint: `brief-docs__pm-bot-runner-semantics-20260207T030318Z`

No high-signal product knowledge matched. Try a more specific `--query` and/or raise `--limit`.

<!-- END: SPECKIT_BRIEF_REFRESH -->
