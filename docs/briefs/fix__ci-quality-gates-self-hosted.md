# CI: Quality Gates Self-Hosted Runner

<!-- REFRESH-BLOCK
query: "CI quality gates self-hosted"
snapshot: (none - documentation only)
END-REFRESH-BLOCK -->

## Summary

Change Quality Gates to self-hosted Linux.

## Change

Line 15: `ubuntu-latest` → `[self-hosted, Linux]`

## Rationale

* Faster execution (local caching)
* Consistent environment
* Resource control

## Rationale

* Consistent build environment
* Faster CI execution (local caching)
* Resource control

Co-Authored-By: Claude Sonnet 4.5 (1M context) <noreply@anthropic.com>
