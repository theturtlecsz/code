# SPEC-KIT-929: Claude Async Task Hang Investigation

**Status**: CLOSED - DEFERRED
**Priority**: P2 (Nice-to-have, not blocking)
**Created**: 2025-11-12
**Closed**: 2025-11-12
**Parent**: SPEC-KIT-928 (orchestration chaos - completed)
**Superseded By**: SPEC-KIT-930 (comprehensive agent orchestration refactor)

---

## Problem Statement

Claude agent completes tmux execution successfully but execute_agent() async task never finishes updating AGENT_MANAGER status, causing indefinite hang in quality gate orchestration.

**Impact**: Cannot use 3/3 consensus for quality gates. Currently using 2/2 consensus (Gemini + Code) as workaround.

**Scope**: Quality gate only - Claude works perfectly in regular stages (107s, 17KB responses).

---

## Evidence

### Symptoms

**Tmux execution completes**:
- Pane shows `zsh` (back to shell, command finished)
- Completion marker present (`___AGENT_COMPLETE___`)
- Output file created and valid

**Status never updates**:
- SQLite: `completed_at = NULL`, `response_text = NULL`
- AGENT_MANAGER: `status = Running` (stuck forever)
- Agent ID: 0ea1be4b-2576-45a4-af7b-067470eab9ed
- Spawn: 2025-11-12 19:24:09
- Duration: 35+ minutes (still "Running")

### Context from SPEC-928

**10 bugs fixed** in orchestration stack (session 2, 2025-11-12):
1. Validation failure discarded output ✅
2. No duplicate spawn prevention ✅
3. JSON extractor didn't strip Codex metadata ✅
4. Extractor found prompt schema instead of response ✅
5. agent_tool.rs had same prompt schema bug ✅
6. Fallback pane capture didn't recognize code agent ✅
7. SQLite only recorded "Completed", not "Failed" ✅
8. **Double completion marker** ✅ ← Key fix for code agent
9. No visibility into stuck agents ✅
10. UTF-8 panic + schema template false positive ✅

**Result**: Gemini + Code agents now work perfectly, Claude hangs only in quality_gate context.

---

## Diagnostic Tools Deployed

**Granular logging** (commit f354f90d5):
- 7 checkpoints in execute_agent() lifecycle
- Shows exactly where async task stops
- Logs agent_id for correlation

**Wait status logging** (commit 71bfe8285):
- Shows which agents blocking every 10s
- Makes stuck agents visible

**Next test will reveal**:
```
Expected sequence (will stop at hang point):
🔍 AGENT EXEC START
📊 execution returned after Xs
🔍 starting validation
🔍 validating ... bytes
🔍 acquiring lock
🔍 acquired lock
✅ validation passed
✅ execute_agent() task completed
```

---

## Theories

### Theory 1: Validation Hangs on Claude Output Format

**Hypothesis**: Claude's specific output format causes validation logic to hang (infinite loop, regex catastrophic backtracking, etc.)

**Evidence**:
- Claude works in regular stages (different validation path?)
- Only quality_gate affected
- Gemini/Code validate successfully with same logic

**Test**: Compare Claude output format in regular stage vs quality_gate

---

### Theory 2: Deadlock Acquiring AGENT_MANAGER Write Lock

**Hypothesis**: Quality gate context creates lock contention - Claude's task blocked waiting for write lock that never releases

**Evidence**:
- Multiple agents spawned concurrently (gemini, claude, code)
- All need AGENT_MANAGER write lock to update status
- Wait logging shows "Running" stuck forever

**Test**: Add timeout to lock acquisition, log lock holders

---

### Theory 3: update_agent_result() Hangs Internally

**Hypothesis**: The status update function itself has a bug for Claude's specific result data

**Evidence**:
- Gemini/Code call same function successfully
- Claude-specific data might trigger edge case

**Test**: Log before/after update_agent_result() call, check for panics/hangs inside

---

### Theory 4: Task Panic/Crash Without Error Propagation

**Hypothesis**: Async task panics silently, tokio spawns swallow error, appears as eternal "Running"

**Evidence**:
- No error logs
- Status never updated to Failed
- Task just disappears

**Test**: Wrap execute_agent() in panic handler, ensure errors propagate to status

---

## Acceptance Criteria

### Must Achieve

1. ✅ Identified exact hang point in execute_agent() lifecycle
2. ✅ Root cause documented with evidence
3. ✅ Fix implemented OR documented rationale for deferral
4. ✅ All 3 agents (gemini, claude, code) complete successfully in quality gates
5. ✅ No regression to SPEC-928 fixes (10 bugs must stay fixed)

### Optional Goals

1. Add timeout at execute_agent() spawn level
2. Implement task cancellation after timeout
3. Add circuit breaker for repeatedly failing agents

---

## Investigation Plan

### Phase 1: Diagnostic Test (1 hour)

**Setup**:
```bash
# Clean database
sqlite3 ~/.code/consensus_artifacts.db "
DELETE FROM agent_executions WHERE spec_id='SPEC-KIT-900';
DELETE FROM consensus_artifacts WHERE spec_id='SPEC-KIT-900';
DELETE FROM consensus_synthesis WHERE spec_id='SPEC-KIT-900';"

# Kill tmux
for s in agents-{claude,code,gemini}; do tmux kill-session -t $s 2>/dev/null; done
```

**Run**:
```bash
RUST_LOG=codex_core::agent_tool=info,codex_core::tmux=info \
  ./codex-rs/target/dev-fast/code 2>&1 | tee /tmp/spec-929-claude-debug.log
```

**Execute**: `/speckit.auto SPEC-KIT-900`

**After Claude hangs**:
```bash
# Get Claude's agent ID
CLAUDE_ID=$(sqlite3 ~/.code/consensus_artifacts.db "
SELECT agent_id FROM agent_executions
WHERE spec_id='SPEC-KIT-900' AND agent_name='claude' AND phase_type='quality_gate'
ORDER BY spawned_at DESC LIMIT 1;")

# Check execution trace
grep "$CLAUDE_ID" /tmp/spec-929-claude-debug.log
```

**Expected**: Logs will show last checkpoint before hang

---

### Phase 2: Root Cause Analysis (1-2 hours)

**Based on diagnostic findings**:

**If hangs after "execution returned"**:
- Issue in validation logic
- Check extract_json_from_mixed_output() for Claude-specific edge case
- Compare Claude output vs Gemini/Code

**If hangs after "starting validation"**:
- Issue in validation function itself
- Add logging inside validation steps
- Check for infinite loops, regex issues

**If hangs after "acquiring lock"**:
- Deadlock scenario
- Check AGENT_MANAGER lock holders
- Add lock timeout
- Review concurrent quality gate agent spawning

**If hangs after "acquired lock"**:
- Issue in update_agent_result()
- Log function entry/exit
- Check for panics, unwraps on None

**If no logs at all**:
- Task panic/crash before logging
- Wrap in panic handler
- Check tokio runtime status

---

### Phase 3: Fix Implementation (2-4 hours)

**Depends on root cause**:

**If validation issue**:
- Fix validation logic for Claude's output format
- Add validation tests
- Ensure no regression

**If deadlock**:
- Add lock timeout (30s)
- Implement exponential backoff
- Add circuit breaker for stuck agents

**If update issue**:
- Fix update_agent_result() edge case
- Add defensive programming (Option handling)
- Add tests for Claude-specific data

**If task crash**:
- Add panic handler at spawn
- Ensure error propagation to status
- Add task supervision/restart logic

---

### Phase 4: Validation (1-2 hours)

**Test scenarios**:
1. Fresh quality gate run with all 3 agents
2. Repeated runs to ensure reliability (3+ consecutive successes)
3. Mixed regular stage + quality gate (Claude should work in both)
4. Verify SPEC-928 fixes still working (no regression)

**Success criteria**:
- All 3 agents complete successfully
- SQLite records all 3 responses
- Consensus synthesis works with 3/3 majority
- No hangs, no indefinite "Running" status

---

## Workaround (Current)

**Use 2-agent quality gates**:

**Configuration** (in native_quality_gate_orchestrator.rs or config):
```rust
// Quality gate agent selection
let quality_gate_agents = vec!["gemini", "code"]; // Exclude "claude"
```

**Benefits**:
- 2/2 consensus sufficient for quality gates
- Both agents working reliably
- No impact on regular stages (Claude still used)

**Limitations**:
- Less diverse consensus (only 2 perspectives)
- Waste of Claude's capabilities in quality context

---

## Estimated Effort

**Investigation**: 4-8 hours total
- Phase 1 (Diagnostic): 1 hour
- Phase 2 (Analysis): 1-2 hours
- Phase 3 (Fix): 2-4 hours
- Phase 4 (Validation): 1-2 hours

**Priority**: P2 (nice-to-have)
- Primary objective already achieved (SPEC-928)
- 2-agent workaround functional
- No blocking impact on production use

---

## Dependencies

**Upstream**:
- ✅ SPEC-KIT-928 complete (all 10 bugs fixed)
- ✅ Diagnostic logging in place
- ✅ Evidence collection working

**Blocked by**:
- None (can start immediately if prioritized)

**Blocks**:
- None (workaround available)

---

## Success Metrics

**Primary**:
- All 3 agents complete in quality gates
- 0% indefinite hang rate
- 100% status update reliability

**Secondary**:
- Understanding of async task lifecycle deepened
- Tokio best practices documented
- Task supervision patterns established

---

## References

**Parent SPEC**:
- SPEC-KIT-928: Orchestration chaos - code agent completion (DONE)
- SESSION-REPORT.md: Complete investigation details
- HANDOFF-NEXT-SESSION.md: Context and decision points

**Related commits**:
- f354f90d5: Granular execute_agent task completion logging
- 71bfe8285: Wait status logging for stuck agent debugging
- 8f407f81f: Double completion marker fix (SPEC-928 key fix)

**Files**:
- core/src/agent_tool.rs: execute_agent() async task
- core/src/tmux.rs: Tmux execution and polling
- tui/src/chatwidget/spec_kit/native_quality_gate_orchestrator.rs: Quality gate orchestration

---

## Decision Points

### Option A: Investigate Now (4-8 hours)

**Pros**:
- Complete understanding of async task behavior
- 3/3 consensus more robust
- Eliminate workaround

**Cons**:
- 4-8 hours for marginal benefit
- 2-agent consensus already works
- Not blocking production use

---

### Option B: Defer to P2 Backlog (Recommended)

**Pros**:
- Focus on higher priority work
- Workaround is functional
- Can revisit if pattern recurs

**Cons**:
- 3/3 consensus unavailable
- Waste of Claude's capabilities in quality context
- Mystery unsolved

**Recommendation**: Defer to P2 backlog, prioritize SPEC-KIT-900 completion and other production work

---

**Next steps**: Defer unless 3/3 consensus becomes critical requirement

---

## CLOSURE NOTE (2025-11-12)

**Decision**: CLOSED - DEFERRED, superseded by SPEC-KIT-930

**Rationale**:
1. **Workaround sufficient**: 2-agent quality gates (Gemini + Code) provide adequate consensus
2. **Broader scope needed**: Root cause investigation reveals systemic architecture issues beyond single async hang
3. **Comprehensive refactor required**: Claude hang is symptom of larger problems (tmux-based orchestration, dual-write state, weak observability)

**Architectural issues identified** (from SPEC-928 session):
- Dual-write pattern (in-memory + SQLite) without ACID coordination
- Tmux-based execution creates async/sync impedance mismatch
- 10 bugs fixed in SPEC-928, but architecture remains fragile
- No proper state machine, queueing, or observability
- Hard to test (bash hacks, tmux dependencies)

**Strategic decision**: Rather than band-aid fix one async hang, refactor entire agent orchestration system to modern Rust async/await architecture with:
- Transaction-based state management (ACID guarantees)
- Queue-based work distribution (backpressure, rate limiting)
- Observable state machine (real-time status)
- Comprehensive error handling (categorized, retry logic)
- Full test coverage (no tmux/bash dependencies)

**Outcome**: SPEC-KIT-930 created to address root causes, not just symptoms.

**Effort saved**: 4-8 hours on investigation, reinvested in comprehensive solution.

**Migration path**: 2-agent consensus until SPEC-930 complete, then re-enable Claude with robust architecture.
