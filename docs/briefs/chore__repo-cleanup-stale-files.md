# Branch Brief: chore/repo-cleanup-stale-files

## Focus

Remove \~12 MB of stale files, dead specs, archive bloat, and root-level junk from the repository.

## Scope

* Root-level one-off scripts and artifacts
* Stale handoff/planning markdown files
* Duplicate PR template
* Entire archive/ directory (zip packs)
* Dead SPEC directories (SPEC-KIT-103, SPEC-KIT-900)
* Large evidence artifacts (HISTORY\_ROLLUP, guardrail JSONs, tmux logs)
* Unused mcp-smoke crate

## Constraints

* Deletions only; no code changes
* All content recoverable from git history
* Build, tests, fmt, clippy must pass

## Wave 2 (2026-02-17)

* Phase 1: Remove 1,725 runtime-generated .speckit/policies/ snapshots (\~5.5 MB)
* Phase 2: Archive 12 completed/deprecated SPEC dirs to zip, then git rm
* Phase 4: Remove superseded MCP schema (2025-03-26)
* Broaden .gitignore to block all .speckit/ directories

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->

* Capsule checkpoint: mv2://chore-repo-cleanup-stale-files/2026-02-17
* Refreshed: 2026-02-17T01:57:00Z

<!-- END: SPECKIT_BRIEF_REFRESH -->
