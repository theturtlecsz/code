# Upstream Sync State

Living document tracking synchronization state between local fork and upstream Codex.

## Current Sync Point

| Attribute | Value |
|-----------|-------|
| **Local Branch** | `main` |
| **Last Sync Commit** | `65ae1d449` (tui2 port from upstream) |
| **Sync Date** | 2024-12-23 |
| **tui2 Source** | Upstream codex-cli (OpenAI fork) |

## Divergence Inventory

### Intentional Local Extensions

These are features added locally that do not exist upstream:

| Component | Feature | Rationale |
|-----------|---------|-----------|
| `codex-protocol` | JsonSchema derives on all types | Supports app-server-protocol MCP integration |
| `codex-core` | Spec-kit integration | Local development workflow tooling |
| `codex-tui` | Stage0 protocol handler | Multi-stage execution model |

### Upstream Features Not Ported

These upstream features are intentionally not included:

| Feature | Reason | Strategy |
|---------|--------|----------|
| `oss` module | OSS-specific provider logic | Not needed for local fork |
| `features` module | Runtime feature flags | Use compile-time features instead |
| `skills` module | Skills system | Deferred (may port later) |
| `models_manager` | Dynamic model management | Use static config |
| Terminal interaction events | Complex terminal handling | Simplified implementation |

### API Divergences

See [`docs/upstream/TYPE_MAPPING.md`](docs/upstream/TYPE_MAPPING.md) for detailed type mapping.

Key divergences:

1. **Config struct**: Missing upstream fields (`animations`, `notices`, `features`, `cli_auth_credentials_store_mode`, etc.)
2. **EventMsg variants**: Missing streaming/MCP events (`McpStartupUpdate`, `ElicitationRequest`, etc.)
3. **SandboxPolicy**: No `ExternalSandbox` variant (upstream feature)
4. **RateLimitSnapshot**: Different field set (no `credits`, `plan_type`)

## Commit Conventions

Use these prefixes to track upstream-related changes:

| Prefix | Meaning | Example |
|--------|---------|---------|
| `#upstream-fix` | Bug fix backported from upstream | `fix(core): handle null response #upstream-fix` |
| `#local-only` | Local-only change, never sync | `feat(tui): spec-kit commands #local-only` |
| `#sync-compat` | Change to improve sync compatibility | `refactor(protocol): align with upstream types #sync-compat` |

## Sync Checklist (For Future Upstream Pulls)

When pulling changes from upstream:

- [ ] Create feature branch: `upstream-sync-YYYY-MM-DD`
- [ ] Identify changed files: `git diff <old-sync-commit>..<new-upstream-commit> --name-only`
- [ ] For each changed file:
  - [ ] Check TYPE_MAPPING.md for known divergences
  - [ ] Apply changes carefully, preserving local extensions
  - [ ] Add new divergences to TYPE_MAPPING.md if needed
- [ ] Run full test suite: `cargo test --workspace`
- [ ] Update this sync point section
- [ ] Commit with: `sync(upstream): merge upstream changes YYYY-MM-DD`

## Version Tagging

Local patches on upstream versions use this convention:

```
v1.2.3-local.1    # First local patch on upstream v1.2.3
v1.2.3-local.2    # Second local patch
```

## Files Changed This Session

Changes made during SYNC-028 (tui2 port):

| File | Change Type | Status |
|------|-------------|--------|
| `tui2/*` | New (ported from upstream) | Compiling with errors |
| `backend-client/*` | New (ported from upstream) | BUILDS |
| `app-server-protocol/*` | New (ported from upstream) | BUILDS |
| `codex-protocol/*` | Modified (JsonSchema derives) | BUILDS |
| `codex-core/*` | Modified (type compatibility) | BUILDS |

---

_Last updated: 2024-12-24 (SYNC-028 Session 7)_
