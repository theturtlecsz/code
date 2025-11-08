**SPEC-ID**: SPEC-KIT-920
**Feature**: TUI Automation Support for Test Scripts
**Status**: Backlog
**Created**: 2025-11-07
**Priority**: P1 (blocks automated testing)
**Owner**: Code

---

## Problem Statement

Test automation scripts (`test-speckit-auto.sh`, `spec-kit-tools.sh`) cannot control the TUI because it doesn't respond to `tmux send-keys`. This blocks:
- Automated regression testing
- CI/CD pipeline integration
- Headless workflow validation
- SPEC-KIT-900 validation testing

**Current Workaround**: Manual testing only (run TUI interactively, type commands)

---

## Current Behavior

### What Happens
```bash
./scripts/spec-kit-tools.sh test SPEC-KIT-900

1. ✓ Builds binary
2. ✓ Starts TUI in tmux session
3. ✓ Sends command via: tmux send-keys "/speckit.auto SPEC-KIT-900" Enter
4. ✗ TUI never receives/processes the command
5. ⏸ TUI sits idle indefinitely
6. ⏸ Test script waits for completion that never comes
```

**Evidence**:
- TUI process running (responsive to manual commands)
- No database activity (no agents spawned)
- No file updates
- Test script started TUI 3+ hours ago, nothing happened

---

## Desired Behavior

### Option A: Stdin Command Support (Recommended)
```bash
# Start TUI with initial command
./code --command "/speckit.auto SPEC-KIT-900"

# Or pipe command
echo "/speckit.auto SPEC-KIT-900" | ./code --batch
```

### Option B: Command File Support
```bash
# Write command to file
echo "/speckit.auto SPEC-KIT-900" > /tmp/tui-command.txt

# TUI reads on startup
./code --command-file /tmp/tui-command.txt
```

### Option C: HTTP/Socket Control
```bash
# TUI listens on socket
./code --control-socket /tmp/code.sock &

# Send command via socket
echo "/speckit.auto SPEC-KIT-900" | nc -U /tmp/code.sock
```

---

## Requirements

### FR1: Command Line Argument
**Requirement**: TUI accepts `--command <slash-command>` flag
**Behavior**:
- Starts TUI
- Automatically executes specified command
- Continues running for interactive use after
- Exits on pipeline completion if --exit-on-complete flag set

**Example**:
```bash
./code --command "/speckit.auto SPEC-KIT-900" --exit-on-complete
```

### FR2: Batch Mode
**Requirement**: TUI accepts `--batch` flag for non-interactive execution
**Behavior**:
- Starts in headless mode (no terminal display)
- Reads commands from stdin
- Executes commands sequentially
- Exits after all commands complete
- Returns exit code 0 (success) or 1 (failure)

**Example**:
```bash
cat <<EOF | ./code --batch
/speckit.auto SPEC-KIT-900
/speckit.verify SPEC-KIT-900
EOF
echo "Exit code: $?"
```

### FR3: Automation-Friendly Output
**Requirement**: Structured output for parsing
**Behavior**:
- --json flag: Output progress/results as JSON lines
- --quiet flag: Suppress TUI, show only results
- --log-file <path>: Write full log to file

**Example**:
```bash
./code --command "/speckit.auto SPEC-KIT-900" --json --quiet > /tmp/result.json
```

---

## Acceptance Criteria

### AC1: Basic Automation
- [ ] `--command` flag executes single command
- [ ] Command completes successfully
- [ ] Database shows agent executions
- [ ] Output files created
- [ ] Evidence exported

### AC2: Test Script Integration
- [ ] `test-speckit-auto.sh` works end-to-end
- [ ] Can run: `./scripts/spec-kit-tools.sh test SPEC-KIT-900`
- [ ] Script detects completion correctly
- [ ] Script validates deliverables
- [ ] Script reports pass/fail

### AC3: CI/CD Ready
- [ ] Can run in headless environment
- [ ] Exit codes meaningful (0=success, 1=failure)
- [ ] Output parseable for automation
- [ ] Timeouts configurable
- [ ] No manual intervention required

---

## Technical Approach

### Phase 1: Command Argument Support (4-6 hours)

**Files to Modify**:
1. `cli/src/main.rs` - Add --command argument parsing
2. `tui/src/app.rs` - Accept initial command
3. `tui/src/chatwidget/mod.rs` - Auto-submit command after init

**Implementation**:
```rust
// cli/src/main.rs
#[derive(Parser)]
struct Args {
    #[arg(long)]
    command: Option<String>,

    #[arg(long)]
    exit_on_complete: bool,
}

// Pass to TUI
let initial_command = args.command;
```

```rust
// tui/src/app.rs
pub struct App {
    initial_command: Option<String>,
    exit_on_complete: bool,
}

// After TUI loads
if let Some(cmd) = self.initial_command.take() {
    // Auto-submit command
    self.handle_slash_command(&cmd);
}
```

### Phase 2: Completion Detection (2-3 hours)

**Add event for pipeline completion**:
```rust
// tui/src/app_event.rs
PipelineCompleted {
    spec_id: String,
    success: bool,
    duration_secs: u64,
}

// tui/src/app.rs
if self.exit_on_complete && pipeline_completed {
    std::process::exit(if success { 0 } else { 1 });
}
```

### Phase 3: Structured Output (2-3 hours)

**JSON progress events**:
```rust
// Optional JSON output
if args.json {
    println!("{{\"event\":\"stage_start\",\"stage\":\"plan\"}}");
    println!("{{\"event\":\"stage_complete\",\"stage\":\"plan\",\"duration\":120}}");
}
```

**Total Effort**: 8-12 hours

---

## Testing Plan

### Unit Tests
- [ ] --command argument parsing
- [ ] Initial command execution
- [ ] Exit-on-complete logic

### Integration Tests
```bash
# Test 1: Basic command execution
./code --command "/speckit.status SPEC-KIT-900" --exit-on-complete
echo "Exit code: $?"  # Should be 0

# Test 2: Full pipeline
timeout 3600 ./code --command "/speckit.auto SPEC-KIT-900" --exit-on-complete
echo "Exit code: $?"  # Should be 0 if successful

# Test 3: Error handling
./code --command "/speckit.auto NONEXISTENT" --exit-on-complete
echo "Exit code: $?"  # Should be 1
```

### Validation
- [ ] test-speckit-auto.sh runs successfully
- [ ] Database shows new agent executions
- [ ] Output files created
- [ ] Evidence exported
- [ ] Script validates deliverables
- [ ] Script reports results correctly

---

## Dependencies

**Blocks**:
- SPEC-KIT-900 validation with corrected prompts
- Automated regression testing
- CI/CD integration
- Nightly smoke tests

**Depends On**:
- None (standalone feature)

---

## Alternatives Considered

### Alternative 1: expect/pexpect
**Pros**: Can automate interactive programs
**Cons**: Complex, fragile, requires expect installed
**Verdict**: Rejected - better to fix TUI

### Alternative 2: Separate Batch Binary
**Pros**: Clean separation
**Cons**: Code duplication, two binaries to maintain
**Verdict**: Rejected - single binary better

### Alternative 3: Keep Manual Only
**Pros**: No code changes
**Cons**: Blocks automation, testing, CI/CD
**Verdict**: Rejected - automation critical

---

## Risk Analysis

**Technical Risks**:
- Low: Straightforward flag handling
- TUI lifecycle changes needed (moderate)
- Exit-on-complete may conflict with other features (low)

**Schedule Risks**:
- 8-12 hour implementation
- Testing adds 2-4 hours
- Total: 1-2 days

**Mitigation**:
- Start with minimal --command support
- Add --batch and --json incrementally
- Thorough testing with existing test suite

---

## Success Metrics

**Must Have**:
- `./code --command "/speckit.auto SPEC-ID"` works
- Test automation scripts functional
- Can run SPEC-KIT-900 validation headlessly

**Nice to Have**:
- --batch mode for multiple commands
- --json for structured output
- --exit-on-complete for CI/CD

**Success**: When `./scripts/spec-kit-tools.sh test SPEC-KIT-900` completes successfully without manual intervention

---

## Priority Justification

**P1 (High Priority)** because:
1. Blocks SPEC-KIT-900 validation (which blocks SPEC-KIT-070 validation)
2. Blocks automated testing infrastructure
3. Blocks CI/CD integration
4. Currently requires 45-50 minutes of manual babysitting per test

**Effort**: 1-2 days
**Impact**: Unblocks automated testing, enables CI/CD

---

**Created**: 2025-11-07
**Estimated Effort**: 8-12 hours implementation + 2-4 hours testing
**Priority**: P1 (blocks other work)
**Status**: Backlog (ready for implementation)
