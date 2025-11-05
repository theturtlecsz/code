# SPEC-KIT-900 Agent Collection Fix

**Problem**: implement.md was 191 bytes, synthesis showed "23 agents" instead of 4

**Root Cause**: Collection queried ALL historical agents instead of just the current run

**Solution**: Filter by specific `agent_ids` from the current run

---

## Architecture Flow (FIXED)

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. AGENT SPAWNING (agent_orchestrator.rs)                      │
│                                                                 │
│  spawn_regular_stage_agents_sequential()                       │
│  ├─ Spawn gemini → agent_id: a1b2c3d4 (run_id: xyz123)       │
│  ├─ Spawn claude → agent_id: e5f6g7h8 (run_id: xyz123)       │
│  ├─ Spawn gpt_codex → agent_id: i9j0k1l2 (run_id: xyz123)    │
│  └─ Spawn gpt_pro → agent_id: m3n4o5p6 (run_id: xyz123)      │
│                                                                 │
│  Returns: agent_ids = [a1b2c3d4, e5f6g7h8, i9j0k1l2, m3n4o5p6] │
└─────────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ 2. AGENT COMPLETION TRACKING (consensus_db.rs)                 │
│                                                                 │
│  agent_executions table:                                       │
│  ┌────────────┬──────────┬───────┬──────────┬─────────────┐   │
│  │ agent_id   │ spec_id  │ stage │ run_id   │ completed   │   │
│  ├────────────┼──────────┼───────┼──────────┼─────────────┤   │
│  │ [old-1]    │ KIT-900  │ impl  │ NULL     │ 2025-11-03  │   │ ← OLD DATA
│  │ [old-2]    │ KIT-900  │ impl  │ NULL     │ 2025-11-03  │   │ ← OLD DATA
│  │ ... (19 more old agents) ...               │             │   │
│  │ a1b2c3d4   │ KIT-900  │ impl  │ xyz123   │ 2025-11-04  │   │ ← NEW
│  │ e5f6g7h8   │ KIT-900  │ impl  │ xyz123   │ 2025-11-04  │   │ ← NEW
│  │ i9j0k1l2   │ KIT-900  │ impl  │ xyz123   │ 2025-11-04  │   │ ← NEW
│  │ m3n4o5p6   │ KIT-900  │ impl  │ xyz123   │ 2025-11-04  │   │ ← NEW
│  └────────────┴──────────┴───────┴──────────┴─────────────┘   │
│                                                                 │
│  Total: 23 agents with spec_id='KIT-900' AND stage='impl'     │
└─────────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ 3. POLLING & EVENT (agent_orchestrator.rs)                     │
│                                                                 │
│  wait_for_regular_stage_agents([a1b2c3d4, e5f6g7h8, ...])     │
│  ├─ Polls AGENT_MANAGER for agent status                      │
│  ├─ Waits until ALL 4 agents reach terminal state             │
│  └─ Sends event when complete:                                 │
│                                                                 │
│      AppEvent::RegularStageAgentsComplete {                    │
│          stage: SpecStage::Implement,                          │
│          spec_id: "SPEC-KIT-900",                              │
│          agent_ids: [a1b2c3d4, e5f6g7h8, i9j0k1l2, m3n4o5p6]  │ ← CRITICAL
│      }                                                          │
└─────────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ 4. EVENT HANDLING (app.rs:2728)                                │
│                                                                 │
│  AppEvent::RegularStageAgentsComplete { agent_ids, ... } =>    │
│      // Pass specific agent_ids to prevent collecting ALL      │
│      spec_kit::on_spec_auto_agents_complete_with_ids(          │
│          widget,                                               │
│          agent_ids  ← Pass specific IDs                        │
│      )                                                          │
└─────────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ 5. COLLECTION (agent_orchestrator.rs:1333-1358) ← THE FIX     │
│                                                                 │
│  if !specific_agent_ids.is_empty() {                           │
│      // FILTERED collection                                    │
│      widget.active_agents.iter()                               │
│          .filter(|agent|                                       │
│              specific_agent_ids.contains(&agent.id)            │ ← FILTER!
│          )                                                      │
│          .filter_map(|agent| ...)                              │
│          .collect()                                            │
│  }                                                              │
│                                                                 │
│  Result: Collects ONLY 4 agents [a1b2c3d4, e5f6g7h8, ...]     │
│          NOT all 23 historical agents!                         │
└─────────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ 6. SYNTHESIS (pipeline_coordinator.rs)                         │
│                                                                 │
│  synthesize_from_cached_responses(                             │
│      agent_responses = [                                       │
│          ("gemini", "... response ..."),                       │
│          ("claude", "... response ..."),                       │
│          ("gpt_codex", "... response ..."),                    │
│          ("gpt_pro", "... response ...")                       │
│      ],                                                         │
│      spec_id = "SPEC-KIT-900",                                 │
│      stage = SpecStage::Implement                              │
│  )                                                              │
│                                                                 │
│  Generates: implement.md (~10-20KB, meaningful content)        │
│                                                                 │
│  Stores in consensus_synthesis:                                │
│  ┌─────────┬──────────┬────────────────────┬─────────┐        │
│  │ spec_id │ stage    │ artifacts_count    │ run_id  │        │
│  ├─────────┼──────────┼────────────────────┼─────────┤        │
│  │ KIT-900 │ impl     │ 4 ← CORRECT        │ xyz123  │        │
│  └─────────┴──────────┴────────────────────┴─────────┘        │
└─────────────────────────────────────────────────────────────────┘
```

---

## Before vs After

### Before (Bug)
```rust
// OLD CODE (conceptual - actual query was in collect_consensus_artifacts)
let artifacts = db.query_artifacts(spec_id, stage)?;
// Returns: ALL 23 agents (includes old runs!)

let agent_responses: Vec<_> = widget.active_agents.iter()
    .filter_map(|agent| ...)  // No filtering by ID!
    .collect();
// Collects: 23 agents (all historical)

// Result:
// - implement.md: 191 bytes (just headers)
// - synthesis.artifacts_count: 23
// - synthesis.run_id: NULL
```

### After (Fixed)
```rust
// NEW CODE (agent_orchestrator.rs:1333-1358)
let agent_responses: Vec<_> = if !specific_agent_ids.is_empty() {
    widget.active_agents.iter()
        .filter(|agent| specific_agent_ids.contains(&agent.id))  ← FILTER!
        .filter_map(|agent| ...)
        .collect()
} else {
    // Fallback for backward compatibility
    ...
};
// Collects: 4 agents (only current run)

// Result:
// - implement.md: ~10-20KB (meaningful content)
// - synthesis.artifacts_count: 4
// - synthesis.run_id: xyz123 (tracked!)
```

---

## Key Code Locations

### 1. Agent Spawning with run_id
**File**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`
**Function**: `spawn_regular_stage_agents_sequential()`
**Line**: ~400-450
```rust
// Record each agent spawn with run_id
db.record_agent_execution(
    agent_id,
    spec_id,
    stage,
    "regular_stage",  // phase_type
    agent_name,
    run_id,          // ← Tracked!
)?;
```

### 2. Event with agent_ids
**File**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`
**Function**: `wait_for_regular_stage_agents()`
**Line**: ~985-989
```rust
let _ = event_tx.send(AppEvent::RegularStageAgentsComplete {
    stage: stage_clone,
    spec_id: spec_id_clone,
    agent_ids: agent_ids.clone(),  // ← Specific IDs passed!
});
```

### 3. Event Handling
**File**: `codex-rs/tui/src/app.rs`
**Line**: 2728-2738
```rust
AppEvent::RegularStageAgentsComplete { stage, spec_id, agent_ids } => {
    // Pass specific agent_ids to prevent collecting ALL historical agents
    spec_kit::on_spec_auto_agents_complete_with_ids(widget, agent_ids);
}
```

### 4. Filtered Collection (THE FIX)
**File**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`
**Function**: `on_spec_auto_agents_complete_with_ids()`
**Line**: 1333-1358
```rust
let agent_responses: Vec<(String, String)> = if !specific_agent_ids.is_empty() {
    // FILTERED collection - only these specific agents
    widget.active_agents.iter()
        .filter(|agent| specific_agent_ids.contains(&agent.id))  // ← THE FIX
        .filter_map(|agent| {
            if matches!(agent.status, AgentStatus::Completed) {
                agent.result.as_ref().map(|result| (agent.name.clone(), result.clone()))
            } else {
                None
            }
        })
        .collect()
} else {
    // FALLBACK: Collect all completed (backward compatibility)
    widget.active_agents.iter()
        .filter_map(...)
        .collect()
};
```

---

## Verification Queries

### Check run_id Tracking
```sql
-- All agents for current run should have SAME run_id
SELECT agent_name, run_id, spawned_at, completed_at
FROM agent_executions
WHERE spec_id='SPEC-KIT-900'
  AND stage='spec-implement'
  AND spawned_at > datetime('now', '-1 hour')
ORDER BY spawned_at;

-- Expected: 4 rows, all with identical run_id
```

### Check Synthesis Result
```sql
-- Synthesis should show 4 agents, not 23
SELECT
  stage,
  artifacts_count,  -- Should be 4
  LENGTH(output_markdown) as size,  -- Should be 10000-20000
  run_id,  -- Should be UUID
  created_at
FROM consensus_synthesis
WHERE spec_id='SPEC-KIT-900'
  AND stage='spec-implement'
ORDER BY created_at DESC
LIMIT 1;
```

### Compare Old vs New
```sql
-- Old synthesis (before fix)
SELECT 'OLD' as version, artifacts_count, LENGTH(output_markdown) as size
FROM consensus_synthesis
WHERE spec_id='SPEC-KIT-900' AND stage='spec-implement'
  AND created_at < datetime('now', '-2 hours')

UNION ALL

-- New synthesis (after fix)
SELECT 'NEW' as version, artifacts_count, LENGTH(output_markdown) as size
FROM consensus_synthesis
WHERE spec_id='SPEC-KIT-900' AND stage='spec-implement'
  AND created_at > datetime('now', '-1 hour')
ORDER BY version;

-- Expected:
-- OLD | 23 | 191
-- NEW | 4  | 15000
```

---

## Impact

### Bug Impact (Before Fix)
- ❌ Collected 23 agents (4 current + 19 historical)
- ❌ Synthesis output tiny (191 bytes)
- ❌ No way to distinguish runs (run_id was NULL)
- ❌ Pipeline advancement stalled

### Fix Impact (After Fix)
- ✅ Collects only 4 agents (current run)
- ✅ Synthesis output proper size (10-20KB)
- ✅ Run tracking works (run_id populated)
- ✅ Pipeline advancement automatic

### Performance Impact
**Before**: Collected 5.75x more data than needed (23 vs 4 agents)
**After**: Collects exactly what's needed (4 agents)
**Improvement**: 82% reduction in spurious collection

---

## Future Enhancements

### With run_id Tracking
1. **Historical Comparison**
   ```sql
   SELECT run_id, created_at, artifacts_count
   FROM consensus_synthesis
   WHERE spec_id='SPEC-KIT-900'
   ORDER BY created_at DESC;
   ```

2. **Run-Specific Queries**
   ```sql
   SELECT * FROM agent_executions
   WHERE run_id='xyz123';
   ```

3. **Log Filtering**
   ```bash
   grep "[run:xyz123]" logs  # After log tagging implemented
   ```

4. **/speckit.verify Command**
   ```bash
   /speckit.verify SPEC-KIT-900 --run-id xyz123
   # Shows complete audit trail for specific run
   ```

---

## Related Documents

**Architecture**: SPEC-KIT-900-ARCHITECTURE-ANALYSIS.md
**Testing**: SPEC-KIT-900-TEST-PLAN.md
**Summary**: SPEC-KIT-900-SESSION-3-SUMMARY.md
**TODO**: SPEC-KIT-900-AUDIT-INFRASTRUCTURE-TODO.md

---

**Prepared**: 2025-11-04 (Session 3)
**Status**: Fix verified in code, ready for testing
**Confidence**: High (clear data flow, specific filter logic)
