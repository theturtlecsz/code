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

## Required Elements

Branch briefs must contain:

1. **Non-empty content** — At least some text describing the session goal
2. **Refresh block** — The `<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->` marker block
3. **Capsule snapshot** — An `mv2://` URI and checkpoint line within the refresh block

These elements ensure briefs are:

* Git-committed (version controlled)
* Capsule-snapshotted (immutable artifact reference)

The pre-commit hook and `code speckit brief check --require-capsule-snapshot` enforce these.

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

Create brief with product knowledge and capsule snapshot:

```bash
code speckit brief refresh --query "your feature keywords"
```

This command:

1. Searches local-memory for relevant product knowledge
2. Synthesizes constraints via Ollama
3. Writes the refresh block with capsule metadata
4. Snapshots the brief to the workspace capsule

Alternatively, create a minimal brief first, then refresh:

```bash
code speckit brief init
code speckit brief refresh --query "your feature keywords"
```

## Validation

Check brief exists with all required elements:

```bash
code speckit brief check --require-refresh-block --require-capsule-snapshot
```

The pre-commit hook runs equivalent checks automatically.
