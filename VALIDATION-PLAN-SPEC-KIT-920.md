# SPEC-KIT-920 Manual Validation Plan

**Date**: 2025-11-08
**Status**: Implementation Complete - Ready for Manual Testing
**Priority**: P1 (blocks SPEC-KIT-900 validation)

---

## Implementation Summary

### ✅ What Was Implemented

**Core Feature**: `--initial-command` flag for TUI automation

**Files Modified:**
1. `codex-rs/tui/src/cli.rs` - CLI flag definitions
2. `codex-rs/tui/src/lib.rs` - Pass through initialization
3. `codex-rs/tui/src/app.rs` - App struct + auto-submit logic
4. `codex-rs/tui/src/chatwidget/mod.rs` - ChatWidget parameter
5. `scripts/tui-session.sh` - Updated to use `--initial-command`
6. `scripts/test-initial-command.sh` - Validation test suite (new)

**Build Status**: ✅ Compiles successfully (release binary at `codex-rs/target/release/code`)

---

## Why This Was Needed

**Problem**: `tmux send-keys` doesn't work with TUI because:
- TUI uses alternate screen buffer
- Custom input handling via crossterm raw mode
- Keys sent to terminal buffer don't reach TUI's event loop

**Solution**: `--initial-command` flag injects commands into TUI's internal event queue after initialization completes.

---

## Manual Validation Tests

### Prerequisites

```bash
cd /home/thetu/code

# Ensure binary is built
ls -lh codex-rs/target/release/code
# Should show: Nov 8 21:48 or later
```

### Test 1: Help Text (30 seconds)

```bash
./codex-rs/target/release/code --help | grep -A3 "initial-command"
```

**Expected Output:**
```
  --initial-command <INITIAL_COMMAND>
          Initial slash command to execute after TUI starts (for automation). Example:
          --initial-command "/speckit.auto SPEC-KIT-900"

  --exit-on-complete
          Exit after initial command completes (for automation)
```

**Status**: ✅ VERIFIED (tested successfully)

---

### Test 2: Simple Status Command (2 minutes)

**Terminal 1 - Monitor logs:**
```bash
# Clear old logs
> ~/.code/log/codex-tui.log

# Watch for activity
tail -f ~/.code/log/codex-tui.log | grep --line-buffered -E "920|Auto-submit|Redraw"
```

**Terminal 2 - Run TUI:**
```bash
cd /home/thetu/code
./codex-rs/target/release/code --debug --initial-command "/speckit.status SPEC-KIT-900"
```

**Expected Behavior:**
1. TUI starts (welcome screen appears)
2. After ~2 seconds, status command auto-executes
3. SPEC status appears in TUI

**Expected Logs:**
```
INFO SPEC-KIT-920: App::new called with initial_command=Some("/speckit.status SPEC-KIT-900")
INFO SPEC-KIT-920 DEBUG: Redraw complete, dispatched=false, cmd=Some("/speckit.status SPEC-KIT-900")
INFO SPEC-KIT-920: Auto-submitting initial command: /speckit.status SPEC-KIT-900
```

**Success Criteria:**
- [ ] TUI starts without errors
- [ ] Status command executes automatically (visible in TUI)
- [ ] Logs show "Auto-submitting initial command"
- [ ] TUI remains responsive after command

---

### Test 3: Error Handling - No Slash (1 minute)

```bash
./codex-rs/target/release/code --initial-command "invalid"
```

**Expected:**
- TUI starts normally
- Error message appears: "--initial-command must start with '/'"
- TUI continues running (interactive mode)

---

### Test 4: Error Handling - Invalid Command (1 minute)

```bash
./codex-rs/target/release/code --initial-command "/notarealcommand"
```

**Expected:**
- TUI starts normally
- Error message about invalid command
- TUI continues running

---

### Test 5: Script Integration (2 minutes)

```bash
# Start via script
./scripts/tui-session.sh start "/speckit.status SPEC-KIT-900"

# Wait a moment
sleep 5

# Attach to see if command executed
./scripts/tui-session.sh attach
# (Press Ctrl-b d to detach)

# Clean up
./scripts/tui-session.sh kill
```

**Success Criteria:**
- [ ] Script starts session successfully
- [ ] When attached, TUI shows status output (command executed)
- [ ] No "tmux send-keys" warnings

---

### Test 6: Full Pipeline Integration (45-50 minutes)

**Start the full pipeline:**
```bash
cd /home/thetu/code

# Clear any previous run
rm -rf docs/SPEC-KIT-900-generic-smoke/{plan,tasks,implement,validate,audit,unlock}.md

# Start pipeline
./scripts/tui-session.sh start "/speckit.auto SPEC-KIT-900"
```

**Monitor progress:**
```bash
# Terminal 1: Watch logs
tail -f ~/.code/log/codex-tui.log | grep -E "SPEC-KIT-900|stage|consensus"

# Terminal 2: Watch database
watch -n 30 'sqlite3 ~/.code/db/consensus.db "SELECT spec_id, stage, created_at FROM consensus_runs WHERE spec_id='\''SPEC-KIT-900'\'' ORDER BY created_at DESC LIMIT 5;"'

# Terminal 3: Watch files
watch -n 30 'ls -lh docs/SPEC-KIT-900-generic-smoke/*.md 2>/dev/null | tail -10'
```

**Expected Results** (after 45-50 min):
- [ ] All 6 stages complete (plan, tasks, implement, validate, audit, unlock)
- [ ] Files exist: `docs/SPEC-KIT-900-generic-smoke/{plan,tasks,implement,validate,audit,unlock}.md`
- [ ] Evidence exported: `docs/SPEC-KIT-900-generic-smoke/evidence/`
- [ ] Database shows new consensus_run entries

**Validate deliverables:**
```bash
./scripts/spec-kit-tools.sh validate SPEC-KIT-900 plan
./scripts/spec-kit-tools.sh validate SPEC-KIT-900 tasks
# etc.
```

**Success Criteria:**
- [ ] Pipeline completes without manual intervention
- [ ] Deliverables contain reminder microservice content (not meta-analysis)
- [ ] Validation shows PASS (prompt corrections working)

---

## Troubleshooting

### If TUI doesn't start:
```bash
# Check for errors
./codex-rs/target/release/code --initial-command "/speckit.status SPEC-KIT-900" 2>&1 | head -20
```

### If command doesn't auto-execute:
```bash
# Check logs
cat ~/.code/log/codex-tui.log | grep "920"

# Verify binary has changes
strings codex-rs/target/release/code | grep "SPEC-KIT-920" | head -3
```

### If logs are empty:
```bash
# Check log location
ls -la ~/.code/log/

# Try with RUST_LOG
RUST_LOG=codex_tui=info ./codex-rs/target/release/code --initial-command "/speckit.status SPEC-KIT-900"
```

---

## Known Limitations

1. **--exit-on-complete**: Flag prepared but completion detection not yet implemented
2. **Multiple commands**: Only supports single initial command
3. **Command validation**: Basic validation only (checks for `/` prefix)

---

## Success Metrics

### Must Achieve:
- ✅ CLI flag accepted and shown in help
- ⏳ Command auto-executes after TUI ready
- ⏳ Full pipeline completes without manual input
- ⏳ SPEC-KIT-900 validation shows corrected prompts working

### Nice to Have:
- ⏳ Exit codes for scripting (requires --exit-on-complete implementation)
- ⏳ JSON output mode
- ⏳ Quiet/headless mode

---

## Next Session Notes

If validation successful:
1. Update SPEC.md to mark SPEC-KIT-920 as Complete
2. Run SPEC-KIT-900 validation with corrected prompts
3. Use for SPEC-KIT-070 cost benchmarking
4. Enable CI/CD integration

If issues found:
1. Check which test failed
2. Review logs for error patterns
3. May need to adjust timing (currently triggers on first Redraw)
4. May need to ensure AppState::Chat before dispatch

---

**Prepared**: 2025-11-08 23:59 UTC
**Binary**: codex-rs/target/release/code (Nov 8 21:48)
**Status**: Ready for manual validation
