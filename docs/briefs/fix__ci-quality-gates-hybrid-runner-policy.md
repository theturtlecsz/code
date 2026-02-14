# CI: Quality Gates Hybrid Policy

<!-- REFRESH-BLOCK
query: "CI quality hybrid policy"
snapshot: (none - CI infrastructure)
END-REFRESH-BLOCK -->

## Objective

Prevent queued PR checks - use GitHub-hosted until self-hosted runner registered.

## Change

**Before**: `runs-on: [self-hosted, Linux, turtle-runner]`
**After**: `runs-on: ubuntu-latest`

## Rationale

Repository has 0 registered runners - jobs queue indefinitely.

Migration to self-hosted deferred until runner setup complete.

Co-Authored-By: Claude Sonnet 4.5 (1M context) <noreply@anthropic.com>
