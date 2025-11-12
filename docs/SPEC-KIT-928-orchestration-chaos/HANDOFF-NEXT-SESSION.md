# SPEC-KIT-928: Next Session Handoff

**Date**: 2025-11-12
**Session**: 3
**Previous work**: Fixed 10 bugs, code agent now working
**Remaining**: 1 issue (Claude async task hang)

---

## Quick Start for Next Session

### Read These First

1. **This file** - Handoff instructions
2. `SESSION-REPORT.md` - Complete session 2 work summary
3. `/tmp/spec-928-BREAKTHROUGH.md` - Double completion marker explanation
4. `spec.md` - Original requirements

---

## Current State Summary

### ✅ What Works

**Code Agent** (Primary Objective - ACHIEVED):
- Duration: 73-110 seconds
- Output: 11-12KB with 15 issues
- Format: Valid JSON after extraction
- Success rate: 100% (last 3 runs)

**Gemini Agent**:
- Duration: 35 seconds
- Output: 5.7KB
- Success rate: 100% (when no rate limits)

**Orchestration**:
- Duplicate prevention: 4-layer defense working
- Extraction: Codex marker detection working
- Recording: Failed agents now stored
- Timing: No more premature captures

---

### ❌ What Doesn't Work

**Claude Agent (Quality Gate Only)**:
- Tmux execution completes successfully
- Output file created and read
- But execute_agent() async task never finishes
- Status stuck in "Running" forever
- Data never reaches SQLite

**Note**: Claude works fine in regular stages (107s, 17KB), only quality_gate affected.

---

## The Breakthrough: Double Completion Marker

**This was the key fix** (commit 8f407f81f):

**Bug**: Wrapper scripts had `___AGENT_COMPLETE___` marker added TWICE:
- Internal (inside wrapper, fires after code exec finishes)
- External (in tmux command, fires immediately)

**Result**: Captured output after 27s instead of waiting 77s
- Got: 1,281 bytes (just prompt)
- Missed: 23KB response with 15 issues

**Fix**: Only add external marker for direct commands, not wrapper scripts

**Proof**: Manual wrapper test produces perfect 23KB output in 77s

---

## Decision Point for Next Session

### Option A: Close SPEC-928 Now (Recommended)

**Rationale**:
- Primary objective achieved (code agent works!)
- 2/3 agents working (sufficient for consensus)
- 10 bugs fixed (+442 lines)
- Claude works in other contexts (just quality_gate issue)

**Steps to close**:
1. Update spec.md with results
2. Document known issue (Claude quality_gate async hang)
3. Configure 2-agent quality gates (Gemini + Code)
4. Mark SPEC-928 as Done in SPEC.md
5. Create optional SPEC-929 for Claude investigation

**Time**: 30 minutes

---

### Option B: Fix Claude Async Task Hang

**Rationale**:
- 3/3 consensus more robust
- Should understand async task behavior
- Already have diagnostic logging in place

**Steps to investigate**:
1. Clean database and tmux
2. Run: `RUST_LOG=codex_core::agent_tool=info ./codex-rs/target/dev-fast/code`
3. Execute: `/speckit.auto SPEC-KIT-900`
4. When Claude hangs, check logs for Claude's agent_id
5. Logs will show exact hang point:
   ```bash
   grep "0ea1be4b" /tmp/spec-928-trace.log
   ```
6. Fix discovered issue
7. Test until all 3 agents work

**Time**: 1-3 hours

---

## Test Commands for Next Session

### If Continuing Investigation (Option B)

**Reset system**:
```bash
# Clean database
sqlite3 ~/.code/consensus_artifacts.db "
DELETE FROM agent_executions WHERE spec_id='SPEC-KIT-900';
DELETE FROM consensus_artifacts WHERE spec_id='SPEC-KIT-900';
DELETE FROM consensus_synthesis WHERE spec_id='SPEC-KIT-900';"

# Kill tmux
for s in agents-{claude,code,gemini}; do tmux kill-session -t $s 2>/dev/null; done
```

**Run test with granular logging**:
```bash
RUST_LOG=codex_core::agent_tool=info,codex_core::tmux=info \
  ./codex-rs/target/dev-fast/code 2>&1 | tee /tmp/spec-928-claude-debug.log
```

**Inside TUI**: `/speckit.auto SPEC-KIT-900`

**After Claude hangs**:
```bash
# Get Claude's agent ID
CLAUDE_ID=$(sqlite3 ~/.code/consensus_artifacts.db "
SELECT agent_id FROM agent_executions
WHERE spec_id='SPEC-KIT-900' AND agent_name='claude' AND phase_type='quality_gate'
ORDER BY spawned_at DESC LIMIT 1;")

# Check execution trace
grep "$CLAUDE_ID" /tmp/spec-928-claude-debug.log

# Expected sequence (will stop at hang point):
# 🔍 AGENT EXEC START
# 📊 execution returned after Xs
# 🔍 starting validation
# 🔍 validating ... bytes
# 🔍 acquiring lock
# 🔍 acquired lock
# ✅ validation passed
# ✅ execute_agent() task completed
```

---

### If Closing SPEC-928 (Option A)

**Update spec.md**:
```markdown
## Results (Session 2)

### Achieved
- ✅ Code agent completion fixed (10 bugs, +442 lines)
- ✅ Duplicate spawn prevention (4-layer defense)
- ✅ 2/3 quality gate consensus working (Gemini + Code)
- ✅ Extraction robust (codex marker detection)

### Known Issues
- ⚠️ Claude async task hang in quality_gate orchestration
  - Tmux completes but execute_agent() never finishes
  - Works fine in regular stages
  - Workaround: Use 2-agent quality gates

### Configuration
- Quality gates: Gemini + Code (2/2 consensus)
- Minimum agents: 2
- Timeout: 300s
```

**Update SPEC.md**:
```markdown
| SPEC-KIT-928 | Orchestration Chaos | Done | feature/spec-928 | #PR | Fixed code agent + duplicate prevention. 2/3 agents working. |
```

---

## Files Created This Session

**Session reports**:
- `docs/SPEC-KIT-928-orchestration-chaos/SESSION-REPORT.md` - Comprehensive summary
- `docs/SPEC-KIT-928-orchestration-chaos/HANDOFF-NEXT-SESSION.md` - This file

**Debug artifacts** (/tmp):
- `spec-928-BREAKTHROUGH.md` - Double marker explanation
- `spec-928-FINAL-STATUS.md` - Final status
- `tmux-agent-wrapper-1725226-313-debug.sh` - Proven working wrapper
- `spec-928-trace.log` - Execution logs

**Test scripts** (/tmp):
- `RUN-TEST.sh` - Automated test execution
- `VERIFY-RESULTS.sh` - Result verification
- `FINAL-TEST.txt` - Quick reference

---

## Binary Information

**Current**: 105b4306
**Location**: `./codex-rs/target/dev-fast/code`
**Has**: All 10 fixes
**Requires rebuild**: No (ready to use)

**Verify**:
```bash
sha256sum ./codex-rs/target/dev-fast/code | cut -d' ' -f1 | head -c8
# Should show: 105b4306
```

---

## Database Cleanup Commands

**For fresh test**:
```bash
sqlite3 ~/.code/consensus_artifacts.db "
DELETE FROM agent_executions WHERE spec_id='SPEC-KIT-900';
DELETE FROM consensus_artifacts WHERE spec_id='SPEC-KIT-900';
DELETE FROM consensus_synthesis WHERE spec_id='SPEC-KIT-900';
SELECT 'Database cleared for SPEC-KIT-900';"
```

**To check current state**:
```bash
sqlite3 ~/.code/consensus_artifacts.db "
SELECT agent_name, phase_type,
       CASE WHEN completed_at IS NULL THEN 'STUCK' ELSE 'DONE' END,
       LENGTH(response_text)
FROM agent_executions WHERE spec_id='SPEC-KIT-900'
ORDER BY spawned_at DESC LIMIT 10;"
```

---

## Key Learnings

1. **Wrapper scripts are self-contained** - Don't add duplicate completion signals
2. **Codex output has 5 sections** - Header, prompt, thinking, response, footer
3. **Use "] codex" marker** to find response start (not first `{`)
4. **Two extraction functions** - agent_tool.rs (validation) AND json_extractor.rs (recovery)
5. **Premature capture is silent** - File can be "stable" at partial content
6. **Failed agents need storage** - Broker can apply industrial extraction
7. **Manual wrapper tests reveal truth** - Isolated testing proves script correctness

---

## Immediate Next Steps

**For next session start**:

1. **Read SESSION-REPORT.md** (this file's companion - comprehensive details)

2. **Decide path**:
   - Path A: Close SPEC-928 (30 min)
   - Path B: Fix Claude hang (1-3 hours)

3. **If Path A**: Update docs, configure 2-agent setup, mark complete

4. **If Path B**: Run diagnostic test, analyze Claude hang logs, implement fix

---

## Contact Points

**Latest successful test** (gemini + code):
- Run: 19:24:09
- Results: gemini 35s/5729b ✅, code 73s/11026b ✅

**Stuck Claude agent**:
- Agent ID: 0ea1be4b-2576-45a4-af7b-067470eab9ed
- Spawn: 19:24:09
- Duration: 35+ minutes (still "Running")

**Working wrapper script**: `/tmp/tmux-agent-wrapper-1725226-313-debug.sh`
- Proven to work (manual test: 77s, 23KB, 15 issues)

---

**Session 2 complete. Ready for next session handoff.**
