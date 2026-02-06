# Session Brief

**Branch**: fix/pm-rubric-web-artifacts
**Date**: 2026-02-06

## Intent

- Lock deterministic PRD scoring rubric (>= 90 gate) and web research artifact schema for SPEC-PM-001.
- Add a TODO spec stub for Devin-style bot runner semantics (NeedsResearch / NeedsReview).

## Constraints

- Honor locked decisions: D130 (maieutic mandatory), D133 (multi-surface parity + headless never prompts).
- Capture modes: none | prompts_only | full_io.
- Linux-only assumptions.

## Verification

- python3 scripts/doc_lint.py
- python3 scripts/check_doc_links.py
- bash .githooks/pre-commit
<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->
## Product Knowledge (auto)

- Query: `SPEC-PM-001: deterministic scoring rubric (>=90), web research artifact schema (Tavily MCP + fallback), TODO bot runner semantics for NeedsResearch/NeedsReview`
- Domain: `codex-product`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260206T221518Z/artifact/briefs/fix__pm-rubric-web-artifacts/20260206T221518Z.md`
- Capsule checkpoint: `brief-fix__pm-rubric-web-artifacts-20260206T221518Z`

No high-signal product knowledge matched. Try a more specific `--query` and/or raise `--limit`.

<!-- END: SPECKIT_BRIEF_REFRESH -->
