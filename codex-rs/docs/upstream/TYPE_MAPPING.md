# Local ↔ Upstream Type Mapping

This document maps type differences between the local fork and upstream Codex,
enabling systematic adaptation when porting upstream code.

## Legend

| Compatibility | Meaning |
|---------------|---------|
| **FULL** | Types are identical |
| **PARTIAL** | Some fields/variants differ |
| **MISSING** | Type does not exist locally |
| **LOCAL-ONLY** | Type exists only locally |

## Protocol Types (`codex_protocol::protocol`)

### Core Enums

| Local Type | Upstream Type | Compatibility | Divergence | Strategy |
|------------|---------------|---------------|------------|----------|
| `SandboxPolicy` | `SandboxPolicy` | PARTIAL | Missing `ExternalSandbox` variant | Map to `WorkspaceWrite` |
| `AskForApproval` | `AskForApproval` | PARTIAL | Missing `.get()`, `.set()` methods | Use match expressions |
| `ReviewDecision` | `ReviewDecision` | PARTIAL | Missing `ApprovedExecpolicyAmendment` variant | Use `Allow` |
| `Op` | `Op` | PARTIAL | Missing `ListSkills`, `RunUserShellCommand`, `ResolveElicitation`, `ListMcpTools`, `ListCustomPrompts` | Stub or remove |
| `EventMsg` | `EventMsg` | PARTIAL | Missing 15+ streaming/MCP variants | Stub handlers |

### EventMsg Missing Variants

The following `EventMsg` variants exist upstream but not locally:

| Variant | Purpose | Strategy |
|---------|---------|----------|
| `McpStartupUpdate` | MCP server startup progress | Remove handler |
| `McpStartupComplete` | MCP servers ready | Remove handler |
| `McpListToolsResponse` | MCP tool listing | Remove handler |
| `ListCustomPromptsResponse` | Custom prompts list | Remove handler |
| `ElicitationRequest` | Interactive prompts | Remove handler |
| `StreamError` | Streaming errors | Map to `Error` |
| `TerminalInteraction` | Terminal I/O | Remove handler |
| `DeprecationNotice` | Deprecation warnings | Log and ignore |
| `Warning` | General warnings | Log and ignore |
| `ContextCompacted` | Context compression | Log and ignore |
| `ReasoningContentDelta` | Reasoning streaming | Remove handler |
| `ReasoningRawContentDelta` | Raw reasoning | Remove handler |
| `AgentMessageContentDelta` | Agent message streaming | Remove handler |
| `ItemStarted` / `ItemCompleted` | Response item lifecycle | Remove handlers |
| `ViewImageToolCall` | Image viewing | Remove handler |
| `WebSearchEnd` | Web search completion | Remove handler |
| `SkillsUpdateAvailable` | Skills updates | Remove handler |
| `RawResponseItem` | Raw API response | Remove handler |

### Struct Field Differences

#### `Config` (codex_core::config)

| Missing Field | Type | Strategy |
|--------------|------|----------|
| `animations` | `bool` | Add with default `false` OR remove access |
| `notices` | `NoticesConfig` | Remove access |
| `features` | `FeaturesConfig` | Remove access |
| `cli_auth_credentials_store_mode` | enum | Remove access |
| `forced_login_method` | `Option<String>` | Remove access |
| `forced_chatgpt_workspace_id` | `Option<String>` | Remove access |
| `disable_paste_burst` | `bool` | Remove access |
| `show_tooltips` | `bool` | Remove access |
| `did_user_set_custom_approval_policy_or_sandbox_mode` | `bool` | Remove access |
| `active_project` | `Option<ProjectInfo>` | Remove access |
| `tui_scroll_*` (8 fields) | various | Remove access |

#### `SessionConfiguredEvent`

| Local Fields | Upstream Fields | Divergence |
|--------------|-----------------|------------|
| `model`, `sandbox_policy`, `approval_policy` | + `reasoning_effort`, `initial_messages` | Missing fields |

**Strategy**: Remove destructuring of missing fields

#### `ExecCommandBeginEvent` / `ExecCommandEndEvent`

| Missing Field | Type | Strategy |
|--------------|------|----------|
| `source` | `ExecCommandSource` | Remove access |
| `interaction_input` | `Option<String>` | Remove access |
| `parsed_cmd` | `ParsedCommand` | Remove access |
| `formatted_output` | `String` | Remove access |
| `aggregated_output` | `String` | Remove access |
| `command` | `Vec<String>` | Remove access |

#### `ExecApprovalRequestEvent`

| Missing Field | Type | Strategy |
|--------------|------|----------|
| `proposed_execpolicy_amendment` | `ExecPolicyAmendment` | Remove access |

#### `ReviewRequest`

| Missing Field | Type | Strategy |
|--------------|------|----------|
| `target` | `ReviewTarget` | Remove - use simplified review |

#### `UpdatePlanArgs`

| Missing Field | Type | Strategy |
|--------------|------|----------|
| `explanation` | `String` | Remove from pattern matching |

#### `FileChange::Delete`

| Missing Field | Type | Strategy |
|--------------|------|----------|
| `content` | `String` | Remove from pattern matching |

#### `ApplyPatchApprovalRequestEvent`

| Missing Field | Type | Strategy |
|--------------|------|----------|
| `turn_id` | `String` | Remove from construction |

#### `Event`

| Missing Field | Type | Strategy |
|--------------|------|----------|
| `event_seq` | `u64` | Add field OR remove from patterns |
| `order` | `u64` | Add field OR remove from patterns |

#### `McpServerConfig`

| Missing Field | Type | Strategy |
|--------------|------|----------|
| `transport` | `TransportConfig` | Access via alternate method |
| `enabled` | `bool` | Assume always enabled |

#### `CustomPrompt`

| Missing Field | Type | Strategy |
|--------------|------|----------|
| `description` | `Option<String>` | Remove access |

#### `ConversationItem`

| Missing Field | Type | Strategy |
|--------------|------|----------|
| `created_at` | `DateTime` | Remove access |
| `updated_at` | `DateTime` | Remove access |

#### `Cli` (codex_tui)

| Missing Field | Type | Strategy |
|--------------|------|----------|
| `resume_show_all` | `bool` | Remove access |
| `oss_provider` | `Option<String>` | Remove access |
| `add_dir` | `Vec<PathBuf>` | Remove access |

#### `ConfigOverrides`

| Missing Field | Type | Strategy |
|--------------|------|----------|
| `developer_instructions` | `Option<String>` | Remove from construction |
| `compact_prompt` | `Option<String>` | Remove from construction |
| `additional_writable_roots` | `Vec<PathBuf>` | Remove from construction |

## Missing Imports

### Modules

| Import Path | Status | Strategy |
|-------------|--------|----------|
| `codex_common::oss` | MISSING | Remove import |
| `codex_core::features` | MISSING | Remove import |
| `codex_core::skills` | MISSING | Remove import |
| `codex_core::models_manager` | MISSING | Remove import |
| `codex_core::config::edit` | MISSING | Remove import |
| `codex_core::config::types` | MISSING | Remove import |
| `codex_core::auth::enforce_login_restrictions` | MISSING | Remove import |
| `codex_core::terminal::terminal_info` | MISSING | Stub function |
| `codex_core::env` | MISSING | Remove import |
| `codex_core::path_utils` | MISSING | Remove import |
| `codex_core::bash::extract_bash_command` | MISSING | Remove import |
| `codex_core::parse_command::extract_shell_command` | MISSING | Remove import |
| `codex_core::project_doc::DEFAULT_PROJECT_DOC_FILENAME` | MISSING | Define locally |
| `codex_core::review_prompts` | MISSING | Remove import |
| `codex_core::otel_init` | MISSING | Remove import |

### Constants

| Constant | Upstream Location | Strategy |
|----------|-------------------|----------|
| `INTERACTIVE_SESSION_SOURCES` | `codex_core` | Define locally or remove |
| `PROMPTS_CMD_PREFIX` | `codex_protocol::custom_prompts` | Define locally |
| `DEFAULT_PROJECT_DOC_FILENAME` | `codex_core::project_doc` | Define locally |
| `OLLAMA_OSS_PROVIDER_ID` | `codex_core` | Define locally or remove |
| `LMSTUDIO_OSS_PROVIDER_ID` | `codex_core` | Define locally or remove |
| `DEFAULT_OLLAMA_PORT` | `codex_core` | Define locally or remove |
| `DEFAULT_LMSTUDIO_PORT` | `codex_core` | Define locally or remove |

### Types

| Type | Upstream Location | Strategy |
|------|-------------------|----------|
| `AppExitInfo` | `codex_tui` | Define locally |
| `ApprovedExecpolicyAmendment` | `codex_core::protocol` | Remove usage |
| `RateLimitSnapshot` | `codex_core::protocol` | Define locally or remove |
| `RateLimitWindow` | `codex_core::protocol` | Remove import |
| `ExecCommandSource` | `codex_core::protocol` | Remove import |
| `ExecPolicyAmendment` | `codex_core::protocol` | Remove import |
| `ElicitationAction` | `codex_core::protocol` | Remove import |
| `TerminalInfo` | `codex_core::terminal` | Stub locally |
| `TerminalName` | `codex_core::terminal` | Stub locally |
| `TurnAbortReason` | `codex_core::protocol` | Define locally or remove |
| `ConstraintResult` | `codex_core::config` | Remove import |
| `AuthCredentialsStoreMode` | `codex_core::auth` | Remove import |
| `DeprecationNoticeEvent` | `codex_core::protocol` | Remove import |
| All `*Event` types in missing imports | | Remove or stub |

### Functions

| Function | Upstream Location | Strategy |
|----------|-------------------|----------|
| `parse_turn_item` | `codex_core` | Stub or implement |
| `terminal_info` | `codex_core::terminal` | Stub |
| `read_openai_api_key_from_env` | `codex_core::auth` (private) | Re-implement or remove |
| `resolve_oss_provider` | `codex_core::config` | Remove call |
| `set_project_trust_level` | `codex_core::config` | Remove call |
| `set_default_oss_provider` | `codex_core::config` | Remove call |
| `format_env_display` | `codex_common` | Implement locally |
| `extract_bash_command` | `codex_core::bash` | Remove call |
| `extract_shell_command` | `codex_core::parse_command` | Remove call |

### Methods

| Method | Type | Strategy |
|--------|------|----------|
| `.get()` | `AskForApproval` | Use direct field access |
| `.set()` | `AskForApproval`, `SandboxPolicy` | Use assignment |
| `.get()` | `SandboxPolicy` | Use match expressions |
| `.value()` | `AskForApproval` | Use match expressions |
| `.get_models_manager()` | `ConversationManager` | Remove call |
| `.get_account_email()` | `CodexAuth` | Remove or stub |
| `.upload_feedback()` | `CodexLogSnapshot` | Remove call |
| `.to_error_event()` | `CodexErr` | Implement or remove |
| `.load_with_cli_overrides_and_harness_overrides()` | `Config` | Use existing loader |
| `.from_auth_storage()` | `CodexAuth` | Use existing constructor |
| `.unwrap_or_else()` | `String` | Wrong type - fix call chain |

## External Dependencies

| Crate | Missing Import | Strategy |
|-------|---------------|----------|
| `crossterm` | `query_foreground_color` | Remove (not in crossterm 0.27) |
| `crossterm` | `query_background_color` | Remove (not in crossterm 0.27) |

## Session 9-11 Discoveries (SYNC-028)

### Type Location Divergences

These types exist in different modules between upstream and fork:

| Type | Upstream Location | Fork Location | Strategy |
|------|-------------------|---------------|----------|
| `ReasoningEffort` | `codex_protocol` | `codex_core::config` | Bidirectional conversions in `compat.rs` |
| `RateLimitSnapshot` | `EventMsg::RateLimitEvent` | `codex_protocol::protocol` | Re-export + conversion |
| `ParsedCommand` | `codex_protocol::parsed_command` | `codex_core::parse_command` | Use fork location |
| `TurnAbortReason` | `codex_protocol` | `codex_protocol` | Re-export in compat.rs |

### Missing Enum Variants

| Type | Missing Variant | Strategy |
|------|-----------------|----------|
| `InputItem` | `Skill { ... }` | Remove skill mentions from chat handling |

### Missing Config Fields

| Field | Expected Type | Strategy |
|-------|---------------|----------|
| `check_for_update_on_startup` | `bool` | Disabled via const (upstream checks openai/codex) |

### Function Signature Divergences

| Function | Upstream Signature | Fork Signature | Strategy |
|----------|-------------------|----------------|----------|
| `create_client()` | `fn create_client() -> Client` | `fn create_client(originator: &str) -> Client` | Pass originator string |

### Conversion Helpers (tui2/src/compat.rs)

Created bidirectional conversion functions:

```rust
// ReasoningEffort conversions
fn protocol_to_core_reasoning(re: ProtocolReasoningEffort) -> CoreReasoningEffort
fn core_to_protocol_reasoning(re: CoreReasoningEffort) -> ProtocolReasoningEffort

// RateLimitSnapshot wrapper
pub use codex_protocol::protocol::RateLimitSnapshot;
```

---

_Last updated: 2024-12-24 (SYNC-028 Session 11)_
