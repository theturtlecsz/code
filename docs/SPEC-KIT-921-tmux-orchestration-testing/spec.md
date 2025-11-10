# SPEC-KIT-921: Tmux Orchestration Testing and Validation

**Status**: Draft
**Created**: 2025-11-10
**Priority**: P1 (Critical - blocks SPEC-900 full automation)
**Owner**: Code
**Dependencies**: SPEC-KIT-920 (TUI Automation Support)

---

## Problem Statement

SPEC-920 delivered tmux automation infrastructure (scripts/tmux-automation.sh) that successfully automates individual TUI commands (verified with 6 smoke tests + integration test). However, **critical bug discovered** during SPEC-900 validation:

**Observed Behavior**:
- Manual TUI execution of `/speckit.auto SPEC-KIT-900` works perfectly ✅
- Tmux-automated execution exits immediately after 6-8 seconds ❌
- Shows "Resume from: Plan" message then returns to prompt
- No error messages visible in TUI output
- Happens consistently across multiple runs despite state cleanup

**Impact**:
- SPEC-900 end-to-end validation blocked (cannot capture 60+ minute automated runs)
- Automated CI/CD testing unreliable for long-running commands
- Evidence capture incomplete for full pipeline executions

---

## Root Cause Analysis

### Hypothesis 1: Command Timing Issue
**Evidence**:
- Tmux script waits 12s for TUI initialization (line 102-113 in tmux-automation.sh)
- Uses "Ctrl+H help" marker to detect readiness
- May not account for command-specific initialization delays

**Test**: Add configurable post-init delay before sending command

### Hypothesis 2: Command Delivery Mechanism
**Evidence**:
- Uses `tmux send-keys -t "$SESSION" "$cmd" Enter` (line 139)
- Works for short commands (/speckit.status, /speckit.tasks)
- Fails for complex orchestration (/speckit.auto)

**Test**: Compare send-keys with literal vs C-m, add verification step

### Hypothesis 3: State Detection Logic
**Evidence**:
- Manual run works after same state cleanup (SPEC.md entry, SQLite clear, file renames)
- Tmux run fails with identical state
- Suggests timing-sensitive state checks in pipeline_coordinator.rs

**Test**: Add trace logging to capture pipeline state at initialization

### Hypothesis 4: Environment Differences
**Evidence**:
- Manual TUI: full terminal with user environment
- Tmux TUI: background session with potential env var differences
- Different working directory contexts

**Test**: Capture and compare environment variables between manual and tmux runs

---

## Success Criteria

### Primary Goals
1. **Automated /speckit.auto execution**: Tmux automation successfully runs full 60+ minute pipeline
2. **Evidence capture**: Complete stage artifacts captured for all 6 stages
3. **Reliability**: 100% success rate across 5 consecutive automated runs
4. **Parity**: Tmux behavior identical to manual TUI execution

### Acceptance Criteria
- [ ] Root cause identified with reproducible test case
- [ ] Fix implemented in tmux-automation.sh or TUI command handling
- [ ] SPEC-900 automated run completes all 6 stages via tmux
- [ ] Evidence chain complete (plan.md → unlock.md)
- [ ] Cost tracking accurate ($2.70 target validated)
- [ ] Regression tests added to prevent recurrence

### Non-Goals
- Headless TUI mode (already rejected in SPEC-920, tmux is superior)
- Alternative automation frameworks (tmux proven effective for short commands)
- Manual TUI workflow changes (works perfectly, don't break it)

---

## Technical Approach

### Phase 1: Diagnostic Enhancement (2 hours)
**Goal**: Capture detailed telemetry to identify root cause

1. **Enhanced Logging**:
   - Add trace-level logging to pipeline_coordinator.rs:handle_spec_auto
   - Log state.stages.len(), state.current_index, resume_from value
   - Capture timestamp of each advance_spec_auto call

2. **Tmux Script Instrumentation**:
   - Log exact command sent via send-keys
   - Capture tmux pane content after command send
   - Add --debug flag for verbose output

3. **Comparison Test**:
   - Run manual TUI with RUST_LOG=trace
   - Run tmux TUI with same logging
   - Diff the two logs to identify divergence point

### Phase 2: Hypothesis Testing (3 hours)
**Goal**: Systematically test each hypothesis

1. **Timing Tests**:
   - Add 5s, 10s, 15s delays after TUI init before send-keys
   - Test with --delay flag: `tmux-automation.sh --init-delay 10 ...`
   - Measure impact on success rate

2. **Delivery Method Tests**:
   - Test `send-keys "$cmd" C-m` instead of Enter
   - Test character-by-character delivery: `send-keys -l "$cmd"`
   - Test paste buffer: `load-buffer` + `paste-buffer`

3. **State Verification Tests**:
   - Add pre-flight check: verify SPEC.md entry before command
   - Add SQLite query: confirm no stale records before run
   - Add file check: verify no stage files exist

4. **Environment Tests**:
   - Export all env vars from manual session
   - Apply to tmux session before TUI start
   - Test with identical working directory

### Phase 3: Fix Implementation (2 hours)
**Goal**: Implement validated solution

**Likely Fix Options**:
1. **Add initialization delay** (if timing issue):
   ```bash
   tmux send-keys -t "$SESSION" "$cmd" Enter
   sleep 2  # Allow command processing
   ```

2. **Enhanced readiness detection** (if timing issue):
   ```bash
   wait_for_prompt() {
       while ! tmux capture-pane -t "$SESSION" -p | grep -q "^> "; do
           sleep 0.5
       done
   }
   wait_for_prompt
   tmux send-keys -t "$SESSION" "$cmd" Enter
   ```

3. **Improve command delivery** (if delivery issue):
   ```bash
   # Use literal C-m instead of Enter
   tmux send-keys -t "$SESSION" "$cmd" C-m
   ```

4. **Add verification step** (if state issue):
   ```bash
   send_command "$cmd"
   sleep 1
   if tmux capture-pane -t "$SESSION" -p | grep -q "Resume from: Plan"; then
       # Command didn't execute properly, retry
   fi
   ```

### Phase 4: Validation (1 hour)
**Goal**: Confirm fix resolves issue

1. **SPEC-900 Full Run**:
   - Execute: `tmux-automation.sh SPEC-KIT-900 "/speckit.auto SPEC-KIT-900" 3900`
   - Monitor for 60+ minutes
   - Verify all 6 stages complete
   - Check evidence files generated

2. **Regression Tests**:
   - Re-run SPEC-920 smoke tests (ensure no breakage)
   - Test other long commands (/speckit.validate with 286s consensus)
   - Verify short commands still work (/speckit.status, /speckit.tasks)

3. **Documentation Update**:
   - Update TMUX-AUTOMATION-README.md with findings
   - Document command timing considerations
   - Add troubleshooting section

---

## Testing Strategy

### Unit Tests
- Command parsing and delivery
- Initialization sequence timing
- Environment variable handling

### Integration Tests
- Full SPEC-900 automated run (60+ min)
- Multiple concurrent tmux sessions
- Recovery from interrupted sessions

### Smoke Tests
- Short command execution (< 30s)
- Medium command execution (30s - 5min)
- Long command execution (5min+)
- Very long command execution (60min+) ← NEW

---

## Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Root cause not timing/delivery-related | High | Medium | Comprehensive logging to identify actual issue |
| Fix breaks existing short-command automation | Medium | Low | Full regression test suite before merge |
| Environment-specific issue (not reproducible) | High | Low | Test on multiple machines/environments |
| Requires TUI code changes (complex) | Medium | Low | Prefer script-side fix, escalate if needed |

---

## Success Metrics

### Primary
- **Automated SPEC-900 completion**: 100% (was 0%)
- **Evidence completeness**: 6/6 stages (was 2/6)
- **Execution time**: ~60 min (was 6-8s false positive)
- **Cost accuracy**: $2.70 ± 10% variance

### Secondary
- **Debugging time**: < 8 hours total
- **Code changes**: Minimal (prefer config over code)
- **Test coverage**: 100% pass rate maintained
- **Documentation quality**: Complete root cause analysis documented

---

## Timeline

**Total Estimate**: 8 hours (1 focused session)

- **Phase 1** (Diagnostics): 2 hours
- **Phase 2** (Testing): 3 hours
- **Phase 3** (Implementation): 2 hours
- **Phase 4** (Validation): 1 hour

**Target Completion**: 2025-11-11

---

## References

- **SPEC-920**: TUI Automation Support (foundation)
- **SPEC-900**: End-to-end validation (blocked by this issue)
- **Evidence**: `codex-rs/evidence/tmux-automation/SPEC-KIT-900/` (6-8s immediate exits)
- **Scripts**: `codex-rs/scripts/tmux-automation.sh` (L139: send-keys implementation)
- **Code**: `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` (L119-218: advance logic)

---

## Appendix: Discovery Session Log

**Date**: 2025-11-10
**Context**: SPEC-900 validation attempt revealed tmux automation gap

**Timeline**:
1. Attempted SPEC-900 via tmux: Failed (6s exit)
2. Fixed SPEC.md tracker row: Still failed
3. Cleared SQLite state (695 records): Still failed
4. Renamed stage output files: Still failed
5. **Manual TUI test**: SUCCESS ✅
6. **Conclusion**: Issue is tmux delivery mechanism, not state or code logic

**Key Insight**: Manual vs automated behavior divergence indicates environment or timing-sensitive issue in command delivery layer, NOT in core pipeline logic.
