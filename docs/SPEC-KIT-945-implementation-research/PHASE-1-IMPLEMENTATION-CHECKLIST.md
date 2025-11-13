# Phase 1 Implementation Checklist

**SPEC-945B** (SQLite & Transactions) + **SPEC-945C** (Retry Logic)

**Branch**: feature/phase-1-sqlite-retry
**Duration**: 2-3 weeks (100-140 hours)
**Status**: Prerequisites Complete ✅ - Ready for Week 1

---

## ✅ Prerequisites (COMPLETE)

- [x] Create feature branch `feature/phase-1-sqlite-retry`
- [x] Investigate AGENT_MANAGER location (findings documented)
- [x] Add dependencies (rusqlite, r2d2-sqlite, backon)
- [x] Create module structures (`codex-core/src/db/`, `spec-kit/src/retry/`)
- [x] Create test file structures
- [x] Document investigation findings (PHASE-1-INVESTIGATION-FINDINGS.md)

---

## Week 1: Database Foundation (40 hours)

### Day 1-2: Core Infrastructure (16 hours)

**Files**: `core/src/db/connection.rs`, `core/src/db/mod.rs`

- [ ] Implement `initialize_pool()` function
  - [ ] Create r2d2 pool with SqliteConnectionManager
  - [ ] Configure pool size (default: 10 connections)
  - [ ] Set min idle connections (2)
  - [ ] Enable connection health checks
- [ ] Implement pragma configuration
  - [ ] `PRAGMA journal_mode = WAL`
  - [ ] `PRAGMA synchronous = NORMAL`
  - [ ] `PRAGMA foreign_keys = ON`
  - [ ] `PRAGMA cache_size = -32000` (32MB)
  - [ ] `PRAGMA temp_store = MEMORY`
  - [ ] `PRAGMA auto_vacuum = INCREMENTAL`
  - [ ] `PRAGMA mmap_size = 1073741824` (1GB)
  - [ ] `PRAGMA busy_timeout = 5000` (5s)
- [ ] Implement `verify_pragmas()` function
  - [ ] Verify WAL mode enabled
  - [ ] Verify foreign keys enabled
  - [ ] Return error if pragmas not applied
- [ ] Write unit tests
  - [ ] Test pool creation
  - [ ] Test connection acquisition/release
  - [ ] Test pragma verification
  - [ ] Test concurrent connections (10 parallel)
  - [ ] Test connection health checks
- [ ] Integration test: Pool with real database file

**Acceptance**:
- Connection pool works (10 connections)
- All pragmas applied and verified
- Tests pass (≥5 unit tests, 1 integration test)

---

### Day 3-4: Schema & Migrations (16 hours)

**Files**: `core/src/db/migrations.rs`, `core/src/db/mod.rs`

- [ ] Implement schema version tracking
  - [ ] `get_schema_version()` via `PRAGMA user_version`
  - [ ] `set_schema_version()` via `PRAGMA user_version`
- [ ] Implement `migrate_to_latest()` function
  - [ ] Check current version
  - [ ] Apply migrations sequentially (V0 → V1)
  - [ ] Error if DB newer than app version
- [ ] Create Migration V1: New normalized schema
  - [ ] Create `consensus_runs` table:
    ```sql
    CREATE TABLE IF NOT EXISTS consensus_runs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        spec_id TEXT NOT NULL,
        stage TEXT NOT NULL,
        run_timestamp INTEGER NOT NULL,
        consensus_ok BOOLEAN NOT NULL,
        degraded BOOLEAN DEFAULT 0,
        synthesis_json TEXT,
        UNIQUE(spec_id, stage, run_timestamp)
    );
    ```
  - [ ] Create `agent_outputs` table:
    ```sql
    CREATE TABLE IF NOT EXISTS agent_outputs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        run_id INTEGER NOT NULL,
        agent_name TEXT NOT NULL,
        model_version TEXT,
        content TEXT NOT NULL,
        output_timestamp INTEGER NOT NULL,
        FOREIGN KEY(run_id) REFERENCES consensus_runs(id) ON DELETE CASCADE
    );
    ```
  - [ ] Create indexes:
    ```sql
    CREATE INDEX IF NOT EXISTS idx_consensus_spec_stage
        ON consensus_runs(spec_id, stage);
    CREATE INDEX IF NOT EXISTS idx_consensus_timestamp
        ON consensus_runs(run_timestamp);
    CREATE INDEX IF NOT EXISTS idx_agent_outputs_run
        ON agent_outputs(run_id);
    CREATE INDEX IF NOT EXISTS idx_agent_outputs_agent
        ON agent_outputs(agent_name);
    ```
- [ ] Keep old tables (dual-schema for Phase 2)
- [ ] Write unit tests
  - [ ] Test version tracking
  - [ ] Test migration V0 → V1
  - [ ] Test idempotency (run migration twice, no error)
  - [ ] Test new table creation
  - [ ] Test index creation
- [ ] Integration test: Migrate production DB copy
  - [ ] Backup `~/.code/consensus_artifacts.db`
  - [ ] Run migration on copy
  - [ ] Verify new tables exist
  - [ ] Verify old tables preserved

**Acceptance**:
- Migration system works (V0 → V1)
- New tables created with indexes
- Old tables preserved
- Tests pass (≥6 unit tests, 1 integration test)

---

### Day 5: Auto-Vacuum (8 hours)

**Files**: `core/src/db/vacuum.rs`

- [ ] Implement vacuum statistics
  - [ ] `get_db_size()` via `PRAGMA page_count * page_size`
  - [ ] `get_freelist_size()` via `PRAGMA freelist_count * page_size`
  - [ ] `VacuumStats` struct
- [ ] Implement `run_vacuum_cycle()` function
  - [ ] Get size before vacuum
  - [ ] Execute `PRAGMA incremental_vacuum(20)` (20 pages)
  - [ ] Get size after vacuum
  - [ ] Calculate reclaimed space
  - [ ] Log statistics
- [ ] Implement `spawn_vacuum_daemon()` function
  - [ ] Create tokio background task
  - [ ] Run every 24 hours (`tokio::time::interval`)
  - [ ] Handle errors gracefully (log, don't crash)
  - [ ] Return JoinHandle for graceful shutdown
- [ ] Implement manual vacuum trigger
  - [ ] CLI-callable function
  - [ ] Reports statistics
- [ ] Write unit tests
  - [ ] Test statistics calculation
  - [ ] Test vacuum execution (mock)
  - [ ] Test scheduler timing
- [ ] Integration test: Actual space reclamation
  - [ ] Create test DB with deleted records
  - [ ] Run vacuum
  - [ ] Verify size reduction

**Acceptance**:
- Vacuum scheduler works (daily runs)
- Manual vacuum trigger works
- Space reclamation verified (<5MB target)
- Tests pass (≥4 unit tests, 1 integration test)

---

## Week 2: Transaction Safety & Integration (40 hours)

### Day 1-2: Transaction Helpers (16 hours)

**Files**: `core/src/db/transactions.rs`

- [ ] Implement `execute_in_transaction()` function
  - [ ] Begin transaction with behavior (Deferred/Immediate/Exclusive)
  - [ ] Execute operation closure
  - [ ] Commit on success
  - [ ] Automatic rollback on error (via Drop)
  - [ ] Context preservation for errors
- [ ] Implement `batch_insert()` function
  - [ ] Wrap multiple inserts in single transaction
  - [ ] Use IMMEDIATE behavior (write-heavy)
  - [ ] Return count of inserted rows
- [ ] Implement `upsert_consensus_run()` function
  - [ ] INSERT ... ON CONFLICT DO UPDATE pattern
  - [ ] Handle unique constraint on (spec_id, stage, run_timestamp)
  - [ ] Return inserted/updated row ID
- [ ] Write unit tests
  - [ ] Test transaction commit
  - [ ] Test transaction rollback
  - [ ] Test ACID atomicity (all-or-nothing)
  - [ ] Test batch insert performance
  - [ ] Test upsert conflict resolution
  - [ ] Test concurrent transactions (no deadlock)
- [ ] Integration test: Crash recovery simulation
  - [ ] Use `std::panic::catch_unwind` for controlled panic
  - [ ] Verify rollback on panic
  - [ ] Verify no partial updates

**Acceptance**:
- Transaction helpers work (commit, rollback, batch)
- ACID guarantees verified
- Crash recovery tested
- Tests pass (≥8 unit tests, 1 integration test)

---

### Day 3: Dual-Write Implementation (8 hours)

**Files**: `tui/src/chatwidget/spec_kit/consensus_db.rs`

- [ ] Modify `store_artifact()` to write to both schemas
  - [ ] Write to old `consensus_artifacts` table (existing)
  - [ ] Write to new `consensus_runs` + `agent_outputs` tables
  - [ ] Use same timestamp for both
- [ ] Add validation logic
  - [ ] After dual-write, read from both schemas
  - [ ] Compare results (should be identical)
  - [ ] Log mismatch errors (target: 0%)
  - [ ] Add telemetry (dual_write_mismatch counter)
- [ ] Update `store_synthesis()` to dual-write
  - [ ] Write to old `consensus_synthesis` table
  - [ ] Write to new `consensus_runs.synthesis_json` column
  - [ ] Validate consistency
- [ ] Integration test: Dual-write consistency
  - [ ] Write 100 consensus artifacts
  - [ ] Verify 0 mismatches
  - [ ] Read from both schemas
  - [ ] Compare results

**Acceptance**:
- Dual-write works (both old + new schemas)
- Validation detects mismatches (0% observed)
- Telemetry captures write success rate
- Tests pass (≥2 integration tests)

---

### Day 4: Benchmarking (8 hours)

**Files**: `core/benches/db_performance.rs` (NEW)

- [ ] Create benchmark suite with criterion
  - [ ] Add criterion to dev-dependencies
  - [ ] Create `benches/` directory
- [ ] Baseline: Current `consensus_db.rs` performance
  - [ ] Measure consensus artifact write time
  - [ ] Measure read throughput (SELECTs/second)
  - [ ] Measure concurrent access (10 parallel operations)
- [ ] New: Connection pool + WAL performance
  - [ ] Same operations with new db/ module
  - [ ] Measure improvement
- [ ] Compare old vs new
  - [ ] Target: No regression (minimum)
  - [ ] Goal: 5× speedup if MCP was involved
  - [ ] Prove WAL mode benefit (6.6× read speedup)
- [ ] Document results
  - [ ] Create benchmark report
  - [ ] Include statistical analysis (mean, std dev, p-value)

**Acceptance**:
- Benchmarks run successfully
- Performance targets met (no regression)
- Results documented
- Criterion reports generated

---

### Day 5: Integration & Testing (8 hours)

**Files**: Multiple

- [ ] Replace `consensus_db.rs` internals with `codex_core::db`
  - [ ] Update imports
  - [ ] Use connection pool instead of `Arc<Mutex<Connection>>`
  - [ ] Keep API compatibility (no breaking changes)
- [ ] Run full test suite
  - [ ] `cargo test --workspace`
  - [ ] All existing tests pass
  - [ ] No regressions
- [ ] TUI manual testing
  - [ ] Launch TUI
  - [ ] Spawn agents (`/speckit.plan`, `/speckit.validate`)
  - [ ] Verify database writes
  - [ ] Check consensus artifacts in SQLite
- [ ] Document Week 2 completion
  - [ ] Update PHASE-1-INVESTIGATION-FINDINGS.md with progress
  - [ ] List any blockers or issues

**Acceptance**:
- TUI works with new database layer
- All tests pass
- No regressions observed
- Week 2 complete

---

## Week 2-3: Retry Logic (20 hours, parallel)

### Day 1-2: Retry Module (8 hours)

**Files**: `spec-kit/src/retry/strategy.rs`, `spec-kit/src/retry/mod.rs`

- [ ] Implement `RetryConfig` struct
  - [ ] Default: 3 attempts, 100ms → 10s backoff
  - [ ] Configurable multiplier (default: 2.0)
  - [ ] Configurable jitter factor (default: 0.5)
- [ ] Implement `execute_with_backoff()` using backon
  - [ ] Use `backon::ExponentialBuilder`
  - [ ] Apply jitter (±25-50%)
  - [ ] Classify errors (retryable vs permanent)
  - [ ] Max attempts enforcement
  - [ ] Telemetry (log retry attempts)
- [ ] Write unit tests
  - [ ] Test backoff progression (100ms → 200ms → 400ms → 800ms → 1600ms)
  - [ ] Test jitter randomness (within expected range)
  - [ ] Test max attempts (stop after 3)
  - [ ] Test error classification integration
- [ ] Integration test: Retry timing validation
  - [ ] Measure actual backoff delays
  - [ ] Verify exponential curve
  - [ ] Verify jitter spread

**Acceptance**:
- Retry module works (exponential backoff + jitter)
- Timing correct (100ms → 1.6s progression)
- Tests pass (≥5 unit tests, 1 integration test)

---

### Day 3: Error Classification (6 hours)

**Files**: `spec-kit/src/retry/classifier.rs`, `spec-kit/src/error.rs`

- [ ] Extend `SpecKitError` with `RetryClassifiable` trait
  - [ ] Implement `classify()` method
  - [ ] Implement `is_retryable()` method
  - [ ] Implement `suggested_backoff()` method
- [ ] Classify error types
  - [ ] Retryable: Timeout, 503, SQLITE_BUSY, Connection refused
  - [ ] Permanent: Auth failure, Invalid input, Not found
  - [ ] Degraded: 2/3 consensus (retryable once)
- [ ] Write unit tests
  - [ ] Test classification correctness (each error type)
  - [ ] Test suggested backoff (rate limit → custom delay)
  - [ ] Test is_retryable() for all error types

**Acceptance**:
- Error classification works
- All error types classified
- Tests pass (≥8 unit tests)

---

### Day 4-5: Integration (6 hours)

**Files**: `core/src/db/connection.rs`, `tui/src/chatwidget/spec_kit/*.rs`

- [ ] Integrate retry into database operations
  - [ ] Wrap SQLite operations in `execute_with_backoff()`
  - [ ] Retry on SQLITE_BUSY errors
  - [ ] Max 3 attempts with exponential backoff
- [ ] Add telemetry
  - [ ] Log retry attempts (attempt count, backoff delay)
  - [ ] Track success rate after retry
  - [ ] Monitor retry rate (alert if >20%)
- [ ] Write integration tests
  - [ ] Test SQLITE_BUSY recovery (simulate lock)
  - [ ] Test transient error recovery (mock timeout)
  - [ ] Test retry telemetry

**Acceptance**:
- Retry integrated into database layer
- SQLITE_BUSY errors recover automatically
- Telemetry captures retry context
- Tests pass (≥3 integration tests)

---

## Week 3: Validation & Completion (10 hours)

### Final Testing (6 hours)

- [ ] Run full test suite
  - [ ] `cargo test --workspace`
  - [ ] All tests pass (≥50 total tests)
- [ ] Run benchmarks
  - [ ] Validate all performance targets
  - [ ] No regressions detected
  - [ ] Document results
- [ ] Manual TUI testing
  - [ ] End-to-end workflows (`/speckit.auto`)
  - [ ] Verify database writes
  - [ ] Check vacuum reclaimed space
  - [ ] Test retry on transient errors
- [ ] Load testing
  - [ ] 100 concurrent operations
  - [ ] Verify no deadlocks
  - [ ] Verify connection pool handles load
- [ ] Stress testing
  - [ ] 1000 operations sequentially
  - [ ] Verify database stable (<10MB)
  - [ ] Verify auto-vacuum running

**Acceptance**:
- All tests pass
- Performance targets met
- Load/stress tests successful
- No data corruption observed

---

### Documentation (4 hours)

- [ ] Update CLAUDE.md
  - [ ] Document new `codex_core::db` module
  - [ ] Document schema migration
  - [ ] Document retry integration
- [ ] Document migration completion
  - [ ] Update PHASE-1-INVESTIGATION-FINDINGS.md
  - [ ] Mark all tasks complete
  - [ ] List any deferred work (Phase 2)
- [ ] Create Phase 2 planning doc
  - [ ] AGENT_MANAGER orchestration fix
  - [ ] Parallel agent spawning
  - [ ] Quality gate retry integration
  - [ ] Full SPEC-933/938 completion

**Acceptance**:
- Documentation complete
- CLAUDE.md updated
- Phase 2 plan created

---

## Acceptance Criteria Summary

### SPEC-945B (SQLite & Transactions) ✅

- [ ] Connection pool created (10 connections, r2d2-sqlite)
- [ ] WAL mode enabled (100k SELECTs/sec achieved)
- [ ] Pragmas configured (all 8 pragmas applied)
- [ ] Schema migration complete (new tables created)
- [ ] ACID transactions prevent data loss (crash recovery tested)
- [ ] Auto-vacuum reduces DB size (153MB → <5MB target)
- [ ] All unit tests pass (≥20 tests)
- [ ] All integration tests pass (≥10 tests)
- [ ] Zero data loss in migration (0% mismatch validated)
- [ ] Performance: No regression, ideally 5× faster

### SPEC-945C (Retry Logic) ✅

- [ ] Retry module created with exponential backoff
- [ ] Error classification trait implemented
- [ ] Transient errors recover (≥90% success rate)
- [ ] Backoff timing correct (100ms → 1.6s verified)
- [ ] SQLITE_BUSY retries succeed (integration tested)
- [ ] All unit tests pass (≥15 tests)
- [ ] All integration tests pass (≥5 tests)
- [ ] Telemetry captures retry context

### Integration Success ✅

- [ ] TUI works with new database layer
- [ ] Consensus storage faster or equal
- [ ] No data corruption (stress test: 1000 ops)
- [ ] Code quality: cargo fmt + clippy clean
- [ ] Documentation complete

---

## Next Steps (Phase 2)

**After Phase 1 Complete**:

1. **AGENT_MANAGER Transaction Coordination** (30-40 hours)
   - Coordinate in-memory state + SQLite writes
   - Eliminate dual-write corruption risk
   - Full SPEC-933 completion

2. **Parallel Agent Spawning** (8-16 hours)
   - Replace sequential with parallel spawning
   - 3× speedup (150ms → 50ms)
   - SPEC-933 Component 3

3. **Quality Gate Retry Integration** (12-16 hours)
   - Retry failed agents before degrading to 2/3
   - Target: 95% achieve 3/3 consensus
   - Full SPEC-938 completion

**Total Phase 2 Estimate**: 50-72 hours (1.5-2 weeks)

---

**Status**: Ready for Week 1 Day 1
**Branch**: feature/phase-1-sqlite-retry
**Prerequisites**: Complete ✅
**Next Action**: Begin `core/src/db/connection.rs` implementation
