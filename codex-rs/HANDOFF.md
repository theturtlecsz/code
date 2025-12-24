# SYNC-028 Session 5 Handoff - TUI v2 Port (JsonSchema Phase)

**Date**: 2024-12-24
**Session**: 5 of SYNC-028
**Commit Before**: 65ae1d449 (tui2 + dependencies ported)

## Session Summary

This session focused on porting missing protocol files and adding JsonSchema derives to enable `app-server-protocol` schema generation compatibility.

### Completed Work

1. **Ported 6 protocol files** from upstream:
   - `account.rs` (PlanType enum)
   - `approvals.rs` (ExecApprovalRequestEvent, ElicitationRequestEvent, etc.)
   - `user_input.rs` (UserInput enum)
   - `items.rs` (TurnItem, UserMessageItem, AgentMessageItem, etc.)
   - `openai_models.rs` (ReasoningEffort, ModelPreset, ModelInfo, etc.)
   - Updated `lib.rs` to export new modules and re-export ConversationId

2. **Added missing types to codex_protocol/src/protocol.rs**:
   - NetworkAccess, CreditsSnapshot, SessionSource, SubAgentSource
   - ReviewDelivery, McpAuthStatus, SkillScope
   - SkillMetadata, SkillErrorInfo, CodexErrorInfo

3. **Added missing types to codex_core/src/protocol.rs**:
   - Same types as above plus UndoCompletedEvent, UndoStartedEvent
   - ListSkillsResponseEvent, SkillsListEntry
   - Added UndoCompleted, UndoStarted, ListSkillsResponse to EventMsg enum

4. **Added JsonSchema derives** to many types:
   - config_types.rs: ReasoningEffort, ReasoningSummary, Verbosity, SandboxMode
   - mcp_protocol.rs: ConversationId
   - parse_command.rs: ParsedCommand
   - plan_tool.rs: StepStatus, PlanItemArg, UpdatePlanArgs
   - items.rs: TurnItem and inner types
   - protocol.rs: Bulk update of ~50+ types

5. **Updated workspace Cargo.toml**:
   - Added `uuid1` feature to schemars for Uuid JsonSchema support

### Remaining Errors (15 types missing JsonSchema)

```
ApprovedCommandMatchKind       - needs JsonSchema
As<Base64>                     - serde_with type, needs special handling
CodexErrorInfo                 - already added, may need re-check
CustomPrompt                   - in custom_prompts.rs
HistoryEntry                   - in message_history.rs
mcp_types::CallToolResult      - external crate, use #[schemars(skip)]
ResponseItem                   - in models.rs (2 occurrences)
ReviewContextMetadata          - in protocol.rs
ReviewDecision                 - in protocol.rs (2 occurrences)
SandboxPolicy                  - in protocol.rs (3 occurrences)
Tool (mcp_types)               - external crate, use #[schemars(skip)]
```

### Files Modified (uncommitted)

- `codex-rs/Cargo.toml` - workspace schemars uuid1 feature
- `codex-rs/protocol/Cargo.toml` - added schemars dependency
- `codex-rs/protocol/src/lib.rs` - new module exports
- `codex-rs/protocol/src/account.rs` - NEW
- `codex-rs/protocol/src/approvals.rs` - NEW
- `codex-rs/protocol/src/user_input.rs` - NEW
- `codex-rs/protocol/src/items.rs` - NEW
- `codex-rs/protocol/src/openai_models.rs` - NEW
- `codex-rs/protocol/src/config_types.rs` - added JsonSchema
- `codex-rs/protocol/src/mcp_protocol.rs` - added JsonSchema to ConversationId
- `codex-rs/protocol/src/parse_command.rs` - added JsonSchema
- `codex-rs/protocol/src/plan_tool.rs` - added JsonSchema
- `codex-rs/protocol/src/protocol.rs` - added types + JsonSchema
- `codex-rs/core/src/protocol.rs` - added types + EventMsg variants

---

## Continue Prompt for Next Session

```
Continue SYNC-028 (TUI v2 port) **ultrathink** - JsonSchema Completion

## Context
Session 5 ported protocol files and added JsonSchema to ~50+ types.
15 types still need JsonSchema or #[schemars(skip)] annotations.

## Remaining Tasks (in order)

1. Fix remaining JsonSchema errors (15 types):
   a. Add JsonSchema to:
      - ApprovedCommandMatchKind (protocol.rs)
      - ReviewContextMetadata (protocol.rs)
      - ReviewDecision (protocol.rs)
      - SandboxPolicy (protocol.rs)
      - CustomPrompt (custom_prompts.rs)
      - HistoryEntry (message_history.rs)
      - ResponseItem (models.rs)

   b. Add #[schemars(skip)] for external types:
      - mcp_types::CallToolResult
      - mcp_types::Tool
      - serde_with::As<Base64>

2. Verify codex-protocol builds: cargo check -p codex-protocol

3. Build codex-tui2: cargo check -p codex-tui2

4. Fix any remaining compile errors in tui2 or app-server-protocol

5. Run tui2 tests: cargo test -p codex-tui2

6. Add CLI --tui2 flag for opt-in launch

7. Add features.tui2 config option

8. Create UPSTREAM_SYNC.md tracking document

9. Commit all changes with message:
   "feat(protocol): add JsonSchema derives for tui2 compat (SYNC-028)"

## Key Files to Focus On
- protocol/src/protocol.rs (lines 243-294 for SandboxPolicy, ReviewDecision)
- protocol/src/custom_prompts.rs
- protocol/src/message_history.rs
- protocol/src/models.rs

## For schemars(skip) Pattern
```rust
#[serde(skip_serializing_if = "Option::is_none")]
#[schemars(skip)]
pub external_field: Option<ExternalType>,
```

## Build Commands
```bash
cargo check -p codex-protocol
cargo check -p codex-tui2
cargo test -p codex-tui2
```

## Success Criteria
- [ ] cargo build -p codex-protocol succeeds
- [ ] cargo build -p codex-tui2 succeeds
- [ ] cargo test -p codex-tui2 passes (some failures OK)
- [ ] codex-tui2 binary runs
- [ ] Existing codex-tui still works
```

---

## User Decisions (Session 5)

1. **External types approach**: Add JsonSchema to mcp-types crate (complete schema)
2. **Feature flag**: CLI flag only (`--tui2`) - simple, matches upstream
3. **Test scope**: tui2 tests only - focused verification
