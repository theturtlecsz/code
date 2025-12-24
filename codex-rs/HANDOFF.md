# SYNC-028 Session 7 Handoff - Migration Documentation & tui2 Reconciliation

**Date**: 2024-12-24
**Session**: 7 of SYNC-028
**Last Commit**: a2637d802 (JsonSchema derives for app-server-protocol compat)

---

## Session 6 Summary (Completed)

### Committed Work (a2637d802)

1. **JsonSchema derives** added to 60+ protocol types
2. **mcp-types JsonSchema** - all 100+ types now have JsonSchema derive
3. **app-server-protocol conversions** - fixed From impls, type mappings
4. **backend-client fixes** - rate limit mappings, type conversions
5. **codex-tui EventMsg** - added new variant handlers

### Build Status After Session 6

| Crate | Status |
|-------|--------|
| codex-protocol | BUILDS |
| codex-core | BUILDS |
| codex-tui (original) | BUILDS |
| codex-app-server-protocol | BUILDS |
| codex-backend-client | BUILDS |
| codex-tui2 | **262 ERRORS** (API mismatch) |

### User Decisions (Session 6)

1. **tui2 integration**: **Option B** - Modify tui2 to use local APIs
2. **Migration docs**: Create documentation before continuing tui2 work

---

## Session 7 Tasks

### Phase 1: Create Migration Documentation (FIRST)

Before fixing tui2, create documentation infrastructure for tracking upstream divergences.

#### 1.1 Create `UPSTREAM_SYNC.md`
Living document tracking sync state:
```markdown
- Current upstream sync point (commit hash, date)
- Divergence inventory (categorized)
- Sync checklist for future updates
- Commit tagging convention (#upstream-fix, #local-only)
```

#### 1.2 Create `docs/upstream/TYPE_MAPPING.md`
API divergence matrix:
```markdown
| Local Type | Upstream Type | Divergence | Strategy |
|------------|---------------|------------|----------|
| SandboxPolicy | SandboxPolicy | Missing ExternalSandbox | Map to WorkspaceWrite |
| RateLimitSnapshot | RateLimitSnapshot | Missing credits, plan_type | Skip fields |
| ... | ... | ... | ... |
```

#### 1.3 Create `docs/adr/ADR-001-tui2-local-api-adaptation.md`
Architecture Decision Record for Option B:
```markdown
# ADR-001: Adapt tui2 to Local APIs

## Status: Accepted
## Context: tui2 ported from upstream expects APIs not in local crates
## Decision: Modify tui2 to use local APIs (Option B)
## Consequences: Diverges from upstream, but preserves local stability
```

### Phase 2: Fix tui2 Errors (Option B)

After documentation is in place, systematically fix tui2:

#### Error Categories (262 total)
```
E0609 (63): No field on type - stub or remove field access
E0432 (57): Unresolved import - remove or stub imports
E0599 (54): No method on type - implement locally or remove
E0308 (35): Type mismatches - adjust types
E0433 (16): Failed to resolve - fix module paths
E0061 (9):  Wrong argument count - adjust calls
E0560 (8):  Struct has no field - remove field
E0063 (5):  Missing fields - add defaults
```

#### Key Missing Imports to Stub/Remove
```rust
codex_common::oss                              // OSS-specific features
codex_core::INTERACTIVE_SESSION_SOURCES        // Constant
codex_core::auth::enforce_login_restrictions   // Auth function
codex_core::config::resolve_oss_provider       // Config function
codex_core::config::edit                       // Config module
codex_core::terminal::terminal_info            // Terminal utils
codex_core::features                           // Feature flags
codex_protocol::custom_prompts::PROMPTS_CMD_PREFIX
```

---

## Continue Prompt for Session 7

```
Continue SYNC-028 (TUI v2 port) **ultrathink** - Migration Docs & tui2 Fix

## Context
Session 6 completed and committed (a2637d802). User chose:
- Option B: Modify tui2 to use local APIs
- Create migration documentation FIRST before fixing tui2

## Phase 1: Migration Documentation (DO FIRST)

Create three documents to track upstream divergences:

1. UPSTREAM_SYNC.md (root level)
   - Current sync point
   - Divergence inventory
   - Sync checklist
   - Commit conventions

2. docs/upstream/TYPE_MAPPING.md
   - Local ↔ Upstream type mapping
   - Field differences
   - Compatibility strategies

3. docs/adr/ADR-001-tui2-local-api-adaptation.md
   - Document Option B decision
   - Context, consequences, alternatives considered

## Phase 2: Fix tui2 (262 errors)

After docs are created, fix tui2 systematically:
1. Fix E0432 import errors (57) - remove/stub missing imports
2. Fix E0609 field access errors (63) - stub or remove
3. Fix E0599 method errors (54) - implement or remove
4. Fix remaining errors

## Build Commands
```bash
cargo check -p codex-tui      # Verify original still works
cargo check -p codex-tui2 2>&1 | grep "^error\[E" | wc -l  # Track progress
```

## Success Criteria
- [ ] UPSTREAM_SYNC.md created
- [ ] TYPE_MAPPING.md created
- [ ] ADR-001 created
- [ ] tui2 error count reduced (target: <50)
- [ ] Commit documentation + tui2 fixes
```

---

## Research References (Session 6)

Migration documentation approach based on:
- [Fork maintenance best practices](https://gruchalski.com/posts/2024-03-03-maintaining-a-fork-of-a-repository/)
- [ADR tools](https://github.com/npryce/adr-tools) for decision records
- [Schema evolution patterns](https://martinfowler.com/articles/evodb.html)
- [Upstream sync conventions](https://joaquimrocha.com/2024/09/22/how-to-fork/)

Key patterns to implement:
- Commit prefixes: `#upstream-fix`, `#local-only`
- Version tags: `v1.2.3-local.1` for local patches
- Compatibility modes: BACKWARD, FORWARD, FULL

---

## tui2 Error Details

### Missing Modules (E0432)
```
codex_common::oss
codex_core::INTERACTIVE_SESSION_SOURCES
codex_core::auth::enforce_login_restrictions
codex_core::config::resolve_oss_provider
codex_core::config::edit
codex_core::terminal::terminal_info
codex_core::features (3 occurrences)
codex_core::protocol::ElicitationAction
codex_core::protocol::ExecPolicyAmendment
codex_protocol::custom_prompts::PROMPTS_CMD_PREFIX (3 occurrences)
codex_core::config::ConstraintResult
codex_core::config::types
codex_core::project_doc::DEFAULT_PROJECT_DOC_FILENAME
codex_core::protocol::DeprecationNoticeEvent
```

### Missing Types (E0412/E0422)
```
AppExitInfo
ApprovedExecpolicyAmendment
```

### Missing Functions (E0425)
```
parse_turn_item (codex_core)
```

### Struct Field Mismatches (E0026/E0027)
```
SessionConfiguredEvent.reasoning_effort
UpdatePlanArgs.explanation
FileChange::Delete.content
Event.event_seq, Event.order
```
