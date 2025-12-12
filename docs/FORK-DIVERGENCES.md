# Fork Divergences

**Status**: Current
**Created**: 2025-11-28 (SPEC-958 Session 12)
**Repository**: theturtlecsz/code (fork; see `UPSTREAM-SYNC.md`)

This document describes where fork behavior differs from upstream in ways that affect tests and development.

---

## 1. Request Payload Structure

### Upstream (Expected by Original Tests)

Tests expect 3 messages in request payload:
1. System/developer message with base instructions
2. Environment context message
3. User input message

### Fork (Current Implementation)

Fork sends 5 messages:
1. **Base instructions** - System message with core behavior
2. **Environment context** - XML-formatted runtime context
3. **User instructions** - CLAUDE.md and user preferences
4. **User message** - Actual user input
5. **System status** - Runtime state message

### Impact on Tests

- `compact_resume_fork` tests: Ignored (expect 3 messages, receive 5)
- `prefixes_context_and_instructions_once_and_consistently_across_requests`: Made format-agnostic

### Role Changes

| Context | Upstream | Fork |
|---------|----------|------|
| Instructions message | `developer` | `user` |
| Context message | `developer` | `user` |

---

## 2. Tool Registry

### Fork-Added Tools

**Browser Tools** (`browser_enabled` flag):
- `browser_open` - Open browser at URL
- `browser_close` - Close browser
- `browser_status` - Get browser state
- `browser_click` - Click element
- `browser_move` - Move cursor
- `browser_type` - Type text
- `browser_key` - Press key
- `browser_javascript` - Execute JavaScript
- `browser_scroll` - Scroll viewport
- `browser_history` - Browser history
- `browser_inspect` - Inspect element
- `browser_console` - Console operations
- `browser_cleanup` - Cleanup browser
- `browser_cdp` - Chrome DevTools Protocol

**Agent Tools**:
- `run_agent` - Spawn sub-agent
- `check_agent_status` - Query agent status
- `get_agent_result` - Get agent output
- `list_agents` - List running agents
- `cancel_agent` - Cancel agent
- `wait_for_agent` - Block until agent completes

**Pro Mode Tools**:
- `pro_recommend` - Post recommendation to HUD
- `pro_submit_user` - Submit follow-up message
- `assist_core` - Inject developer instructions

**Search Tools**:
- `web_search` - Native Responses API web search
- `history_search` - Search conversation history

**Plan Tool**:
- `plan` - Create structured plans

### Upstream Tools Not Present

- `view_image` - Different implementation approach
- `apply_patch` - Different patching mechanism

### Impact on Tests

- `prompt_tools_are_consistent_across_requests`: Updated expected tool list
  - File: `core/tests/suite/prompt_caching.rs`
  - Added browser, agent, web tools to expected set

---

## 3. Auto-Compact Behavior

### Upstream Expectation

Auto-compact triggers based on token count:
- Configuration: `model_auto_compact_token_limit`
- Trigger: When total tokens exceed threshold
- Behavior: Proactive compaction to stay under limit

### Fork Reality

Auto-compact only triggers on error messages:
- Detection: API returns "exceeds the context window" error
- Trigger: Reactive, after context overflow
- `model_auto_compact_token_limit`: Config exists but unused

### Config Reference

```rust
// core/src/config.rs:91
pub model_auto_compact_token_limit: Option<i64>,
```

### Impact on Tests

3 auto_compact tests ignored:
- `auto_compact_runs_after_token_limit_hit`
- `auto_compact_stops_after_failed_attempt`
- `auto_compact_allows_multiple_attempts_when_interleaved_with_other_turn_events`

Ignore reason: "Token-based auto-compact not implemented (only error-message triggered)"

---

## 4. API Differences

### Removed Operations

| Operation | Upstream | Fork Alternative |
|-----------|----------|------------------|
| `Op::GetPath` | Query rollout path | `SessionConfiguredEvent.rollout_path` |
| `Op::UserTurn` | Per-turn config | `ConfigureSession` + `Op::UserInput` |

### Partial Implementations

**`Op::OverrideTurnContext`**:
- Exposed in `codex_core::protocol::Op`
- Handler logs but doesn't persist changes
- Full implementation requires Session mutability (RwLock)
- 6 tests depend on full implementation

### New API Extensions

**`SessionConfiguredEvent.rollout_path`** (Session 9):
```rust
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SessionConfiguredEvent {
    pub session_id: Uuid,
    pub model: String,
    pub history_log_id: u64,
    pub history_entry_count: usize,

    // Fork addition - path to rollout file
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub rollout_path: Option<PathBuf>,
}
```

---

## 5. Environment Context Format

### Format Evolution

Context has evolved from JSON to XML-like format:

```xml
<environment_context>
  <cwd>/path/to/project</cwd>
  <approval_policy>auto-approve</approval_policy>
  <sandbox_mode>enabled</sandbox_mode>
  <network_access>allowed</network_access>
  <writable_roots>
    <root>/tmp</root>
    <root>/home/user/project</root>
  </writable_roots>
  <os_info>
    <family>Linux</family>
    <version>5.15</version>
    <arch>x86_64</arch>
  </os_info>
  <common_tools>
    <tool>git</tool>
    <tool>node</tool>
  </common_tools>
  <shell>bash</shell>
</environment_context>
```

### Tool Detection

Common tools are detected via `TOOL_CANDIDATES` in `environment_context.rs`:
- Checks for executables in PATH
- Reports available tools in context
- Helps model understand available capabilities

---

## 6. Multi-Provider Model Support

### Supported Providers

| Provider | Models | Auth Method |
|----------|--------|-------------|
| ChatGPT | gpt-5, gpt-5-codex | Native OAuth |
| Claude | claude-opus-4-5, claude-sonnet-4-5, claude-haiku-4-5 | CLI routing |
| Gemini | gemini-2.5-pro, gemini-2.5-flash, gemini-2.0-flash | CLI routing |

### Fork-Specific Model Routing

Claude and Gemini models route through external CLI tools rather than direct API access.

---

## Test Migration Summary

| Category | Affected Tests | Status | Cause |
|----------|----------------|--------|-------|
| Payload structure | 2 compact_resume_fork | Ignored | 5 vs 3 messages |
| Tools list | 1 prompt_tools | Fixed | Updated expected tools |
| Auto-compact | 3 compact | Ignored | Token trigger not implemented |
| Per-turn context | 6 model_overrides/prompt_caching | Stubbed | Op::OverrideTurnContext partial |

---

## Related Documentation

- [SPEC-958-test-migration.md](SPEC-958-test-migration.md) - Test migration tracking
- [TEST-ARCHITECTURE.md](testing/TEST-ARCHITECTURE.md) - Test infrastructure guide
- [CLAUDE.md](../CLAUDE.md) - Project instructions
