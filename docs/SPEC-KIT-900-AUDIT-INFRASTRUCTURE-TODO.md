# Complete Audit Infrastructure - Implementation Checklist

**Date**: 2025-11-04 (Session 3)
**Priority**: HIGH
**Estimated**: 4-6 hours remaining work

---

## Current Status

### ✅ Completed (Session 3)
1. Agent completion recording (sequential + parallel paths)
2. Filtered agent collection (specific IDs, not all history)
3. ACID-compliant directory resolution
4. Intelligent data extraction (JSON-only, skip debug)
5. Stage-specific execution patterns
6. Quality gate 2/3 tolerance
7. Comprehensive logging (synthesis, advancement)
8. run_id column added to schema (+ migration)

### ⏸️ Remaining Work

## Task 1: Propagate run_id Throughout System (2-3 hours)

**Files to Update**:

### consensus_db.rs
- [x] Add run_id column to schema
- [x] Update record_agent_spawn signature
- [ ] Update all spawn call sites (10+ locations)

### agent_orchestrator.rs
- [ ] Update spawn_and_wait_for_agent calls (pass run_id)
- [ ] Update spawn_regular_stage_agents_sequential (get run_id from state)
- [ ] Update spawn_regular_stage_agents_parallel (get run_id from state)
- [ ] Pass run_id to record_agent_spawn (6 call sites)

### native_quality_gate_orchestrator.rs
- [ ] Update spawn_quality_gate_agents_native (get run_id from state)
- [ ] Pass run_id to record_agent_spawn (3 call sites)

**How to get run_id**:
```rust
let run_id = widget.spec_auto_state.as_ref()
    .and_then(|s| s.run_id.as_deref());
```

---

## Task 2: Tag All Logs with [run_id] and Agent Type (1 hour)

**Pattern**:
```rust
// Instead of:
tracing::warn!("🎬 SEQUENTIAL: Spawning gemini");

// Do:
tracing::warn!("[run:{}] [type:regular_stage] 🎬 SEQUENTIAL: Spawning gemini",
    run_id.unwrap_or("none"));
```

**Apply to**:
- All spawn logs
- All completion logs
- All synthesis logs
- All advancement logs

**Benefit**: Can filter logs by run_id to see only one pipeline execution

---

## Task 3: Record Quality Gate Completions (30 min)

**File**: `native_quality_gate_orchestrator.rs`

**Add after line 200 (when agent completes)**:
```rust
AgentStatus::Completed => {
    // Record completion to SQLite
    if let Ok(db) = ConsensusDb::init_default() {
        if let Some(result) = &agent.result {
            let _ = db.record_agent_completion(agent_id, result);
        }
    }
}
```

---

## Task 4: Create /speckit.verify Command (1-2 hours)

**File**: New `codex-rs/tui/src/chatwidget/spec_kit/commands/verify.rs`

**Command**: `/speckit.verify SPEC-ID [--run-id UUID]`

**Output**:
```
SPEC-KIT-900 Verification Report
=================================

Run: b7c5b8d0-1a14-4553-8811-2fb10a87530b
Started: 2025-11-04 02:00:00
Completed: 2025-11-04 02:45:30
Duration: 45m 30s

Stage Execution:
├─ Plan (3 agents, sequential)
│  ├─ gemini: 02:01:00 → 02:05:00 (4m) ✓
│  ├─ claude: 02:05:00 → 02:10:00 (5m) ✓
│  └─ gpt_pro: 02:10:00 → 02:14:00 (4m) ✓
│  Output: plan.md (12KB)
│
├─ Tasks (3 agents, sequential)
│  ├─ gemini: 02:15:00 → 02:19:00 (4m) ✓
│  ├─ claude: 02:19:00 → 02:24:00 (5m) ✓
│  └─ gpt_pro: 02:24:00 → 02:28:00 (4m) ✓
│  Output: tasks.md (15KB)
│
├─ Implement (4 agents, sequential)
│  ├─ gemini: 02:29:00 → 02:34:00 (5m) ✓
│  ├─ claude: 02:34:00 → 02:40:00 (6m) ✓
│  ├─ gpt_codex: 02:40:00 → 02:48:00 (8m) ✓
│  └─ gpt_pro: 02:48:00 → 02:54:00 (6m) ✓
│  Output: implement.md (18KB)
│
└─ [Continue for Validate, Audit, Unlock]

Quality Gates:
├─ before-specify: 3/3 agents ✓
├─ after-specify: 2/3 agents (degraded) ✓
└─ after-tasks: 3/3 agents ✓

Output Files:
├─ plan.md: 12KB ✓
├─ tasks.md: 15KB ✓
├─ implement.md: 18KB ✓
├─ validate.md: 10KB ✓
├─ audit.md: 8KB ✓
└─ unlock.md: 6KB ✓

SQLite Verification:
✓ All agents recorded to agent_executions
✓ All completions have completed_at timestamps
✓ All stages have synthesis records
✓ Artifact counts match expected (3-4 per stage)

PASS: Pipeline completed successfully
```

**Implementation**:
```rust
pub struct SpecKitVerifyCommand;

impl SpecKitCommand for SpecKitVerifyCommand {
    fn name(&self) -> &'static str {
        "speckit.verify"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        // Query SQLite for complete run data
        // Format as verification report
        // Display in TUI
    }
}
```

---

## Task 5: Automated Post-Run Verification (30 min)

**File**: `pipeline_coordinator.rs`

**After Unlock stage completes, add**:
```rust
if state.current_index >= state.stages.len() {
    tracing::warn!("🎉 PIPELINE COMPLETE");

    // Automated verification
    let report = generate_verification_report(&state.spec_id, state.run_id.as_deref());
    widget.history_push(PlainHistoryCell::new(report.lines(), HistoryCellType::Notice));

    // Check for issues
    if report.has_errors() {
        widget.history_push(new_error_event("⚠️ Verification found issues - see report above"));
    } else {
        widget.history_push(PlainHistoryCell::new(
            vec![Line::from("✅ Verification PASSED - All stages completed successfully")],
            HistoryCellType::Notice
        ));
    }
}
```

---

## Implementation Order

**Priority 1** (Complete auditing):
1. Propagate run_id to all spawn calls (30 min)
2. Tag logs with run_id (30 min)
3. Record quality gate completions (15 min)

**Priority 2** (Verification):
4. Create /speckit.verify command (60 min)
5. Add automated post-run verification (30 min)

**Total**: ~2.5 hours to complete

---

## Testing Checklist

After implementation:
- [ ] Run `/speckit.auto SPEC-KIT-900`
- [ ] Query SQLite: All agents have run_id
- [ ] Query SQLite: All completions have completed_at
- [ ] Check logs: All tagged with [run:UUID]
- [ ] Run `/speckit.verify SPEC-KIT-900`
- [ ] Verify report shows complete data flow
- [ ] Check automated verification runs after Unlock

---

## Why This Matters

**Without complete auditing**:
- ❌ Can't distinguish Run 1 from Run 2
- ❌ Can't trace which agents belong to which run
- ❌ Can't verify pipeline executed correctly
- ❌ No post-run confidence check

**With complete auditing**:
- ✅ Full traceability (run → stage → agents → outputs)
- ✅ Automated verification catches issues
- ✅ Can replay/debug any run from SQLite
- ✅ Confidence in system correctness

This is foundational infrastructure for a production multi-agent system.
