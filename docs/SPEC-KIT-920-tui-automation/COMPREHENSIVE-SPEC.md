# SPEC-KIT-920: TUI Automation Support - COMPREHENSIVE SPECIFICATION

**SPEC-ID**: SPEC-KIT-920
**Feature**: Enable headless TUI automation for test scripts and CI/CD
**Status**: Implementation Complete ✅ (2025-11-09)
**Priority**: P1 (blocks SPEC-KIT-900 validation, automated testing, CI/CD)
**Effort**: 8-12 hours (1-2 days)
**Created**: 2025-11-07
**Owner**: Code

---

## 🎯 CORE PURPOSE

**Enable orchestration testing from within Claude Code without manual intervention.**

This feature exists to allow Claude Code (and CI/CD) to:
1. ✅ **Run `/speckit.auto` pipelines automatically** (45-50 min multi-agent workflows)
2. ✅ **Test orchestration workflows headlessly** (via PTY wrapper, no human required)
3. ✅ **Validate multi-agent consensus** in automated test suites
4. ✅ **Execute spec-kit commands programmatically** from scripts

**Before SPEC-920**: Manual testing only (human must type commands in TUI)
**After SPEC-920**: Fully automated (scripts can trigger and monitor TUI commands)

**Critical Use Case**: Claude Code running `bash scripts/validate-spec-kit-920.sh` to test orchestration automatically, completing the development-test loop without leaving the AI session.

---

## Problem Statement (Why This Exists)

### Current Blocker

Test automation scripts **cannot control the TUI**, blocking:
1. ❌ SPEC-KIT-900 validation (need to test with corrected prompts)
2. ❌ Automated regression testing
3. ❌ CI/CD pipeline integration
4. ❌ Headless workflow validation
5. ❌ Nightly smoke tests
6. ❌ **Claude Code orchestration testing** (the primary blocker)

**Current Workaround**: Manual testing only (45-50 min per test, error-prone, requires human)

### What Happens Now

```bash
./scripts/spec-kit-tools.sh test SPEC-KIT-900

Step 1: ✓ Build binary
Step 2: ✓ Kill old sessions
Step 3: ✓ Start TUI in tmux
Step 4: ✓ Send command via tmux send-keys
Step 5: ✗ TUI never receives/processes command
Step 6: ⏸ TUI sits idle for hours
Step 7: ⏸ Test script waits for completion that never comes
Step 8: ✗ Script exits with false positive or timeout
```

**Root Cause**: TUI uses alternate screen buffer and custom input handling - `tmux send-keys` doesn't reach the TUI's input system.

**Evidence**:
- TUI process running 2h50m (responsive to manual commands)
- Zero database activity (no agents spawned)
- No file updates
- Test script thinks it completed but nothing happened

---

## Solution Design

### Approach: Add `--initial-command` Flag

**Minimal implementation** (8-12 hours):
```rust
// Enable:
./code --initial-command "/speckit.auto SPEC-KIT-900"

// Behavior:
1. TUI starts normally
2. After initialization, auto-submits slash command
3. Command executes as if user typed it
4. TUI continues running (can be backgrounded in tmux)
```

**Enables**: Test scripts to trigger commands, monitor completion, validate results

---

## Requirements

### FR1: Initial Command Flag
**Requirement**: TUI accepts `--initial-command <slash-command>` argument

**Behavior**:
- Start TUI normally (full initialization)
- After TUI ready (config loaded, widgets initialized)
- Auto-submit specified slash command
- Execute exactly as if user typed it
- TUI continues running after command completes

**Example**:
```bash
./code --initial-command "/speckit.auto SPEC-KIT-900"
```

**Acceptance**:
- [ ] Flag accepted by CLI parser
- [ ] Command stored during initialization
- [ ] Command auto-submitted after TUI ready
- [ ] Command executes correctly
- [ ] Database shows agent activity
- [ ] Output files created
- [ ] Evidence exported

### FR2: Command Validation
**Requirement**: Validate command before execution

**Behavior**:
- Check command starts with `/`
- Verify command is a valid slash command
- If invalid, show error and continue to interactive mode

**Example**:
```bash
./code --initial-command "invalid"
# Shows: "Error: --initial-command must be a slash command (start with /)"
# Continues to interactive TUI
```

### FR3: Completion Detection (Optional but Recommended)
**Requirement**: Detect when initial command completes

**Behavior**:
- Track initial command execution
- Detect completion (success or failure)
- Optional `--exit-on-complete` flag to exit after initial command

**Example**:
```bash
./code --initial-command "/speckit.status SPEC-KIT-900" --exit-on-complete
# Executes command, then exits with code 0 (success) or 1 (failure)
```

**Use Case**: Enables scripting:
```bash
if ./code --initial-command "/speckit.status SPEC-KIT-900" --exit-on-complete; then
    echo "SPEC is ready"
else
    echo "SPEC has issues"
fi
```

---

## Implementation Plan

### Phase 1: CLI Argument (1 hour)

**File**: `codex-rs/tui/src/cli.rs`

**Add field**:
```rust
pub struct Cli {
    // ... existing fields ...

    /// Initial slash command to execute after TUI starts (for automation).
    /// Example: --initial-command "/speckit.auto SPEC-KIT-900"
    #[arg(long = "initial-command")]
    pub initial_command: Option<String>,

    /// Exit after initial command completes (for automation).
    #[arg(long = "exit-on-complete", requires = "initial_command")]
    pub exit_on_complete: bool,
}
```

**Test**:
```bash
./code --initial-command "/speckit.status SPEC-KIT-900" --help
# Should show the new flags
```

---

### Phase 2: Pass Through Initialization (2 hours)

**File**: `codex-rs/tui/src/app.rs`

**Update ChatWidgetArgs**:
```rust
pub(crate) struct ChatWidgetArgs {
    // ... existing fields ...
    initial_command: Option<String>, // SPEC-KIT-920
}
```

**Update construction sites** (multiple locations in app.rs, lib.rs):
```rust
let chat_widget_args = ChatWidgetArgs {
    config: config.clone(),
    initial_prompt,
    initial_images,
    initial_command: cli.initial_command.clone(), // SPEC-KIT-920
    // ... rest ...
};
```

**Files to modify**:
- `codex-rs/tui/src/lib.rs` (main initialization)
- `codex-rs/tui/src/app.rs` (ChatWidgetArgs construction - 2-3 sites)
- `codex-rs/tui/src/chatwidget/mod.rs` (ChatWidget::new signature)

**Test**: Code compiles

---

### Phase 3: Auto-Submit Command (3-4 hours)

**File**: `codex-rs/tui/src/chatwidget/mod.rs` or `codex-rs/tui/src/app.rs`

**Detect TUI Ready**:
```rust
// In App::run() or first event loop iteration
if let Some(initial_cmd) = self.initial_command.take() {
    // Validate slash command
    if !initial_cmd.starts_with('/') {
        // Show error, continue to interactive
        self.show_error("--initial-command must start with /");
    } else {
        // Auto-submit command
        tracing::info!("SPEC-KIT-920: Auto-submitting initial command: {}", initial_cmd);
        self.handle_slash_command_input(&initial_cmd);
    }
}
```

**Key**: Find the right place to inject this (after all initialization, before first render)

**Test**:
```bash
./code --initial-command "/speckit.status SPEC-KIT-900"
# Should auto-execute command after TUI loads
```

---

### Phase 4: Completion Detection (2-3 hours, Optional)

**Track command completion**:
```rust
// In App struct
initial_command_executing: bool,
exit_on_complete: bool,

// When initial command starts
self.initial_command_executing = true;

// When pipeline completes (or command finishes)
if self.initial_command_executing && self.exit_on_complete {
    // Determine success/failure
    let exit_code = if pipeline_successful { 0 } else { 1 };
    tracing::info!("SPEC-KIT-920: Initial command complete, exiting with code {}", exit_code);
    std::process::exit(exit_code);
}
```

**Hook points**:
- Pipeline completion event
- Error events
- Command completion detection

**Test**:
```bash
./code --initial-command "/speckit.status SPEC-KIT-900" --exit-on-complete
echo "Exit code: $?"
```

---

### Phase 5: Integration Testing (2 hours)

**Test Cases**:

1. **Basic command execution**:
```bash
./code --initial-command "/speckit.status SPEC-KIT-900"
# TUI starts, command executes, TUI stays running
```

2. **Full pipeline automation**:
```bash
./code --initial-command "/speckit.auto SPEC-KIT-900"
# TUI starts, pipeline executes (45-50 min), TUI stays running
```

3. **Exit on complete**:
```bash
./code --initial-command "/speckit.status SPEC-KIT-900" --exit-on-complete
echo $?  # Should be 0 or 1
```

4. **Invalid command**:
```bash
./code --initial-command "not-a-slash-command"
# Shows error, continues to interactive
```

5. **Test script integration**:
```bash
./scripts/spec-kit-tools.sh test SPEC-KIT-900
# Should work end-to-end
```

6. **Tmux integration**:
```bash
tmux new-session -d -s test "cd /home/thetu/code && ./codex-rs/target/dev-fast/code --initial-command '/speckit.auto SPEC-KIT-900'"
# TUI runs in background, command executes
```

---

## Files to Modify

### Core Implementation (4-6 files)

1. **codex-rs/tui/src/cli.rs** (+5 lines)
   - Add `initial_command: Option<String>` field
   - Add `exit_on_complete: bool` field

2. **codex-rs/tui/src/app.rs** (+20-30 lines)
   - Add `initial_command` to ChatWidgetArgs struct
   - Add `initial_command_executing` tracking
   - Add `exit_on_complete` flag
   - Add completion detection logic

3. **codex-rs/tui/src/lib.rs** (+10-15 lines)
   - Pass `cli.initial_command` through initialization
   - Update ChatWidgetArgs construction

4. **codex-rs/tui/src/chatwidget/mod.rs** (+15-20 lines)
   - Store initial_command in ChatWidget
   - Detect TUI ready state
   - Auto-submit command

5. **codex-rs/tui/src/app_event.rs** (+10 lines, optional)
   - Add InitialCommandComplete event
   - For clean completion tracking

6. **scripts/tui-session.sh** (+2 lines)
   - Update to use --initial-command instead of send-keys

### Testing Files (2 files)

7. **scripts/test-speckit-auto.sh** (+5 lines)
   - Update to use --initial-command flag

8. **tests/spec_kit_automation.rs** (new file, +100 lines)
   - Integration tests for automation
   - Test all scenarios above

---

## Acceptance Criteria

### Must Have (MVP)
- [ ] `--initial-command` flag accepted
- [ ] Command auto-submitted after TUI ready
- [ ] Command executes correctly (database activity)
- [ ] Output files created
- [ ] Evidence exported
- [ ] Test script works: `./scripts/spec-kit-tools.sh test SPEC-KIT-900`

### Should Have
- [ ] `--exit-on-complete` flag works
- [ ] Exit codes meaningful (0=success, 1=failure)
- [ ] Invalid command handling (error + continue)
- [ ] Tmux integration tested

### Nice to Have
- [ ] `--json` output for parsing
- [ ] `--quiet` mode (suppress TUI, show only results)
- [ ] Multiple commands support
- [ ] Command file support (--command-file)

---

## Testing Procedure

### After Implementation

1. **Unit test the flag**:
```bash
./code --help | grep "initial-command"
# Should show the new flag
```

2. **Test basic command**:
```bash
./code --initial-command "/speckit.status SPEC-KIT-900"
# Watch TUI - command should auto-execute
```

3. **Test automation**:
```bash
./scripts/spec-kit-tools.sh test SPEC-KIT-900
# Should complete successfully (45-50 min)
```

4. **Validate results**:
```bash
./scripts/spec-kit-tools.sh validate SPEC-KIT-900 plan
# Should show PASS (with corrected prompts)
```

---

## Dependencies & Blockers

### Blocks
- SPEC-KIT-900 validation testing
- SPEC-KIT-070 cost validation
- Automated regression testing
- CI/CD integration

### Blocked By
- None (standalone feature)

### Depends On
- Corrected prompts (already merged: commit 5ffa267ae) ✓
- Session 3 fixes (already merged) ✓

---

## Risk Analysis

### Technical Risks
- **Low**: Straightforward argument passing
- **Medium**: Finding right injection point for auto-submit
- **Low**: Exit-on-complete may need pipeline completion event

### Testing Risks
- **Low**: Can test manually first
- **Medium**: Tmux integration needs validation
- **Low**: Error handling straightforward

### Schedule Risks
- **Medium**: Estimated 8-12 hours could expand to 12-16
- **Mitigation**: Start with MVP (no exit-on-complete), add later

---

## Success Criteria

### Primary Success
**When this works**:
```bash
./scripts/spec-kit-tools.sh test SPEC-KIT-900
# Output:
✓ Build complete
✓ Session started
✓ Command sent
✓ Command completed (after 2847s)
✓ All deliverables validated
✓ Test PASSED
```

### Validation Success
**SPEC-KIT-900 deliverables** (with corrected prompts):
- plan.md: Reminder microservice work breakdown ✓
- tasks.md: 8-12 tasks to build sync service ✓
- implement.md: Code proposals (5-20KB) ✓
- validate.md: Test strategy for microservice ✓
- audit.md: Compliance checks ✓
- unlock.md: Final approval ✓

**No meta-analysis**, **no debug logs**, **actual smoke test content** ✓

### SPEC-KIT-070 Validation Success
**Can benchmark**:
- Run with premium models → capture cost
- Run with cheap models → capture cost
- Compare results
- Validate 75% cost reduction claim
- Confirm quality equivalent

---

## Current State

### What's Done
- ✅ SPEC-KIT-920 spec created
- ✅ Test automation scripts exist (but can't control TUI)
- ✅ Validation scripts exist
- ✅ Prompts fixed (commit 5ffa267ae)
- ✅ Infrastructure working (Session 3)

### What's Needed
- ❌ `--initial-command` implementation
- ❌ Auto-submit logic
- ❌ Completion detection
- ❌ Script integration

### Estimated Timeline
- Phase 1 (CLI arg): 1 hour
- Phase 2 (Pass through): 2 hours
- Phase 3 (Auto-submit): 3-4 hours
- Phase 4 (Completion): 2-3 hours
- Phase 5 (Testing): 2 hours
**Total**: 10-12 hours

---

## Related Work

### Leverages
- Session 3: Audit infrastructure (run_id, logging, verification)
- Session 3: Direct results refactor (collection fixes)
- Session 3: Automatic evidence export
- Prompt fix: 5ffa267ae (workload execution, not meta-analysis)

### Enables
- SPEC-KIT-900 validation with corrected prompts
- SPEC-KIT-070 cost benchmarking
- Future automated testing
- CI/CD integration
- Nightly regression tests

---

## Alternative Approaches Considered

### Alternative 1: Fix tmux send-keys
**Idea**: Make TUI respond to tmux send-keys
**Issues**: TUI uses alternate screen, complex terminal handling
**Verdict**: Rejected - intrusive, fragile

### Alternative 2: Headless mode
**Idea**: Run TUI without terminal (`--batch` mode)
**Issues**: Major refactor, TUI heavily tied to ratatui
**Verdict**: Deferred - phase 2 enhancement

### Alternative 3: HTTP API
**Idea**: TUI exposes HTTP endpoint for commands
**Issues**: Over-engineered for this use case
**Verdict**: Rejected - too complex

### Alternative 4: Keep manual
**Idea**: No automation, always manual
**Issues**: Blocks testing, CI/CD, validation
**Verdict**: Rejected - automation critical

**Chosen**: `--initial-command` flag (simplest, most effective)

---

## Implementation Notes

### Code Locations

**CLI Argument** (codex-rs/tui/src/cli.rs:105+):
```rust
/// Initial slash command to execute after TUI starts
#[arg(long = "initial-command")]
pub initial_command: Option<String>,
```

**ChatWidgetArgs** (codex-rs/tui/src/app.rs:188+):
```rust
initial_command: Option<String>, // SPEC-KIT-920
```

**Auto-Submit Logic** (find in app.rs after initialization):
```rust
// After TUI ready, before first render
if let Some(cmd) = self.initial_command.take() {
    if cmd.starts_with('/') {
        tracing::info!("Auto-submitting: {}", cmd);
        self.handle_slash_command_input(&cmd);
    }
}
```

**Update Test Script** (scripts/tui-session.sh:70):
```bash
# OLD:
tmux new-session -d -s "$SESSION_NAME" -c "$REPO_ROOT" "$BINARY"
sleep 2
tmux send-keys -t "$SESSION_NAME" "$command" Enter

# NEW:
tmux new-session -d -s "$SESSION_NAME" -c "$REPO_ROOT" "$BINARY --initial-command '$command'"
```

---

## Testing Checklist

After implementation:

### Manual Tests
- [ ] `./code --initial-command "/speckit.status SPEC-KIT-900"` executes command
- [ ] Command shows in TUI history
- [ ] Database updated (for multi-agent commands)
- [ ] Files created (for pipeline commands)
- [ ] TUI continues running after command
- [ ] Invalid command shows error, continues

### Automation Tests
- [ ] `./scripts/spec-kit-tools.sh test SPEC-KIT-900` completes
- [ ] Database shows new run_id
- [ ] All 6 stages execute
- [ ] Output files created
- [ ] Evidence exported (12 files)
- [ ] Validation passes

### Edge Cases
- [ ] Empty command (error handling)
- [ ] Command with quotes (escaping)
- [ ] Long-running command (timeout)
- [ ] Failed command (error propagation)
- [ ] Ctrl-C during execution (cleanup)

---

## Rollout Plan

### Step 1: Implement & Test (Day 1)
- Morning: Phases 1-2 (CLI arg, pass through)
- Afternoon: Phase 3 (auto-submit)
- Evening: Local testing

### Step 2: Integration & Validation (Day 2 morning)
- Phase 4: Completion detection
- Phase 5: Integration testing
- Validate with SPEC-KIT-900

### Step 3: Deploy & Document (Day 2 afternoon)
- Update documentation
- Update test scripts
- Commit and push
- Run validation

---

## Success Metrics

**Must Achieve**:
1. ✓ Test script completes without manual intervention
2. ✓ SPEC-KIT-900 validation shows deliverables match workload
3. ✓ Can run SPEC-KIT-070 benchmarking

**Nice to Achieve**:
4. ✓ Exit codes enable scripting
5. ✓ CI/CD ready (headless execution)
6. ✓ Multiple commands supported

**Failure Modes**:
- Command not submitted (TUI sits idle)
- Command submitted but not executed (wrong timing)
- TUI crashes (initialization order)
- Script can't detect completion (timeout)

---

## Documentation Updates Needed

### User-Facing
- README.md: Add --initial-command example
- CLAUDE.md: Add automation section
- scripts/README-TOOLKIT.md: Update usage examples

### Developer-Facing
- Architecture doc: Explain initialization flow
- Testing guide: How to write automated tests
- SPEC-KIT-920/spec.md: Mark as DONE

---

**Prepared**: 2025-11-07 20:00 UTC
**Effort**: 10-12 hours (1-2 focused days)
**Priority**: P1 (unblocks testing)
**Status**: Ready for implementation
