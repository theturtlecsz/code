# SPEC-KIT-922: Auto-Commit Stage Artifacts

## Overview

Automated git commits for stage artifacts during `/speckit.auto` pipeline execution to maintain a clean tree throughout the workflow.

## Problem

The `/speckit.auto` pipeline generates stage artifacts (plan.md, tasks.md, etc.) that dirty the git tree, causing guardrail failures at subsequent stages. Previously, users had to manually commit artifacts between stages or use `SPEC_OPS_ALLOW_DIRTY=1` (which bypasses important safety checks).

## Solution

Auto-commit stage artifacts immediately after consensus succeeds for each stage, maintaining a clean tree throughout the pipeline while preserving the full evidence chain.

## Implementation

### Location

- **Module**: `codex-rs/tui/src/chatwidget/spec_kit/git_integration.rs`
- **Integration**: `pipeline_coordinator.rs::check_consensus_and_advance_spec_auto()`
- **Configuration**: Environment variable or config via `shell_environment_policy`

### Committed Artifacts

For each stage, auto-commit includes:

1. **Stage output file**: `docs/SPEC-<ID>/<stage>.md` (plan.md, tasks.md, etc.)
2. **Consensus artifacts**:
   - `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/<SPEC-ID>/<stage>_synthesis.json`
   - `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/<SPEC-ID>/<stage>_verdict.json`
3. **Cost tracking**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/costs/<SPEC-ID>_cost_summary.json`

### Commit Message Format

```
feat(SPEC-KIT-XXX): complete <Stage> stage

Automated commit from /speckit.auto pipeline

Stage artifacts:
- <stage>.md
- Consensus synthesis and verdict
- Updated cost tracking

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

### Execution Flow

```
/speckit.auto pipeline
├─► Guardrail (native or shell)
├─► Multi-agent execution
├─► Consensus synthesis
│   └─► ✅ Consensus OK
│       ├─► persist_cost_summary()
│       ├─► auto_commit_stage_artifacts()  ← NEW
│       │   ├─► Collect artifact paths
│       │   ├─► git add <paths>
│       │   ├─► Check staged changes
│       │   └─► git commit -m "..."
│       └─► advance to next stage
└─► Next stage (clean tree)
```

## Configuration

### Default Behavior

**Enabled by default** (`true`) for automated workflows.

### Environment Variable Override

```bash
# Disable auto-commit
export SPEC_KIT_AUTO_COMMIT=false

# Enable auto-commit (explicit)
export SPEC_KIT_AUTO_COMMIT=true
```

### Config File Override

**~/.code/config.toml**:

```toml
[shell_environment_policy.set]
SPEC_KIT_AUTO_COMMIT = "true"
```

### Checking Configuration

```rust
// In ChatWidget
fn spec_kit_auto_commit_enabled(&self) -> bool {
    spec_kit::state::spec_kit_auto_commit_enabled(&self.config.shell_environment_policy)
}
```

## Error Handling

Auto-commit failures are **non-fatal** and do not halt the pipeline:

```rust
match super::git_integration::auto_commit_stage_artifacts(...) {
    Ok(()) => {
        tracing::info!("Auto-commit successful for {} stage", stage);
    }
    Err(err) => {
        tracing::warn!("Auto-commit failed (non-fatal): {}", err);
        widget.history_push(PlainHistoryCell::new(
            vec![Line::from(format!("⚠ Auto-commit failed (continuing): {}", err))],
            HistoryCellType::Notice,
        ));
    }
}
```

### Failure Scenarios

1. **Git not available**: Pipeline continues with dirty tree (user notified)
2. **No staged changes**: Silent success (files already committed)
3. **Git commit fails**: Warning logged, pipeline continues
4. **Auto-commit disabled**: Silent skip (debug log only)

## Benefits

### Clean Tree Maintenance

- No manual commits required between stages
- Guardrails pass without `SPEC_OPS_ALLOW_DIRTY=1`
- Evidence chain preserved in git history

### Full Audit Trail

Each stage commit includes:
- Stage output (plan.md, tasks.md, etc.)
- Consensus artifacts (synthesis + verdict)
- Cost tracking updates

### Non-Disruptive

- Defaults to enabled (zero configuration)
- Failures don't halt pipeline
- Can be disabled via environment variable

## Testing

### Manual Testing

```bash
# Start pipeline
/speckit.auto SPEC-KIT-900

# Observe auto-commits after each stage
git log --oneline

# Check clean tree between stages
git status
```

### Expected Output

```
feat(SPEC-KIT-900): complete Plan stage
feat(SPEC-KIT-900): complete Tasks stage
feat(SPEC-KIT-900): complete Implement stage
feat(SPEC-KIT-900): complete Validate stage
feat(SPEC-KIT-900): complete Audit stage
feat(SPEC-KIT-900): complete Unlock stage
```

### Disabled Auto-Commit Testing

```bash
# Disable auto-commit
export SPEC_KIT_AUTO_COMMIT=false

# Run pipeline (tree will get dirty)
/speckit.auto SPEC-KIT-900

# Verify no auto-commits
git log --oneline | grep "complete.*stage"  # Should return nothing
```

## MCP Side Effects

### Problem

MCP servers (serena, code-graph-context) create local directories (`.serena/`, `.code/`) as side effects during operations, dirtying the tree.

### Solution

Add to `.gitignore`:

```gitignore
# MCP server side effects (SPEC-KIT-922)
.serena/
.code/
```

## Code Locations

### New Files

- `tui/src/chatwidget/spec_kit/git_integration.rs` (225 lines)

### Modified Files

- `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`
  - Line ~790: Auto-commit integration after consensus
- `tui/src/chatwidget/spec_kit/state.rs`
  - Line ~988: `spec_kit_auto_commit_enabled()` function
- `tui/src/chatwidget/mod.rs`
  - Line ~1037: ChatWidget wrapper method
- `tui/src/chatwidget/spec_kit/mod.rs`
  - Line 35: Module declaration
- `.gitignore`
  - Lines 9-11: MCP side effect exclusions
- `config.toml.example`
  - Lines 91-95: Configuration documentation

## Future Enhancements

### Potential Improvements

1. **Atomic commits**: Bundle all stage artifacts into a single atomic commit
2. **Squash on completion**: Optionally squash all stage commits at pipeline end
3. **Commit metadata**: Add cost/duration to commit message body
4. **Branch strategy**: Auto-create feature branch at pipeline start

### Non-Goals

- Git push automation (too risky, requires auth)
- Rebase/merge automation (complex, error-prone)
- Commit message customization (consistency > flexibility)

## References

- **SPEC**: `docs/SPEC-KIT-922-auto-commit-artifacts/spec.md`
- **Implementation**: `codex-rs/tui/src/chatwidget/spec_kit/git_integration.rs`
- **Tests**: `git_integration::tests::*`

## Rollout Status

- ✅ Implementation complete
- ✅ Integration tested
- ✅ Documentation complete
- ⏳ Validation via `/speckit.auto` run pending
