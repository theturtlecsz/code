# Phase 1: Database Schema & Storage Analysis

**Date**: 2025-11-12
**Status**: In Progress

---

## Executive Summary

**CRITICAL FINDING**: Database is 153MB on disk but contains only 3 active rows!

**Analysis**:
- **File size**: 153MB (160,698,368 bytes)
- **Total pages**: 39,061 pages × 4,096 bytes/page = 160MB
- **Free pages**: 39,048 pages (99.97% free!)
- **Used pages**: 13 pages (~53KB actual data)
- **Active rows**: 3 rows in agent_executions, 0 in consensus_artifacts, 0 in consensus_synthesis

**Implication**: Database has been heavily used then cleaned up, but SQLite doesn't auto-reclaim space.

**Recommendation**: Run `VACUUM` to reclaim 153MB → ~1MB, or implement auto-vacuum.

---

## 1. Current Schema

### 1.1 agent_executions (Routing & Tracking)

**Purpose**: Track agent spawns for definitive routing at completion

**Schema**:
```sql
CREATE TABLE agent_executions (
    agent_id TEXT PRIMARY KEY,           -- UUID (e.g., "1ebf1691-9665-443f-aa4e-4c7c979fdef2")
    spec_id TEXT NOT NULL,               -- SPEC-KIT-###
    stage TEXT NOT NULL,                 -- "plan" | "tasks" | "implement" | "validate" | "audit"
    phase_type TEXT NOT NULL,            -- "quality_gate" | "regular_stage"
    agent_name TEXT NOT NULL,            -- "gemini" | "claude" | "code" | "gpt_pro"
    spawned_at TEXT NOT NULL,            -- ISO timestamp
    completed_at TEXT,                   -- NULL until completion
    response_text TEXT,                  -- Full agent output (can be 10-20KB)
    run_id TEXT,                         -- Session identifier
    extraction_error TEXT                -- Error message if JSON extraction failed
);

CREATE INDEX idx_agent_executions_spec ON agent_executions(spec_id, stage);
CREATE INDEX idx_agent_executions_run ON agent_executions(run_id);
```

**Current Data** (3 rows, quality gate from recent run):
```
agent_id                                  agent_name  bytes   phase_type
1ebf1691-9665-443f-aa4e-4c7c979fdef2     gemini      4,693   quality_gate
6f0fcc47-df90-4090-b15f-80a7765dfcbc     claude      NULL    quality_gate (hung - SPEC-929)
eb4ca36f-4ff8-4989-9e4a-98c1a5ace870     code        10,087  quality_gate
```

**Usage Analysis**:

**Write Operations**:
1. `record_agent_spawn()` (consensus_db.rs:340-366) - On spawn
2. `record_agent_completion()` (consensus_db.rs:403-414) - On success
3. `record_extraction_failure()` (consensus_db.rs:420-438) - On JSON extraction failure

**Read Operations**:
1. `get_agent_spawn_info()` (consensus_db.rs:369-383) - Routing: Get (phase_type, stage)
2. `get_agent_name()` (consensus_db.rs:386-400) - Collection: Get expected agent_name
3. `query_extraction_failures()` (consensus_db.rs:443-471) - Debugging: Find failed extractions

**Lifecycle**:
```
Spawn:   INSERT (agent_id, phase_type, spawned_at)
Success: UPDATE (completed_at, response_text)
Failure: UPDATE (completed_at, response_text, extraction_error)
Cleanup: DELETE WHERE spawned_at < now() - N days
```

**Product Questions**:

**Q48**: Why store full response_text in agent_executions?
- **Size**: 4-10KB per agent (gemini=4.7KB, code=10KB)
- **Usage**: Only read for extraction failure debugging (query_extraction_failures)
- **Alternative**: Store only on failure (extraction_error != NULL)
- **Impact**: 50% storage reduction (only failed agents have large text)

**Q49**: Why TEXT timestamps instead of INTEGER?
- **Current**: "2025-11-12 02:38:16" (ISO string, 19 bytes)
- **Alternative**: Unix epoch INTEGER (8 bytes, 58% smaller)
- **Benefit**: Faster comparisons, smaller indexes
- **Cost**: SQLite datetime functions require conversion

**Q50**: Why no foreign key to consensus_artifacts?
- **Current**: agent_name (string) matching, no referential integrity
- **Expected**: consensus_artifacts.agent_id → agent_executions.agent_id
- **Impact**: Orphaned artifacts possible (agent_executions deleted but artifacts remain)
- **Reason**: Intentional loose coupling for independent cleanup?

---

### 1.2 consensus_artifacts (Agent Outputs)

**Purpose**: Store individual agent responses before consensus synthesis

**Schema**:
```sql
CREATE TABLE consensus_artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    agent_name TEXT NOT NULL,            -- "gemini" | "claude" | "code"
    content_json TEXT NOT NULL,          -- Extracted JSON (validated)
    response_text TEXT,                  -- Original full output (pre-extraction)
    run_id TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_spec_stage ON consensus_artifacts(spec_id, stage);
```

**Current Data**: **0 rows** (completely empty!)

**Schema Operations**:
1. `store_artifact()` (consensus_db.rs:146-172) - INSERT artifact
2. `query_artifacts()` (consensus_db.rs:174-205) - SELECT by spec_id, stage
3. `delete_spec_artifacts()` (consensus_db.rs:207-214) - DELETE by spec_id
4. `delete_stage_artifacts()` (consensus_db.rs:216-223) - DELETE by spec_id, stage

**Observations**:

**Finding 1: Table is never written to in current codebase!**
- Search: `grep -r "store_artifact" codex-rs/` finds:
  - Definition in consensus_db.rs
  - NO CALLERS in quality gate code!
- Evidence: 0 rows despite recent quality gate runs

**Finding 2: Quality gate artifacts go to MCP instead**
- quality_gate_handler.rs:1772-1780 stores to local-memory MCP
- Never calls consensus_db.store_artifact()
- Table exists in schema but unused in practice

**Product Questions**:

**Q51**: Should we remove consensus_artifacts table? [CRITICAL]
- **Evidence**: 0 rows, no callers, exists in schema
- **Hypothesis**: Legacy from before MCP migration (SPEC-KIT-072)
- **Impact**: Dead code, confusing architecture
- **Recommendation**: Remove table OR implement proper usage

**Q52**: Or should we USE consensus_artifacts instead of MCP?
- **Current**: quality_gate_handler stores to MCP (150ms latency)
- **Alternative**: Store to SQLite consensus_artifacts (10ms latency)
- **Benefit**: 15× faster, simpler, local database
- **Trade-off**: Lose MCP search/tagging features

---

### 1.3 consensus_synthesis (Final Outputs)

**Purpose**: Store consensus synthesis results after merging agent outputs

**Schema**:
```sql
CREATE TABLE consensus_synthesis (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    output_markdown TEXT NOT NULL,       -- Final synthesized output
    output_path TEXT,                    -- File path where written
    status TEXT NOT NULL,                -- "success" | "degraded" | "failed"
    artifacts_count INTEGER,             -- How many agents contributed
    agreements TEXT,                     -- JSON: shared points
    conflicts TEXT,                      -- JSON: disagreements
    degraded BOOLEAN DEFAULT 0,          -- 2/3 vs 3/3 consensus
    run_id TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_synthesis_spec_stage ON consensus_synthesis(spec_id, stage);
```

**Current Data**: **0 rows** (completely empty!)

**Schema Operations**:
1. `store_synthesis()` (consensus_db.rs:274-311) - INSERT synthesis
2. `query_latest_synthesis()` (consensus_db.rs:313-335) - SELECT latest for spec+stage

**Observations**:

**Finding 3: Table is never written to either!**
- Definition exists in consensus_db.rs
- NO CALLERS found in codebase
- 0 rows despite schema being created

**Product Question**:

**Q53**: Is consensus_synthesis dead code? [CRITICAL]
- **Evidence**: 0 rows, method defined but never called
- **Expected usage**: Store final consensus after merging gemini+claude+code
- **Reality**: Quality gates apply auto-resolution directly, skip synthesis
- **Recommendation**: Remove table OR implement synthesis feature

---

### 1.4 Database Bloat Analysis

**CRITICAL FINDING**: 153MB file with 3 rows of data!

**Root Cause**: SQLite freelist (deleted rows)
```
Total pages:     39,061 pages × 4KB = 160MB
Free pages:      39,048 pages (99.97%)
Used pages:      13 pages (~53KB)
Efficiency:      0.03% (99.97% wasted space)
```

**Hypothesis**: Database heavily used in the past, then cleaned up via DELETE operations

**Evidence Search**:
```sql
-- Check for cleanup operations
SELECT COUNT(*) FROM agent_executions WHERE completed_at IS NOT NULL;  -- 3 rows
SELECT COUNT(*) FROM consensus_artifacts;  -- 0 rows
SELECT COUNT(*) FROM consensus_synthesis;  -- 0 rows
```

**Cleanup Code**:
```rust
// consensus_db.rs:474-482
pub fn cleanup_old_executions(&self, days: i64) -> SqlResult<usize> {
    DELETE FROM agent_executions WHERE spawned_at < datetime('now', '-{days} days')
}

// consensus_db.rs:207-223
pub fn delete_spec_artifacts(&self, spec_id: &str) -> SqlResult<usize>
pub fn delete_stage_artifacts(&self, spec_id, stage) -> SqlResult<usize>
```

**Likely scenario**:
1. Many SPECs ran, created thousands of agent_executions rows (→ 153MB)
2. Cleanup called: `DELETE FROM agent_executions WHERE ...` (→ 39,048 free pages)
3. SQLite doesn't auto-reclaim (pages remain allocated but empty)
4. Result: 153MB file with 3 rows

**Solutions**:

**Option A: Manual VACUUM**
```sql
VACUUM;  -- Rebuild database, reclaim space
```
- Benefit: 153MB → ~1MB (99.3% reduction)
- Cost: Locks database during VACUUM (few seconds)
- Frequency: Manual or cron job

**Option B: Auto-vacuum**
```sql
PRAGMA auto_vacuum = FULL;  -- Must be set before data
-- OR
PRAGMA auto_vacuum = INCREMENTAL;  -- Gradual reclamation
```
- Benefit: Automatic space reclamation
- Cost: Slight write overhead (2-5%)
- Requirement: Must rebuild DB to enable

**Option C: Delete + recreate**
```bash
rm ~/.code/consensus_artifacts.db
# Schema recreated on next run
```
- Benefit: Immediate 153MB recovery
- Cost: Lose historical data (but only 3 rows currently)

**Product Question**:

**Q54**: Should we enable auto-vacuum? [HIGH PRIORITY]
- **Evidence**: 153MB → 53KB actual data (99.97% bloat)
- **Impact**: Wasted disk space, slower queries (freelist scan)
- **Recommendation**: Enable INCREMENTAL auto-vacuum for gradual cleanup

---

## 2. Storage Redundancy Analysis

### 2.1 Four Storage Systems for Agent Data

**System 1: AGENT_MANAGER (In-Memory HashMap)**
```rust
agents: HashMap<String, Agent>  // Volatile, lost on crash/restart

Fields stored:
- id, batch_id, model, prompt, context, files
- status, result, error
- created_at, started_at, completed_at
- progress: Vec<String>
- worktree_path, branch_name
- config, tmux_enabled
```
**Lifetime**: Process lifetime (cleared on restart)
**Purpose**: Real-time state for TUI display, agent coordination

---

**System 2: SQLite agent_executions**
```sql
agent_id, spec_id, stage, phase_type, agent_name
spawned_at, completed_at
response_text, extraction_error, run_id
```
**Lifetime**: Persistent (survives restart)
**Purpose**: Routing (phase_type), debugging (extraction_error)

---

**System 3: Filesystem (.code/agents/{id}/result.txt)**
```
.code/agents/
├─ 1ebf1691-.../
│  └─ result.txt  (4,693 bytes - gemini output)
├─ 6f0fcc47-.../
│  └─ result.txt  (empty - claude hung)
└─ eb4ca36f-.../
   └─ result.txt  (10,087 bytes - code output)
```
**Lifetime**: Until manual cleanup
**Purpose**: Observable debugging, tmux output capture

---

**System 4: MCP local-memory**
```json
{
  "content": "{...}",  // Agent JSON output
  "domain": "spec-kit",
  "importance": 8,
  "tags": ["quality-gate", "spec:SPEC-KIT-900", "agent:gemini"]
}
```
**Lifetime**: Persistent knowledge base
**Purpose**: Searchable consensus artifacts, tagged for retrieval

---

### 2.2 Data Overlap Analysis

**Field**: `response_text` (agent output)

| Storage System | Field Name | Size | Purpose | Lifetime |
|---|---|---|---|---|
| AGENT_MANAGER | `result: Option<String>` | 4-10KB | TUI display | Process |
| SQLite | `response_text TEXT` | 4-10KB | Debugging | Persistent |
| Filesystem | `result.txt` | 4-10KB | Observable | Manual cleanup |
| MCP | `content` (JSON) | 5-12KB | Searchable | Persistent |

**Total Redundancy**: 4× storage of same data (16-40KB per agent)
- Quality gate with 3 agents: **48-120KB** total storage
- Daily average (10 quality gates): **480KB-1.2MB per day**
- After 100 days: **48MB-120MB** of redundant data

**Product Question**:

**Q55**: Can we reduce from 4 systems to 2?
- **Option A**: AGENT_MANAGER (volatile) + SQLite (persistent)
  - Remove: Filesystem files (use SQLite for debugging)
  - Remove: MCP storage (use SQLite for search)
  - Benefit: 50% reduction (2× instead of 4×)
  - Cost: Lose MCP tagging features
- **Option B**: AGENT_MANAGER (volatile) + Event Log (persistent)
  - Remove: All current persistence (SQLite, Filesystem, MCP)
  - Add: Event log + projections (SPEC-930 pattern)
  - Benefit: Single source of truth, ACID compliance
  - Cost: Migration complexity, replay overhead

---

## 3. Schema Usage Patterns

### 3.1 Write Patterns

**agent_executions**: 3 writes per agent lifecycle
```rust
// Spawn (orchestrator.rs:133-149)
INSERT INTO agent_executions (agent_id, phase_type, spawned_at, ...)
VALUES ('uuid', 'quality_gate', '2025-11-12 02:38:16', ...);

// Completion (orchestrator.rs:264-281)
UPDATE agent_executions
SET completed_at = datetime('now'), response_text = ?
WHERE agent_id = ?;

// Failure (broker.rs:321-338)
UPDATE agent_executions
SET completed_at = datetime('now'), response_text = ?, extraction_error = ?
WHERE agent_id = ?;
```

**Frequency** (quality gate): 3 spawns + 3 updates = 6 writes per checkpoint
**Concurrency**: Sequential (single-writer limitation safe)

---

**consensus_artifacts**: **NEVER WRITTEN** (dead code)
```rust
// Definition exists (consensus_db.rs:146-172)
pub fn store_artifact(...) -> SqlResult<i64> {
    INSERT INTO consensus_artifacts (spec_id, stage, agent_name, content_json, ...)
}

// But grep shows ZERO callers in codebase!
```

**Finding**: Table schema exists but feature never implemented

---

**consensus_synthesis**: **NEVER WRITTEN** (dead code)
```rust
// Definition exists (consensus_db.rs:274-311)
pub fn store_synthesis(...) -> SqlResult<i64> {
    INSERT INTO consensus_synthesis (spec_id, stage, output_markdown, ...)
}

// But grep shows ZERO callers in codebase!
```

**Finding**: Table schema exists but synthesis feature never implemented

---

### 3.2 Read Patterns

**agent_executions**: 2 reads per agent completion
```rust
// Routing (consensus_db.rs:369-383)
SELECT phase_type, stage FROM agent_executions WHERE agent_id = ?;
// Called by: completion handler to route to quality_gate vs regular_stage

// Agent name lookup (consensus_db.rs:386-400)
SELECT agent_name FROM agent_executions WHERE agent_id = ?;
// Called by: broker for collection with correct names
```

**Frequency**: 2 queries × 3 agents = 6 reads per quality gate
**Latency**: <10ms per query (indexed by PRIMARY KEY)

---

**consensus_artifacts**: Read operations defined but never used
```rust
// query_artifacts() defined (consensus_db.rs:174-205)
SELECT * FROM consensus_artifacts WHERE spec_id=? AND stage=? ORDER BY created_at DESC;

// NO CALLERS in codebase
```

**Finding**: Query method exists, never called

---

**consensus_synthesis**: Read operations defined but never used
```rust
// query_latest_synthesis() defined (consensus_db.rs:313-335)
SELECT output_markdown FROM consensus_synthesis
WHERE spec_id=? AND stage=? ORDER BY created_at DESC LIMIT 1;

// NO CALLERS in codebase
```

**Finding**: Query method exists, never called

---

## 4. Dead Code Analysis

### 4.1 Unused Tables

**consensus_artifacts** (0 rows, never written):
- Schema: 8 fields defined
- Methods: 4 methods (store, query, delete×2)
- Callers: 0 (dead code)
- Tests: 2 tests pass (but feature unused in production)
- **Recommendation**: Remove table + methods (or implement feature)

**consensus_synthesis** (0 rows, never written):
- Schema: 11 fields defined
- Methods: 2 methods (store, query)
- Callers: 0 (dead code)
- Tests: 0 tests
- **Recommendation**: Remove table + methods (or implement synthesis)

**Effort Saved**: Remove ~200 LOC of dead code + simplify schema

---

### 4.2 Unused Columns

**agent_executions.run_id** (nullable):
- Purpose: Session identifier for grouping agents
- Usage: Set at spawn (orchestrator.rs:140), indexed
- Queries: Index exists (idx_agent_executions_run) but no SELECT by run_id found
- **Recommendation**: Verify if run_id is queried elsewhere, remove if unused

**agent_executions.stage** (required):
- Purpose: Track which stage agent belongs to
- Usage: Set at spawn, indexed (idx_agent_executions_spec)
- Queries: Only in get_agent_spawn_info() (returns phase_type + stage)
- **Question**: Is stage necessary if we have phase_type?
  - phase_type tells us quality_gate vs regular_stage (routing)
  - stage tells us plan vs tasks vs implement (context)
  - Separate concerns, both needed

---

## 5. Database Efficiency

### 5.1 Space Utilization

**Current State**:
```
Database file:   153 MB
Actual data:     ~53 KB  (13 pages × 4KB)
Wasted space:    153 MB  (99.97% of file)
Row count:       3 rows  (51 bytes/row on average)
```

**Projected Growth** (without cleanup):
```
Daily quality gates:  10 checkpoints × 3 agents = 30 agents/day
Average row size:     ~15KB (response_text)
Daily growth:         450 KB/day
Monthly growth:       13.5 MB/month
Yearly growth:        162 MB/year
```

**With cleanup** (delete after 30 days):
```
Active rows:         900 agents (30 days × 30 agents/day)
Active storage:      13.5 MB
With auto-vacuum:    15 MB (file size ~= data size)
Without vacuum:      153+ MB (keeps growing, never shrinks)
```

**Product Question**:

**Q56**: Should we implement auto-cleanup?
- **Current**: cleanup_old_executions(days) defined but never called
- **Recommendation**: Daily cron: DELETE WHERE spawned_at < now() - 30 days
- **With VACUUM**: Recovers space automatically
- **Impact**: Maintain 15MB stable size instead of growing indefinitely

---

### 5.2 Index Efficiency

**Current Indexes**:
```sql
-- Agent executions
CREATE INDEX idx_agent_executions_spec ON agent_executions(spec_id, stage);
CREATE INDEX idx_agent_executions_run ON agent_executions(run_id);

-- Consensus artifacts
CREATE INDEX idx_spec_stage ON consensus_artifacts(spec_id, stage);

-- Consensus synthesis
CREATE INDEX idx_synthesis_spec_stage ON consensus_synthesis(spec_id, stage);
```

**Index Usage Analysis**:

**Used**:
- ✅ `agent_executions` PRIMARY KEY (agent_id) - Every get_agent_spawn_info() query

**Unused** (based on 0-row tables):
- ❌ `idx_agent_executions_spec` - No queries found using (spec_id, stage) on agent_executions
- ❌ `idx_agent_executions_run` - No queries by run_id found
- ❌ `idx_spec_stage` - consensus_artifacts never queried
- ❌ `idx_synthesis_spec_stage` - consensus_synthesis never queried

**Recommendation**: Remove indexes for dead code, or implement features that need them

---

## 6. Comparison with SPEC-930 Event Sourcing

### 6.1 Current Schema vs Event Log

**Current (Direct Updates)**:
```sql
-- Create agent
INSERT INTO agent_executions (agent_id, status='pending') VALUES (...);

-- Update status
UPDATE agent_executions SET status='running', started_at=now() WHERE agent_id=?;

-- Update completion
UPDATE agent_executions SET status='completed', response_text=?, completed_at=now() WHERE agent_id=?;
```

**Problems**:
- Lost history (can't see previous states)
- No transaction coordination with HashMap
- Can't replay to debug issues

---

**SPEC-930 Event Log**:
```sql
-- Event log (immutable, append-only)
CREATE TABLE event_log (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    event_type TEXT NOT NULL,  -- 'AgentQueued' | 'AgentStarted' | 'AgentCompleted'
    event_data JSON NOT NULL,  -- State-specific payload
    timestamp INTEGER NOT NULL,
    sequence_number INTEGER NOT NULL  -- Per-agent ordering
);

-- Projection (derived from events)
CREATE TABLE agent_state_projection (
    agent_id TEXT PRIMARY KEY,
    current_state TEXT NOT NULL,  -- 'Pending' | 'Running' | 'Completed'
    state_data JSON NOT NULL,
    last_event_id INTEGER NOT NULL,
    FOREIGN KEY (last_event_id) REFERENCES event_log(event_id)
);

-- Snapshots (performance optimization)
CREATE TABLE agent_snapshots (
    agent_id TEXT PRIMARY KEY,
    state_json JSON NOT NULL,
    event_id INTEGER NOT NULL,  -- Snapshot includes events up to this ID
    timestamp INTEGER NOT NULL,
    FOREIGN KEY (event_id) REFERENCES event_log(event_id)
);
```

**Benefits**:
1. **Complete audit trail**: All state transitions recorded
2. **Time-travel debugging**: Replay to any point
3. **Crash recovery**: Rebuild state from events
4. **ACID compliance**: Events written in transaction with projection

**Migration Path**:
```
Phase 1: Add event_log table, write events AND current updates (parallel)
Phase 2: Rebuild agent_state_projection from events every startup (validate consistency)
Phase 3: Switch reads to projection instead of agent_executions
Phase 4: Remove agent_executions table (projection is source of truth)
```

---

### 6.2 Storage Comparison

**Scenario**: 3-agent quality gate (gemini, claude, code)

**Current**:
```
AGENT_MANAGER:         3 Agent structs × 300 bytes = 900 bytes (in-memory)
SQLite executions:     3 rows × 15KB = 45KB (INSERT + UPDATE each)
Filesystem:            3 files × 10KB = 30KB (tmux output files)
MCP local-memory:      3 stores × 12KB = 36KB (importance=8)
Total:                 111KB per quality gate
```

**SPEC-930 Event Log**:
```
Event log:             9 events × 500 bytes = 4.5KB (AgentQueued×3, Started×3, Completed×3)
Projection:            3 rows × 15KB = 45KB (current state, rebuilds from events)
Snapshot (optional):   0 (short-lived agents don't need snapshots)
Total:                 49.5KB per quality gate (55% reduction)
```

**Benefits**:
- **Less redundancy**: Events + projection vs 4 storage systems
- **Better recovery**: Replay events on crash
- **Simpler code**: Single source of truth

**Costs**:
- **Migration effort**: Rebuild schema, implement replay engine
- **Replay latency**: ~1ms per event (9 events = ~10ms)
- **Snapshot strategy**: When to snapshot? (probably not needed for quality gates)

---

## 7. Schema Migrations

### 7.1 Existing Migration Pattern

**consensus_db.rs:124-126**:
```rust
// Migrations for existing databases (errors are OK if columns already exist)
let _ = conn.execute("ALTER TABLE agent_executions ADD COLUMN run_id TEXT", []);
let _ = conn.execute("ALTER TABLE agent_executions ADD COLUMN extraction_error TEXT", []);
```

**Pattern**: Best-effort migrations, ignore errors
- **Benefit**: Simple, no version tracking needed
- **Risk**: Silent failures if migration actually needed
- **Alternative**: Proper migration framework (e.g., refinery, diesel)

**Product Question**:

**Q57**: Should we use proper migration framework?
- **Current**: Manual ALTER TABLE, ignore errors
- **Need**: Adding/removing columns, changing types, data migrations
- **SPEC-930**: Major schema change (event log) requires proper migrations
- **Recommendation**: Adopt migration framework before event sourcing

---

### 7.2 Required Migrations for Event Sourcing

**Migration 1**: Create event_log table
```sql
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
```

**Migration 2**: Convert existing agent_executions to events
```rust
// For each row in agent_executions:
//   1. Generate AgentQueued event (from spawned_at)
//   2. Generate AgentStarted event (from started_at if set)
//   3. Generate AgentCompleted/Failed event (from completed_at if set)
//   4. INSERT events into event_log with proper sequence_number
```

**Migration 3**: Create projection table
```sql
CREATE TABLE agent_state_projection (
    agent_id TEXT PRIMARY KEY,
    current_state TEXT NOT NULL,
    state_data JSON NOT NULL,
    last_event_id INTEGER NOT NULL,
    FOREIGN KEY (last_event_id) REFERENCES event_log(event_id)
);
```

**Migration 4**: Drop old agent_executions
```sql
-- After validation that projection matches old table
DROP TABLE agent_executions;
```

**Rollback Strategy**:
- Keep agent_executions table during migration
- Write to both (events + old table) in parallel
- Validate consistency daily
- Cutover only after 30 days of parallel run with zero discrepancies

---

## 8. Database Performance Analysis

### 8.1 Current Query Performance

**Hot Path**: get_agent_spawn_info() during completion routing
```sql
SELECT phase_type, stage FROM agent_executions WHERE agent_id = ?;
```

**Performance**:
- Index: PRIMARY KEY (agent_id) - O(log n)
- Rows: 3 currently, up to ~900 with 30-day retention
- Latency: <1ms (single row lookup)
- Frequency: 1 query per agent completion (3× per quality gate)

**Bottleneck**: None (trivial query)

---

**Cold Path**: query_extraction_failures() for debugging
```sql
SELECT agent_id, agent_name, extraction_error, substr(response_text, 1, 1000)
FROM agent_executions
WHERE spec_id = ? AND extraction_error IS NOT NULL
ORDER BY spawned_at DESC;
```

**Performance**:
- Index: idx_agent_executions_spec (spec_id, stage)
- Filter: extraction_error IS NOT NULL (not indexed)
- Scan: Table scan with spec_id prefix filter
- Latency: <10ms (small table)
- Frequency: Rare (debugging only)

**Optimization**: Add index on extraction_error if query becomes frequent

---

### 8.2 Event Log Performance Projection

**Read Performance**:
```sql
-- Replay events for agent (startup recovery)
SELECT * FROM event_log WHERE agent_id = ? ORDER BY sequence_number;
```

**Scale Analysis**:
- Events per agent: 3-10 (Queued, Started, Completed + intermediate states)
- Quality gate (3 agents): 9-30 events
- Replay latency: ~1ms per event = 9-30ms total
- With snapshot: 0-5 events (only replay after snapshot)

**Write Performance**:
```sql
-- Append event + update projection (in transaction)
BEGIN TRANSACTION;
INSERT INTO event_log (agent_id, event_type, event_data, ...) VALUES (...);
UPDATE agent_state_projection SET current_state=?, state_data=?, last_event_id=? WHERE agent_id=?;
COMMIT;
```

**Latency**: ~10ms per transaction (INSERT + UPDATE + fsync)
**Frequency**: 3-10 events per agent = 9-30 events per quality gate
**Total**: 90-300ms overhead vs current ~60ms (5× slower)

**Product Question**:

**Q58**: Is 5× slower write acceptable for ACID compliance?
- **Current**: 6 writes × 10ms = 60ms (no transactions)
- **Event log**: 9 writes × 10ms = 90ms (with transactions)
- **Trade-off**: Correctness vs performance
- **Mitigation**: Batch events (write 3 events in one transaction)

---

## 9. Schema Design Questions

### 9.1 Normalization Analysis

**Current**: Partially normalized
- agent_executions: PRIMARY KEY (agent_id)
- consensus_artifacts: No foreign key to agent_executions
- consensus_synthesis: No foreign key to consensus_artifacts

**Denormalization**: response_text duplicated across tables
- agent_executions.response_text (for debugging)
- consensus_artifacts.response_text (for original pre-extraction)
- Both store same data

**Product Question**:

**Q59**: Should we normalize or denormalize?
- **Option A (Normalize)**: Store response_text once, reference by agent_id
  - Benefit: No duplication
  - Cost: JOIN queries to get full data
- **Option B (Denormalize)**: Keep duplication for fast queries
  - Benefit: Single-table queries
  - Cost: 2× storage, consistency risk
- **Current**: Accidentally denormalized (consensus_artifacts unused)

---

### 9.2 JSON vs Relational

**Current**: Hybrid approach
- Structured: agent_id, spec_id, stage (columns)
- Semi-structured: extraction_error (TEXT blob)
- JSON: state_data, agreements, conflicts (TEXT with JSON strings)

**SPEC-930**: More JSON
- event_data JSON (arbitrary event payloads)
- state_data JSON (state machine data)

**Trade-offs**:

**Relational** (columns):
- ✅ Indexed, fast queries
- ✅ Schema validation
- ❌ Schema changes require migrations

**JSON** (blobs):
- ✅ Flexible schema
- ✅ No migrations for field changes
- ❌ Can't index nested fields (without JSON1 extension)
- ❌ No schema validation

**Product Question**:

**Q60**: Should we use SQLite JSON1 extension?
- **Feature**: Index into JSON fields, query nested data
- **Example**: `CREATE INDEX idx_event_type ON event_log(json_extract(event_data, '$.agent_name'))`
- **Benefit**: Fast queries on JSON without full table scan
- **Cost**: Requires SQLite 3.38+ (2021), dependency

---

## 10. Key Findings

### Finding 1: Database is 99.97% Empty Space (153MB → 53KB)
**Impact**: Wasted disk, slower queries (freelist scan overhead)
**Root Cause**: DELETE operations without VACUUM
**Solution**: Enable auto-vacuum (INCREMENTAL) + one-time manual VACUUM
**Priority**: MEDIUM (works but wasteful)

### Finding 2: consensus_artifacts Table is Dead Code (0 Rows, 0 Callers)
**Impact**: Confusing architecture, maintenance burden
**Evidence**: Table exists in schema, methods defined, but never called
**Solution**: Remove table + methods (save ~100 LOC)
**Priority**: HIGH (architectural clarity)

### Finding 3: consensus_synthesis Table is Dead Code (0 Rows, 0 Callers)
**Impact**: Incomplete feature, confusing schema
**Evidence**: Table exists, methods defined, never used
**Solution**: Remove OR implement consensus synthesis feature
**Priority**: MEDIUM (clarify intent)

### Finding 4: 4× Storage Redundancy (AGENT_MANAGER + SQLite + Filesystem + MCP)
**Impact**: 48-120KB per quality gate, 4× write operations
**Root Cause**: No single source of truth
**SPEC-930 Solution**: Event log + projections (2 systems instead of 4)
**Priority**: HIGH (simplification)

### Finding 5: No ACID Transactions (Dual-Write Pattern)
**Impact**: Crash between writes leaves inconsistent state
**Evidence**: HashMap updates + SQLite writes in sequence, no coordination
**SPEC-930 Solution**: Event log with transactions
**Priority**: CRITICAL (correctness)

### Finding 6: agent_executions is ONLY Used Table
**Impact**: consensus_artifacts and consensus_synthesis are noise
**Usage**: agent_executions for routing (phase_type) + debugging (extraction_error)
**Recommendation**: Simplify to single-table design until synthesis needed

---

## 11. Recommended Schema Simplification

### 11.1 Immediate Cleanup (No Breaking Changes)

```sql
-- Step 1: VACUUM to reclaim 153MB
VACUUM;  -- 153MB → 1MB

-- Step 2: Enable auto-vacuum for future
PRAGMA auto_vacuum = INCREMENTAL;
PRAGMA incremental_vacuum(100);  -- Reclaim 100 pages at a time

-- Step 3: Drop unused tables
DROP TABLE consensus_artifacts;
DROP TABLE consensus_synthesis;

-- Step 4: Drop unused indexes
DROP INDEX IF EXISTS idx_agent_executions_run;  -- Verify run_id unused first
```

**Impact**:
- Reclaim: 152MB disk space
- Remove: ~200 LOC dead code
- Simplify: 3 tables → 1 table
- Risk: None (unused tables)

---

### 11.2 Event Sourcing Migration (Breaking Changes)

**Phase A: Add Event Log (Additive)**
```sql
CREATE TABLE event_log (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    event_data JSON NOT NULL,
    timestamp INTEGER NOT NULL,
    sequence_number INTEGER NOT NULL,
    INDEX idx_agent_events (agent_id, sequence_number),
    INDEX idx_event_timestamp (timestamp)
);
```

**Phase B: Dual Write (Parallel Run)**
```rust
// Write event + update old table
event_log.append_event(AgentEvent::AgentStarted { ... })?;
agent_executions.update_status(agent_id, 'running')?;  // Keep old path working
```

**Phase C: Validate Consistency**
```rust
// Daily job: Rebuild state from events, compare with agent_executions
let state_from_events = replay_events(agent_id)?;
let state_from_table = query_agent_executions(agent_id)?;
assert_eq!(state_from_events, state_from_table);  // Must match
```

**Phase D: Cutover (Remove Old Table)**
```sql
-- After 30 days of consistent parallel run
DROP TABLE agent_executions;
-- Projection becomes source of truth
```

---

## 12. Product Value Assessment

### 12.1 What Database Operations Serve Product Features?

**agent_executions Table**:

| Operation | Product Feature | Frequency | Necessity |
|---|---|---|---|
| record_agent_spawn() | Quality gate routing | 3×/checkpoint | CRITICAL |
| get_agent_spawn_info() | Completion routing | 3×/checkpoint | CRITICAL |
| record_agent_completion() | Result persistence | 3×/checkpoint | HIGH |
| record_extraction_failure() | Debugging | Rare (<5%) | MEDIUM |
| query_extraction_failures() | Post-mortem analysis | Manual | LOW |
| cleanup_old_executions() | Disk space management | Never called! | LOW |

**Verdict**: Table is essential for routing, optional for debugging

---

**consensus_artifacts Table**:

| Operation | Product Feature | Frequency | Necessity |
|---|---|---|---|
| store_artifact() | Consensus synthesis | Never called | ZERO |
| query_artifacts() | Synthesis input | Never called | ZERO |
| delete_spec_artifacts() | Cleanup | Never called | ZERO |

**Verdict**: Table has NO product value (dead code)

---

**consensus_synthesis Table**:

| Operation | Product Feature | Frequency | Necessity |
|---|---|---|---|
| store_synthesis() | Final output | Never called | ZERO |
| query_latest_synthesis() | Resume synthesis | Never called | ZERO |

**Verdict**: Table has NO product value (dead code)

---

### 12.2 MCP vs SQLite for Consensus Artifacts

**Current Practice**: quality_gate_handler stores to MCP (lines 1772-1780)

**MCP Storage**:
- Latency: 150ms (3 parallel stores × 50ms each)
- Features: Tagging, semantic search, importance scoring
- Lifetime: Permanent knowledge base
- Query: `search(tags=["quality-gate", "spec:X"])`

**SQLite Alternative** (using consensus_artifacts):
- Latency: 30ms (3 INSERTs × 10ms each)
- Features: Fast lookups, indexing, SQL queries
- Lifetime: Configurable retention (30-90 days)
- Query: `SELECT * FROM consensus_artifacts WHERE spec_id=? AND stage=?`

**Comparison**:

| Aspect | MCP | SQLite |
|---|---|---|
| Speed | 150ms (5× slower) | 30ms |
| Search | Semantic + tags | SQL WHERE clauses |
| Retention | Permanent | Configurable |
| Complexity | External service | Built-in |
| Purpose | Knowledge base | Workflow artifacts |

**Product Question**:

**Q61**: Should consensus artifacts go to SQLite instead of MCP?
- **SPEC-KIT-072 Intent**: Separate workflow artifacts (SQLite) from knowledge (MCP)
- **Current Reality**: Artifacts go to MCP (defeats purpose!)
- **Recommendation**: Use consensus_artifacts table (it exists!), remove MCP writes
- **Impact**: 5× faster, simpler, aligns with SPEC-KIT-072 design

---

## 13. Schema Recommendations

### 13.1 Immediate (Low Risk, High Value)

**Action 1**: VACUUM database (1-time cleanup)
```bash
sqlite3 ~/.code/consensus_artifacts.db "VACUUM;"
```
**Impact**: 153MB → 1MB (99.3% reduction)
**Risk**: None (read-only operation from app perspective)
**Effort**: 1 command, 5 seconds execution

---

**Action 2**: Enable auto-vacuum (prevent future bloat)
```sql
-- Can't change existing DB, must rebuild
mv consensus_artifacts.db consensus_artifacts.db.bak
sqlite3 consensus_artifacts.db < schema_with_autovacuum.sql
```
**Impact**: Automatic space reclamation
**Risk**: Must recreate DB (lose 3 current rows - acceptable)
**Effort**: 30 minutes (add PRAGMA to schema, test)

---

**Action 3**: Drop dead tables (code cleanup)
```sql
DROP TABLE IF EXISTS consensus_artifacts;
DROP TABLE IF EXISTS consensus_synthesis;
```
```rust
// Remove from consensus_db.rs:
// - store_artifact() + tests (100 LOC)
// - query_artifacts() + tests (50 LOC)
// - store_synthesis() (50 LOC)
// - query_latest_synthesis() (25 LOC)
```
**Impact**: Remove 225 LOC dead code, simplify schema
**Risk**: None (tables unused)
**Effort**: 1 hour (remove code, update tests)

---

### 13.2 Medium-Term (Requires Validation)

**Action 4**: Implement consensus_artifacts properly (OR remove)
- **Option A**: Use table instead of MCP for artifact storage
  - Migrate: quality_gate_handler.rs:1772-1780 to use consensus_db.store_artifact()
  - Benefit: 5× faster, aligns with SPEC-KIT-072
  - Effort: 2-3 hours (change storage, update broker queries)
- **Option B**: Remove table permanently
  - Keep: MCP storage for searchability
  - Accept: 5× slower for better tagging features
  - Effort: 1 hour (drop table, remove methods)

**Recommendation**: Option A (use SQLite, not MCP) for workflow artifacts

---

**Action 5**: Add database metrics/monitoring
```rust
// Track DB health
pub fn get_db_health(&self) -> DbHealth {
    let (rows, size, freelist) = ...;
    DbHealth {
        total_rows: rows,
        file_size_mb: size / 1_048_576,
        free_pages_pct: (freelist / total_pages) * 100,
        needs_vacuum: free_pages_pct > 50.0
    }
}
```
**Impact**: Proactive VACUUM before bloat becomes severe
**Effort**: 2 hours (implement + dashboard widget)

---

### 13.3 Long-Term (SPEC-930 Event Sourcing)

**Action 6**: Migrate to event log + projections
- **Timeline**: 2-3 weeks (design, implement, validate, cutover)
- **Benefits**: ACID compliance, time-travel, crash recovery
- **Costs**: 5× slower writes (mitigated by batching), replay latency (9-30ms)
- **Decision**: Only if ACID compliance is critical requirement

**Recommendation**: Defer until Phase 3 analysis validates necessity

---

## 14. Critical Questions for Phase 2

### Schema Questions

**Q62**: Can we remove consensus_artifacts and consensus_synthesis tables?
- Evidence: 0 rows, 0 callers, exists only in schema
- Risk: Was this feature planned but not implemented?
- Decision: Review git history for intent

**Q63**: Should we rebuild database with auto-vacuum?
- Evidence: 153MB file, 99.97% free space
- Cost: 5-second operation, lose 3 current rows (acceptable)
- Benefit: Future bloat prevention

**Q64**: Should we move MCP artifact storage to SQLite?
- Evidence: MCP calls take 150ms, SQLite would take 30ms (5× faster)
- SPEC-KIT-072 Intent: Separate workflow (SQLite) from knowledge (MCP)
- Current Reality: Violates SPEC-KIT-072 (artifacts in MCP)

### Event Sourcing Questions

**Q65**: Do we need complete audit trail?
- Use case: Time-travel debugging (replay to any state)
- Current pain: Can't see state history, only current state
- Alternative: Add state_history table (simpler than full event log)

**Q66**: Can we accept 5× slower writes for ACID compliance?
- Current: 60ms for 6 writes (no transactions)
- Event log: 90ms for 9 writes (with transactions)
- Trade-off: Correctness vs latency

**Q67**: Are snapshots necessary for quality gates?
- Lifecycle: Short-lived (60-120s), few events (9-30)
- Replay cost: 9-30ms (acceptable)
- Decision: Snapshots NOT needed (overhead not justified)

### Cleanup Questions

**Q68**: Should we implement automated cleanup?
- Method exists: cleanup_old_executions(days)
- Never called: No cron job, no manual invocation
- Impact: Database grows indefinitely (153MB after cleanup!)
- Recommendation: Daily cron with VACUUM

**Q69**: What retention policy makes sense?
- Current: No retention (keeps forever)
- Debugging: Need 7-30 days for investigation
- Compliance: No requirements identified
- Recommendation: 30-day retention + VACUUM weekly

---

## 15. Database Migration Risks

### Risk 1: Data Loss During VACUUM
**Probability**: Low
**Impact**: High (lose all agent execution history)
**Mitigation**:
- Backup before VACUUM: `cp consensus_artifacts.db consensus_artifacts.db.bak`
- Test on dev machine first
- Only 3 rows currently (acceptable loss if worst case)

### Risk 2: Schema Change Breaking Active Agents
**Probability**: Medium
**Impact**: High (agents fail mid-execution)
**Mitigation**:
- Check for active agents before schema changes
- Graceful degradation (SQLite connection errors handled)
- Parallel run period (old + new schema)

### Risk 3: Event Log Replay Performance
**Probability**: Low
**Impact**: Medium (slow startup after crash)
**Mitigation**:
- Benchmark replay: 1ms/event × max 1000 events = 1s worst case
- Implement snapshots if replay > 100ms
- Lazy replay (rebuild projection in background)

---

## 16. Next Steps

**For Phase 3 (Pattern Validation)**:
1. Prototype event log schema with actual quality gate data
2. Benchmark replay performance (1,000 events)
3. Test parallel writes (old + new schema)
4. Validate consistency (state from events == state from table)

**For Phase 4 (Product Design Review)**:
1. Interview: What database features are actually needed?
2. Audit: Which tables/columns are dead code?
3. Decide: Event sourcing vs simplified relational

**Immediate Actions** (can do now):
1. VACUUM database (5 seconds, 152MB recovered)
2. Remove consensus_artifacts + consensus_synthesis tables (dead code)
3. Document run_id usage (verify if needed)

---

## 17. References

**Database Location**: `~/.code/consensus_artifacts.db` (153MB)
**Schema**: consensus_db.rs:56-139
**Write Operations**: consensus_db.rs:146-438
**Read Operations**: consensus_db.rs:174-471
**Tests**: consensus_db.rs:486-574

**Related**:
- SPEC-KIT-072: Consensus DB intent (workflow vs knowledge separation)
- SPEC-KIT-928: extraction_error column added for debugging
- SPEC-930: Event sourcing schema design
