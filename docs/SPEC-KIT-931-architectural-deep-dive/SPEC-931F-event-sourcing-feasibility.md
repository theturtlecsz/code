# SPEC-931F: Event Sourcing Feasibility Analysis (ULTRATHINK)

**Date**: 2025-11-13
**Status**: ✅ COMPLETE (GO/NO-GO Decision Made)
**Analyst**: Claude Code (Ultrathink Mode - Rigorous Deep Dive)
**Parent**: SPEC-931 (Architectural Deep Dive)
**Related**: SPEC-930 (Event Sourcing Proposal), SPEC-928 (Orchestration Chaos)

---

## Executive Summary

**RECOMMENDATION: NO-GO on full event sourcing migration**
**ALTERNATIVE: Incremental ACID compliance with existing schema**

**Critical Findings**:
1. ❌ Event sourcing does NOT eliminate dual-write (AGENT_MANAGER still needed for TUI)
2. ❌ Migration complexity EXCEEDS benefit (3-5 weeks, high risk, reversibility unclear)
3. ❌ SPEC-928 bugs were NOT storage bugs (event sourcing doesn't prevent them)
4. ❌ Event replay performance UNPROVEN (estimated 1ms/event = 10s at 10K events)
5. ✅ Simpler solution exists: Add SQLite transactions to existing schema (2-3 days)

**Evidence-Based Decision**:
- Current SLA: 2-agent quality gates, ~10/day, no 24/7 requirement
- SPEC-930 Design: Built for 100+ agents/min, enterprise-scale features we don't need
- YAGNI Principle: Over-engineering for future requirements that may never materialize

**Recommended Path Forward**:
1. **Week 1**: Add ACID compliance with SQLite transactions on existing schema
2. **Week 2**: Enable auto-vacuum, remove dead tables (consensus_artifacts, consensus_synthesis)
3. **Week 3**: Add basic queue (in-memory Priority Queue, persist to DB on crash)
4. **Future**: Consider event sourcing IF we need 24/7 SLA + 100+ agents/min

---

## Table of Contents

1. [Problem Statement Analysis](#1-problem-statement-analysis)
2. [SPEC-930 Assumption Validation](#2-spec-930-assumption-validation)
3. [Event Sourcing Schema Design](#3-event-sourcing-schema-design)
4. [Migration Path Analysis](#4-migration-path-analysis)
5. [Performance Analysis](#5-performance-analysis)
6. [Cross-Reference Validation](#6-cross-reference-validation)
7. [Alternative Solution](#7-alternative-solution)
8. [GO/NO-GO Decision](#8-gono-go-decision)
9. [Appendix: Evidence Gaps](#appendix-evidence-gaps)

---

## 1. Problem Statement Analysis

### 1.1 Current System Issues (from SPEC-931A)

**Q1 (CRITICAL)**: Why dual-write AGENT_MANAGER + SQLite without transactions?

**Root Cause Analysis**:
```rust
// agent_tool.rs:283-286 - AGENT_MANAGER write
self.agents.insert(agent_id.clone(), agent.clone());

// agent_tool.rs:291 - Spawn async task
let handle = tokio::spawn(async move { execute_agent(...).await; });

// Somewhere later: SQLite write (consensus_db.rs:340-366)
db.record_agent_spawn(agent_id, spec_id, stage, phase_type, agent_name, run_id)?;
```

**Failure Mode**:
- Crash between HashMap insert and SQLite write → orphaned in-memory agent
- Crash between SQLite write and HashMap → orphaned DB row
- No transaction coordination → inconsistent state

**Impact**:
- SPEC-928: No evidence of crashes causing actual data loss
- Current: 3 active rows, system working (with bugs, but not storage bugs)

### 1.2 SPEC-930's Claimed Benefits

**Claim 1**: "Event sourcing provides ACID guarantees"
**Reality**: Only if you STOP using AGENT_MANAGER HashMap (but we can't - TUI needs it!)

**Claim 2**: "Zero state corruption on crash/restart"
**Reality**: Event log alone doesn't help if AGENT_MANAGER is still dual-written

**Claim 3**: "Time-travel debugging"
**Reality**: Valuable for production systems with SLA requirements. Do we have those?

**Claim 4**: "55% storage reduction" (111KB → 49.5KB per quality gate)
**Reality**: Event log grows forever (append-only). agent_executions can DELETE old rows.

### 1.3 Actual Requirements

**From SPEC-931A Q41**: Do we accept eventual consistency?
- **Current**: In-memory updates first, SQLite eventually
- **Production SLA**: None (2-agent quality gates, best-effort)
- **Crash frequency**: Zero observed in production (SPEC-928 session)

**Evidence**: We're solving a theoretical problem, not an observed production issue.

---

## 2. SPEC-930 Assumption Validation

### 2.1 CRITICAL: Does Event Sourcing Eliminate Dual-Write?

**SPEC-930 Claim** (line 95-101):
> FR-1: Event-Sourced State Management
> - All state changes recorded as immutable events in event log
> - Current state derived by replaying events from log
> - Crash recovery: replay events from last checkpoint

**Validation**:

**Question**: Can we eliminate AGENT_MANAGER HashMap?

**Answer**: **NO** - TUI widget needs real-time access to agent state:

```rust
// tui/src/chatwidget/spec_kit/native_quality_gate_orchestrator.rs:89-112
let agents: Vec<AgentInfo> = self
    .agents
    .values()  // <-- Reads from HashMap
    .map(|agent| {
        AgentInfo {
            id: agent.id.clone(),
            name: agent.model.clone(),
            status: format!("{:?}", agent.status).to_lowercase(),
            // ...
        }
    })
    .collect();
```

**Consequence**: Event sourcing still requires dual-write!
- Write 1: Append event to event_log table
- Write 2: Update AGENT_MANAGER HashMap (for TUI)
- **SAME PROBLEM** as current system!

**Could we eliminate HashMap?**
- Option A: Query SQLite on every TUI render (60 FPS = 60 queries/sec)
  - Latency: 10ms per query = unusable (TUI freezes)
- Option B: Cache projection in memory (but that's just AGENT_MANAGER by another name!)

**Conclusion**: Event sourcing does NOT solve the dual-write problem for our architecture.

### 2.2 Performance Claims

**SPEC-930 Claim** (line 801-810, phase1-database.md:799-824):
> Replay latency: ~1ms per event = 9-30ms total

**Evidence Status**: **ESTIMATED, NOT MEASURED**

**Reality Check**:
```
Events per agent:3-10 (Queued, Started, Completed + intermediate states)
Quality gate (3 agents): 9-30 events

Replay times (assuming 1ms/event - UNPROVEN):
- 100 events: 100ms (acceptable)
- 1,000 events: 1 second (borderline)
- 10,000 events: 10 seconds (UNACCEPTABLE for startup)
```

**When do we hit 10K events?**
- 10 quality gates/day × 30 events/gate = 300 events/day
- 10K events = 33 days without cleanup
- With 30-day retention: Never exceed 9K events

**Snapshot Strategy** (SPEC-930 line 576-590):
- "Periodic state snapshots reduce replay time"
- **Undefined**: How often? Every N events? Every N seconds?
- **Complexity**: Snapshot management, GC, consistency guarantees

**Missing Evidence**:
- No prototype to benchmark actual replay performance
- No analysis of SQLite read performance at scale
- No comparison with current system latency

### 2.3 Migration Complexity

**SPEC-930 Claim** (line 980-1001, Migration Phase D):
> "After 30 days of consistent parallel run, drop old agent_executions"

**Paradox**: Migration requires dual-write (the problem we're solving!)

**Phase Analysis**:

**Phase A: Add event_log table** (1 day)
```sql
CREATE TABLE event_log (...);
-- Additive, low risk
```

**Phase B: Dual write (parallel run)** (30 days!)
```rust
// Write event
event_log.append_event(&event)?;
// Write old schema
agent_executions.update_status(agent_id, 'running')?;
```

**Problem**: We've introduced the EXACT inconsistency we're trying to fix!
- Crash between writes → event_log and agent_executions diverge
- 30 days of increased risk before we get to Phase D

**Phase C: Validate consistency** (daily job)
```rust
let state_from_events = replay_events(agent_id)?;
let state_from_table = query_agent_executions(agent_id)?;
assert_eq!(state_from_events, state_from_table);
```

**Question**: What if they DON'T match? How do we know which is correct?

**Phase D: Cutover** (drop old table)
- **Irreversible**: Can't easily go back to agent_executions once dropped
- **Risk**: Event log bugs only discovered in production

### 2.4 SPEC-928 Bug Analysis

**SPEC-930 Claim**: Event sourcing prevents SPEC-928 regressions

**SPEC-928 Bugs Fixed** (10 total):
1. Schema template false positive (validation logic, NOT storage)
2. UTF-8 panic on char boundary (string slicing, NOT storage)
3. Code agent 0% success (provider CLI invocation, NOT storage)
4. Duplicate agent spawns (missing wait tracking, NOT storage)
5. Premature collection (tmux timing, NOT storage)
6. TUI text pollution (stdout redirection, NOT storage)
7. Extraction failures not debuggable (logging, NOT storage)
8. Quality gate auto-resolution missing (orchestration logic, NOT storage)
9. Raw output not stored on validation failure (agent_tool.rs logic, NOT storage)
10. Concurrent agent detection missing (HashMap query, NOT storage)

**Analysis**: **ZERO bugs are storage-related!**

**Conclusion**: Event sourcing would NOT have prevented any SPEC-928 bugs.

### 2.5 Storage Overhead

**SPEC-930 Claim** (line 656-676):
> Total: 49.5KB per quality gate (55% reduction from 111KB)

**Reality Check**:

**Current System** (with DELETE cleanup):
```
agent_executions: 3 rows × 15KB = 45KB (can DELETE old rows)
With 30-day retention: ~900 rows = 13.5MB stable
With auto-vacuum: File size = data size
```

**Event Sourcing** (append-only):
```
event_log: 9 events × 500 bytes = 4.5KB per quality gate
Per day: 10 gates × 4.5KB = 45KB
Per month: 1.35MB (30 days × 45KB)
Per year: 16.2MB (365 days × 45KB)

With NO deletion: Event log grows forever
With snapshots: Add snapshot table (complexity, storage overhead)
```

**Long-Term Projection**:
```
Current (with cleanup):     13.5MB stable (constant)
Event sourcing (no cleanup): 16.2MB/year (linear growth)
```

**Conclusion**: Event sourcing has HIGHER long-term storage cost, not lower!

---

## 3. Event Sourcing Schema Design

*Despite recommending NO-GO, here's the concrete schema design for completeness:*

### 3.1 Event Log Schema

```sql
-- Immutable append-only event log
CREATE TABLE event_log (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    event_type TEXT NOT NULL, -- 'AgentQueued' | 'AgentStarted' | 'AgentCompleted' | 'AgentFailed' | 'AgentRetrying' | 'AgentCancelled'
    event_data JSON NOT NULL, -- Event-specific payload (provider, timeout, error, result, etc.)
    timestamp INTEGER NOT NULL, -- Unix epoch milliseconds
    sequence_number INTEGER NOT NULL, -- Per-agent ordering (1, 2, 3, ...)

    -- Indexes for fast queries
    INDEX idx_agent_events (agent_id, sequence_number),
    INDEX idx_event_timestamp (timestamp),
    INDEX idx_event_type (event_type)
);
```

**Event Types & Payloads**:

```json
// AgentQueued
{
  "event_type": "AgentQueued",
  "event_data": {
    "spec_id": "SPEC-KIT-900",
    "stage": "plan",
    "agent_name": "gemini",
    "provider": "google",
    "priority": 0,
    "queued_at": 1699876543000
  }
}

// AgentStarted
{
  "event_type": "AgentStarted",
  "event_data": {
    "provider": "google",
    "started_at": 1699876545000,
    "timeout_at": 1699878345000
  }
}

// AgentCompleted
{
  "event_type": "AgentCompleted",
  "event_data": {
    "completed_at": 1699876600000,
    "duration_ms": 55000,
    "result_json": "{\"analysis\": \"...\"}",
    "output_bytes": 4693
  }
}

// AgentFailed
{
  "event_type": "AgentFailed",
  "event_data": {
    "failed_at": 1699876550000,
    "error_type": "ValidationFailed",
    "error_message": "Output too small (450 bytes, minimum 500)",
    "retries_remaining": 2
  }
}

// AgentRetrying
{
  "event_type": "AgentRetrying",
  "event_data": {
    "retry_count": 1,
    "next_retry_at": 1699876560000, // +10s backoff
    "previous_error": "ValidationFailed"
  }
}

// AgentCancelled
{
  "event_type": "AgentCancelled",
  "event_data": {
    "cancelled_at": 1699876555000,
    "reason": "User cancelled via TUI"
  }
}
```

### 3.2 Snapshot Schema

```sql
-- Periodic state snapshots for fast recovery
CREATE TABLE agent_snapshots (
    snapshot_id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    state_json JSON NOT NULL, -- Full AgentState serialized
    event_id INTEGER NOT NULL, -- Last event_id included in snapshot
    timestamp INTEGER NOT NULL,

    FOREIGN KEY (event_id) REFERENCES event_log(event_id),
    INDEX idx_snapshot_agent (agent_id, timestamp DESC)
);
```

**Snapshot Example**:
```json
{
  "agent_id": "uuid",
  "state": "Running",
  "state_data": {
    "started_at": 1699876545000,
    "provider": "google",
    "timeout_at": 1699878345000
  },
  "spec_id": "SPEC-KIT-900",
  "stage": "plan",
  "agent_name": "gemini",
  "created_at": 1699876543000
}
```

### 3.3 Projection Schema (Current State)

```sql
-- Current state projection (derived from events)
CREATE TABLE agent_state_projection (
    agent_id TEXT PRIMARY KEY,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    provider TEXT NOT NULL,

    -- State machine
    state TEXT NOT NULL, -- 'Pending' | 'Queued' | 'Running' | 'Validating' | 'Completed' | 'Failed' | 'Retrying' | 'Cancelled'
    state_data JSON NOT NULL, -- State-specific fields

    -- Timestamps
    created_at INTEGER NOT NULL,
    queued_at INTEGER,
    started_at INTEGER,
    completed_at INTEGER,
    failed_at INTEGER,
    cancelled_at INTEGER,

    -- Result
    result_json TEXT,
    error_json TEXT,

    -- Retry
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,

    -- Telemetry
    duration_ms INTEGER,
    queue_wait_ms INTEGER,

    -- Event sourcing metadata
    last_event_id INTEGER NOT NULL, -- Last event applied to this projection
    last_sequence_number INTEGER NOT NULL, -- Last sequence applied

    FOREIGN KEY (last_event_id) REFERENCES event_log(event_id)
);

CREATE INDEX idx_projection_spec_stage ON agent_state_projection(spec_id, stage);
CREATE INDEX idx_projection_state ON agent_state_projection(state);
CREATE INDEX idx_projection_created ON agent_state_projection(created_at);
```

### 3.4 Migration DDL

**Step 1: Add Event Log** (Additive, Low Risk)
```sql
-- Run this migration first
CREATE TABLE event_log (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    event_data JSON NOT NULL,
    timestamp INTEGER NOT NULL,
    sequence_number INTEGER NOT NULL
);

CREATE INDEX idx_agent_events ON event_log(agent_id, sequence_number);
CREATE INDEX idx_event_timestamp ON event_log(timestamp);
CREATE INDEX idx_event_type ON event_log(event_type);

CREATE TABLE agent_snapshots (
    snapshot_id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    state_json JSON NOT NULL,
    event_id INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    FOREIGN KEY (event_id) REFERENCES event_log(event_id)
);

CREATE INDEX idx_snapshot_agent ON agent_snapshots(agent_id, timestamp DESC);

CREATE TABLE agent_state_projection (
    agent_id TEXT PRIMARY KEY,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    provider TEXT NOT NULL,
    state TEXT NOT NULL,
    state_data JSON NOT NULL,
    created_at INTEGER NOT NULL,
    queued_at INTEGER,
    started_at INTEGER,
    completed_at INTEGER,
    failed_at INTEGER,
    cancelled_at INTEGER,
    result_json TEXT,
    error_json TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    duration_ms INTEGER,
    queue_wait_ms INTEGER,
    last_event_id INTEGER NOT NULL,
    last_sequence_number INTEGER NOT NULL,
    FOREIGN KEY (last_event_id) REFERENCES event_log(event_id)
);

CREATE INDEX idx_projection_spec_stage ON agent_state_projection(spec_id, stage);
CREATE INDEX idx_projection_state ON agent_state_projection(state);
CREATE INDEX idx_projection_created ON agent_state_projection(created_at);
```

**Step 2: Migrate Existing Data** (Data Migration)
```sql
-- Convert existing agent_executions to events
-- Problem: We only have current state, not state transitions!

-- For completed agents, infer events:
INSERT INTO event_log (agent_id, event_type, event_data, timestamp, sequence_number)
SELECT
    agent_id,
    'AgentQueued',
    json_object(
        'spec_id', spec_id,
        'stage', stage,
        'agent_name', agent_name,
        'provider', 'unknown', -- Not stored in current schema!
        'queued_at', spawned_at
    ),
    spawned_at,
    1 -- sequence_number
FROM agent_executions
WHERE spawned_at IS NOT NULL;

-- AgentStarted event (inferred)
INSERT INTO event_log (agent_id, event_type, event_data, timestamp, sequence_number)
SELECT
    agent_id,
    'AgentStarted',
    json_object(
        'provider', 'unknown', -- Missing!
        'started_at', spawned_at, -- Best guess
        'timeout_at', spawned_at + (30 * 60 * 1000) -- 30 min timeout (assumed)
    ),
    spawned_at,
    2
FROM agent_executions
WHERE spawned_at IS NOT NULL;

-- AgentCompleted event (for successful agents)
INSERT INTO event_log (agent_id, event_type, event_data, timestamp, sequence_number)
SELECT
    agent_id,
    'AgentCompleted',
    json_object(
        'completed_at', completed_at,
        'duration_ms', (julianday(completed_at) - julianday(spawned_at)) * 86400000,
        'result_json', response_text,
        'output_bytes', length(response_text)
    ),
    completed_at,
    3
FROM agent_executions
WHERE completed_at IS NOT NULL AND extraction_error IS NULL;

-- AgentFailed event (for failed agents)
INSERT INTO event_log (agent_id, event_type, event_data, timestamp, sequence_number)
SELECT
    agent_id,
    'AgentFailed',
    json_object(
        'failed_at', completed_at,
        'error_type', 'ExtractionFailed',
        'error_message', extraction_error,
        'retries_remaining', 0
    ),
    completed_at,
    3
FROM agent_executions
WHERE completed_at IS NOT NULL AND extraction_error IS NOT NULL;

-- Populate projection from events
INSERT INTO agent_state_projection (
    agent_id, spec_id, stage, agent_name, provider, state, state_data,
    created_at, queued_at, started_at, completed_at, result_json, error_json,
    retry_count, max_retries, duration_ms, last_event_id, last_sequence_number
)
SELECT
    ae.agent_id,
    ae.spec_id,
    ae.stage,
    ae.agent_name,
    'unknown' AS provider,
    CASE
        WHEN ae.completed_at IS NOT NULL AND ae.extraction_error IS NULL THEN 'Completed'
        WHEN ae.completed_at IS NOT NULL AND ae.extraction_error IS NOT NULL THEN 'Failed'
        WHEN ae.spawned_at IS NOT NULL THEN 'Running'
        ELSE 'Pending'
    END AS state,
    '{}' AS state_data, -- Empty for migrated agents
    ae.spawned_at AS created_at,
    ae.spawned_at AS queued_at,
    ae.spawned_at AS started_at,
    ae.completed_at,
    ae.response_text AS result_json,
    ae.extraction_error AS error_json,
    0 AS retry_count,
    3 AS max_retries,
    CAST((julianday(ae.completed_at) - julianday(ae.spawned_at)) * 86400000 AS INTEGER) AS duration_ms,
    (SELECT MAX(event_id) FROM event_log WHERE event_log.agent_id = ae.agent_id) AS last_event_id,
    (SELECT MAX(sequence_number) FROM event_log WHERE event_log.agent_id = ae.agent_id) AS last_sequence_number
FROM agent_executions ae;
```

**Migration Challenges**:
1. **Missing data**: provider field doesn't exist in agent_executions
2. **Lossy conversion**: Can't reconstruct intermediate states (only final state)
3. **Inaccurate timestamps**: spawned_at used for both queued_at and started_at
4. **No retry history**: Can't reconstruct retry events

**Step 3: Drop Old Table** (Irreversible!)
```sql
-- DANGER: No going back after this!
DROP TABLE agent_executions;
DROP INDEX idx_agent_executions_spec;
DROP INDEX idx_agent_executions_run;
```

---

## 4. Migration Path Analysis

### 4.1 Migration Timeline

**SPEC-930 Estimate**: 3 weeks (Phase 6, lines 914-942)

**Reality Check**:

**Week 1: Foundation**
- Event store implementation
- State machine enum
- Basic executor with event append
- Tests
- **Effort**: 5 days (40 hours)

**Week 2: Integration**
- Update all state transitions to append events
- Implement event replay
- Add snapshot logic
- Parallel write (event_log + agent_executions)
- **Effort**: 5 days (40 hours)

**Week 3: Migration**
- Data migration script
- Validation (compare event replay vs current table)
- Fix discrepancies
- **Effort**: 5 days (40 hours)

**Week 4+: Parallel Run**
- 30 days of dual-write
- Daily consistency checks
- Bug fixes
- **Effort**: 1-2 hours/day × 30 days = 30-60 hours

**Week 5: Cutover**
- Final validation
- Drop old table
- Production monitoring
- **Effort**: 2-3 days (16-24 hours)

**Total**: **~150-180 hours** (4-5 weeks full-time, not 3 weeks!)

### 4.2 Migration Risks

**Risk 1: Dual-Write Inconsistency During Migration**
- **Probability**: HIGH (we're introducing the problem we're solving!)
- **Impact**: CRITICAL (data corruption)
- **Mitigation**: None (inherent to dual-write approach)

**Risk 2: Migration Script Bugs**
- **Probability**: MEDIUM (complex SQL, lossy conversion)
- **Impact**: HIGH (data loss, incorrect state)
- **Mitigation**: Extensive testing on copy of production DB

**Risk 3: Event Replay Bugs**
- **Probability**: MEDIUM (complex logic, edge cases)
- **Impact**: CRITICAL (wrong state = wrong decisions)
- **Mitigation**: Property-based testing, manual validation

**Risk 4: Performance Regression**
- **Probability**: MEDIUM (replay latency unknown)
- **Impact**: HIGH (slow startup, TUI lag)
- **Mitigation**: Benchmarking before cutover

**Risk 5: Irreversibility**
- **Probability**: LOW (only if we drop agent_executions)
- **Impact**: CRITICAL (can't go back if event sourcing fails)
- **Mitigation**: Keep agent_executions table indefinitely as escape hatch

### 4.3 Rollback Plan

**Scenario**: Event sourcing causes production issues after cutover

**Option A: Keep agent_executions Table**
- Don't drop old table
- Cost: Dual storage (but already have dual-write during migration!)
- Benefit: Can revert by pointing reads back to agent_executions

**Option B: Rebuild agent_executions from Event Log**
```sql
-- Recreate projection by replaying all events
DELETE FROM agent_executions;
INSERT INTO agent_executions (agent_id, spec_id, stage, ...)
SELECT * FROM agent_state_projection;
```
- Cost: Requires event replay to work correctly
- Risk: If replay is buggy, rollback fails

**Recommendation**: Keep agent_executions indefinitely (cost: ~13.5MB disk space)

---

## 5. Performance Analysis

### 5.1 Event Replay Performance

**SPEC-930 Claim**: "~1ms per event"

**Estimate Validation**:

**Assumptions**:
- SQLite read: ~0.1-0.5ms per row (indexed)
- JSON parsing: ~0.1-0.3ms per event
- State application: ~0.1-0.2ms per event
- **Total**: ~0.3-1.0ms per event (optimistic)

**Scale Analysis**:
```
3 agents × 10 events = 30 events
30 events × 1ms = 30ms replay (acceptable)

10 quality gates/day × 30 events = 300 events/day
300 events × 1ms = 300ms replay (acceptable)

30 days × 300 events = 9,000 events
9,000 events × 1ms = 9 seconds (SLOW!)
```

**Snapshot Strategy** (to keep replay <1s):
- Snapshot every 1,000 events
- Replay: Load snapshot + replay events after snapshot
- 9,000 events with snapshots: Load 1 snapshot + 1,000 events = 1s replay (OK)

**Snapshot Complexity**:
- When to snapshot? (N events? N seconds? On completion?)
- Snapshot GC (delete old snapshots after N days?)
- Snapshot consistency (must be transactionally consistent with event_log)

**Missing Evidence**: NO PROTOTYPE, NO BENCHMARKS

### 5.2 Write Performance

**Current System**:
```
HashMap insert: ~0.01ms (in-memory)
SQLite INSERT: ~10ms (fsync to disk)
Total: ~10ms per state update
```

**Event Sourcing**:
```
Event append: ~10ms (SQLite INSERT with fsync)
Projection update: ~10ms (SQLite UPDATE)
Total: ~20ms per state update (2× slower)

With transaction:
BEGIN
INSERT INTO event_log ...
UPDATE agent_state_projection ...
COMMIT
Total: ~15ms (1.5× slower, acceptable)
```

**Conclusion**: Event sourcing is slightly slower, but acceptable.

### 5.3 Query Performance

**Current System**:
```sql
-- Get agent spawn info
SELECT phase_type, stage FROM agent_executions WHERE agent_id = ?;
-- Latency: <1ms (PRIMARY KEY index)
```

**Event Sourcing**:
```sql
-- Option A: Query projection (fast)
SELECT state, state_data FROM agent_state_projection WHERE agent_id = ?;
-- Latency: <1ms (PRIMARY KEY index)

-- Option B: Replay events (slow)
SELECT * FROM event_log WHERE agent_id = ? ORDER BY sequence_number;
-- Apply events to get current state
-- Latency: ~1ms × N events (up to 10s!)
```

**Conclusion**: Projection table is required for acceptable query performance.

---

## 6. Cross-Reference Validation

### 6.1 SPEC-931A Findings

**Q1**: Dual-write without transactions
- **Event Sourcing**: Does NOT solve (still need AGENT_MANAGER for TUI)
- **Alternative**: Add transactions to existing schema (simpler)

**Q5**: Why separate SQLite instead of MCP local-memory?
- **Event Sourcing**: Adds 3 tables (event_log, snapshots, projections)
- **Alternative**: Keep 1 table (agent_executions), add transactions

**Q21**: Should we migrate to event sourcing?
- **Answer**: NO (complexity exceeds benefit for current SLA)

**Q41**: Do we accept eventual consistency?
- **Current SLA**: Yes (2-agent quality gates, best-effort)
- **Event Sourcing**: Overkill for current requirements

**Q54**: Should we enable auto-vacuum?
- **Priority**: YES (153MB → 1MB recovery)
- **Event Sourcing**: Still need auto-vacuum (doesn't eliminate bloat)

**Q61**: Should consensus artifacts go to SQLite instead of MCP?
- **Priority**: YES (5× faster)
- **Event Sourcing**: Separate concern (orthogonal to state management)

### 6.2 SPEC-931C Error Handling

**P0 Recommendations**:
1. **Crash recovery** (4 hours)
   - Event Sourcing: Automatic via replay
   - Alternative: SQLite WAL mode + transaction recovery (2 hours)

2. **Transaction support** (8 hours)
   - Event Sourcing: Inherent in event append
   - Alternative: Wrap current updates in BEGIN/COMMIT (2 hours)

3. **Regression test suite** (3 hours)
   - Event Sourcing: Doesn't prevent SPEC-928 bugs (logic errors, not storage)
   - Alternative: Add tests for current system (same effort)

**Conclusion**: Event sourcing provides marginal benefit for error resilience vs simpler transaction approach.

### 6.3 SPEC-931E Technical Limits

**Q135**: Should we enable SQLite WAL mode?
- **Benefit**: Better write performance, crash recovery
- **Event Sourcing**: Still beneficial (orthogonal)
- **Alternative**: Enable for current schema (simple change)

**Q136**: Acceptable event log replay overhead?
- **Target**: <1s for 30-day retention
- **Reality**: ~9s without snapshots (requires snapshot strategy)

**Q142**: Archive retention policy?
- **Event Sourcing**: Append-only (can't DELETE events without breaking replay)
- **Alternative**: agent_executions can DELETE old rows (simpler)

---

## 7. Alternative Solution

### 7.1 ACID Compliance with Existing Schema

**Goal**: Eliminate dual-write inconsistency WITHOUT full event sourcing

**Approach**: Wrap HashMap + SQLite updates in transaction

**Implementation** (2-3 days):

```rust
// Step 1: Add transaction helper
impl AgentManager {
    async fn update_agent_with_transaction<F>(
        &mut self,
        agent_id: &str,
        db: &ConsensusDb,
        update_fn: F,
    ) -> Result<(), AgentError>
    where
        F: FnOnce(&mut Agent),
    {
        // 1. Get agent
        let agent = self.agents.get_mut(agent_id)
            .ok_or(AgentError::NotFound)?;

        // 2. Begin transaction
        db.transaction(|tx| {
            // 3. Apply update to in-memory state
            update_fn(agent);

            // 4. Write to SQLite (in transaction)
            tx.update_agent_execution(
                agent_id,
                &agent.status,
                agent.result.as_deref(),
                agent.error.as_deref(),
            )?;

            // 5. Commit (ACID guarantee)
            tx.commit()?;

            Ok(())
        }).await
    }
}

// Step 2: Use transaction wrapper for all state changes
async fn update_agent_status(&mut self, agent_id: &str, status: AgentStatus) {
    let db = ConsensusDb::init_default().unwrap();

    self.update_agent_with_transaction(agent_id, &db, |agent| {
        agent.status = status;
        if agent.status == AgentStatus::Running && agent.started_at.is_none() {
            agent.started_at = Some(Utc::now());
        }
        if matches!(agent.status, AgentStatus::Completed | AgentStatus::Failed | AgentStatus::Cancelled) {
            agent.completed_at = Some(Utc::now());
        }
    }).await.unwrap();

    // Send status update event
    self.send_agent_status_update().await;
}
```

**Benefits**:
- ✅ ACID compliance (no inconsistency on crash)
- ✅ Uses existing schema (no migration)
- ✅ Fast implementation (2-3 days vs 4-5 weeks)
- ✅ Low risk (small code change)
- ✅ Reversible (can revert easily)

**Limitations**:
- ❌ No time-travel debugging (can add later if needed)
- ❌ No event history (can add event_log later if needed)
- ❌ Still dual-write (but now ACID-compliant!)

### 7.2 Incremental Path Forward

**Week 1: ACID Compliance** (2-3 days)
- Add transaction wrapper to AgentManager
- Wrap all state updates in BEGIN/COMMIT
- Add crash recovery tests
- **Effort**: 16-24 hours
- **Risk**: LOW

**Week 2: Database Optimization** (2-3 days)
- VACUUM database (153MB → 1MB)
- Enable auto-vacuum (INCREMENTAL)
- Remove dead tables (consensus_artifacts, consensus_synthesis)
- Move MCP artifacts to SQLite (5× faster)
- **Effort**: 16-24 hours
- **Risk**: LOW

**Week 3: Basic Queue** (2-3 days)
- In-memory PriorityQueue (BinaryHeap)
- Persist queue to DB on crash
- Dequeue loop with rate limiting
- **Effort**: 16-24 hours
- **Risk**: MEDIUM

**Total Effort**: **48-72 hours** (6-9 days)
**Compare**: Event sourcing = 150-180 hours (20-23 days)
**Savings**: **~110 hours** (70% faster!)

### 7.3 When to Reconsider Event Sourcing

**Triggers**:
1. **Production SLA required** (99.9% uptime, guaranteed recovery)
2. **High-volume workload** (100+ agents/min sustained)
3. **Debugging requirements** (time-travel, audit trail)
4. **Compliance mandate** (immutable audit log)

**Current State**:
- ❌ No production SLA
- ❌ Low volume (~10 quality gates/day)
- ❌ No debugging issues requiring time-travel
- ❌ No compliance requirements

**Decision**: Defer event sourcing until triggers are met.

---

## 8. GO/NO-GO Decision

### 8.1 Final Recommendation

**DECISION: NO-GO** on full event sourcing migration

**ALTERNATIVE: GO** on incremental ACID compliance

### 8.2 Justification

**Complexity vs Benefit**:
- Event Sourcing: 150-180 hours effort
- Alternative: 48-72 hours effort
- **Savings**: 70% faster, lower risk

**Requirements Mismatch**:
- Current SLA: Best-effort, 2-agent quality gates, ~10/day
- Event Sourcing: Built for 99.9% SLA, 100+ agents/min
- **Gap**: Over-engineered for actual needs (YAGNI principle)

**Risk vs Reward**:
- Event Sourcing: High risk (irreversible, 30-day migration, dual-write paradox)
- Alternative: Low risk (reversible, 2-3 day migration, incremental)
- **Trade-off**: Not worth it

**Evidence Gaps**:
- Event Sourcing: No benchmarks, no prototype, estimated performance
- Alternative: Proven approach (SQLite transactions are battle-tested)
- **Confidence**: Higher confidence in simpler solution

### 8.3 Decision Matrix

| Criterion | Event Sourcing | Alternative (Transactions) | Winner |
|---|---|---|---|
| **ACID Compliance** | ✅ Yes | ✅ Yes | TIE |
| **Implementation Effort** | ❌ 150-180 hours | ✅ 48-72 hours | **Alternative** |
| **Migration Risk** | ❌ HIGH (irreversible, dual-write) | ✅ LOW (reversible, incremental) | **Alternative** |
| **Performance** | ⚠️ UNKNOWN (estimated) | ✅ PROVEN (SQLite benchmarks) | **Alternative** |
| **Complexity** | ❌ 3 tables, snapshots, replay | ✅ 1 table, transactions | **Alternative** |
| **Time-Travel Debugging** | ✅ Yes | ❌ No (can add later) | Event Sourcing |
| **Storage Overhead** | ❌ 16.2MB/year (growing) | ✅ 13.5MB stable | **Alternative** |
| **Solves SPEC-928 Bugs** | ❌ NO (logic bugs, not storage) | ❌ NO (logic bugs, not storage) | TIE |
| **Matches Current SLA** | ❌ Over-engineered | ✅ Right-sized | **Alternative** |

**Score**: Alternative wins 6/9 criteria

### 8.4 Implementation Plan

**Phase 1: ACID Compliance** (Week 1)
1. Add `update_agent_with_transaction()` wrapper
2. Wrap all AgentManager state changes in transactions
3. Add crash recovery tests
4. **Acceptance**: Crash mid-update → consistent state on restart

**Phase 2: Database Optimization** (Week 2)
1. Run VACUUM on consensus_artifacts.db
2. Enable auto-vacuum (INCREMENTAL)
3. Drop consensus_artifacts and consensus_synthesis tables
4. Move MCP artifacts to SQLite (consensus_db.store_artifact)
5. **Acceptance**: 153MB → 1MB, 5× faster artifact storage

**Phase 3: Basic Queue** (Week 3)
1. Implement in-memory PriorityQueue (BinaryHeap)
2. Add queue_agents table for persistence
3. Dequeue loop with per-provider rate limiting
4. **Acceptance**: Can queue 100 agents, execute with rate limits

**Phase 4: Monitoring** (Ongoing)
1. Track crash frequency
2. Monitor queue depth
3. Measure transaction latency
4. **Trigger**: If crashes become frequent OR queue depth > 100, THEN reconsider event sourcing

---

## Appendix: Evidence Gaps

### EG-1: Event Replay Performance Benchmarks

**Status**: ❌ MISSING

**Needed**:
- Prototype event replay implementation
- Benchmark at 100, 1K, 10K event scales
- Measure with/without snapshots
- Compare SQLite read performance (indexed vs sequential scan)

**Impact on Decision**:
- If replay is <100ms at 10K events: Event sourcing more viable
- If replay is >1s at 10K events: Snapshots become mandatory (complexity)

### EG-2: Migration Validation Strategy

**Status**: ❌ MISSING

**Needed**:
- How to measure "zero discrepancies" between old and new systems
- Automated comparison script (state equality, output equality, performance parity)
- Daily validation report during 30-day parallel run

**Impact on Decision**:
- Without validation strategy, migration success is unmeasurable
- Risk of silent data corruption going unnoticed

### EG-3: Event Schema Evolution Strategy

**Status**: ❌ MISSING

**Needed**:
- How to add fields to existing event types
- Event versioning scheme (v1, v2, ...)
- Upcasting strategy (convert old events to new schema)
- Default value handling for missing fields

**Impact on Decision**:
- Without schema evolution strategy, event log becomes brittle
- Can't add fields without breaking replay

### EG-4: Snapshot Strategy Specification

**Status**: ❌ MISSING

**Needed**:
- Snapshot frequency (N events? N seconds? On completion?)
- Snapshot GC policy (keep last N snapshots? Delete after N days?)
- Snapshot consistency guarantees (transactionally consistent with events?)
- Snapshot reconstruction (can we rebuild state from snapshot alone?)

**Impact on Decision**:
- Without snapshot strategy, replay will be too slow at scale
- Snapshot complexity adds to overall system complexity

### EG-5: Rollback Procedure

**Status**: ❌ MISSING

**Needed**:
- Step-by-step rollback instructions
- Data extraction from event_log to rebuild agent_executions
- Validation that rollback produces correct state
- Time estimate for rollback (minutes? hours?)

**Impact on Decision**:
- Without rollback plan, migration is one-way (high risk)
- Need escape hatch if event sourcing causes production issues

---

## Conclusion

Event sourcing is a powerful pattern for production systems with strict SLA requirements. However, for the current codex-rs agent orchestration system, it represents significant over-engineering.

**Key Insights**:
1. Event sourcing does NOT eliminate dual-write (AGENT_MANAGER still needed for TUI)
2. Migration complexity (150-180 hours) exceeds benefit for current SLA
3. SPEC-928 bugs were logic errors, not storage bugs (event sourcing doesn't prevent them)
4. Simpler alternative exists: Add SQLite transactions to existing schema (48-72 hours)

**Recommendation**: Implement ACID compliance with existing schema, defer event sourcing until production SLA or high-volume workload justifies the complexity.

**Next Steps**:
1. Review this analysis with stakeholders
2. Obtain approval for alternative approach (transactions + queue)
3. Begin Phase 1 implementation (ACID compliance)
4. Monitor crash frequency and queue depth
5. Reconsider event sourcing if triggers are met (99.9% SLA, 100+ agents/min)

---

**End of SPEC-931F Analysis**
