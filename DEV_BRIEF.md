# DEV\_BRIEF.md

> **Tier-1 Truth Anchor** — Required for every session. Update before starting work.

**Last Updated**: 2026-02-03

## Current Focus

<!-- What are we working on right now? -->

## Scope / Constraints

* Local-memory: CLI-only (no MCP) — see [MEMORY-POLICY.md](MEMORY-POLICY.md)
* Historical docs under `docs/SPEC-KIT-*` are frozen

## Open Questions

<!-- Unresolved decisions or clarifications needed -->

## Verification

```bash
python3 scripts/doc_lint.py      # Must pass
bash .githooks/pre-commit        # Must pass
```
