# SYNC-028 Session 6 Handoff - TUI v2 Port (API Reconciliation Phase)

**Date**: 2024-12-24
**Session**: 6 of SYNC-028
**Commit Before**: 65ae1d449 (tui2 + dependencies ported)

## Session 6 Summary

This session completed the JsonSchema phase and discovered significant API mismatches between tui2 (ported from upstream) and the local protocol/core crates.

### Completed Work

1. **JsonSchema derives added to all remaining types**:
   - `ApprovedCommandMatchKind`, `SandboxPolicy`, `CodexErrorInfo`, `ReviewContextMetadata`, `ReviewDecision` (protocol.rs)
   - `CustomPrompt` (custom_prompts.rs)
   - `HistoryEntry` (message_history.rs)
   - `ResponseItem` and related types (models.rs)
   - `FunctionCallOutputPayload` (models.rs)
   - `#[schemars(with = "String")]` for serde_with Base64 field

2. **Added JsonSchema to mcp-types crate** (per user preference):
   - Added schemars dependency to mcp-types/Cargo.toml
   - All 100+ types now have JsonSchema derive

3. **Fixed app-server-protocol conversions**:
   - ParsedCommand::Read now has `path: Option<String>`
   - RateLimitSnapshot/RateLimitWindow conversions fixed
   - TokenUsage i64/u64 conversions added
   - SandboxPolicy::ExternalSandbox mapped to WorkspaceWrite
   - AbsolutePathBuf/PathBuf conversions added
   - EventMsg match exhaustiveness fixed

4. **Fixed backend-client**:
   - `get_codex_user_agent(None)` argument fix
   - RateLimitSnapshot field removals (credits, plan_type)
   - RateLimitWindow field rename (resets_at -> resets_in_seconds)
   - Type conversions (i32 -> u64)

5. **Fixed codex-tui (original)**:
   - Added EventMsg handlers for UndoStarted, UndoCompleted, ListSkillsResponse

### Build Status

| Crate | Status |
|-------|--------|
| codex-protocol | BUILDS |
| codex-core | BUILDS |
| codex-tui (original) | BUILDS |
| codex-app-server-protocol | BUILDS (1 warning) |
| codex-backend-client | BUILDS |
| codex-tui2 | **262 ERRORS** |

### tui2 Error Analysis

The tui2 crate has 262 compile errors due to API mismatches with local crates. Key issues:

```
E0026: Struct fields exist in upstream but not locally:
  - SessionConfiguredEvent.reasoning_effort
  - UpdatePlanArgs.explanation
  - FileChange::Delete.content

E0027: Pattern missing fields:
  - Event.event_seq, Event.order

E0412/E0422: Missing types:
  - AppExitInfo
  - ApprovedExecpolicyAmendment

E0425: Missing functions:
  - parse_turn_item

E0308: Type mismatches (user_facing_hint: String vs Option)
```

### Root Cause

The tui2 crate was ported from a different upstream version that has evolved independently from our local protocol/core crates. The upstream has additional fields, types, and different APIs.

### Options for Next Session

**Option A: Update local crates to match upstream (RECOMMENDED)**
- Add missing fields to SessionConfiguredEvent, Event, FileChange, etc.
- Add missing types (AppExitInfo, ApprovedExecpolicyAmendment)
- Add missing functions (parse_turn_item)
- Pros: Closer alignment with upstream, easier future syncs
- Cons: More invasive changes to working code

**Option B: Modify tui2 to use local APIs**
- Adjust tui2 code to work with existing local types
- May require removing features that depend on missing APIs
- Pros: Less risk to working code
- Cons: Diverges from upstream, harder future syncs

**Option C: Defer tui2 integration**
- Keep tui2 as reference but don't build it yet
- Focus on stabilizing current TUI
- Pros: Lowest risk
- Cons: Delays new TUI features

---

## Files Modified (uncommitted, this session)

### Protocol/Core changes:
- `codex-rs/mcp-types/Cargo.toml` - added schemars
- `codex-rs/mcp-types/src/lib.rs` - added JsonSchema to all types
- `codex-rs/protocol/src/protocol.rs` - added JsonSchema to 5 types
- `codex-rs/protocol/src/custom_prompts.rs` - added JsonSchema
- `codex-rs/protocol/src/message_history.rs` - added JsonSchema
- `codex-rs/protocol/src/models.rs` - added JsonSchema to 10+ types
- `codex-rs/protocol/src/parse_command.rs` - added path field to Read

### Conversion fixes:
- `codex-rs/app-server-protocol/src/protocol/v2.rs` - fixed From impls
- `codex-rs/app-server-protocol/src/protocol/thread_history.rs` - removed UndoCompleted
- `codex-rs/backend-client/src/client.rs` - fixed rate limit mappings
- `codex-rs/core/src/parse_command.rs` - added path: None to Read
- `codex-rs/tui/src/chatwidget/mod.rs` - added new EventMsg variants

---

## Continue Prompt for Next Session

```
Continue SYNC-028 (TUI v2 port) **ultrathink** - API Reconciliation

## Context
Session 6 completed JsonSchema phase. codex-protocol, codex-core, and codex-tui
all build. codex-tui2 has 262 errors due to API mismatches with local crates.

## Decision Required
Choose approach for tui2 integration:
A) Update local crates to match upstream (recommended)
B) Modify tui2 to use local APIs
C) Defer tui2 integration

## If Option A:
1. Add missing fields to protocol types:
   - SessionConfiguredEvent.reasoning_effort
   - Event.event_seq, Event.order
   - FileChange::Delete.content
   - UpdatePlanArgs.explanation

2. Add missing types:
   - AppExitInfo
   - ApprovedExecpolicyAmendment
   - parse_turn_item function

3. Fix type mismatches (user_facing_hint: String vs Option)

4. Build and test tui2

## Build Commands
```bash
# Verify existing builds still work
cargo check -p codex-protocol
cargo check -p codex-tui

# Check tui2 error count
cargo check -p codex-tui2 2>&1 | grep "^error\[E" | wc -l

# Full workspace build
cargo build --workspace
```

## Success Criteria
- [ ] codex-tui (original) still builds
- [ ] codex-tui2 builds (or decision made to defer)
- [ ] Tests pass
- [ ] Changes committed
```

---

## User Decisions (Session 5-6)

1. **External types approach**: Add JsonSchema to mcp-types crate (complete schema) - DONE
2. **Feature flag**: CLI flag only (`--tui2`) - simple, matches upstream - PENDING
3. **Test scope**: tui2 tests only - focused verification - PENDING
4. **tui2 integration**: Decision needed (A/B/C above)
