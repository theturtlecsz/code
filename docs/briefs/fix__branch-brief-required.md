# Session Brief — fix/branch-brief-required

## Goal

Enforce a non-empty per-branch session brief at `docs/briefs/<branch>.md` (with `/` → `__`) to reduce context drift, while keeping `DEV_BRIEF.md` stable on `main` (`Current Focus: Idle`).

## Scope / Constraints

* Enforced locally via `.githooks/pre-commit` (hard block)
* `DEV_BRIEF.md` remains required and non-empty
* No changes to frozen historical docs under `docs/SPEC-KIT-*`

## Plan

1. Add `docs/briefs/README.md` with naming rules and template
2. Add pre-commit enforcement for `docs/briefs/<branch>.md` on non-main branches
3. Update agent instruction docs to document the requirement

## Open Questions

* None

## Verification

```bash
python3 scripts/doc_lint.py
bash .githooks/pre-commit
```

