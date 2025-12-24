# SPEC-TUI2-STUBS: Stubbed Features in TUI v2 Port

## Overview

This document tracks features that were stubbed out during the TUI v2 port (SYNC-028) due to API divergences between the upstream Codex codebase and the local fork.

**Port Completed**: Session 10 (Compilation)
**Warning Cleanup**: Session 12 (117 → 0 warnings)
**External Crate Cleanup**: Session 13 (2 additional warnings fixed)
**Build Status**: `cargo build -p codex-tui2 --release` succeeds with 0 warnings

---

## Stub Categories

| Category | Intent | Count |
|----------|--------|-------|
| **Intentional** | Not relevant to fork | 8 |
| **Temporary** | Needs future implementation | 6 |
| **Type Adaptation** | Different types between fork/upstream | 5 |

---

## Intentional Stubs (Not Planned for Implementation)

These features are not relevant to the local fork and are intentionally stubbed.

### 1. OSS Provider Integration

**Module**: `compat::oss`

**What**: Ollama, LM Studio, and other local model providers.

**Stubbed Functions**:
- `ensure_oss_provider_ready()` → Returns `Ok(())`
- `get_default_model_for_oss_provider()` → Returns `None`

**Why Not Implemented**: Fork uses upstream API providers only.

### 2. Feature Flags System

**Module**: `compat::features`

**What**: Runtime feature toggles for experimental features.

**Stubbed Functions**:
- `apply_patch_amendment_enabled()` → Returns `false`
- `elicitation_enabled()` → Returns `false`
- `Features::enabled()` → Always returns `false`

**Why Not Implemented**: Fork doesn't need experimental feature gating.

### 3. Terminal Detection

**Module**: `compat::terminal`

**What**: Terminal type detection for optimized rendering.

**Stubbed Functions**:
- `terminal_info()` → Returns `Unknown` terminal

**Why Not Implemented**: Generic terminal handling is sufficient.

### 4. Auth Restrictions

**Module**: `compat::auth`

**What**: Login restrictions and OAuth flow management.

**Stubbed Functions**:
- `enforce_login_restrictions()` → Returns `Ok(())`
- `read_openai_api_key_from_env()` → Reads `OPENAI_API_KEY`

**Why Not Implemented**: Fork uses simple API key auth only.

### 5. Config Persistence Edits

**Module**: `compat::config::edit`

**What**: Runtime config modifications (hiding warnings, setting features).

**Stubbed Functions**:
- `ConfigEditsBuilder` methods → No-ops
- `set_default_oss_provider()` → No-op
- `set_project_trust_level()` → No-op

**Why Not Implemented**: Config is managed externally in fork.

### 6. Review Prompts

**Module**: `compat::review_prompts`

**What**: Built-in code review prompt templates.

**Stubbed Functions**:
- `get_review_prompt()` → Returns `None`
- `user_facing_hint()` → Returns generic text

**Why Not Implemented**: Fork doesn't use built-in review workflows.

### 7. Notices Configuration

**Module**: `compat::NoticesConfig`

**What**: Suppression flags for various UI notices.

**Fields**: All default to `None`/`false`.

**Why Not Implemented**: Fork shows all notices by default.

### 8. Skills Management

**Module**: `compat::skills`

**What**: Skill discovery and metadata.

**Stubbed Functions**:
- `list_skills()` → Returns empty list

**Why Not Implemented**: Fork doesn't use skill system.

---

## Temporary Stubs (Future Implementation Candidates)

These features may need implementation if the functionality becomes important.

### 1. Model Migration Prompts

**Status**: TEMPORARY
**Priority**: Medium
**Blocking**: ReasoningEffort enum mismatch

**Issue**: `ReasoningEffort` has different variants between `codex_core::config_types` and `codex_protocol::openai_models`.

| codex_core | codex_protocol |
|------------|----------------|
| High, Medium, Low, Minimal, None | High, XHigh, Medium, Low, Minimal, None |

**Current Fix**: Conversion functions in `compat.rs`:
- `convert_reasoning_effort()` (Protocol → Core)
- `convert_reasoning_effort_to_protocol()` (Core → Protocol)

**To Implement Properly**: Unify enum definitions or add `XHigh` to core.

### 2. Credits Display

**Status**: TEMPORARY
**Priority**: Low
**Blocking**: Protocol type divergence

**Issue**: Fork's `RateLimitSnapshotEvent` lacks `credits` and `plan_type` fields.

**Current Fix**: `convert_rate_limit_snapshot()` maps available fields.

**To Implement**: Add credits fields to fork's protocol if needed.

### 3. MCP Tools Display

**Status**: TEMPORARY
**Priority**: Low
**Blocking**: Type mismatch

**Issue**: `McpListToolsResponseEvent.tools` is `Vec<String>` but display expects `HashMap<String, Tool>`.

**Current Fix**: `on_list_mcp_tools()` is a no-op.

**To Implement**: Update fork's MCP response types.

### 4. Custom Prompts

**Status**: TEMPORARY
**Priority**: Low
**Blocking**: Type mismatch

**Issue**: `ListCustomPromptsResponseEvent.custom_prompts` is `Vec<String>` but expects `Vec<CustomPrompt>`.

**Current Fix**: `on_list_custom_prompts()` is a no-op.

**To Implement**: Update fork's custom prompts types.

### 5. User Shell Commands

**Status**: TEMPORARY
**Priority**: Medium
**Blocking**: Missing protocol variant

**Issue**: `Op::RunUserShellCommand` doesn't exist in fork's protocol.

**Current Fix**: Shows error message when `!` prefix detected in chat.

**To Implement**: Add `RunUserShellCommand` to `Op` enum.

### 6. Skill Mentions

**Status**: TEMPORARY
**Priority**: Low
**Blocking**: Missing protocol variant

**Issue**: `InputItem::Skill` variant doesn't exist in fork.

**Current Fix**: `@skill` mentions silently ignored.

**To Implement**: Add `Skill` variant to `InputItem`.

---

## Type Adaptation Stubs

These provide compatibility layers for type mismatches.

### 1. Exec Command Source

**Module**: `compat::protocol::ExecCommandSource`

**Stub Type**: Enum with `Model`, `User`, `UnifiedExecInteraction`, `UserShell` variants.

**Used By**: Extension traits on `ExecCommandBeginEvent` and `ExecCommandEndEvent`.

### 2. Elicitation Action

**Module**: `compat::protocol::ElicitationAction`

**Stub Type**: Enum with `Confirm`, `Cancel`, `Input`, `Accept`, `Decline` variants.

**Note**: Elicitation feature not implemented, so this is dead code.

### 3. Exec Policy Amendment

**Module**: `compat::protocol::ExecPolicyAmendment`

**Stub Type**: Struct with `command_pattern` field.

**Used By**: Extension trait on `ExecApprovalRequestEvent`.

### 4. MCP Startup Events

**Module**: `compat::protocol`

**Stub Types**:
- `McpStartupUpdateEvent`
- `McpStartupCompleteEvent`
- `McpStartupStatus`
- `FailedMcpServer`

**Note**: MCP startup flow uses different events in fork.

### 5. Scroll Input Mode

**Module**: `compat::config_types::ScrollInputMode`

**Stub Type**: Enum with `Auto`, `Wheel`, `Line`, `Trackpad` variants.

**Used By**: `ConfigExt` extension trait.

---

## Extension Traits

Extension traits provide upstream-compatible methods on fork types.

| Trait | Target Type | Purpose |
|-------|-------------|---------|
| `SandboxPolicyExt` | `SandboxPolicy` | get/set methods |
| `ModelFamilyExt` | `ModelFamily` | `get_model_slug()`, `context_window()` |
| `ConfigExt` | `Config` | 20+ config accessors |
| `ConversationManagerExt` | `ConversationManager` | `get_models_manager()` |
| `ExecCommandBeginEventExt` | `ExecCommandBeginEvent` | `source()`, `interaction_input()` |
| `ExecCommandEndEventExt` | `ExecCommandEndEvent` | `source()`, `command()`, etc. |
| `SessionConfiguredEventExt` | `SessionConfiguredEvent` | `initial_messages()` |
| `ExecApprovalRequestEventExt` | `ExecApprovalRequestEvent` | `proposed_execpolicy_amendment()` |

---

## Files with `#[allow(dead_code)]` Annotations

These files have module-level or function-level dead code suppression for stubbed functionality.

| File | Scope | Reason |
|------|-------|--------|
| `compat.rs` | Module | All stubs |
| `model_migration.rs` | Module | Upstream prompts not used |
| `custom_prompt_view.rs` | Module | Type mismatch |
| `app.rs` | Functions | Migration prompt handlers |
| `chatwidget.rs` | Impl block | Stubbed event handlers |
| `app_event.rs` | Enum | Some events unused |
| `history_cell.rs` | Functions | Stubbed constructors |
| `resume_picker.rs` | Items | Page loading stubs |
| `bottom_pane/*.rs` | Functions | Stubbed methods |
| `interrupts.rs` | Function | `push_elicitation` |
| `onboarding/auth.rs` | Widget | `AuthModeWidget` |

---

## Session History

| Session | Focus | Outcome |
|---------|-------|---------|
| S1-S9 | Initial port | ~262 errors |
| S10 | Compilation | 0 errors, 117 warnings |
| S11 | Runtime testing | --help, --version work; headless blocked |
| S12 | Warning cleanup | 0 warnings in codex-tui2 |
| S13 | External crates | 0 warnings in backend-client, app-server-protocol |

---

## Diagnostic Commands

```bash
# Check build status
cargo build -p codex-tui2 --release 2>&1 | tail -5

# Verify no warnings
cargo build -p codex-tui2 2>&1 | grep -c warning
# Expected: 0

# Run with debug logging
RUST_LOG=debug ./target/release/codex-tui2
```

---

## Next Steps

1. **Interactive Testing**: Verify TUI runs in real terminal (blocked on headless)
2. **High-Priority Stubs**: Address model migration type alignment
3. **Upstream Sync**: Consider aligning fork types where feasible
4. **Documentation**: Keep this file updated as stubs are resolved
