# Next Session: SPEC-KIT-920 Implementation

**Task**: Implement `--initial-command` flag for TUI automation
**Priority**: P1 (blocks SPEC-KIT-900 validation and all automated testing)
**Effort**: 10-12 hours (1-2 focused sessions)

---

## Context

**Repository**: theturtlecsz/code (fork of just-every/code)
**Branch**: main (clean, up to date)
**Location**: /home/thetu/code

**Problem**: Test automation scripts cannot control TUI (tmux send-keys doesn't work)
**Impact**: Cannot run automated tests, blocks SPEC-KIT-900 validation, blocks CI/CD

---

## What to Implement

### Add `--initial-command` Flag to TUI

**Enables**:
```bash
./code --initial-command "/speckit.auto SPEC-KIT-900"
```

**Behavior**:
1. TUI starts normally
2. After initialization completes
3. Auto-submits specified slash command
4. Command executes as if user typed it
5. TUI continues running

**Result**: Test scripts can trigger commands, wait for completion, validate results

---

## Implementation Plan (10-12 hours)

### Phase 1: CLI Argument (1 hour)
**File**: `codex-rs/tui/src/cli.rs`
- Add `initial_command: Option<String>` field with #[arg(long = "initial-command")]
- Add `exit_on_complete: bool` field (optional, for later)

### Phase 2: Pass Through (2 hours)
**Files**: `codex-rs/tui/src/app.rs`, `codex-rs/tui/src/lib.rs`
- Add `initial_command` to ChatWidgetArgs struct
- Pass `cli.initial_command` through all initialization paths
- Update ChatWidget::new() signature if needed

### Phase 3: Auto-Submit (3-4 hours)
**Files**: `codex-rs/tui/src/app.rs` or `codex-rs/tui/src/chatwidget/mod.rs`
- Detect when TUI is ready (after initialization, before first user input)
- Auto-submit command: `self.handle_slash_command_input(&cmd)`
- Validate command starts with '/'
- Log execution for debugging

### Phase 4: Testing (2 hours)
- Test basic: `./code --initial-command "/speckit.status SPEC-KIT-900"`
- Test pipeline: `./code --initial-command "/speckit.auto SPEC-KIT-900"`
- Update scripts/tui-session.sh to use flag
- Run: `./scripts/spec-kit-tools.sh test SPEC-KIT-900`

### Phase 5: Validation (2 hours)
- Wait for pipeline completion (~45-50 min)
- Run: `./scripts/spec-kit-tools.sh validate SPEC-KIT-900 [stage]`
- Verify deliverables have corrected content (reminder microservice, not meta-analysis)
- Confirm SPEC-KIT-900 can validate SPEC-KIT-070

---

## Key Files to Modify

1. `codex-rs/tui/src/cli.rs` - Add CLI argument
2. `codex-rs/tui/src/app.rs` - Add to ChatWidgetArgs, pass through
3. `codex-rs/tui/src/lib.rs` - Wire through initialization
4. `codex-rs/tui/src/chatwidget/mod.rs` - Auto-submit logic
5. `scripts/tui-session.sh` - Use --initial-command flag

---

## Testing Strategy

### Quick Validation (5 minutes)
```bash
# After implementation
./code --initial-command "/speckit.status SPEC-KIT-900"
# Should auto-execute, show status
```

### Full Validation (50 minutes)
```bash
# Full pipeline test
./scripts/spec-kit-tools.sh test SPEC-KIT-900
# Should complete all 6 stages automatically
```

### Result Validation
```bash
# Check deliverables
./scripts/spec-kit-tools.sh validate SPEC-KIT-900 plan
./scripts/spec-kit-tools.sh validate SPEC-KIT-900 tasks
# etc.

# Expect: PASS (no meta-analysis, actual reminder microservice content)
```

---

## Expected Outcomes

### Immediate
- ✓ `--initial-command` flag works
- ✓ Test automation functional
- ✓ Can run SPEC-KIT-900 headlessly

### Follow-On
- ✓ SPEC-KIT-900 validation completes (with corrected prompts)
- ✓ Deliverables show reminder microservice content
- ✓ Can use for SPEC-KIT-070 benchmarking
- ✓ CI/CD integration possible

---

## Documentation

**Complete SPEC**: `docs/SPEC-KIT-920-tui-automation/COMPREHENSIVE-SPEC.md`

**Read Before Starting**:
1. COMPREHENSIVE-SPEC.md (this task)
2. docs/SPEC-KIT-920-tui-automation/spec.md (original SPEC)
3. DEBUG-REPORT-SPEC-KIT-900.md (why automation is needed)

**Reference During**:
- Implementation plan (phases 1-5)
- Code locations (files to modify)
- Testing procedure (validation steps)

---

## Restart Prompt

```
I'm implementing SPEC-KIT-920: adding --initial-command flag to TUI for automation.

Context:
- Repository: /home/thetu/code (theturtlecsz/code fork)
- Branch: main (clean, up to date)
- Task: Enable headless TUI command execution
- Blocker: Test automation can't control TUI (tmux send-keys doesn't work)

Objective:
Implement --initial-command flag so test scripts can trigger /speckit.auto commands automatically.

Implementation:
1. Add CLI argument (codex-rs/tui/src/cli.rs)
2. Pass through initialization (app.rs, lib.rs)
3. Auto-submit after TUI ready (chatwidget/mod.rs)
4. Update test scripts (scripts/tui-session.sh)
5. Test and validate

Effort: 10-12 hours
Priority: P1 (blocks SPEC-KIT-900 validation)

Read: docs/SPEC-KIT-920-tui-automation/COMPREHENSIVE-SPEC.md for complete plan

Ready to implement.
```

---

**Prepared**: 2025-11-07 20:05 UTC
**Repository**: Clean, ready for work
**SPEC**: Complete and documented
**Priority**: High (blocks testing)
