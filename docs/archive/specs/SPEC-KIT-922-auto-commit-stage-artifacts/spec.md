# SPEC-KIT-922: Auto-Commit Stage Artifacts

**Status**: Draft
**Created**: 2025-11-10
**Priority**: P0 (Critical - blocks automated CI/CD)
**Owner**: Code
**Dependencies**: SPEC-KIT-900 (discovered the issue)

---

## Problem Statement

SPEC-900 validation revealed **critical workflow flaw**: `/speckit.auto` pipeline generates stage artifacts (plan.md, tasks.md, implement.md, etc.) that dirty the git tree, causing guardrail failures at subsequent stages.

**Current Behavior**:
```
Plan stage → generates plan.md (tree dirty)
↓
Tasks stage → generates tasks.md (tree still dirty)
↓
Implement stage → generates implement.md (tree still dirty)
↓
Validate stage → guardrail FAILS (clean-tree check) ❌
Pipeline BLOCKED - requires manual commit intervention
```

**Impact**:
- ❌ Automated `/speckit.auto` runs fail mid-pipeline
- ❌ Requires manual intervention to commit artifacts
- ❌ Breaks CI/CD automation
- ❌ Defeats purpose of end-to-end pipeline
- ❌ Guardrails become friction instead of safety

**Discovery**: During SPEC-900 run (2025-11-10), Validate stage blocked with:
```
✗ clean-tree: Working tree has 1 unexpected changes (stage artifacts excluded)
Status: FAILED
```

Files causing block:
- `docs/SPEC-KIT-900/plan.md` (new)
- `docs/SPEC-KIT-900/tasks.md` (new)
- `docs/SPEC-KIT-900/implement.md` (new)
- Modified consensus JSON files
- MCP side effects (`codex-rs/.serena/memories/`)

---

## Root Cause Analysis

### Why This Happens

1. **Stage artifacts are evidence** → Must be committed to git for validation
2. **Pipeline generates artifacts sequentially** → Tree dirty after each stage
3. **Guardrails enforce clean tree** → Correct behavior, ensures code quality
4. **No automated commit logic** → Gap in pipeline implementation

### Why We Can't Just Relax Guardrails

Guardrails exist to prevent:
- Accidental commits of temp files
- Debug artifacts in repo
- Uncommitted code changes
- Sensitive data leaks
- Build artifacts

**Relaxing guardrails = removing safety** ❌

### The Real Solution

**Embrace the guardrails**, add proper artifact lifecycle management:
- Auto-commit stage artifacts as they're generated
- Maintain clean tree throughout pipeline
- Preserve full evidence chain
- Enable automated runs

---

## Success Criteria

### Primary Goals
1. **Automated pipeline completion**: `/speckit.auto` runs end-to-end without manual commits
2. **Clean tree maintained**: Every stage starts with clean tree (guardrails pass)
3. **Evidence preserved**: All stage artifacts committed to git
4. **Granular history**: Clear git log showing stage-by-stage progress

### Acceptance Criteria
- [ ] Auto-commit logic integrated into pipeline_coordinator.rs
- [ ] Config flag: `spec_kit.auto_commit` (default: true)
- [ ] Commits after each stage: plan, tasks, implement, validate, audit, unlock
- [ ] Descriptive commit messages with stage context
- [ ] MCP side effects properly gitignored
- [ ] SPEC-900 re-run completes without manual intervention
- [ ] Git history shows 6 commits (one per stage)

### Non-Goals
- Single mega-commit at pipeline end (loses granularity)
- Gitignoring stage artifacts (loses evidence)
- User-facing `/speckit.commit` command (over-engineering)

---

## Technical Design

### Architecture

**Component**: `pipeline_coordinator.rs` (pipeline orchestration)

**Integration Point**: After consensus succeeds, before advancing to next stage

**Call Flow**:
```
check_consensus_and_advance_spec_auto()
  ↓
consensus_ok = true
  ↓
persist_cost_summary()
  ↓
auto_commit_stage_artifacts()  ← NEW
  ↓
advance_spec_auto()
```

### Implementation Details

#### 1. Auto-Commit Function

**Location**: `tui/src/chatwidget/spec_kit/git_integration.rs` (new module)

```rust
/// Auto-commit stage artifacts to maintain clean tree
pub fn auto_commit_stage_artifacts(
    spec_id: &str,
    stage: SpecStage,
    cwd: &Path,
    config: &SpecKitConfig,
) -> Result<(), SpecKitError> {
    if !config.auto_commit {
        return Ok(()); // Feature disabled
    }

    // 1. Collect paths to commit
    let paths_to_commit = collect_stage_artifact_paths(spec_id, stage, cwd)?;

    // 2. Stage files
    stage_files(&paths_to_commit, cwd)?;

    // 3. Check if there are changes to commit
    if !has_staged_changes(cwd)? {
        tracing::info!("No stage artifacts to commit for {} stage", stage.display_name());
        return Ok(());
    }

    // 4. Commit with descriptive message
    let commit_msg = format_stage_commit_message(spec_id, stage);
    commit_staged_files(&commit_msg, cwd)?;

    tracing::info!("Auto-committed {} stage artifacts", stage.display_name());
    Ok(())
}

fn collect_stage_artifact_paths(
    spec_id: &str,
    stage: SpecStage,
    cwd: &Path,
) -> Result<Vec<PathBuf>, SpecKitError> {
    let mut paths = Vec::new();

    // Stage output file
    let stage_file = cwd
        .join("docs")
        .join(spec_id)
        .join(format!("{}.md", stage.display_name().to_lowercase()));

    if stage_file.exists() {
        paths.push(stage_file);
    }

    // Consensus artifacts
    let consensus_dir = cwd
        .join("docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus")
        .join(spec_id);

    if consensus_dir.exists() {
        // Add synthesis and verdict files for this stage
        let stage_name = stage.command_name(); // "spec-plan", "spec-tasks", etc.

        let synthesis = consensus_dir.join(format!("{}_synthesis.json", stage_name));
        if synthesis.exists() {
            paths.push(synthesis);
        }

        let verdict = consensus_dir.join(format!("{}_verdict.json", stage_name));
        if verdict.exists() {
            paths.push(verdict);
        }
    }

    // Cost summary (updated after each stage)
    let cost_file = cwd
        .join("docs/SPEC-OPS-004-integrated-coder-hooks/evidence/costs")
        .join(format!("{}_cost_summary.json", spec_id));

    if cost_file.exists() {
        paths.push(cost_file);
    }

    Ok(paths)
}

fn stage_files(paths: &[PathBuf], cwd: &Path) -> Result<(), SpecKitError> {
    for path in paths {
        let relative_path = path.strip_prefix(cwd)
            .map_err(|e| SpecKitError::from_string(format!("Invalid path: {}", e)))?;

        let output = Command::new("git")
            .args(&["add", relative_path.to_str().unwrap()])
            .current_dir(cwd)
            .output()
            .map_err(|e| SpecKitError::from_string(format!("Git add failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecKitError::from_string(format!("Git add failed: {}", stderr)));
        }
    }

    Ok(())
}

fn has_staged_changes(cwd: &Path) -> Result<bool, SpecKitError> {
    let output = Command::new("git")
        .args(&["diff", "--cached", "--quiet"])
        .current_dir(cwd)
        .status()
        .map_err(|e| SpecKitError::from_string(format!("Git diff failed: {}", e)))?;

    // Exit code 1 means there are staged changes
    Ok(!output.success())
}

fn commit_staged_files(message: &str, cwd: &Path) -> Result<(), SpecKitError> {
    let output = Command::new("git")
        .args(&["commit", "-m", message])
        .current_dir(cwd)
        .output()
        .map_err(|e| SpecKitError::from_string(format!("Git commit failed: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SpecKitError::from_string(format!("Git commit failed: {}", stderr)));
    }

    Ok(())
}

fn format_stage_commit_message(spec_id: &str, stage: SpecStage) -> String {
    format!(
        "feat({}): complete {} stage

Automated commit from /speckit.auto pipeline

Stage artifacts:
- {}.md
- Consensus synthesis and verdict
- Updated cost tracking

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>",
        spec_id,
        stage.display_name(),
        stage.display_name().to_lowercase()
    )
}
```

#### 2. Configuration Schema

**File**: `tui/src/config.rs`

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct SpecKitConfig {
    /// Auto-commit stage artifacts during /speckit.auto pipeline
    #[serde(default = "default_auto_commit")]
    pub auto_commit: bool,

    // ... existing fields ...
}

fn default_auto_commit() -> bool {
    true // Enable by default for automated workflows
}
```

**File**: `config.toml`

```toml
[spec_kit]
# Auto-commit stage artifacts during /speckit.auto
# Maintains clean git tree throughout pipeline
# Set to false for manual commit control
auto_commit = true
```

#### 3. Integration into Pipeline

**File**: `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`

**Location**: `check_consensus_and_advance_spec_auto()` function (line ~850)

```rust
// After consensus succeeds
if consensus_ok {
    // ... existing success handling ...

    persist_cost_summary(widget, &spec_id);

    // NEW: Auto-commit stage artifacts
    if let Err(err) = super::git_integration::auto_commit_stage_artifacts(
        &spec_id,
        current_stage,
        &widget.config.cwd,
        &widget.config.spec_kit,
    ) {
        tracing::warn!("Auto-commit failed for {} stage: {}", current_stage.display_name(), err);
        // Don't halt pipeline - commit failure is non-critical
        // User can manually commit artifacts later
    }

    // Advance to next stage
    if let Some(state) = widget.spec_auto_state.as_mut() {
        state.reset_cost_tracking(current_stage);
        state.phase = SpecAutoPhase::Guardrail;
        state.current_index += 1;
    }

    advance_spec_auto(widget);
}
```

#### 4. Gitignore MCP Side Effects

**File**: `.gitignore`

```gitignore
# MCP-generated directories (not evidence, just caches)
codex-rs/.serena/
codex-rs/.code/
.byterover/

# SQLite databases (state, not evidence)
*.db
*.db-shm
*.db-wal
```

---

## Testing Strategy

### Unit Tests

**File**: `tui/src/chatwidget/spec_kit/git_integration_tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_stage_artifact_paths() {
        // Mock filesystem with stage artifacts
        // Verify correct paths collected
    }

    #[test]
    fn test_format_stage_commit_message() {
        let msg = format_stage_commit_message("SPEC-KIT-900", SpecStage::Plan);
        assert!(msg.contains("SPEC-KIT-900"));
        assert!(msg.contains("Plan stage"));
        assert!(msg.contains("plan.md"));
    }

    #[test]
    fn test_auto_commit_disabled() {
        let mut config = SpecKitConfig::default();
        config.auto_commit = false;

        let result = auto_commit_stage_artifacts("SPEC-TEST", SpecStage::Plan, Path::new("/tmp"), &config);
        assert!(result.is_ok()); // Should no-op gracefully
    }
}
```

### Integration Tests

**File**: `tui/tests/spec_kit_auto_commit_integration_tests.rs`

```rust
#[test]
fn test_spec_auto_pipeline_commits_each_stage() {
    // Setup: Clean repo with SPEC-TEST
    // Run: /speckit.auto SPEC-TEST
    // Verify: 6 commits created (one per stage)
    // Verify: Each commit message contains stage name
    // Verify: Working tree clean at end
}

#[test]
fn test_spec_auto_with_auto_commit_disabled() {
    // Setup: config.auto_commit = false
    // Run: /speckit.auto SPEC-TEST
    // Verify: No auto-commits created
    // Verify: All artifacts remain uncommitted
}

#[test]
fn test_spec_auto_commit_failure_non_fatal() {
    // Setup: Make git commit fail (e.g., no email configured)
    // Run: /speckit.auto SPEC-TEST
    // Verify: Pipeline continues despite commit failure
    // Verify: Warning logged
}
```

### E2E Validation

**Test**: Re-run SPEC-900 with auto-commit enabled

```bash
# 1. Reset SPEC-900 state
git checkout main
rm -rf docs/SPEC-KIT-900/*.md
rm -rf docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-900/
sqlite3 ~/.code/consensus_artifacts.db "DELETE FROM consensus_artifacts WHERE spec_id='SPEC-KIT-900';"

# 2. Enable auto-commit
# config.toml: spec_kit.auto_commit = true

# 3. Run pipeline
/speckit.auto SPEC-KIT-900

# 4. Verify results
git log --oneline | head -6  # Should show 6 auto-commits
git status                    # Should be clean
ls docs/SPEC-KIT-900/         # Should have all 6 stage files
```

**Expected Git Log**:
```
abc1234 feat(SPEC-KIT-900): complete unlock stage
def5678 feat(SPEC-KIT-900): complete audit stage
ghi9012 feat(SPEC-KIT-900): complete validate stage
jkl3456 feat(SPEC-KIT-900): complete implement stage
mno7890 feat(SPEC-KIT-900): complete tasks stage
pqr1234 feat(SPEC-KIT-900): complete plan stage
```

---

## Migration Strategy

### Phase 1: Implementation (2 hours)
1. Create `git_integration.rs` module
2. Implement auto-commit function
3. Add config schema
4. Integrate into pipeline_coordinator.rs
5. Update .gitignore

### Phase 2: Testing (1 hour)
1. Write unit tests
2. Write integration tests
3. Run existing test suite (ensure no regressions)

### Phase 3: Validation (30 min)
1. Re-run SPEC-900 with auto-commit
2. Verify 6 commits created
3. Verify clean tree throughout
4. Check commit messages

### Phase 4: Documentation (30 min)
1. Update CLAUDE.md with auto-commit behavior
2. Document config flag in config.toml
3. Add troubleshooting guide

**Total**: ~4 hours

---

## Rollout Plan

### Enabling Auto-Commit

**Default**: `auto_commit = true` (enabled by default)

**Rationale**:
- Most users want automated workflows
- Manual commit control available via config flag
- Guardrails remain enforced

### Disabling Auto-Commit

Users who want manual control:

```toml
[spec_kit]
auto_commit = false  # Manual commit control
```

**Use Case**:
- Research/experimentation (don't want commits)
- Manual review before committing
- Custom git workflows

### Backward Compatibility

**Existing runs**: No impact (new feature, additive only)

**Config migration**: Default to `true` if not specified

---

## Success Metrics

### Primary
- **Automated pipeline success rate**: 100% (was 0% due to guardrail blocks)
- **Manual intervention required**: 0 (was 1 commit per stage = 6 total)
- **Evidence completeness**: 6/6 stages committed
- **Git history quality**: 6 descriptive commits (one per stage)

### Secondary
- **Guardrail pass rate**: 100% (clean tree maintained)
- **User satisfaction**: Feedback on automation improvement
- **CI/CD enablement**: Automated runs possible

---

## Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Git commit fails (no email config) | Pipeline continues, artifacts uncommitted | Low | Log warning, non-fatal, user can manually commit |
| Commit message formatting breaks | Minor git log ugliness | Low | Unit tests validate format |
| Auto-commit performance overhead | Slight pipeline slowdown | Medium | Async commit in background (future optimization) |
| User wants manual control | Feature friction | Low | Config flag to disable |
| Merge conflicts in concurrent runs | Multiple SPECs running simultaneously | Low | Each SPEC commits to separate directory |

---

## Future Enhancements

### Phase 2 (Post-922)
- **Async commits**: Don't block pipeline on git operations
- **Batch commits**: Option to commit N stages at once
- **Custom commit templates**: User-defined commit message format
- **Commit hooks integration**: Pre-commit, commit-msg hooks

### Phase 3 (Advanced)
- **Selective auto-commit**: Only commit certain stages
- **Branch-per-SPEC**: Auto-create feature branches
- **PR automation**: Auto-create PR after unlock stage

---

## References

- **SPEC-900**: End-to-end validation (discovered this issue)
- **SPEC-920**: TUI automation (works, but blocked by dirty tree)
- **SPEC-921**: Tmux orchestration (separate issue)
- **Evidence**: SPEC-900 guardrail failure telemetry
- **Code**: `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs:850`

---

## Appendix: Example Git History

### Current (Manual Commits)
```
a1b2c3d feat(spec-900): add implement stage evidence and artifacts
(manual intervention required)
```

### With Auto-Commit (Target)
```
a1b2c3d feat(SPEC-KIT-900): complete unlock stage
b2c3d4e feat(SPEC-KIT-900): complete audit stage
c3d4e5f feat(SPEC-KIT-900): complete validate stage
d4e5f6g feat(SPEC-KIT-900): complete implement stage
e5f6g7h feat(SPEC-KIT-900): complete tasks stage
f6g7h8i feat(SPEC-KIT-900): complete plan stage
```

**Benefits**:
- Clear stage-by-stage progression
- Easy rollback to any stage
- Automated workflow enabled
- Guardrails enforced throughout
