# Session Brief — docs/prd-review-20260209

**Date**: 2026-02-09

## Goal

Review all non-complete PRDs and confirm alignment with current product vision:

- A) Gold Run (`SPEC-DOGFOOD-002`)
- B) Product Knowledge dogfood/measurement (`SPEC-PK-001`)
- C) Capsule-backed project/product management (`SPEC-PM-001` direction)

## Scope

- Enumerate `docs/**/PRD.md` + `codex-rs/docs/**/PRD.md` and filter out completed PRDs by Status header.
- Keep/supersede/deprecate+archive per `docs/DEPRECATIONS.md` policy.
- Reduce discovery false-positives where reasonable (avoid non-PRD docs ending in `*-prd.md`).

## Non-Goals

- Changing locked decisions in `docs/DECISIONS.md`.
- Implementing PM bot runner/system (tracked as `SPEC-PM-002` / `SPEC-PM-003`).

## Validation

- `python3 scripts/doc_lint.py`
- `python3 scripts/check_doc_links.py`
- `bash .githooks/pre-commit`

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->
## Product Knowledge (auto)

- Query: `PRD review deprecations SPEC-PM-001`
- Domain: `codex-product`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260209T010211Z/artifact/briefs/docs__prd-review-20260209/20260209T010211Z.md`
- Capsule checkpoint: `brief-docs__prd-review-20260209-20260209T010211Z`

No high-signal product knowledge matched. Try a more specific `--query` and/or raise `--limit`.

<!-- END: SPECKIT_BRIEF_REFRESH -->
