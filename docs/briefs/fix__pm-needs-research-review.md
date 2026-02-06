# Session Brief

**Branch**: fix/pm-needs-research-review
**Date**: 2026-02-06

## Intent

Update SPEC-PM-001 + SPEC-DOGFOOD-002 docs to add optional PM states (NeedsResearch/NeedsReview) and lock assisted maieutic + Tavily MCP decisions.

## Constraints

- Honor locked decisions: D130 (maieutic mandatory), D133 (multi-surface parity + headless never prompts).
- Linux-only assumptions.

## Verification

- python3 scripts/doc_lint.py
- python3 scripts/check_doc_links.py
- bash .githooks/pre-commit
<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->
## Product Knowledge (auto)

- Query: `SPEC-PM-001: add NeedsResearch/NeedsReview PM states; assisted maieutic scoring gates; Tavily MCP pinned local; update DOGFOOD SPEC`
- Domain: `codex-product`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260206T220148Z/artifact/briefs/fix__pm-needs-research-review/20260206T220148Z.md`
- Capsule checkpoint: `brief-fix__pm-needs-research-review-20260206T220148Z`

No high-signal product knowledge matched. Try a more specific `--query` and/or raise `--limit`.

<!-- END: SPECKIT_BRIEF_REFRESH -->
