# SPEC-KIT-925: Agent Status Not Syncing to AGENT_MANAGER

**Status**: Draft  
**Priority**: Critical (blocks multi-agent execution)  
**Created**: 2025-11-11  
**Discovered During**: SPEC-KIT-924 testing  
**Related**: SPEC-923 (Observable Agent Execution)

---

## Problem Statement

Sequential agent orchestration hangs indefinitely because agent completion status is not propagated from output files back to AGENT_MANAGER. The orchestrator spawns the first agent successfully, the agent completes and writes output to a temp file, but AGENT_MANAGER never receives the status update. This causes the orchestrator to poll indefinitely, blocking subsequent agents from spawning.

**Impact**: Multi-agent consensus completely broken - only first agent runs, synthesis never happens.

---

## Symptoms

### Observable Behavior

1. **First agent spawns successfully**
   - Agent ID created in AGENT_MANAGER
   - Prompt sent via tmux or direct execution
   - Output file created: `/tmp/tmux-agent-output-{pid}-{seq}.txt`

2. **Agent produces output**
   - Output file written completely (2.3KB for gemini test)
   - Valid JSON response with proper content
   - File closed (no longer open by any process)
   - Timestamp: 14:51:00 UTC

3. **Status never updates**
   - AGENT_MANAGER.get_agent(id) returns agent in non-Completed state
   - Orchestrator polls every 500ms waiting for AgentStatus::Completed
   - Subsequent agents (claude, gpt_pro) never spawn
   - No timeout error (timeout likely 600s = 10min)

4. **User experience**
   - TUI shows "STAGE: PLAN (Tier 2)" with "Agents: gemini, claude, gpt_pro"
   - No progress indication after first agent
   - No error messages
   - System appears hung

### Evidence from Testing

**Test**: `/speckit.plan SPEC-KIT-900` (2025-11-11 14:50 UTC)

```bash
# Agent output exists
$ ls -lh /tmp/tmux-agent-output-4051297-181.txt
-rw-r--r-- 1 thetu thetu 2.3K Nov 11 14:51 /tmp/tmux-agent-output-4051297-181.txt

# Output is complete and valid
$ cat /tmp/tmux-agent-output-4051297-181.txt | jq .stage
"spec-plan"

# File not being updated (closed)
$ lsof /tmp/tmux-agent-output-4051297-181.txt
File not currently open

# No new agent outputs created after 14:51
$ find /tmp -name "*4051297*.txt" -mmin -10
/tmp/tmux-agent-output-4051297-181.txt  # Only one file

# Only gemini session exists from old run
$ tmux list-sessions | grep agents
agents-claude: 1 windows (created Tue Nov 11 04:02:26 2025)
agents-gemini: 1 windows (created Tue Nov 11 04:02:25 2025)
```

**Database Check**:
```bash
$ sqlite3 ~/.code/consensus_artifacts.db \
  "SELECT spec_id, stage, agent_name, substr(created_at,1,19) 
   FROM consensus_artifacts 
   WHERE spec_id='SPEC-KIT-900' 
   ORDER BY created_at DESC LIMIT 3"

SPEC-KIT-900|spec-implement|gemini|2025-11-11 04:38:47
SPEC-KIT-900|spec-implement|claude|2025-11-11 04:38:47
SPEC-KIT-900|spec-implement|gpt_codex|2025-11-11 04:38:47
```

No entries from current run (14:50) - confirms status never updated.

---

## Root Cause Analysis

### Architecture Overview

**Sequential execution flow** (`agent_orchestrator.rs:spawn_and_wait_for_agent`):

```rust
// 1. Spawn agent via AGENT_MANAGER
let agent_id = {
    let mut manager = AGENT_MANAGER.write().await;
    manager.create_agent_from_config_name(
        config_name,
        agent_configs,
        prompt.clone(),
        false,
        Some(batch_id.to_string()),
        tmux_enabled, // SPEC-KIT-923 flag
    ).await?
};

// 2. Poll AGENT_MANAGER for completion
loop {
    let manager = AGENT_MANAGER.read().await;
    if let Some(agent) = manager.get_agent(&agent_id) {
        match agent.status {
            AgentStatus::Completed => {
                return Ok((agent_id, agent.result.clone()));
            }
            AgentStatus::Failed => { ... }
            _ => { /* continue polling */ }
        }
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
}
```

### Expected vs Actual

**Expected**:
1. Agent spawns → tmux wrapper executes
2. Agent completes → output written to file
3. **Tmux wrapper reads output → updates AGENT_MANAGER status**
4. Orchestrator detects Completed status → returns result
5. Next agent spawns

**Actual**:
1. Agent spawns → tmux wrapper executes ✅
2. Agent completes → output written to file ✅
3. **❌ Status update never happens**
4. ❌ Orchestrator stuck in polling loop
5. ❌ Subsequent agents never spawn

### Suspected Breakpoint

The status update mechanism (step 3) is broken. Possible causes:

**A. Tmux wrapper not reading output file**
- Wrapper may complete without reading result
- Output written but not collected
- Related to SPEC-923 changes

**B. Missing callback to AGENT_MANAGER**
- Output read but status not updated
- Callback mechanism broken or missing
- agent_tool.rs update_agent_status() not called

**C. Status update races with polling**
- Update happens but polling misses it
- Timing issue in async coordination

**D. Observable agents flag side effects**
- `SPEC_KIT_OBSERVABLE_AGENTS=1` changes execution path
- Tmux mode uses different completion mechanism
- Completion marker not written or detected

---

## Investigation Tasks

### 1. Trace Agent Execution Path

**Files to examine**:
- `codex-rs/core/src/agent_tool.rs` - AGENT_MANAGER implementation
  - `create_agent_from_config_name()` - How agents spawn
  - `update_agent_status()` - How status updates
  - Agent lifecycle management

- `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`
  - `spawn_and_wait_for_agent()` - Current location
  - Output collection mechanism
  - Status polling logic

**Questions**:
- Does `create_agent_from_config_name` start a background task?
- Who is responsible for reading output and updating status?
- Where is the tmux wrapper script?
- How does output get from file to AGENT_MANAGER?

### 2. Check SPEC-923 Changes

SPEC-923 introduced `SPEC_KIT_OBSERVABLE_AGENTS` flag and tmux execution.

**Verify**:
- Does tmux mode have different completion mechanism?
- Is `___AGENT_COMPLETE___` marker required?
- Does wrapper script exist and is it correct?
- Compare old working run (04:32) vs new broken run (14:50)

### 3. Reproduce Minimal Case

**Test without observable agents**:
```bash
unset SPEC_KIT_OBSERVABLE_AGENTS
./target/dev-fast/code
# Run: /speckit.plan SPEC-KIT-900
```

If this works, issue is in SPEC-923 observable agents path.

**Test with different stage** (single agent):
```bash
export SPEC_KIT_OBSERVABLE_AGENTS=1
# Try a quality command (single agent, simpler)
# Run: /speckit.clarify SPEC-KIT-900
```

If this works, issue is specific to multi-agent sequential orchestration.

### 4. Check Agent Tool Logs

Enable detailed logging:
```bash
export RUST_LOG=codex_core::agent_tool=trace,codex_tui::chatwidget::spec_kit=trace
./target/dev-fast/code 2>&1 | tee agent_debug.log
```

Look for:
- Agent spawn messages
- Status update attempts
- Output file reads
- Completion signals

### 5. Examine Tmux Wrapper

**Find wrapper script**:
```bash
find codex-rs -name "*.sh" -o -name "*tmux*" -o -name "*agent*wrapper*"
grep -r "tmux.*agent\|agent.*wrapper" codex-rs/
```

**Check wrapper responsibilities**:
- Does it read output file?
- Does it call back to AGENT_MANAGER?
- Does it write completion marker?
- Does it handle errors?

---

## Hypotheses

### Most Likely: Broken Output Collection

**Hypothesis**: The tmux wrapper or background task that should read output files and update AGENT_MANAGER status is not executing or failing silently.

**Evidence**:
- Output file exists and is complete
- No status update in AGENT_MANAGER
- No errors in TUI

**Test**: Add explicit logging in output collection code to see if it's even running.

### Alternative: Status Update Not Propagating

**Hypothesis**: Output is read, status update is called, but update doesn't propagate to AGENT_MANAGER due to async timing or lock issues.

**Evidence**:
- Async read/write locks on AGENT_MANAGER
- Polling happens every 500ms
- Possible race condition

**Test**: Add logging before/after AGENT_MANAGER write lock acquisition.

### Less Likely: Completion Marker Required

**Hypothesis**: SPEC-923 introduced requirement for `___AGENT_COMPLETE___` marker in output, but it's not being written.

**Evidence**:
- Old claude output (04:02) had marker
- New gemini output (14:51) lacks marker
- But this seems like an implementation detail

**Test**: Check if completion detection looks for marker.

---

## Acceptance Criteria

Fix is successful when:

1. ✅ First agent spawns and completes (already working)
2. ✅ Agent status updates to Completed in AGENT_MANAGER
3. ✅ Orchestrator detects completion and returns result
4. ✅ Second agent spawns automatically
5. ✅ Third agent spawns automatically
6. ✅ All agent outputs stored to consensus database
7. ✅ Synthesis runs and creates plan.md
8. ✅ No hangs or timeouts
9. ✅ Works with both `SPEC_KIT_OBSERVABLE_AGENTS=0` and `=1`

### Success Test

```bash
# Clean state
rm -f docs/SPEC-KIT-900/plan.md
rm -rf docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-900

# Run test
export SPEC_KIT_OBSERVABLE_AGENTS=1
./target/dev-fast/code
# In TUI: /speckit.plan SPEC-KIT-900

# Verify (should complete in ~5-8 minutes)
ls -lh docs/SPEC-KIT-900/plan.md       # Should exist, >5KB
wc -c docs/SPEC-KIT-900/plan.md        # Should be >5000 bytes
grep '${' docs/SPEC-KIT-900/plan.md    # Should return nothing

# Check all agents ran
sqlite3 ~/.code/consensus_artifacts.db \
  "SELECT agent_name, substr(created_at,1,19) 
   FROM consensus_artifacts 
   WHERE spec_id='SPEC-KIT-900' AND stage='spec-plan' 
   ORDER BY created_at DESC"

# Should show 3 entries: gemini, claude, gpt_pro (all within ~5 min)
```

---

## Related Files

**Core**:
- `codex-rs/core/src/agent_tool.rs` - AGENT_MANAGER
- Agent spawn and lifecycle management

**Orchestration**:
- `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`
  - `spawn_and_wait_for_agent()` (line 285-456)
  - `spawn_regular_stage_agents_sequential()` (line 460+)

**SPEC-923 Related**:
- `codex-rs/tui/src/chatwidget/spec_kit/` - Observable agents implementation
- Tmux wrapper scripts (location TBD)

**Database**:
- `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs`
- `~/.code/consensus_artifacts.db` - Status tracking

---

## Notes

- **Not a regression of SPEC-KIT-924**: Template variable substitution works correctly
- **Possibly a SPEC-923 regression**: Observable agents flag changes execution path
- **Critical blocker**: Multi-agent consensus completely non-functional
- **Workaround**: Run without observable agents flag (if that works)

---

## Next Steps

1. **Investigation** (1-2 hours):
   - Trace agent execution path in agent_tool.rs
   - Find tmux wrapper or output collection code
   - Enable detailed logging and reproduce

2. **Root Cause** (30 min - 2 hours):
   - Identify where status update should happen
   - Determine why it's not happening
   - Verify with minimal test case

3. **Fix** (1-2 hours):
   - Implement status update mechanism
   - Add error handling and logging
   - Ensure backward compatibility

4. **Testing** (30 min):
   - Test with observable agents on/off
   - Test all stages (plan, tasks, implement, validate)
   - Verify multiple test runs

**Total Estimate**: 3-6 hours

---

**Dependencies**:
- SPEC-923 (Observable Agent Execution) - May need review/rollback
- SPEC-KIT-900 (Test SPEC) - Use for validation

**Blocks**:
- All multi-agent operations
- Consensus synthesis
- Full /speckit.auto pipeline
