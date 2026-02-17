# Branch Brief: chore/repo-cleanup-stale-files

## Focus

Remove ~12 MB of stale files, dead specs, archive bloat, and root-level junk from the repository.

## Scope

- Root-level one-off scripts and artifacts
- Stale handoff/planning markdown files
- Duplicate PR template
- Entire archive/ directory (zip packs)
- Dead SPEC directories (SPEC-KIT-103, SPEC-KIT-900)
- Large evidence artifacts (HISTORY_ROLLUP, guardrail JSONs, tmux logs)
- Unused mcp-smoke crate

## Constraints

- Deletions only; no code changes
- All content recoverable from git history
- Build, tests, fmt, clippy must pass

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->
- Capsule checkpoint: chore/repo-cleanup-stale-files (mv2://cleanup-session)
<!-- END: SPECKIT_BRIEF_REFRESH -->
