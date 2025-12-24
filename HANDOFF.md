# Session Handoff — SYNC-028 TUI v2 Port

**Last updated:** 2025-12-24
**Status:** SYNC-028 In Progress - Error Reduction Phase
**Priority:** SYNC-028 (TUI v2) → SPEC-KIT-926 (Progress Visibility)

---

## Session Summary (2025-12-24 - Session 8)

### Error Reduction Progress

| Metric | Session Start | Session End | Change |
|--------|---------------|-------------|--------|
| Total errors | 173 | 149 | -24 (-14%) |
| E0599 (methods) | 17 | 2 | -15 |
| E0308 (types) | 88 | 88 | 0 |

### Fixes Applied This Session

| Category | Fix | Files |
|----------|-----|-------|
| Import errors | Fixed review_prompts, ApprovedExecpolicyAmendment, AuthMode imports | chatwidget.rs, history_cell.rs, lib.rs, onboarding/auth.rs |
| ConfigEditsBuilder | Added missing methods: set_hide_world_writable_warning, set_hide_rate_limit_model_nudge, record_model_migration_seen | compat.rs |
| McpStartupCompleteEvent | Fixed stub to use Vec<FailedMcpServer> and Vec<String> instead of wrong types | compat.rs |
| UndoStartedEvent | Adapted code for String instead of Option<String> | chatwidget.rs |
| Missing EventMsg variants | Commented out McpStartupUpdate, McpStartupComplete handlers | chatwidget.rs |
| Missing Op variants | Stubbed RunUserShellCommand, ListMcpTools as unsupported | chatwidget.rs |
| Elicitation/ExecPolicy | Stubbed out non-existent features | approval_overlay.rs |
| SandboxPolicy | Direct enum assignment instead of .set() wrapper | app.rs, card.rs |
| AskForApproval | Direct enum access instead of .value()/.set() wrappers | chatwidget.rs |
| ModelFamily | Constructed explicit struct instead of ::default() | compat.rs |
| ErrorEvent | Constructed directly instead of to_error_event() | agent.rs |
| ReviewRequest | Handle user_facing_hint as String not Option<String> | chatwidget.rs |
| upload_feedback | Stubbed as unsupported feature | feedback_view.rs |
| get_account_email | Changed to get_account_id | helpers.rs |
| ExternalSandbox | Removed non-existent variant from match | card.rs |

### Remaining Work (149 errors)

**Major Categories:**

| Error Type | Count | Description |
|------------|-------|-------------|
| E0308 | 88 | Type mismatches - bulk of remaining work |
| E0609 | 15 | Missing struct fields |
| E0061 | 10 | Function argument count mismatches |
| E0599 | 2 | Missing methods (ok(), display()) |
| Other | 34 | Misc (patterns, borrow, generics) |

---

## Next Session: Continue SYNC-028

### Continuation Prompt

```
Continue SYNC-028 (TUI v2 port) Session 9 **ultrathink**

## Context
Session 8 reduced errors from 173 to 149 (24 fixed).
Focus was on E0599 (method/variant) errors and import issues.

## Remaining Work (in priority order)
1. E0308 type mismatches (88): These need individual analysis
   - Many are Option<T> vs T differences between upstream/fork
   - Some are struct field type differences

2. E0609 missing fields (15): Add fields to compat stubs
   - model, description, display_name on String types (code expects struct)
   - context_window on ModelFamily
   - Various MCP config fields

3. E0061 arg count mismatches (10): Adjust function signatures

4. Other misc errors (34): Pattern matching, borrows, generics

## Strategy Recommendation
Focus on the 88 E0308 errors - they're the bulk. Sample a few, identify patterns,
then apply batch fixes. Many likely share root causes.

## Files Most Affected
- tui2/src/chatwidget.rs (main widget, most errors)
- tui2/src/compat.rs (compatibility layer)
- tui2/src/bottom_pane/*.rs (approval/feedback)
- tui2/src/status/*.rs (status display)

## Success Criteria
- [ ] cargo check -p codex-tui2 succeeds with only warnings
- [ ] cargo build -p codex-tui2 succeeds
- [ ] codex-tui2 binary runs
- [ ] Existing codex-tui still works
```

---

## Architecture Decision Record

### Question: Adopt Two-Workspace Model?

**Decision:** No

**Rationale:**
1. Fork has diverged significantly (14K+ LOC custom crates, 150K+ TUI)
2. MAINT-11 refactoring already achieved good isolation (<100 LOC conflict surface)
3. Migration cost (2-4 weeks) outweighs benefits
4. Every Code's model is for tracking OpenAI Codex; our effective upstream is Every Code

**Alternative adopted:** Direct port of tui2 into existing workspace.

See: `~/.claude/plans/deep-gathering-cosmos.md` for full analysis.

---

## Upstream Relationship

```
openai/codex (archived)
    ↓
just-every/code ("Every Code" - active, has tui2)
    ↓
~/code (your fork - "Planner" with Spec-Kit)
```

**Key insight:** Your fork is essentially a distinct product, not just customizations.

---

## Key API Divergences Discovered

### Wrapper Types (Upstream) vs Direct Enums (Fork)

| Field | Upstream | Fork |
|-------|----------|------|
| `config.sandbox_policy` | Wrapper with `.get()`/`.set()` | Direct `SandboxPolicy` enum |
| `config.approval_policy` | Wrapper with `.value()`/`.set()` | Direct `AskForApproval` enum |

### Option vs Direct Types

| Field | Upstream | Fork |
|-------|----------|------|
| `ReviewRequest.user_facing_hint` | `Option<String>` | `String` |
| `UndoStartedEvent.message` | `Option<String>` | `String` |
| `McpStartupCompleteEvent.cancelled` | `Vec<String>` | `bool` |

### Missing Features (Not in Fork)

- `Op::ResolveElicitation` (elicitation feature)
- `Op::RunUserShellCommand` (user shell)
- `Op::ListMcpTools` (MCP tool listing)
- `EventMsg::McpStartupUpdate`
- `EventMsg::McpStartupComplete`
- `ReviewDecision::ApprovedExecpolicyAmendment` (execpolicy)
- `SandboxPolicy::ExternalSandbox` (external sandbox)
- `CodexLogSnapshot::upload_feedback()` (feedback upload)
- `CodexErr::to_error_event()` (error conversion)
- `CodexAuth::get_account_email()` (use get_account_id)

---

## Files Reference

| File | Purpose |
|------|---------|
| `~/.claude/plans/deep-gathering-cosmos.md` | Architecture analysis + implementation plan |
| `docs/adr/ADR-001-tui2-local-api-adaptation.md` | ADR for compat layer approach |
| `docs/upstream/TYPE_MAPPING.md` | Type mapping matrix |
| `UPSTREAM_SYNC.md` | Sync tracking document |

---

## Rollback Plan

If issues arise:
```bash
cd ~/code/codex-rs
rm -rf utils/absolute-path codex-backend-openapi-models app-server-protocol backend-client windows-sandbox-rs tui2
git checkout Cargo.toml core/Cargo.toml protocol/src/lib.rs
```
