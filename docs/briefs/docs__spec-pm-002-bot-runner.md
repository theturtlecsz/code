# Session Brief — docs/spec-pm-002-bot-runner

## Goal

Create and track a dedicated SPEC for Devin-style research/review automation bots (manual-only PM holding states).

## Scope / Constraints

- Docs-only changes (no Rust changes in this PR).
- Linux-only assumptions; no cross-platform commitments.

## Plan

- Add `SPEC-PM-002` doc stub under `docs/`.
- Add a `codex-rs/SPEC.md` Planned row for `SPEC-PM-002`.
- Keep `SPEC-PM-001` PRD pointers consistent.

## Open Questions

## Verification

Run local doc gates:

- `python3 scripts/doc_lint.py`
- `python3 scripts/check_doc_links.py`
- `bash .githooks/pre-commit`

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->
## Product Knowledge (auto)

- Query: `SPEC-PM-002 bot runner devin-style needsresearch needsreview`
- Domain: `codex-product`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260207T023054Z/artifact/briefs/docs__spec-pm-002-bot-runner/20260207T023054Z.md`
- Capsule checkpoint: `brief-docs__spec-pm-002-bot-runner-20260207T023054Z`

No high-signal product knowledge matched. Try a more specific `--query` and/or raise `--limit`.

<!-- END: SPECKIT_BRIEF_REFRESH -->
