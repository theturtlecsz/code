# Session Brief — fix/archive-sync-docs-report

## Goal

Archive deprecated ARB Pass 1/2 working docs (including stale SYNC references) into `archive/` and register them in `docs/DEPRECATIONS.md`.

## Scope / Constraints

* Docs-only change (no Rust/product behavior changes)
* Canonical sources: `docs/DECISIONS.md` and `codex-rs/SPEC.md`

## Plan

1. Create archive pack for ARB working docs
2. Remove deprecated docs from working tree
3. Update `docs/DEPRECATIONS.md`
4. Run doc lint + link check + pre-commit

## Open Questions

## Verification

```bash
python3 scripts/doc_lint.py
python3 scripts/check_doc_links.py
bash .githooks/pre-commit
```

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->
## Product Knowledge (auto)

- Query: `archive docs/report stale docs report artifacts (SYNC references)`
- Domain: `codex-product`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260205T210701Z/artifact/briefs/fix__archive-sync-docs-report/20260205T210701Z.md`
- Capsule checkpoint: `brief-fix__archive-sync-docs-report-20260205T210701Z`

No high-signal product knowledge matched. Try a more specific `--query` and/or raise `--limit`.

<!-- END: SPECKIT_BRIEF_REFRESH -->
