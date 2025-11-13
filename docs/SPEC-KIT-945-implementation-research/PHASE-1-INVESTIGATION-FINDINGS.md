# Phase 1 Investigation Findings

**Date**: 2025-11-13
**Branch**: feature/phase-1-sqlite-retry
**Status**: Prerequisites Complete - Ready for Implementation

---

## Executive Summary

Investigation complete for Phase 1 (SPEC-945B SQLite + SPEC-945C Retry) implementation. Key findings:

1. **No separate AGENT_MANAGER HashMap** - Agent state stored directly in SQLite `agent_executions` table
2. **Current database**: `consensus_db.rs` with `Arc<Mutex<Connection>>` (single connection)
3. **Dual-write pattern**: In-memory `SpecAutoPhase` state + SQLite writes (not HashMap + SQLite)
4. **MCP status**: Likely already migrated (SQLite consensus_artifacts has data, no MCP calls found)
5. **Architecture decision**: Database module goes to `codex-core/src/db/` per user choice

---

## 1. AGENT_MANAGER Investigation Results

### Finding: No Explicit HashMap-Based AGENT_MANAGER

**Searched**:
- `grep -r "AGENT_MANAGER"` → No results
- Checked: `state.rs`, `agent_orchestrator.rs`, `quality_gate_handler.rs`

**Actual Implementation**:
- Agent state stored in SQLite `agent_executions` table
- In-memory coordination via `SpecAutoPhase` enum (in `state.rs`)
- No separate HashMap tracking agent lifecycle

### Current Agent State Management

**SQLite Table** (`consensus_db.rs:109-122`):
```sql
CREATE TABLE IF NOT EXISTS agent_executions (
    agent_id TEXT PRIMARY KEY,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    phase_type TEXT NOT NULL,  -- "quality_gate" | "regular_stage"
    agent_name TEXT NOT NULL,
    run_id TEXT,
    spawned_at TEXT NOT NULL,
    completed_at TEXT,
    response_text TEXT,
    extraction_error TEXT
)
```

**Key Methods** (`consensus_db.rs:340-414`):
```rust
pub fn record_agent_spawn(...) -> SqlResult<()>        // Line 340
pub fn get_agent_spawn_info(...) -> SqlResult<...>    // Line 369
pub fn get_agent_name(...) -> SqlResult<...>          // Line 386
pub fn record_agent_completion(...) -> SqlResult<()>  // Line 403
pub fn record_extraction_failure(...) -> SqlResult<()> // Line 420
```

**Usage Pattern** (`agent_orchestrator.rs`):
- Line 337: `db.record_agent_spawn(...)` when spawning agents
- Line 374: `db.record_agent_completion(...)` on agent success
- Line 635: Multiple spawn calls in quality gate orchestration
- Line 862: Completion recording in native consensus

### Dual-Write Pattern Identified

**NOT** HashMap + SQLite (as SPEC-933 suggests)
**ACTUALLY** In-Memory State + SQLite:

1. **In-Memory** (`state.rs:17-22`):
   ```rust
   SpecAutoPhase::ExecutingAgents {
       expected_agents: Vec<String>,
       completed_agents: HashSet<String>,
   }
   ```

2. **SQLite** (`consensus_db.rs:340`):
   ```rust
   db.record_agent_spawn(agent_id, spec_id, stage, ...)
   ```

**Potential Race Condition**:
- TUI updates `completed_agents` HashSet (in-memory)
- Separate call to `record_agent_completion()` writes SQLite
- **Gap**: Crash between HashSet update and SQLite write = inconsistency

---

## 2. Current Database Architecture

### Existing Implementation

**Location**: `tui/src/chatwidget/spec_kit/consensus_db.rs`

**Connection Management**:
```rust
pub struct ConsensusDb {
    conn: Arc<Mutex<Connection>>,  // Single connection, mutex-protected
}
```

**Initialization** (`consensus_db.rs:39-143`):
- Creates `consensus_artifacts`, `consensus_synthesis`, `agent_executions` tables
- Default path: `~/.code/consensus_artifacts.db`
- No pragmas configured (uses SQLite defaults)
- No connection pooling

**Schema** (see `consensus_db.rs:62-138`):
- `consensus_artifacts`: Agent outputs (7 columns)
- `consensus_synthesis`: Final consensus results (11 columns)
- `agent_executions`: Agent lifecycle tracking (9 columns)

**Missing from Current Implementation**:
- ❌ Connection pooling (r2d2-sqlite)
- ❌ WAL mode pragma
- ❌ Optimized pragmas (cache_size, synchronous, mmap_size)
- ❌ Auto-vacuum (incremental or full)
- ❌ Transaction coordination (ACID guarantees)
- ❌ Schema versioning/migrations

---

## 3. Schema Migration Requirements

### User Decision: Migrate to New Normalized Schema

**Current Schema** (flat, mixed concerns):
```sql
consensus_artifacts (
    id, spec_id, stage, agent_name,  -- Agent identity
    content_json,                     -- Agent output
    response_text, run_id, created_at -- Metadata
)
```

**Target Schema** (SPEC-945B proposal):
```sql
consensus_runs (
    id, spec_id, stage, run_timestamp,
    consensus_ok, degraded, synthesis_json  -- Run metadata
)

agent_outputs (
    id, run_id,                        -- Foreign key to runs
    agent_name, model_version, content, -- Agent-specific data
    output_timestamp
)
```

### Migration Strategy (5 Phases)

**Phase 1: Create New Tables** (additive, zero downtime):
- Add `consensus_runs` and `agent_outputs` tables
- Keep existing tables operational
- No data migration yet

**Phase 2: Dual-Write Mode** (validation period, 1 week):
- Write to BOTH old and new schemas
- Validate data consistency (0% mismatch target)
- Compare read results from both schemas

**Phase 3: Flip Reads** (use new schema):
- Read from `consensus_runs` + `agent_outputs`
- Keep dual-write active (rollback safety)
- Monitor for errors (can revert to old schema if needed)

**Phase 4: Write to New Only** (complete migration):
- Stop writing to old schema
- New schema is source of truth
- Keep old tables for emergency rollback (don't drop yet)

**Phase 5: Cleanup** (after 2 weeks validation):
- Drop old tables (`consensus_artifacts`, etc.)
- Remove old schema code
- Document migration completion

### Data Preservation Requirements

**Existing Data** (must preserve):
```bash
$ sqlite3 ~/.code/consensus_artifacts.db "SELECT COUNT(*) FROM consensus_artifacts"
# Output: <actual count> rows
```

**Migration Script** (one-time, Phase 2):
```sql
-- Extract runs from artifacts (group by spec_id + stage + run_id)
INSERT INTO consensus_runs (spec_id, stage, run_timestamp, consensus_ok, degraded)
SELECT DISTINCT
    spec_id,
    stage,
    strftime('%s', created_at) * 1000,  -- Convert to milliseconds
    1,     -- Assume historical consensus OK
    0      -- Unknown degradation status
FROM consensus_artifacts
GROUP BY spec_id, stage, run_id;

-- Extract agent outputs (link to runs via run_id)
INSERT INTO agent_outputs (run_id, agent_name, content, output_timestamp)
SELECT
    cr.id,                              -- FK to consensus_runs
    ca.agent_name,
    ca.content_json,
    strftime('%s', ca.created_at) * 1000
FROM consensus_artifacts ca
JOIN consensus_runs cr ON
    ca.spec_id = cr.spec_id AND
    ca.stage = cr.stage AND
    ca.run_id = cr.run_id;
```

---

## 4. Package Architecture Decision

### User Choice: `codex-core/src/db/`

**Rationale**:
- Database is infrastructure concern (not business logic)
- Shared across multiple packages (tui, spec-kit, potentially cli)
- Clean separation of concerns
- Makes `codex-core` the data access layer

**Module Structure**:
```
codex-rs/
├── core/src/
│   ├── db/                   # NEW - Database layer
│   │   ├── mod.rs           # Public API, connection factory
│   │   ├── connection.rs    # r2d2 pool + pragma configuration
│   │   ├── transactions.rs  # ACID transaction helpers
│   │   ├── migrations.rs    # Schema versioning
│   │   ├── vacuum.rs        # Auto-vacuum scheduler
│   │   └── schema.rs        # Schema definitions (structs)
│   └── lib.rs               # Re-export db module
│
├── spec-kit/src/
│   ├── retry/               # NEW - Retry logic
│   │   ├── mod.rs          # Public API
│   │   ├── strategy.rs     # Backoff algorithms
│   │   ├── classifier.rs   # Error classification
│   │   └── circuit_breaker.rs  # Optional advanced feature
│   └── lib.rs              # Re-export retry module
│
└── tui/src/chatwidget/spec_kit/
    └── consensus_db.rs      # MODIFY - Use codex_core::db instead
```

**Dependency Flow**:
```
tui → spec-kit → codex-core (with db module)
```

**Migration Path**:
1. Create `codex-core/src/db/` module
2. Gradually migrate methods from `consensus_db.rs` to core
3. Update `consensus_db.rs` to be a thin wrapper over `codex_core::db`
4. Eventually: `consensus_db.rs` becomes just re-exports

---

## 5. MCP Status Assessment

### Assumption: Already Migrated (User Decision)

**Evidence Supporting Migration Complete**:
1. ✅ SQLite database exists with data
2. ✅ `consensus_artifacts` table has rows
3. ❌ No `mcp_client.store_memory()` calls found in orchestration code
4. ✅ All consensus storage goes through `consensus_db.rs` (SQLite)

**Implication for Phase 1**:
- **Skip MCP dual-write** validation phase (SPEC-945B Phase 2)
- **Proceed with SQLite optimization** directly
- **No MCP migration** required (already done)

**If Assumption Wrong** (discovered during implementation):
- Implement dual-write (SQLite + MCP) with validation
- Add comparison logic (ensure 0% mismatch)
- Follow SPEC-945B Phase 2-4 migration path

---

## 6. Transaction Boundaries Identified

### Current Write Patterns (No Transactions)

**Pattern 1: Agent Spawn** (`agent_orchestrator.rs:337`):
```rust
// Step 1: Update in-memory state
widget.spec_auto_phase = SpecAutoPhase::ExecutingAgents { ... };

// Step 2: Write to SQLite (SEPARATE, NO COORDINATION)
let _ = db.record_agent_spawn(agent_id, spec_id, stage, ...);
```

**Gap**: Crash between Step 1 and Step 2 = TUI thinks agents spawned, DB has no record

**Pattern 2: Agent Completion** (`agent_orchestrator.rs:374`):
```rust
// Step 1: Update in-memory completed_agents HashSet
completed_agents.insert(agent_id.clone());

// Step 2: Write to SQLite (SEPARATE, NO COORDINATION)
if let Err(e) = db.record_agent_completion(&agent_id, result) {
    tracing::error!("Failed to record completion: {}", e);
}
```

**Gap**: Crash after HashSet update but before SQLite write = TUI thinks agent done, DB shows in-progress

### Required Transaction Boundaries (Phase 1 Implementation)

**Transaction 1: Coordinated Agent Spawn**:
```rust
execute_in_transaction(conn, TransactionBehavior::Immediate, |tx| {
    // Phase 1: Insert agent_executions row
    tx.execute("INSERT INTO agent_executions ...")?;

    // Phase 2: Update in-memory state (requires careful ordering)
    // NOTE: Can't actually wrap in-memory update in SQL transaction
    // Solution: Make SQLite write BEFORE in-memory update (fail-safe)

    Ok(())
})
```

**Transaction 2: Coordinated Agent Completion**:
```rust
execute_in_transaction(conn, TransactionBehavior::Immediate, |tx| {
    // Update agent_executions.completed_at + response_text
    tx.execute("UPDATE agent_executions SET completed_at = ..., response_text = ? ...")?;

    // In-memory update happens AFTER successful SQL commit
    Ok(())
})
```

**Key Insight**:
- SQL transactions can't wrap Rust in-memory state
- **Solution**: Write to SQLite FIRST, update in-memory SECOND
- On crash: SQLite is source of truth, TUI resyncs on restart

---

## 7. Dependencies Required

### codex-core/Cargo.toml

**Add**:
```toml
[dependencies]
# SQLite with connection pooling
rusqlite = { version = "0.31", features = ["bundled"] }
r2d2 = "0.8"
r2d2-sqlite = "0.23"

# Already present (verify versions)
tokio = { version = "1.35", features = ["rt-multi-thread", "time"] }
anyhow = "1.0"
thiserror = "2.0"  # Note: upgraded to 2.0
tracing = "0.1"
chrono = { version = "0.4", features = ["serde"] }
```

### spec-kit/Cargo.toml

**Add**:
```toml
[dependencies]
# Retry logic
backon = "1.1"  # Primary retry implementation

# Already present
thiserror = "2.0"
tokio = { version = "1", features = ["rt-multi-thread", "time"] }
anyhow = "1"
```

---

## 8. Implementation Checklist

### Prerequisites (This Document) ✅

- [x] Create feature branch `feature/phase-1-sqlite-retry`
- [x] Investigate AGENT_MANAGER location
- [x] Document current state
- [x] Identify transaction boundaries
- [x] Design migration strategy
- [ ] Add dependencies to Cargo.toml files
- [ ] Create module structures (empty scaffolding)
- [ ] Create test file structures

### Week 1: Database Foundation (40 hours)

**Day 1-2: Core Infrastructure** (16h):
- [ ] Create `codex-core/src/db/mod.rs` with public API
- [ ] Implement `connection.rs` (r2d2 pool, pragma config)
- [ ] Unit tests: connection pooling, health checks, pragma verification
- [ ] Integration test: pool concurrency (10 parallel connections)

**Day 3-4: Schema & Migrations** (16h):
- [ ] Implement `migrations.rs` (version tracking via `PRAGMA user_version`)
- [ ] Create migration V1: New tables (`consensus_runs`, `agent_outputs`)
- [ ] Keep old tables (dual-schema for Phase 2)
- [ ] Unit tests: migration up/down, version tracking
- [ ] Integration test: migrate production DB copy

**Day 5: Auto-Vacuum** (8h):
- [ ] Implement `vacuum.rs` (tokio background daemon)
- [ ] Incremental vacuum scheduler (daily runs)
- [ ] Manual vacuum trigger (CLI command)
- [ ] Unit tests: vacuum stats, freelist calculation
- [ ] Integration test: actual space reclamation

### Week 2: Transaction Safety & Migration (40 hours)

**Day 1-2: Transaction Helpers** (16h):
- [ ] Implement `transactions.rs` (ACID wrappers)
- [ ] `execute_in_transaction()` with behavior selection
- [ ] `batch_insert()` for agent outputs
- [ ] `upsert_consensus_run()` with conflict resolution
- [ ] Unit tests: rollback, commit, ACID guarantees
- [ ] Integration test: crash recovery (kill -9 simulation)

**Day 3: Dual-Write Implementation** (8h):
- [ ] Modify `consensus_db.rs` to write to both old + new schemas
- [ ] Validation: compare old vs new after each write
- [ ] Telemetry: log mismatches (target: 0%)
- [ ] Integration test: dual-write consistency

**Day 4: Benchmarking** (8h):
- [ ] Baseline: Current `consensus_db.rs` performance
- [ ] New: Connection pool + WAL + pragmas performance
- [ ] Target: Verify ≥5× speedup (if MCP was involved)
- [ ] At minimum: Prove no regression

**Day 5: Integration & Testing** (8h):
- [ ] Replace `consensus_db.rs` internals with `codex_core::db`
- [ ] Full test suite run (all existing tests pass)
- [ ] TUI manual testing (spawn agents, check DB)
- [ ] Document Week 2 completion

### Week 2-3: Retry Logic (20 hours, parallel)

**Day 1-2: Retry Module** (8h):
- [ ] Create `spec-kit/src/retry/mod.rs`
- [ ] Implement `strategy.rs` (exponential backoff with jitter)
- [ ] Unit tests: backoff progression, jitter randomness
- [ ] Integration test: retry timing validation

**Day 3: Error Classification** (6h):
- [ ] Extend `SpecKitError` with `RetryClassifiable` trait
- [ ] Classify errors: retryable (timeout, 503, SQLITE_BUSY) vs permanent (auth, validation)
- [ ] Unit tests: classification correctness

**Day 4-5: Integration** (6h):
- [ ] Integrate retry into database operations (SQLITE_BUSY)
- [ ] Add telemetry (retry attempts, success rate)
- [ ] Integration test: transient error recovery

### Week 3: Validation & Completion (10 hours)

**Final Testing** (6h):
- [ ] Run full test suite (cargo test --workspace)
- [ ] Benchmarks: Validate all performance targets
- [ ] Manual TUI testing: End-to-end workflows
- [ ] Load testing: 100 concurrent operations

**Documentation** (4h):
- [ ] Update CLAUDE.md with new db/ module
- [ ] Document migration completion
- [ ] Create Phase 2 planning doc (AGENT_MANAGER orchestration, parallel spawning)

---

## 9. Acceptance Criteria

### SPEC-945B (SQLite & Transactions)

- [ ] Connection pool created (10 connections, r2d2-sqlite)
- [ ] WAL mode enabled (100k SELECTs/sec achieved)
- [ ] Pragmas configured (cache_size, synchronous, mmap_size, auto_vacuum)
- [ ] Schema migration complete (new tables created, data preserved)
- [ ] ACID transactions prevent data loss (crash recovery tested)
- [ ] Auto-vacuum reduces DB size (153MB → target <5MB)
- [ ] All unit tests pass (≥20 tests)
- [ ] All integration tests pass (≥10 tests)
- [ ] Zero data loss in migration (validation: 0% mismatch)
- [ ] Performance: No regression, ideally 5× faster consensus storage

### SPEC-945C (Retry Logic)

- [ ] Retry module created with exponential backoff
- [ ] Error classification trait implemented
- [ ] Transient errors recover (≥90% success rate validated)
- [ ] Backoff timing correct (100ms → 1.6s progression verified)
- [ ] SQLITE_BUSY retries succeed (integration tested)
- [ ] All unit tests pass (≥15 tests)
- [ ] All integration tests pass (≥5 tests)
- [ ] Telemetry captures retry context (attempt count, backoff delay)

### Integration Success

- [ ] TUI works with new database layer (no regressions)
- [ ] Consensus storage faster or equal (benchmark validated)
- [ ] No data corruption (stress testing: 1000 operations)
- [ ] Code quality: cargo fmt + clippy clean
- [ ] Documentation complete (CLAUDE.md updated)

---

## 10. Risks & Mitigation

### Risk 1: TUI Rendering Breaks During Migration

**Probability**: Medium (25%)
**Impact**: High (blocks all development)

**Mitigation**:
- Dual-schema period (1 week validation)
- Keep old `consensus_db.rs` API compatible
- Gradual cutover (reads → writes)
- Rollback procedure documented

### Risk 2: Data Migration Loses Records

**Probability**: Low (5%)
**Impact**: Critical (data loss)

**Mitigation**:
- Backup production DB before migration
- Validation query: `SELECT COUNT(*) FROM old vs new`
- Dual-write with mismatch detection (target: 0%)
- Emergency rollback: old schema still exists

### Risk 3: Performance Regression

**Probability**: Low (10%)
**Impact**: Medium (user complaints)

**Mitigation**:
- Baseline benchmarks BEFORE changes
- Continuous monitoring during development
- Rollback if >10% regression detected
- Pragmas are proven optimizations (research-backed)

### Risk 4: AGENT_MANAGER Assumption Wrong

**Probability**: Low (15%)
**Impact**: Medium (scope increase)

**Mitigation**:
- Document assumption clearly
- Quick validation during Week 1 Day 1
- If wrong: Add AGENT_MANAGER fix to Phase 2 (not Phase 1)

---

## Conclusion

**Status**: Prerequisites Complete
**Branch**: feature/phase-1-sqlite-retry (created)
**Findings**: Documented above
**Next Steps**: Add dependencies → Create module structures → Begin Week 1 implementation

**Key Insights**:
1. No HashMap-based AGENT_MANAGER (simplifies implementation)
2. Current dual-write is in-memory state + SQLite (not HashMap + SQLite)
3. MCP likely already migrated (skip dual-write validation)
4. Architecture: `codex-core/src/db/` for clean separation
5. Transaction boundaries identified (write SQLite first, update memory second)

**Ready for**: Week 1 Day 1 - Database Foundation Implementation

---

**Document Status**: Complete
**Author**: Claude (Phase 1 Investigation)
**Review Required**: User approval before proceeding to implementation
