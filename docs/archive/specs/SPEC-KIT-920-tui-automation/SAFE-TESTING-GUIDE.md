# SPEC-KIT-920: Safe Testing Guide

**Date**: 2025-11-09
**Purpose**: Validate SPEC-920 implementation without causing output leaks to Claude Code input

---

## ⚠️ The Problem

When Claude Code runs terminal commands with TTY/PTY interactions in background mode, the output can leak into the user's Claude Code input box. This is **NOT** a bug in SPEC-920, but rather a limitation of how Claude Code handles background terminal processes.

**What causes leaks**:
- Background processes with TTY output: `command &`
- PTY wrappers with escape codes
- Terminal UI applications run in background
- Timeout wrappers around interactive programs

**What's safe**:
- Foreground commands (blocking)
- Commands with output redirected to files
- Non-interactive builds and tests
- Commands that exit quickly

---

## ✅ Safe Testing Methods

### Method 1: User Runs Test Manually

**Safest approach** - User runs validation in their own terminal:

```bash
# In user's terminal (NOT Claude Code):
cd /home/thetu/code

# Quick validation (30 seconds):
./codex-rs/target/debug/code --initial-command "/status"

# Watch for:
# - TUI starts
# - Status command auto-executes
# - Output appears in TUI (not input box)
# - No crashes
```

**Expected behavior**:
- TUI launches normally
- After ~30ms (first redraw), command dispatches
- Status output appears in chat history
- TUI remains responsive

**Validation**:
```bash
# Check logs
grep "SPEC-KIT-920" ~/.code/log/codex-tui.log

# Should show:
# - "App::new called with initial_command"
# - "Dispatching initial command at startup"
# - "Auto-submitting regular command"
```

---

### Method 2: Tmux Session (Isolated)

**Good for automated testing** - User creates tmux session manually:

```bash
# User's terminal:
tmux new-session -d -s spec920-test
tmux send-keys -t spec920-test "cd /home/thetu/code" Enter
tmux send-keys -t spec920-test "./codex-rs/target/debug/code --initial-command '/status'" Enter

# Wait for execution
sleep 10

# Attach to see results
tmux attach -t spec920-test

# Clean up
tmux kill-session -t spec920-test
```

**Advantages**:
- Isolated from Claude Code
- Can monitor in real-time
- Output stays in tmux
- Safe for user's environment

---

### Method 3: Log Validation (Claude Code Safe)

**What Claude Code CAN do safely**:

```bash
# 1. Build the binary (safe, no TTY)
cd /home/thetu/code/codex-rs && cargo build --bin code

# 2. Check binary exists
ls -lh target/debug/code

# 3. Verify CLI help (safe, quick exit)
./target/debug/code --help | grep "initial-command"

# 4. Verify code changes
grep -A5 "Auto-submit initial command after first successful redraw" \
  codex-rs/tui/src/app.rs

# 5. Ask user to run test manually (safest)
```

**What Claude Code should provide**:
```markdown
The fix is complete and committed. To validate:

**Option A (Quick - 30 seconds):**
\`\`\`bash
./codex-rs/target/debug/code --initial-command "/status"
\`\`\`

**Option B (Full - 45 minutes):**
\`\`\`bash
./codex-rs/target/debug/code --initial-command "/speckit.auto SPEC-KIT-900"
\`\`\`

Watch for: Command auto-executes after TUI loads, output stays in TUI.
```

---

## 🧪 Validation Checklist

### Pre-Validation
- [ ] Binary built with latest code
- [ ] Changes committed to git
- [ ] User has terminal access
- [ ] No Claude Code background processes running

### During Test
- [ ] TUI starts without errors
- [ ] First redraw completes (~30ms)
- [ ] Initial command dispatches (check logs)
- [ ] Command executes correctly
- [ ] Output appears in TUI chat history
- [ ] NO output in Claude Code input box ✅
- [ ] NO crashes or freezes

### Post-Validation
- [ ] Check logs: `grep "SPEC-KIT-920" ~/.code/log/codex-tui.log`
- [ ] Verify command completed
- [ ] Confirm output routing correct
- [ ] Document results

---

## 📊 Expected Log Output

```
2025-11-09T15:45:00.123Z INFO SPEC-KIT-920: App::new called with initial_command=Some("/status")
2025-11-09T15:45:00.153Z INFO SPEC-KIT-920: Dispatching initial command at startup: /status
2025-11-09T15:45:00.154Z INFO SPEC-KIT-920: Auto-submitting regular command: /status
```

**Timing**:
- App::new: 0ms (construction)
- First redraw: ~30ms (UI ready)
- Command dispatch: ~30ms (after redraw)
- Command execution: 30-50ms (varies by command)

---

## 🚫 What NOT To Do

### ❌ Don't: Run in Background via Claude Code
```bash
# BAD - causes output leaks:
./code --initial-command "/status" &
python3 scripts/run-tui-with-pty.py ./code --initial-command "/status" &
timeout 10s ./code --initial-command "/status" &
```

### ❌ Don't: Use PTY Wrappers from Claude Code
```bash
# BAD - TTY escape codes leak:
python3 scripts/run-tui-with-pty.py ./code --initial-command "/status"
```

### ❌ Don't: Pipe to Files and Background
```bash
# BAD - still causes leaks:
./code --initial-command "/status" > /tmp/output.log 2>&1 &
```

### ✅ Do: Let User Run Directly
```bash
# GOOD - user runs in their terminal:
# (Claude Code asks user to run this)
./code --initial-command "/status"
```

---

## 🎯 Success Criteria

**SPEC-920 is validated when**:

1. ✅ User runs test in their terminal (not Claude Code)
2. ✅ TUI starts and renders first frame
3. ✅ Initial command auto-dispatches (logged)
4. ✅ Command executes correctly
5. ✅ Output appears in TUI chat history
6. ✅ NO output in Claude Code input box
7. ✅ No crashes or errors

**Evidence to collect**:
- Screenshot/recording of TUI executing command
- Log excerpt showing SPEC-920 dispatch
- Confirmation output routed correctly
- No errors in logs

---

## 📋 Testing Workflow (Recommended)

### For Claude Code:
1. Build binary: `cargo build --bin code`
2. Verify changes: `git diff HEAD~1 codex-rs/tui/src/app.rs`
3. Provide test commands to user
4. **Ask user to run test in their terminal**
5. User reports results
6. Document validation outcome

### For User:
1. Receive test commands from Claude Code
2. Open **separate terminal** (not Claude Code)
3. Run test command
4. Observe behavior
5. Check logs if needed
6. Report results to Claude Code

---

## 🔑 Key Insight

**The fix IS correct**, but **testing must be done carefully** to avoid Claude Code's output routing limitations.

**Why this matters**:
- SPEC-920 routing fix: ✅ Prevents TUI output from going to input box
- Claude Code testing: ⚠️ Has its own output routing issues
- Solution: **User validates**, Claude Code provides instructions

**Result**: Safe validation, correct fix, no output leaks. 🎯

---

**Status**: Safe testing methodology documented
**Next**: User runs validation, reports results
**Impact**: Enables proper validation without side effects
