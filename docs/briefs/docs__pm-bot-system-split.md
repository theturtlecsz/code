# Session Brief — docs/pm-bot-system-split

## Goal

Split the bot-runner documentation into:

- `SPEC-PM-002`: interface contract (commands, headless contract, artifacts, caller-visible safety/write modes)
- `SPEC-PM-003`: bot system design (runner/service/tooling internals)

## Scope / Constraints

- Docs-only (no Rust changes).
- Tier‑1 parity and headless “never prompt” remain locked constraints (D113/D133).
- NotebookLM is a hard requirement for `NeedsResearch` (no fallback research).
- Write mode (for review) must remain isolated (worktree/branch) and never silently destructive.

## Changes

- Added `docs/SPEC-PM-003-bot-system/spec.md` (system design draft).
- Refocused `docs/SPEC-PM-002-bot-runner/spec.md` into an interface contract.
- Updated cross-references in `docs/SPEC-PM-001-project-management/PRD.md` and `codex-rs/SPEC.md`.

## Verification

- `python3 scripts/doc_lint.py`
- `python3 scripts/check_doc_links.py`
- `bash .githooks/pre-commit`

