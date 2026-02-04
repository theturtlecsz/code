# Branch Briefs (per PR/session)

Each feature branch / PR is treated as a "session".

To prevent context drift, every branch must include a **non-empty** branch brief:

```
docs/briefs/<branch>.md
```

Where `<branch>` is your git branch name with `/` replaced by `__`.

Example:

* Branch: `fix/doc-lint-warnings`
* Brief: `docs/briefs/fix__doc-lint-warnings.md`

This is enforced by `.githooks/pre-commit` (hard block). Use `DEV_BRIEF.md` on `main`
as the stable Tier-1 anchor (typically `Current Focus: Idle`).

## Minimal template

Copy this into your branch brief:

```md
# Session Brief — <branch>

## Goal

## Scope / Constraints

## Plan

## Open Questions

## Verification
```

## Quick Start

Initialize brief for your feature branch:

```bash
code speckit brief init
```

Then (optionally) enrich with product knowledge:

```bash
code speckit brief refresh --query "your feature keywords"
```

## Validation

Check brief exists before committing (CI use):

```bash
code speckit brief check
```
