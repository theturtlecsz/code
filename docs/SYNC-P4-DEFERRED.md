# Deferred Sync Items (P4+)

**Created**: 2025-11-29
**Last Updated**: 2025-11-29 (P4 session)

---

## Overview

Items identified during P3 sync triage that require significant effort or have dependencies. These are tracked but not scheduled for immediate work.

---

## SYNC-009: Footer Module (COMPLETED)

**Status**: ✅ COMPLETE
**Session**: P4 (2025-11-29)
**Effort**: ~2 hours

### What Was Done
1. Wired `FooterMode` into `ChatComposer` struct
2. Added "?" key handler to toggle shortcut overlay (when composer empty)
3. Integrated `render_footer()` for `ShortcutOverlay` mode
4. Added 3 insta snapshot tests for footer rendering
5. Removed `#[allow(dead_code)]` from wired components

### Integration Approach
- **Hybrid design**: Fork's inline footer for status display, upstream footer module for shortcut overlay
- "?" toggles between `ShortcutSummary` and `ShortcutOverlay` modes
- Multi-line overlay shows keyboard shortcuts in 2-column layout

### Files Changed
- `tui/src/bottom_pane/chat_composer.rs`: +footer import, +FooterMode field, +"?" handler, +render integration
- `tui/src/bottom_pane/footer.rs`: Updated docs, added allow(dead_code) for future modes
- `tui/src/bottom_pane/snapshots/`: 3 new snapshot files

---

## SYNC-010: Auto Drive Patterns

**Status**: DEFERRED
**Effort**: 10-20 hours
**Reason**: Significant architectural refactor required

### What Upstream Has
- `tools/orchestrator.rs`: ToolOrchestrator for centralized tool execution
- `tools/sandboxing.rs`: SandboxRetryData, ProvidesSandboxRetryData trait
- `tools/runtimes/`: Shell, ApplyPatch, UnifiedExec implement ToolRuntime trait
- Flow: approval → sandbox selection → attempt → automatic retry without sandbox

### What Fork Has
- Flat structure with inline tool execution
- `with_escalated_permissions` parameter in tool calls
- `command_safety/` directory for safety checks
- No automatic retry mechanism

### Gap
- Fork requires explicit escalated_permissions upfront
- Upstream auto-retries failed sandbox commands with user approval
- Different approval flow architecture

### Port Approach (If Needed)
1. Create `tools/` directory structure
2. Port SandboxRetryData and ProvidesSandboxRetryData trait
3. Refactor tool execution to use ToolOrchestrator
4. Update approval flow to support retry semantics

### Decision Criteria
Port if:
- Users report friction with escalated permissions workflow
- Upstream divergence becomes blocking for other features
- Security audit recommends centralized tool orchestration

---

## SYNC-016: Device Code Auth

**Status**: READY (was BLOCKED)
**Effort**: 2-3 hours (revised down from 3-5h)
**Blocker**: ~~codex_core::auth module sync required~~ RESOLVED
**Updated**: 2025-11-29 (P6 session)

### What Upstream Has
- `login/src/device_code_auth.rs` (359 LOC, 11KB)
- User code request/display flow for headless environments
- Token polling with 15-minute timeout
- Integration with PKCE and token exchange
- Cloudflare challenge fallback via browser automation

### Actual Missing Dependencies (Corrected)

**Originally Listed (4 of 5 DON'T EXIST in upstream):**
1. ~~`AuthCredentialsStoreMode` enum~~ - NOT IN UPSTREAM
2. ~~`save_auth` helper function~~ - NOT IN UPSTREAM
3. ~~`cli_auth_credentials_store_mode` field~~ - NOT IN UPSTREAM
4. ~~`ensure_workspace_allowed` function~~ - NOT IN UPSTREAM
5. `CODEX_API_KEY_ENV_VAR` constant - **CONFIRMED MISSING** (only valid blocker)

**Actual Requirements:**
1. `CODEX_API_KEY_ENV_VAR` constant (~2 lines)
2. `read_codex_api_key_from_env()` function (~6 lines)
3. `RefreshTokenError` + `RefreshTokenErrorKind` types (~40 lines)
4. `classify_refresh_failure()` helper (~50 lines)
5. `adopt_rotated_refresh_token_from_disk()` method (~20 lines)
6. `device_code_auth.rs` module (~180 lines after substitution)

### Migration Path
See `docs/AUTH-MODULE-DIFF-REPORT.md` for detailed plan.

**Phase 1**: Core auth enhancements (~150 LOC, low risk)
**Phase 2**: Device code auth port (~180 LOC, medium risk)
**Phase 3**: CLI integration (optional, low risk)

### Pre-Port Verification
- [ ] `codex_browser` crate has `global::get_or_create_browser_manager()`
- [ ] `ServerOptions` struct is compatible
- [ ] `persist_tokens_async()` is exported from server.rs

### Use Cases
- SSH environments without browser access
- Headless servers
- CI/CD pipelines needing auth

---

## Items Confirmed NOT NEEDED

### SYNC-013: Shell MCP Server
**Status**: NOT NEEDED - Fork more complete
- Fork message_processor.rs: 55KB
- Upstream message_processor.rs: 25KB
- Fork has additional: acp_tool_runner, codex_message_processor, conversation_loop

### SYNC-017: Review/Merge Workflows
**Status**: NOT NEEDED - Fork significantly ahead
- Fork chatwidget: 23,036 lines
- Upstream chatwidget: 3,268 lines
- Fork has full /merge command with worktree support

---

## Tracking

| ID | Item | Status | Effort | Blocker |
|----|------|--------|--------|---------|
| SYNC-009 | Footer Module | ✅ Complete | ~2h | - |
| SYNC-010 | Auto Drive Patterns | Deferred | 10-20h | Architectural |
| SYNC-016 | Device Code Auth | ✅ Ready | 2-3h | ~~Auth module sync~~ RESOLVED |
| SYNC-013 | Shell MCP Server | Not Needed | - | Fork ahead |
| SYNC-017 | Review/Merge | Not Needed | - | Fork ahead |
