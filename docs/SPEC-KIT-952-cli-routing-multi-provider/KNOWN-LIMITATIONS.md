# Known Limitations - CLI Routing (SPEC-952)

## Model Switching in Session Mode

**Limitation**: Cannot switch between Claude models (opus/sonnet/haiku) within a session.

**Root Cause**: Global provider instances initialized with empty model string, delegate to CLI default. Single provider instance per CLI type.

```rust
// core/src/cli_executor/claude_pipes.rs
static CLAUDE_PROVIDER: OnceLock<ClaudePipesProvider> = OnceLock::new();
CLAUDE_PROVIDER.get_or_init(|| ClaudePipesProvider::with_cwd("", &cwd))
```

**Impact**:
- Selecting `/model claude-opus-4.1` then `/model claude-sonnet-4.5` won't switch models
- CLI continues using whichever model was initially selected or CLI's default
- Only the first model selection per session takes effect

**Workaround**: Use ChatGPT account for model variety (supports runtime switching)

**Fix Estimate**: 2-3 hours
- Refactor providers to be keyed by model name
- Support multiple provider instances per CLI type (HashMap<String, Provider>)
- Add model switching tests
- Update session management to handle provider lookups by model

**Priority**: P3 - Low (workaround available, doesn't block functionality)

**Related Files**:
- `core/src/cli_executor/claude_pipes.rs:619-657` (provider singleton)
- `tui/src/model_router.rs` (routing logic)
- `tui/src/providers/claude_streaming.rs` (streaming provider)

**See Also**:
- SPEC-KIT-952 main documentation
- SPEC-KIT-954 Task 3 (Long conversation testing validates single-model sessions work correctly)
