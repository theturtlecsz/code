# Session Handoff: SPEC-KIT-900 Agent Spawning Architecture

**Date**: 2025-11-02
**Session**: Session 2 (Continuation)
**Branch**: `debugging-session`
**Status**: **BLOCKED - Architecture incompatibility discovered**

---

## Problem Statement

Plan stage agents print JSON but pipeline stalls - no advancement to Tasks stage.

---

## Root Cause Discovered

**Two incompatible spawning mechanisms exist**:

### Text-Based Spawning (Original - Regular Stages)
```rust
widget.submit_user_message(UserMessage { text: prompt })
```
- ✅ Creates a task (TaskStarted event)
- ✅ Waits for LLM response
- ✅ Triggers TaskComplete when done
- ❌ **Does NOT emit AgentStatusUpdate events**
- ❌ **Agents not tracked in SQLite**

### Direct Spawning (Quality Gates)
```rust
AGENT_MANAGER.create_agent_from_config_name(config, prompt, ...)
```
- ✅ Emits AgentStatusUpdate events
- ✅ Agents tracked in SQLite
- ❌ **Does NOT create a task**
- ❌ **No TaskComplete event**
- ❌ **No built-in completion waiting mechanism**

---

## What Was Attempted (Session 2)

### Attempt 1: SQLite Tracking on AgentStatusUpdate (Commit `7bad46a46`)
- **Approach**: Record agents in SQLite when AgentStatusUpdate arrives
- **Result**: FAILED - Text-based spawning doesn't emit AgentStatusUpdate
- **Outcome**: Only quality gate agents were tracked

### Attempt 2: Direct Agent Spawning (Commits `cfd811ba4`, `5d9c323b8`)
- **Approach**: Make regular agents spawn via AgentManager (like quality gates)
- **Implementation**:
  - Added `spawn_regular_stage_agents_native()` function
  - Modified `auto_submit_spec_stage_prompt()` to use direct spawning
  - Verified config names match `~/.code/config.toml`
- **Result**: PARTIAL SUCCESS
  - ✅ Agents spawn correctly
  - ✅ AgentStatusUpdate events emitted
  - ✅ SQLite tracking works (6 agents: 3 quality_gate + 3 regular_stage)
  - ❌ **No task created → No TaskComplete → Completion handler never triggers**

---

## Current State

**Database** (`~/.code/consensus_artifacts.db`):
```
quality_gate agents (3):  spawned 18:14:52 ✅
regular_stage agents (3): spawned 18:15:56 ✅
  - gemini:   1b8b5a60-3a03-4330-a02f-0b61f115c27b
  - claude:   d46a33f6-b330-46ee-8e2e-fccf52e1ad90
  - gpt_pro:  a63b6531-c908-42ab-b2bc-306b422f568c
```

**Logs show**:
- Agents spawned successfully
- AgentStatusUpdate events sent (6 agents total)
- **NO TaskStarted event** (last one was 17:32:59, hours ago)
- **NO TaskComplete event** (completion handler never triggers)

**Modified Files**:
- `agent_orchestrator.rs`: +95 lines (native spawn function)
- `mod.rs`: Reverted AgentStatusUpdate tracking

---

## The Fundamental Problem

**Quality gates work differently from regular stages**:

1. **Quality Gates**:
   - Spawn agents directly
   - Poll `widget.active_agents` for completion
   - Process results when all agents reach terminal state
   - **No task lifecycle needed**

2. **Regular Stages**:
   - Submit text prompt → Backend creates task
   - Task runs → Backend sends TaskComplete
   - TaskComplete triggers `on_spec_auto_agents_complete()`
   - **Requires task lifecycle**

**Direct spawning removed the task lifecycle, breaking completion detection.**

---

## Possible Solutions

### Option 1: Hybrid Approach (Recommended)
- Keep direct spawning for SQLite tracking
- Add manual task creation/completion signaling
- Poll agent status like quality gates do
- Trigger completion handler when all agents terminal

**Pros**: Unified spawning, reliable tracking
**Cons**: Need to implement polling logic

### Option 2: Fix Text-Based Spawning
- Make `submit_user_message()` emit AgentStatusUpdate events
- Requires backend changes (out of scope for TUI)

**Pros**: Minimal frontend changes
**Cons**: Requires backend modification

### Option 3: Two-Phase Approach
- Use text-based spawning (creates task)
- Extract agent IDs from AgentStatusUpdate
- Record in SQLite retroactively

**Pros**: Works with existing backend
**Cons**: Complex, race conditions possible

---

## Files to Review

**Key Implementation**:
- `agent_orchestrator.rs:36-93` - `spawn_regular_stage_agents_native()`
- `agent_orchestrator.rs:379-415` - Direct spawn call site
- `quality_gate_handler.rs:29-100` - How quality gates handle completion
- `pipeline_coordinator.rs` - Task completion handlers

**Configuration**:
- `~/.code/config.toml` - Agent config names (verified correct)

---

## Next Steps

1. **Understand quality gate completion flow**:
   - How do they detect all agents finished?
   - Where is the polling logic?
   - Can we reuse it for regular stages?

2. **Implement polling for regular stages**:
   - Check `widget.active_agents` for terminal states
   - Trigger completion when all expected agents done
   - Similar to quality gate mechanism

3. **Test end-to-end**:
   - Verify completion detection works
   - Confirm plan.md gets written
   - Validate pipeline advances to Tasks

---

## Diagnostic Commands

```bash
# Check SQLite tracking
sqlite3 ~/.code/consensus_artifacts.db "
SELECT agent_id, phase_type, agent_name, spawned_at
FROM agent_executions
ORDER BY spawned_at;"

# Check logs for task lifecycle
tail -100 ~/.code/log/codex-tui.log | grep -E "TaskStarted|TaskComplete"

# Check agent status updates
tail -100 ~/.code/log/codex-tui.log | grep "AgentStatusUpdate"

# Check spawning
tail -100 ~/.code/log/codex-tui.log | grep "Spawned.*directly"
```

---

## Key Insight

**The architecture requires either**:
1. **Tasks** (text-based spawning) → TaskComplete triggers handlers
2. **Polling** (direct spawning) → Manual completion checking

**We removed tasks but didn't add polling.** That's why it stalls.

---

**Recommendation**: Implement Option 1 (Hybrid) - Add polling logic to match quality gate behavior.
