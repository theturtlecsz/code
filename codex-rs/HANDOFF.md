# SYNC-028 Session 8 Handoff - Continue tui2 Error Fixes

**Date**: 2024-12-24
**Session**: 8 of SYNC-028
**Last Commit**: c830d0778 (migration docs + error reduction 262→173)

---

## Session 7 Summary (Completed)

### Committed Work (c830d0778)

1. **Migration Documentation** (Phase 1 complete)
   - `UPSTREAM_SYNC.md` - sync state tracking, commit conventions
   - `docs/upstream/TYPE_MAPPING.md` - comprehensive type mapping matrix
   - `docs/adr/ADR-001-tui2-local-api-adaptation.md` - Option B decision record

2. **Compatibility Layer** (`tui2/src/compat.rs`)
   - Stub modules: oss, terminal, features, auth, config, skills
   - Protocol stubs: RateLimitSnapshot, ExecCommandSource, ElicitationAction
   - Extension traits: ConfigExt, SandboxPolicyExt, protocol event extensions
   - Constants: INTERACTIVE_SESSION_SOURCES, PROMPTS_CMD_PREFIX, etc.

3. **Error Reduction**: 262 → 173 (34% reduction)
   - E0432 (imports): 57 → 0 ✓
   - E0609/E0599 (fields/methods): Major structural fixes
   - E0308 (type mismatches): 85 remaining (main category)

### Build Status After Session 7

| Crate | Status |
|-------|--------|
| codex-protocol | BUILDS |
| codex-core | BUILDS |
| codex-tui (original) | BUILDS |
| codex-app-server-protocol | BUILDS |
| codex-backend-client | BUILDS |
| codex-tui2 | **173 ERRORS** (down from 262) |

---

## Session 8 Tasks

### Priority: Fix Remaining 173 Errors

The remaining errors are categorized below. Focus on type mismatches first.

#### Error Breakdown (173 total)

```
85 E0308 - mismatched types (PRIORITY)
 3 E0609 - no field (model, description on String types)
 3 E0061 - wrong argument count
 2 E0599 - missing methods (unwrap_or_else, etc.)
 2 E0277 - trait bound not satisfied
 2 E0026 - pattern field issues
 1+ each - various other errors
```

#### Top Priority Fixes

1. **E0308 Type Mismatches (85)**
   - Most are from compat stub return types not matching expected types
   - Analyze each location and fix return type or add conversion

2. **Missing Fields on String Types (3)**
   - `no field 'model' on type '&&String'`
   - `no field 'description' on type '&String'`
   - These suggest wrong type is being passed - trace back to source

3. **Op/EventMsg Missing Variants**
   - RunUserShellCommand, ResolveElicitation, ListMcpTools
   - McpStartupUpdate, McpStartupComplete
   - Either remove code using these or add proper stubs

4. **SkillMetadata Fields**
   - Missing: short_description, scope, path
   - Add to compat::skills::SkillMetadata

5. **ConfigEditsBuilder Methods**
   - Missing: set_hide_world_writable_warning, set_hide_rate_limit_model_nudge, record_model_migration_seen
   - Add to compat module

6. **FileChange::Delete Pattern**
   - Expects `content` field that doesn't exist
   - Fix pattern to match actual struct

---

## Continue Prompt for Session 8

```
Continue SYNC-028 (TUI v2 port) **ultrathink** - Fix remaining tui2 errors

## Context
Session 7 committed (c830d0778). Migration docs complete. Error count: 173.

## Current State
- compat.rs has extension traits and stubs
- Most import errors fixed
- Remaining: 85 type mismatches + 88 other errors

## Priority Tasks

1. Fix E0308 type mismatches (85 errors)
   - Run: `cargo check -p codex-tui2 2>&1 | grep "E0308" | head -20`
   - Analyze each and fix return types or add conversions

2. Fix missing struct fields
   - SkillMetadata: add short_description, scope, path
   - ConfigEditsBuilder: add missing methods

3. Fix Op/EventMsg variant issues
   - Remove or properly handle code using missing variants

4. Verify original tui still builds
   - Run: `cargo check -p codex-tui`

## Build Commands
```bash
cargo check -p codex-tui2 2>&1 | grep "^error\[E" | wc -l  # Track progress
cargo check -p codex-tui2 2>&1 | grep "E0308" | head -30   # Type mismatches
cargo check -p codex-tui                                    # Verify original
```

## Success Criteria
- [ ] tui2 error count < 50
- [ ] Original codex-tui still builds
- [ ] Commit progress
```

---

## Files Modified This Session

| File | Changes |
|------|---------|
| `UPSTREAM_SYNC.md` | NEW - sync state tracking |
| `docs/upstream/TYPE_MAPPING.md` | NEW - type mapping matrix |
| `docs/adr/ADR-001-tui2-local-api-adaptation.md` | NEW - decision record |
| `tui2/src/compat.rs` | NEW - compatibility layer (600+ lines) |
| `tui2/src/*.rs` | Modified imports, stubbed code |
| `tui2/src/**/*.rs` | 30+ files with import fixes |

---

## Key Compat Module Contents

```rust
// Constants
INTERACTIVE_SESSION_SOURCES, PROMPTS_CMD_PREFIX, DEFAULT_PROJECT_DOC_FILENAME
OLLAMA_OSS_PROVIDER_ID, LMSTUDIO_OSS_PROVIDER_ID

// Stub Modules
compat::oss, compat::terminal, compat::features, compat::auth
compat::config, compat::skills, compat::protocol

// Extension Traits
ConfigExt, SandboxPolicyExt, ExecCommandBeginEventExt, ExecCommandEndEventExt
SessionConfiguredEventExt, ExecApprovalRequestEventExt

// Protocol Types
RateLimitSnapshot, ExecCommandSource, ElicitationAction, ExecPolicyAmendment
TurnAbortReason, StreamErrorEvent, McpStartupStatus, etc.
```

---

_Last updated: 2024-12-24 (SYNC-028 Session 7 complete)_
