# SPEC-KIT-928: Resume Testing - Next Session Prompt

**Copy this prompt to start next session:**

---

## Context

I'm resuming work on SPEC-KIT-928 (Orchestration Flow Validation & Code Agent Completion).

**Previous session** (2025-11-12) fixed 10 bugs across 8 files (+442 lines, 10 commits):

1. Validation failure discarded output
2. No duplicate spawn prevention
3. JSON extractor didn't strip Codex metadata
4. Extractor found prompt schema instead of response
5. agent_tool.rs had same bug (two extraction functions!)
6. Fallback didn't recognize "code exec" pattern
7. SQLite only recorded Completed, not Failed
8. **Double completion marker** (KEY FIX - wrapper scripts had marker added twice)
9. No visibility into stuck agents
10. UTF-8 panic + schema template false positive

**Current state**:

**✅ Working**:
- Code agent: 73-110s, 11-12KB output, 15 issues ✅
- Gemini: 35s, 5.7KB output ✅
- Duplicate prevention: 4-layer defense ✅
- Extraction: Codex marker detection ✅

**❌ Remaining issue**:
- Claude quality_gate: execute_agent() async task hangs
  - Tmux completes but status never updates
  - Works fine in regular stages (107s, 17KB)
  - Only quality_gate affected

**Binary**: 105b4306 (has all 10 fixes + granular logging)
**Tree**: Clean (55 commits ahead)
**Branch**: main

---

## Files to Read

**Required**:
1. `docs/SPEC-KIT-928-orchestration-chaos/SESSION-REPORT.md` - Complete session 2 summary
2. `docs/SPEC-KIT-928-orchestration-chaos/HANDOFF-NEXT-SESSION.md` - Next steps
3. `docs/SPEC-KIT-928-orchestration-chaos/spec.md` - Original requirements

**Helpful**:
4. `/tmp/spec-928-BREAKTHROUGH.md` - Double marker explanation

---

## Decision Needed

**Path A: Close SPEC-928 with 2/3 consensus** (Recommended, 30 min):
- Primary objective achieved (code agent works)
- Document known issue (Claude async hang)
- Configure 2-agent quality gates
- Mark complete

**Path B: Debug Claude async hang** (1-3 hours):
- Run diagnostic test with logging
- Identify hang point in execute_agent()
- Implement fix
- Achieve 3/3 consensus

---

## Quick Status Check

Run this to see current test results:

```bash
sqlite3 ~/.code/consensus_artifacts.db "
SELECT agent_name, phase_type,
       CASE WHEN completed_at IS NULL THEN 'STUCK' ELSE 'DONE' END,
       LENGTH(response_text)||'b' as output
FROM agent_executions WHERE spec_id='SPEC-KIT-900'
ORDER BY spawned_at DESC LIMIT 5;"
```

Expected:
```
gemini | quality_gate | DONE  | 5729b
claude | quality_gate | STUCK | (null)
code   | quality_gate | DONE  | 11026b
```

---

## Test Commands (If Continuing)

**Diagnostic test**:
```bash
# Reset
sqlite3 ~/.code/consensus_artifacts.db "DELETE FROM agent_executions WHERE spec_id='SPEC-KIT-900';"
for s in agents-{claude,code,gemini}; do tmux kill-session -t $s 2>/dev/null; done

# Run with logging
RUST_LOG=codex_core::agent_tool=info ./codex-rs/target/dev-fast/code 2>&1 | tee /tmp/claude-hang-debug.log
```

**Inside TUI**: `/speckit.auto SPEC-KIT-900`

**After hang, analyze**:
```bash
CLAUDE_ID=$(sqlite3 ~/.code/consensus_artifacts.db "SELECT agent_id FROM agent_executions WHERE spec_id='SPEC-KIT-900' AND agent_name='claude' AND phase_type='quality_gate' ORDER BY spawned_at DESC LIMIT 1;")
grep "$CLAUDE_ID" /tmp/claude-hang-debug.log
```

---

## Recommendation

**Close SPEC-928 with Path A**:
- Code agent working (primary objective achieved!)
- 2/3 consensus sufficient for testing
- Can investigate Claude separately if needed (lower priority)
- Unblocks quality gate validation with working setup

**Create optional SPEC-929**: "Investigate Claude async task hang in quality gate orchestration"

---

## My Questions for You

1. Should we close SPEC-928 now (Path A) or continue debugging Claude (Path B)?
2. If closing, should I update spec.md and SPEC.md now or next session?
3. Do you want 2-agent quality gates configured or keep 3-agent with known issue?

---

**Ready to proceed with your preferred path.**
