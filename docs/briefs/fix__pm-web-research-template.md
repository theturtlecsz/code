# Session Brief

**Branch**: fix/pm-web-research-template
**Date**: 2026-02-06

## Intent

Expand the WebResearchBundle artifact template beyond titles/snippets, including a temporary cache strategy, while preserving export safety and capture-mode semantics.

## Constraints

- D133 parity + headless never prompts.
- Capture modes: none | prompts_only | full_io.

## Verification

- python3 scripts/doc_lint.py
- python3 scripts/check_doc_links.py
- bash .githooks/pre-commit
<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->
## Product Knowledge (auto)

- Query: `SPEC-PM-001: expand web research artifact template beyond titles/snippets; add temporary cache strategy; preserve export safety`
- Domain: `codex-product`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260207T015316Z/artifact/briefs/fix__pm-web-research-template/20260207T015316Z.md`
- Capsule checkpoint: `brief-fix__pm-web-research-template-20260207T015316Z`

No high-signal product knowledge matched. Try a more specific `--query` and/or raise `--limit`.

<!-- END: SPECKIT_BRIEF_REFRESH -->
