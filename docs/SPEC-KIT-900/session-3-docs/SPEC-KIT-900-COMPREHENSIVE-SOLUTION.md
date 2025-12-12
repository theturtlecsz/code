# SPEC-KIT-900 Comprehensive Solution Plan
**Date**: 2025-11-02
**Session**: 2 (Final)
**Status**: Systematic Root Cause Fix

---

## 🎯 Problem Statement

**User Request**: "address these issues for a final time please **ultrathink**"

**Symptoms**:
1. Quality gate fails → Pipeline halts
2. Plan stage never reached → Architecture changes untested
3. No audit trail visibility → Can't debug what's happening
4. Agents print JSON then nothing → No completion handling

---

## 📊 Complete Issue Map

### Issue 1: Quality Gate Blocks Testing ⚠️ CRITICAL BLOCKER

**Symptom**:
```
✖ Quality Gate: before-specify broker error — Only found 2/3 agents
✖ Quality gate before-specify failed – missing artefacts after 1 attempts
Resume with: /speckit.auto SPEC-KIT-900 --from spec-plan
```

**Root Cause**:
- "code" agent (GPT) produces 25,181 char output
- JSON extraction finds 13,726 chars
- Parse error: "key must be a string at line 18 column 1"
- Quality gate expects valid JSON from 3/3 agents
- Gets 2/3 valid (gemini, claude) + 1/3 invalid (code)
- Fails entire checkpoint → Halts pipeline

**Impact**: Plan stage NEVER reached, our architecture NEVER runs

**Solution Options**:
1. Fix JSON parsing to handle malformed code agent output
2. Make quality gates tolerant (2/3 passing = proceed)
3. Add bypass flag: `SPEC_OPS_SKIP_QUALITY_GATES=1`
4. Use resume mechanism: `/speckit.auto --from spec-plan`

**Recommended**: Option 4 (resume from plan) - Tests our changes immediately

---

### Issue 2: Incomplete Audit Trail ⚠️ OBSERVABILITY GAP

**Current State**:
- app.rs: HAS audit logging (🎯 event handler)
- app_event.rs: HAS RegularStageAgentsComplete event
- agent_orchestrator.rs: MISSING audit logging (was reverted)

**Missing Logs**:
- 🎬 Spawn entry (spec, stage, agents, prompt)
- 🤖 Per-agent spawn details (config, ID, SQLite)
- 📡 Background task lifecycle
- 🔍 Polling start
- 📊 Status updates (every 5s)
- ✅ Completion detection
- 📬 Event sending

**Impact**: No visibility into whether our architecture is even executing

**Solution**: Restore audit logging properly (Edit tool, not sed)

---

### Issue 3: Untested Architecture ⚠️ UNKNOWN VIABILITY

**What We Built** (Session 2, commits cfd811ba4 + 9acbc6264):
```rust
// 1. Direct spawning (like quality gates)
spawn_regular_stage_agents_native() {
    for agent in [gemini, claude, gpt_pro] {
        AGENT_MANAGER.create_agent_from_config_name()
        db.record_agent_spawn(phase_type="regular_stage")
    }
}

// 2. Background polling
wait_for_regular_stage_agents() {
    loop {
        check all agents terminal?
        log status every 5s
    }
}

// 3. Event-based completion
tokio::spawn(async {
    wait_for_regular_stage_agents().await
    send(RegularStageAgentsComplete)
})

// 4. App handler
AppEvent::RegularStageAgentsComplete => {
    on_spec_auto_agents_complete(widget)
}
```

**Status**: Architecture complete, ZERO testing

**Validation Needed**:
1. ✅ Agents spawn via AGENT_MANAGER
2. ✅ SQLite tracking (phase_type=regular_stage)
3. ✅ AgentStatusUpdate events emitted
4. ❓ Background task starts
5. ❓ Polling detects completion
6. ❓ AppEvent sent and handled
7. ❓ Completion handler processes Plan agents
8. ❓ plan.md written
9. ❓ Pipeline advances to Tasks

**Blockers**: Can't test due to quality gate failure

---

## 🔧 Comprehensive Solution Plan

### Phase 1: Enable Testing (IMMEDIATE)

**Goal**: Bypass quality gate to test Plan stage architecture

**Action**: Document resume command
```bash
# In TUI:
/speckit.auto SPEC-KIT-900 --from spec-plan

# This skips quality gates and starts at Plan stage
```

**Expected**: Direct jump to Plan → Our architecture runs

---

### Phase 2: Complete Audit Trail (VISIBILITY)

**Goal**: Full observability from spawn to completion

**Files to Modify**:
1. `agent_orchestrator.rs`: Add WARN-level audit logs
   - spawn_regular_stage_agents_native() entry
   - Per-agent spawn loop
   - SQLite recording
   - Background task launch
   - Polling function (already has logs)

2. Verify app.rs handler logs appear

**Test**: Run with audit trail and verify logs show complete flow

---

### Phase 3: End-to-End Validation (PROOF)

**Test Sequence**:
```bash
# 1. Clean environment
rm -f docs/SPEC-KIT-900-generic-smoke/plan.md ~/.code/consensus_artifacts.db

# 2. Run with resume
./codex-rs/target/dev-fast/code
# Then: /speckit.auto SPEC-KIT-900 --from spec-plan

# 3. Monitor audit trail
tail -f ~/.code/log/codex-tui.log | grep "AUDIT:"

# 4. Verify SQLite
sqlite3 ~/.code/consensus_artifacts.db "
SELECT agent_id, phase_type, agent_name, spawned_at
FROM agent_executions
WHERE spec_id='SPEC-KIT-900'
ORDER BY spawned_at;"

# 5. Verify output
ls -lh docs/SPEC-KIT-900-generic-smoke/plan.md
head -100 docs/SPEC-KIT-900-generic-smoke/plan.md
```

**Success Criteria**:
- ✅ 3 agents in SQLite (gemini, claude, gpt_pro) - phase_type=regular_stage
- ✅ Audit logs show: spawn → poll → complete → event → handler
- ✅ plan.md exists with Plan JSON content
- ✅ Pipeline advances to Tasks stage

---

### Phase 4: Fix Quality Gates (SEPARATE)

**After Plan stage validated**, address quality gate JSON parsing:

**Investigation**:
1. Check what JSON the "code" agent is producing
2. Identify line 18 that causes "key must be a string" error
3. Fix JSON extraction or make quality gates more tolerant

**Options**:
- Improve JSON extraction robustness
- Make quality gates accept 2/3 passing
- Fix code agent prompt to produce valid JSON

---

## 📋 Immediate Action Items

1. **NOW**: Test with resume mechanism
   - Command: `/speckit.auto SPEC-KIT-900 --from spec-plan`
   - Monitor: `tail -f ~/.code/log/codex-tui.log`
   - Observe: Does Plan stage spawn? Do agents complete?

2. **IF agents spawn but no audit logs**: Add proper WARN-level logging to agent_orchestrator.rs

3. **IF agents don't complete**: Check background task actually runs (add entry log)

4. **IF completion handler doesn't trigger**: Verify AppEvent sent and handled

5. **WHEN working**: Validate SQLite, plan.md, pipeline advancement

---

## 🎯 Expected Complete Flow (With Audit Trail)

```
[Quality Gate - SKIPPED via --from spec-plan]

🎬 AUDIT: spawn_regular_stage_agents_native called
  spec_id: SPEC-KIT-900
  stage: Plan
  expected_agents: ["gemini", "claude", "gpt_pro"]

🤖 AUDIT: Spawning agent 1/3: gemini
  config_name: gemini_flash
  ✓ Agent spawned with ID: abc123...
  ✓ SQLite record created

[Repeat for claude, gpt_pro]

🚀 AUDIT: Spawned 3 agents directly
🔄 AUDIT: Starting background polling task

📡 AUDIT: Background task started
🔍 AUDIT: Polling 3 agents (timeout=300s)

📊 AUDIT: Poll #1 @ 0s
  ⏳ abc123: Running
  ⏳ def456: Running
  ⏳ ghi789: Running

[Every 5s until complete...]

✅ AUDIT: All agents terminal after 47 polls (23s)
📬 AUDIT: RegularStageAgentsComplete event sent

🎯 AUDIT: Event handler triggered
  Agent 1/3: abc123...
  [Triggers on_spec_auto_agents_complete()]

[Pipeline processes Plan, writes plan.md, advances to Tasks]
```

---

## ✅ Current State

**Binary**: Ready (`codex-rs/target/dev-fast/code`, hash 85cda00c)
**Git**: Clean tree, 65 commits
**Architecture**: Complete (spawning, polling, events, handlers)
**Testing**: BLOCKED by quality gates
**Solution**: Use resume mechanism or add quality gate bypass

---

## Next Step

**User Action Required**: Choose testing approach:

A. **Resume from Plan** (Recommended - Immediate testing)
   ```
   /speckit.auto SPEC-KIT-900 --from spec-plan
   ```

B. **Add bypass flag** (Requires code change)
   - Add `SPEC_OPS_SKIP_QUALITY_GATES` check
   - Skip quality gate checkpoints if set
   - Rebuild and test

C. **Fix quality gate JSON** (Addresses root cause but takes longer)
   - Debug "code" agent JSON output
   - Fix extraction or make parsing tolerant
   - Rebuild and test from beginning

**Recommendation**: Option A to validate our Plan stage architecture NOW, then address quality gates separately.
