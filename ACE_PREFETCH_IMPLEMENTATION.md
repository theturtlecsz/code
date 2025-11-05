# ACE Pre-fetch Caching Implementation

## Problem Statement

ACE playbook injection was broken due to an async/sync boundary issue. The `inject_ace_section` function in `ace_prompt_injector.rs` could not call async ACE functions from within a synchronous prompt assembly context, resulting in ACE being effectively disabled with warning messages.

## Solution Overview

Implement **pre-fetch caching** strategy:
1. Fetch ACE bullets **BEFORE** prompt assembly (async-safe timing)
2. Cache bullets in `SpecAutoState`
3. Inject synchronously from cache during prompt assembly

## Implementation Details

### File Modified
`/home/thetu/code/codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`

### Function Updated
`auto_submit_spec_stage_prompt()` - Lines 29-107

### Key Changes

1. **Pre-fetch Logic** (lines 36-107):
   - Check if ACE is enabled in config
   - Determine if command should use ACE (`should_use_ace`)
   - Map command to ACE scope (`command_to_scope`)
   - Fetch repository root and branch from git
   - Call `ace_client::playbook_slice()` via `block_on_sync()`
   - Handle three result cases: Ok, Disabled, Error

2. **Caching in State** (lines 103-106):
   ```rust
   if let Some(state) = widget.spec_auto_state.as_mut() {
       state.ace_bullets_cache = ace_bullets;
       state.ace_bullet_ids_used = None; // Reset for new stage
   }
   ```

3. **Timing**: Happens BEFORE prompt assembly (line 103 "let mut arg = spec_id.to_string()")

### Dependencies

- **block_on_sync**: Already imported from `consensus_coordinator` for sync/async bridging
- **should_use_ace**: From `ace_prompt_injector` - checks if ACE enabled for command
- **command_to_scope**: From `ace_prompt_injector` - maps command to ACE scope
- **get_repo_root/get_current_branch**: From `routing` - extracts git context
- **ace_client::playbook_slice**: Async function to fetch bullets from ACE MCP

### Error Handling

- **ACE Disabled**: Logs debug message, returns `None`
- **ACE Error**: Logs warning with stage name and error, returns `None` (graceful degradation)
- **Git Failures**: Fallback to `config.cwd` for repo root, `"main"` for branch
- **No Scope Mapping**: Logs debug, returns `None`

## State Structure

ACE bullets are cached in `SpecAutoState` (already defined in `state.rs:473-475`):
```rust
pub ace_bullets_cache: Option<Vec<super::ace_client::PlaybookBullet>>,
pub ace_bullet_ids_used: Option<Vec<i32>>,
```

## Next Steps (Not Implemented)

1. **Synchronous Injection**: Update `build_stage_prompt_with_mcp()` or similar to inject cached bullets from state
2. **Learning Feedback**: Use `ace_bullet_ids_used` to track which bullets were actually used for ACE learning
3. **Testing**: Verify ACE injection actually works end-to-end with cached bullets

## Compilation Status

✅ **Compiles successfully** with `cargo build -p codex-tui --profile dev-fast`
- No errors
- 89 warnings (pre-existing, unrelated to this change)

## Logging

Adds three log levels:
- **INFO**: ACE pre-fetch successful (bullet count, stage, scope)
- **DEBUG**: ACE disabled, no scope mapping, not enabled for command
- **WARN**: ACE pre-fetch failed (stage name, error message)

## Architecture Benefits

1. **Solves async/sync boundary**: Fetches before synchronous context
2. **Graceful degradation**: Failures don't break pipeline, just skip ACE
3. **Clean separation**: Pre-fetch in orchestrator, injection elsewhere
4. **Reusable pattern**: Can extend to other stages needing ACE data
5. **Minimal invasiveness**: Single function modification, uses existing state

## Performance Considerations

- ACE fetch happens once per stage (not per agent)
- Cached for entire stage execution
- Async fetch time added to pipeline initiation (~50-200ms typical)
- No impact if ACE disabled (fast early return)

## Future Enhancements

1. Cache ACE bullets across retries (avoid re-fetch on agent retry)
2. Implement bullet selection/filtering before caching
3. Add telemetry for ACE fetch timing and success rate
4. Consider pre-fetching for next stage (prefetch N+1 while executing N)
