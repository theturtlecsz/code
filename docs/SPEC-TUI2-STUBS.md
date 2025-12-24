# SPEC-TUI2-STUBS: Stubbed Features in TUI v2 Port

## Overview

This document tracks features that were stubbed out during the TUI v2 port (SYNC-028) due to API divergences between the upstream Codex codebase and the local fork.

**Port Completed**: Session 10
**Final Error Count**: 0 (from 56 initial errors)
**Build Status**: `cargo build -p codex-tui2` succeeds

## Stubbed Features

### 1. Model Migration (High Priority)

**Issue**: `ReasoningEffort` enum has different variants between `codex_core::config_types` and `codex_protocol::openai_models`.

**Impact**: Model migration prompts and reasoning effort selection may not work as expected.

**Stubbed In**:
- `tui2/src/compat.rs`: Added conversion functions `convert_reasoning_effort()` and `convert_reasoning_effort_to_protocol()`
- `tui2/src/chatwidget.rs`: Multiple call sites use conversion

**To Fix**: Either unify the types in core/protocol or create proper bi-directional conversion.

### 2. Credits Display

**Issue**: Fork's protocol doesn't include credits/plan_type in `RateLimitSnapshot`.

**Impact**: User's remaining credits won't be displayed in the status bar.

**Stubbed In**:
- `tui2/src/compat.rs`: `convert_rate_limit_snapshot()` maps `RateLimitSnapshotEvent` to `RateLimitSnapshot`
- Comments in `chatwidget.rs` note the limitation

**To Fix**: Add credits fields to fork's `RateLimitSnapshotEvent` or accept the limitation.

### 3. OSS Provider Integration

**Issue**: `codex_common::oss` module doesn't exist in fork.

**Impact**: No OSS provider management or display.

**Stubbed In**:
- `tui2/src/compat.rs`: Stub `oss` module with no-op functions

**To Fix**: Not planned - OSS providers not relevant to fork.

### 4. Skill Mentions

**Issue**: `codex_core::protocol::InputItem` doesn't have a `Skill` variant.

**Impact**: `@skill` mentions in user input won't be parsed.

**Stubbed In**:
- `tui2/src/chatwidget.rs`: Skill mention handling commented out

**To Fix**: Add `Skill` variant to `InputItem` in codex_core.

### 5. MCP Tools Display

**Issue**: Type mismatch - `ev.tools` is `Vec<String>` but display expects `HashMap<String, Tool>`.

**Impact**: MCP tools listing won't render correctly.

**Stubbed In**:
- `tui2/src/chatwidget.rs`: `on_list_mcp_tools()` is no-op

**To Fix**: Update MCP tools response type in fork to match upstream.

### 6. Custom Prompts

**Issue**: Type mismatch - `ev.custom_prompts` is `Vec<String>` but expects `Vec<CustomPrompt>`.

**Impact**: Custom prompts won't be displayed.

**Stubbed In**:
- `tui2/src/chatwidget.rs`: `on_list_custom_prompts()` is no-op

**To Fix**: Update custom prompts response type in fork.

### 7. User Shell Commands

**Issue**: `Op::RunUserShellCommand` doesn't exist in fork's protocol.

**Impact**: `!cmd` shell commands in chat won't work.

**Stubbed In**:
- `tui2/src/chatwidget.rs`: Shows error message when `!` prefix detected

**To Fix**: Add `RunUserShellCommand` variant to `Op` enum.

### 8. Review Mode Exit Handler

**Issue**: `ExitedReviewModeEvent` vs `Option<ReviewOutputEvent>` type mismatch.

**Impact**: Exiting review mode may not apply changes correctly.

**Stubbed In**:
- `tui2/src/chatwidget.rs`: Minimal handler that restores token info

**To Fix**: Create proper handler for `Option<ReviewOutputEvent>`.

### 9. Execpolicy Amendment

**Issue**: `ReviewDecision::ApprovedExecpolicyAmendment` doesn't exist in fork.

**Impact**: Execpolicy amendment approvals won't render correctly.

**Stubbed In**:
- `tui2/src/history_cell.rs`: Removed match arm

**To Fix**: Add variant to fork's `ReviewDecision` if needed.

## Type Conversion Functions Added

| Function | Location | Purpose |
|----------|----------|---------|
| `convert_reasoning_effort()` | compat.rs | Protocol → Core |
| `convert_reasoning_effort_to_protocol()` | compat.rs | Core → Protocol |
| `convert_rate_limit_snapshot()` | compat.rs:protocol | Event → Snapshot |

## Files Modified

Major changes in:
- `tui2/src/compat.rs` - All compatibility stubs
- `tui2/src/chatwidget.rs` - Main widget adaptations
- `tui2/src/history_cell.rs` - History cell types
- `tui2/src/exec_cell/render.rs` - Command rendering
- `tui2/src/status/card.rs` - Status display types
- `tui2/src/onboarding/*.rs` - Onboarding screens

## Next Steps

1. **Runtime Testing**: Verify tui2 runs without panics
2. **Feature Verification**: Test each stubbed feature for graceful degradation
3. **Incremental Fixes**: Address high-priority stubs (model migration, credits)
4. **Upstream Alignment**: Consider aligning fork types with upstream where feasible
