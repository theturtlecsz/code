# ADR-001: Adapt tui2 to Local APIs (Option B)

| Attribute | Value |
|-----------|-------|
| **Status** | Accepted |
| **Date** | 2024-12-23 |
| **Decision Makers** | Project maintainer |
| **Technical Story** | SYNC-028 - Port TUI v2 from upstream |

## Context

The TUI v2 (`codex-tui2`) was ported from upstream codex-cli (OpenAI fork) in commit
`65ae1d449`. After porting, it had **262 compilation errors** due to API mismatches
between what tui2 expects and what the local `codex-core`, `codex-protocol`, and
`codex-common` crates provide.

The local crates have diverged from upstream in several ways:

1. **Missing upstream modules**: `oss`, `features`, `skills`, `models_manager`, `auth`
2. **Different type shapes**: Config fields, EventMsg variants, protocol types
3. **Missing methods**: Getter/setter methods on enums, various utility functions
4. **Different conventions**: Event sequencing, error handling patterns

## Decision Drivers

- **Development velocity**: How quickly can we get tui2 compiling?
- **Upstream sync ability**: How hard will future upstream merges be?
- **Local stability**: Risk of breaking existing `codex-tui` (v1)?
- **Maintenance burden**: Long-term cost of each approach?

## Options Considered

### Option A: Extend Local APIs to Match Upstream

Modify local crates (`codex-core`, `codex-protocol`, `codex-common`) to add the
types, fields, and methods that upstream tui2 expects.

**Pros:**
- tui2 code remains closer to upstream
- Easier to pull future upstream changes
- Single source of truth for APIs

**Cons:**
- Risk of breaking existing `codex-tui` (v1)
- Must add many unused types/fields just for compatibility
- Increases crate complexity with unused code
- Some upstream features (OSS providers, skills) may not make sense locally

### Option B: Modify tui2 to Use Local APIs (Selected)

Adapt the ported tui2 code to work with local APIs as they exist, stubbing or
removing references to missing upstream features.

**Pros:**
- Zero risk to existing `codex-tui` stability
- No unused compatibility code in core crates
- Can selectively adopt upstream features
- Clear documentation of divergences

**Cons:**
- tui2 diverges from upstream
- Future upstream merges require manual reconciliation
- Must maintain TYPE_MAPPING.md documentation

### Option C: Create Compatibility Shim Layer

Build an intermediate adapter layer that maps between upstream and local APIs.

**Pros:**
- Clean separation of concerns
- Could theoretically support both tui and tui2

**Cons:**
- Most complex to implement
- Additional indirection hurts performance
- Another crate to maintain
- Shim layer would need updates for both upstream and local changes

## Decision

**Option B: Modify tui2 to use local APIs.**

The key factors:

1. **Stability**: The existing `codex-tui` is production-quality and must not regress
2. **Pragmatism**: Many upstream features (OSS providers, skills) aren't needed locally
3. **Documentation**: Creating TYPE_MAPPING.md makes divergences explicit and manageable
4. **Reversibility**: If upstream sync becomes important later, Option A is still possible

## Consequences

### Positive

- `codex-tui` (v1) remains stable and unchanged
- Core crates stay lean without compatibility bloat
- Clear documentation of what's different via TYPE_MAPPING.md
- Selective adoption: can port valuable upstream features individually
- Faster initial compilation (fixes are localized to tui2)

### Negative

- tui2 code diverges from upstream source
- Future upstream features require manual port effort
- Must maintain synchronization documentation

### Neutral

- Commit convention adopted: `#local-only` for tui2-specific changes
- TYPE_MAPPING.md becomes authoritative reference for divergences
- UPSTREAM_SYNC.md tracks sync state

## Implementation Plan

1. **Phase 1**: Create documentation (this ADR, TYPE_MAPPING.md, UPSTREAM_SYNC.md)
2. **Phase 2**: Fix import errors (E0432) - remove/stub missing imports
3. **Phase 3**: Fix field access errors (E0609) - remove field accesses
4. **Phase 4**: Fix method errors (E0599) - stub or remove method calls
5. **Phase 5**: Fix remaining type mismatches (E0308, E0063, etc.)
6. **Phase 6**: Verify tui2 builds and runs

## Error Categories Being Fixed

| Error Code | Count | Fix Strategy |
|------------|-------|--------------|
| E0432 | ~40 | Remove unresolved imports |
| E0609 | ~45 | Remove/stub field access |
| E0599 | ~50 | Remove method calls or stub |
| E0433 | ~16 | Fix module paths |
| E0308 | ~35 | Adjust types |
| E0061 | ~10 | Adjust function arguments |
| E0560 | ~12 | Remove struct fields |
| E0063 | ~9 | Add default fields |
| Other | ~45 | Case-by-case |

## Links

- [UPSTREAM_SYNC.md](../../UPSTREAM_SYNC.md) - Sync state tracking
- [TYPE_MAPPING.md](../upstream/TYPE_MAPPING.md) - Detailed type mappings
- [HANDOFF.md](../../HANDOFF.md) - Session continuation context

---

_ADR format based on [adr-tools](https://github.com/npryce/adr-tools)_
