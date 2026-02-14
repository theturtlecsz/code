# CI: Quality Gates Explicit Runner Label

<!-- REFRESH-BLOCK
query: "CI quality gates explicit runner"
snapshot: (none - CI infrastructure)
END-REFRESH-BLOCK -->

## Objective

Make Quality Gates runner selection valid and explicit.

## Change

`.github/workflows/quality-gates.yml` line 15:

**Before**:
```yaml
runs-on: [self-hosted, Linux]
```

**After**:
```yaml
runs-on: [self-hosted, Linux, turtle-runner]
```

## Policy

**Fixed runner**: Quality Gates requires self-hosted runner with explicit `turtle-runner` label.

**Limitation**: If turtle-runner is offline, workflow will queue indefinitely. GitHub Actions does NOT support automatic fallback to ubuntu-latest.

## Behavior

- Workflow targets self-hosted runner with `turtle-runner` label
- If runner offline: Job queues (no automatic fallback)
- Manual intervention required if runner unavailable

## Trade-off

- **Pro**: Consistent build environment, faster CI (local caching)
- **Con**: PRs blocked if runner offline

Co-Authored-By: Claude Sonnet 4.5 (1M context) <noreply@anthropic.com>
