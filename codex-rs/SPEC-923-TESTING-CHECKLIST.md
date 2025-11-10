# SPEC-923 Testing Checklist

## Quick Verification (5 minutes)

### 1. Build Verification
```bash
cd codex-rs
cargo build --package codex-core  # Should succeed
cargo build --package codex-tui   # Should succeed
```
✅ Expected: Both packages compile without errors

---

### 2. Output File Creation Test
```bash
# Enable observable mode
export SPEC_KIT_OBSERVABLE_AGENTS=1

# Count output files before
ls -1 /tmp/tmux-agent-output-*.txt 2>/dev/null | wc -l

# Run spec command
/speckit.new "Test SPEC-923 output capture"

# Check output files (should be cleaned up)
ls -1 /tmp/tmux-agent-output-*.txt 2>/dev/null | wc -l
```
✅ Expected: Files created during execution, cleaned up after
❌ Failure: Orphaned files remain in /tmp/

---

### 3. SQLite Clean Output Test
```bash
# Check most recent agent response
sqlite3 consensus.db 'SELECT substr(response_text, 1, 200) FROM consensus_responses ORDER BY rowid DESC LIMIT 1;'
```
✅ Expected: Starts with JSON `{"analysis":` or `{"plan":`
❌ Failure: Starts with `thetu@arch-dev` or shell commands

---

### 4. Plan.md Content Test
```bash
# Find most recent SPEC
latest_spec=$(ls -dt docs/SPEC-KIT-* 2>/dev/null | head -1)

# Check plan.md size
stat -c%s "$latest_spec/plan.md" 2>/dev/null || stat -f%z "$latest_spec/plan.md" 2>/dev/null

# View content
head -20 "$latest_spec/plan.md"
```
✅ Expected: >500 bytes, has actual plan content
❌ Failure: 184 bytes (empty template)

---

### 5. Observable Pane Test
```bash
# Terminal 1: Run spec with observable mode
export SPEC_KIT_OBSERVABLE_AGENTS=1
/speckit.plan SPEC-KIT-923

# Terminal 2: Attach to watch
tmux attach -t spec-kit-agents
# Press Ctrl-B, then D to detach
```
✅ Expected: Can watch agents in real-time
❌ Failure: Session doesn't exist or pane is empty

---

## Automated Test Script

```bash
cd codex-rs
./scripts/test-tmux-output-fix.sh
```

**What it checks**:
1. Output file creation and cleanup
2. SQLite responses free of shell noise
3. plan.md has actual content (>500 bytes)
4. Manual verification instructions

---

## Expected Results

### Before Fix (BROKEN)
```
SQLite response_text:
  thetu@arch-dev ~/code/codex-rs (main) $ cd /home/thetu/code/codex-rs && export ...

plan.md:
  184 bytes (empty template)
  # Plan: <SPEC-ID>
  [No content]

Consensus synthesis:
  ❌ Failed to extract JSON
  ❌ Empty plan.md
```

### After Fix (WORKING)
```
SQLite response_text:
  {"analysis": {"work_breakdown": [
    {"phase": "Phase 1", "tasks": [...

plan.md:
  2847+ bytes (full content)
  # Plan: SPEC-KIT-923
  ## Inputs
  - Spec: docs/SPEC-KIT-923/spec.md
  ...
  ## Work Breakdown
  1. Implement output file redirection
  2. Add clean output capture
  ...

Consensus synthesis:
  ✅ JSON extracted successfully
  ✅ plan.md populated with content
```

---

## Debugging Commands

### Check tmux session
```bash
tmux ls  # List sessions
tmux attach -t spec-kit-agents  # Watch agents
```

### Check output files during execution
```bash
# In another terminal while agents are running
ls -lh /tmp/tmux-agent-output-*.txt
cat /tmp/tmux-agent-output-*.txt  # View content
```

### Check SQLite for all responses
```bash
sqlite3 consensus.db "SELECT run_id, agent, length(response_text), substr(response_text, 1, 100) FROM consensus_responses ORDER BY rowid DESC LIMIT 5;"
```

### Enable debug logging
```bash
RUST_LOG=codex_core::tmux=debug /speckit.plan SPEC-KIT-923
```

---

## Common Issues

### Issue: "command too long" error
**Cause**: Arguments passed directly instead of using temp files
**Fix**: Already implemented - large args go to temp files

### Issue: Empty plan.md
**Cause**: Shell noise in agent output
**Fix**: This fix - redirect to output file

### Issue: Orphaned output files
**Cause**: Cleanup not happening
**Check**: Error/timeout paths have cleanup code
**Debug**: Look for `Failed to read agent output file` warnings

### Issue: tmux session not found
**Cause**: Observable mode not enabled
**Fix**: `export SPEC_KIT_OBSERVABLE_AGENTS=1`

---

## Performance Benchmarks

### File I/O Overhead
```bash
# Time output file creation
time echo "test" > /tmp/tmux-test-output.txt
# Expected: <1ms

# Time file read
time cat /tmp/tmux-test-output.txt
# Expected: <1ms

# Cleanup
rm /tmp/tmux-test-output.txt
```

### Agent Execution Time
**Before fix**: ~30-60s per agent (pane capture parsing)
**After fix**: ~30-60s per agent (file I/O negligible)
**Overhead**: <1% (file I/O ~1ms vs 30,000ms agent time)

---

## Sign-off Criteria

✅ **Build**: Both codex-core and codex-tui compile
✅ **Output Files**: Created during execution, cleaned up after
✅ **SQLite**: Responses start with JSON (no shell noise)
✅ **plan.md**: >500 bytes with actual content
✅ **Observable**: Can attach to tmux and watch agents
✅ **Cleanup**: No orphaned files in /tmp/
✅ **Fallback**: Filtered pane capture if file read fails
✅ **Logs**: Debug logs show file operations

**When all criteria pass**: SPEC-923 fix is validated ✅

---

## Next Actions After Testing

1. **If tests pass**:
   - Update SPEC.md status to "Done"
   - Commit changes with proper message
   - Store results in local-memory
   - Document lessons learned

2. **If tests fail**:
   - Check debug logs (`RUST_LOG=codex_core::tmux=debug`)
   - Verify tmux is available (`tmux -V`)
   - Check file permissions (/tmp/ writable)
   - Review error paths in code
   - File issue with reproduction steps

---

**Quick Start**: `./scripts/test-tmux-output-fix.sh`
**Documentation**: `docs/SPEC-KIT-923-OUTPUT-FIX.md`
**Summary**: `SPEC-923-FIX-SUMMARY.md`
