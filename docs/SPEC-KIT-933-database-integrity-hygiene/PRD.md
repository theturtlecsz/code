# PRD: Database Integrity & Hygiene

**SPEC-ID**: SPEC-KIT-933
**Created**: 2025-11-13
**Status**: Draft - **CRITICAL PRIORITY**
**Priority**: **P0** (Data Corruption Risk)
**Owner**: Code
**Estimated Effort**: 65-96 hours (2-3 weeks)
**Dependencies**: None
**Blocks**: SPEC-934 (storage consolidation benefits from transactions)

---

## 🔥 Executive Summary

**Current State**: AGENT_MANAGER HashMap and SQLite database updated separately with no transaction coordination. Crash between writes leaves inconsistent state. Database has 153MB bloat (99.97% waste) from 695 deleted records never reclaimed. Agent spawning is sequential (3 × 50ms = 150ms).

**Proposed State**: ACID transactions ensure HashMap + SQLite consistency. Incremental auto-vacuum eliminates bloat automatically. Parallel agent spawning reduces time to 50ms (3× faster). Daily cleanup cron maintains stable database size.

**Impact**:
- ✅ Eliminates data corruption risk (CRITICAL)
- ✅ Reduces database size 153MB → ~2-5MB (96% reduction)
- ✅ 3× faster agent spawning (150ms → 50ms)
- ✅ Prevents indefinite database growth

**Source**: SPEC-931A architectural analysis identified dual-write problem, database bloat, sequential bottleneck.

**Alternative Rejected**: Event sourcing (SPEC-931F NO-GO - doesn't solve dual-write, 150-180h effort, YAGNI violation).

---

## 1. Problem Statement

### Issue #1: Dual-Write Data Corruption Risk (CRITICAL)

**Current Behavior**:
```rust
// Phase 1: Update in-memory state
AGENT_MANAGER.lock().unwrap().update(agent_id, state);

// Phase 2: Update database (SEPARATE, NO COORDINATION)
db.execute("UPDATE agent_executions SET ...");

// RISK: Crash between Phase 1 and Phase 2
// Result: HashMap updated, SQLite has old state
// On restart: Database shows "In Progress" but agent actually finished
```

**Evidence** (SPEC-931A phase1-dataflows.md:571-574):
- No transaction wrapping both operations
- Crash loses recent HashMap updates (not persisted)
- Manual recovery required for inconsistent state

**Real-World Impact**:
- Agent completes successfully → HashMap updated
- System crashes before SQLite write
- On restart: Quality gate re-spawns same agent (thinks it's still in progress)
- Duplicate work, wasted API costs, confusing state

**Frequency**: Low probability (~1% crash during narrow window), HIGH impact (data loss).

---

### Issue #2: Database Bloat (99.97% Waste)

**Current State** (SPEC-931A phase1-database.md:274-277):
- Database file: **153MB**
- Actual data: **53KB** (695 old records deleted)
- Bloat: **152.9MB** in freelist (99.97% wasted space)
- Auto-vacuum: `NONE` (disabled)

**Impact**:
- Wasted disk space (153MB vs 5MB needed)
- Slower queries (freelist scan overhead)
- Indefinite growth (deletes don't reclaim space)
- Manual `VACUUM` requires exclusive lock (blocks operations)

**Root Cause**: SQLite default behavior doesn't reclaim deleted row space.

---

### Issue #3: Sequential Agent Spawning Bottleneck

**Current Behavior** (SPEC-931A phase1-inventory.md:1092-1096):
```rust
for agent in ["gemini", "claude", "code"] {
    spawn_agent(agent).await;  // Sequential
}
// Total: 3 × 50ms = 150ms
```

**Alternative**:
```rust
let tasks: Vec<_> = ["gemini", "claude", "code"]
    .iter()
    .map(|agent| tokio::spawn(spawn_agent(agent)))
    .collect();
join_all(tasks).await;
// Total: max(50ms) = 50ms (3× faster)
```

**Blocker**: SQLite writes can't happen in parallel without coordination → ACID transactions enable this.

---

### Issue #4: Database Growth Without Bounds

**Current State** (SPEC-931A phase1-database.md:542-547):
- Method exists: `cleanup_old_executions(days: u32)`
- **Never called** (no invocation sites)
- Database grows indefinitely
- Old consensus artifacts accumulate (>30 days)

**Recommendation**: Daily cron to delete records >30 days old.

**Current Growth**: ~500KB/day with 10 quality gates → 15MB/month → 180MB/year without cleanup.

---

## 2. Proposed Solution

### Component 1: ACID Transactions (CRITICAL - 24-36h)

**Implementation**:
```rust
// Wrap both operations in SQLite transaction
let tx = db.transaction()?;

// Phase 1: Update HashMap
AGENT_MANAGER.lock().unwrap().update(agent_id, state);

// Phase 2: Update SQLite
tx.execute("UPDATE agent_executions SET state = ?")?;

// Atomic commit: Either both succeed or neither
tx.commit()?;  // On error, both rollback
```

**Key Features**:
1. **BEGIN TRANSACTION** before HashMap update
2. **COMMIT** only after both succeed
3. **ROLLBACK** on any failure (restore consistency)
4. **Error handling**: Clear error messages, retry logic

**Scope**:
- All agent state updates (spawn, start, complete, fail)
- Consensus artifact storage
- Quality gate state transitions
- Evidence file coordination

**Testing**:
- Crash recovery simulation (kill -9 during transaction)
- Rollback verification (failed SQLite write reverts HashMap)
- Concurrent transaction handling (multiple agents)

---

### Component 2: Incremental Auto-Vacuum (MEDIUM - 6-8h)

**Implementation**:
```sql
-- On database open:
PRAGMA auto_vacuum = INCREMENTAL;

-- On connection pool initialization:
PRAGMA incremental_vacuum(100);  -- Reclaim 100 pages per idle cycle
```

**Migration**:
```sql
-- Step 1: Full vacuum to enable auto-vacuum (one-time, requires exclusive lock)
VACUUM;

-- Step 2: Set pragma
PRAGMA auto_vacuum = INCREMENTAL;

-- Step 3: Future deletes automatically reclaim space
```

**Benefits**:
- Automatic space reclamation (no manual VACUUM)
- Incremental (doesn't block operations)
- Prevents 99%+ bloat accumulation
- 153MB → ~2-5MB stable size

**Trade-off**: Slight overhead on DELETE operations (~1-2ms) for space tracking.

---

### Component 3: Parallel Agent Spawning (MEDIUM - 10-15h)

**Implementation**:
```rust
pub async fn spawn_quality_gate_agents(
    agents: &[&str],
    config: &Config,
) -> Result<Vec<AgentHandle>, SpecKitError> {
    // Spawn all agents in parallel
    let spawn_tasks: Vec<_> = agents
        .iter()
        .map(|agent| {
            let agent = agent.to_string();
            let config = config.clone();
            tokio::spawn(async move {
                spawn_single_agent(&agent, &config).await
            })
        })
        .collect();

    // Wait for all to complete
    let results = join_all(spawn_tasks).await;

    // Batch SQLite writes in single transaction (enabled by Component 1)
    let tx = db.transaction()?;
    for result in results {
        tx.execute("INSERT INTO agent_executions ...")?;
    }
    tx.commit()?;  // Atomic batch insert

    Ok(handles)
}
```

**Performance**:
- Sequential: 3 × 50ms = 150ms
- Parallel: max(50ms) = 50ms
- **Speedup**: 3× faster

**Coordination**: ACID transactions (Component 1) enable safe parallel spawning with batch SQLite writes.

---

### Component 4: Daily Cleanup Cron (MEDIUM - 8-12h)

**Implementation**:
```bash
#!/bin/bash
# /usr/local/bin/codex-cleanup.sh

# Delete agent_executions older than 30 days
/path/to/codex-tui --cleanup-old-executions 30

# Delete consensus artifacts older than 30 days
/path/to/codex-tui --cleanup-old-consensus 30

# Log results
echo "$(date): Cleanup completed" >> /var/log/codex-cleanup.log
```

**Scheduling** (platform-specific):

**Linux (systemd timer)**:
```ini
# /etc/systemd/system/codex-cleanup.timer
[Unit]
Description=Daily Codex Database Cleanup

[Timer]
OnCalendar=daily
OnCalendar=02:00
Persistent=true

[Install]
WantedBy=timers.target
```

**macOS (launchd)**:
```xml
<!-- ~/Library/LaunchAgents/com.codex.cleanup.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.codex.cleanup</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/codex-cleanup.sh</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>2</integer>
        <key>Minute</key>
        <integer>0</integer>
    </dict>
</dict>
</plist>
```

**Windows (Task Scheduler)**:
```powershell
# PowerShell script to create scheduled task
$action = New-ScheduledTaskAction -Execute "powershell.exe" -Argument "C:\codex\cleanup.ps1"
$trigger = New-ScheduledTaskTrigger -Daily -At 2am
Register-ScheduledTask -Action $action -Trigger $trigger -TaskName "Codex Cleanup"
```

**Retention Policy**: 30 days (configurable via `CODEX_RETENTION_DAYS` env var).

**Manual Override**: `codex-tui --cleanup-now` for immediate cleanup.

---

### Component 5: Event Sourcing NO-GO Documentation (LOW - 2h)

**Purpose**: Record decision rationale to prevent future re-litigation.

**Location**: `docs/decisions/933-event-sourcing-rejection.md`

**Content**:
- SPEC-931F ultrathink findings (16 questions answered)
- Why event sourcing doesn't solve dual-write (HashMap still needed)
- YAGNI violation (designing for 1,000× scale we don't have)
- ACID transactions solve actual problem (2-3 days vs 3-5 weeks)
- Trigger condition for revisiting (if scale reaches 100+ agents/min, enterprise SLA)

---

## 3. Acceptance Criteria

### AC1: Data Integrity ✅
- [ ] All HashMap + SQLite updates wrapped in transactions
- [ ] Crash recovery tests pass (kill -9 during transaction → no corruption)
- [ ] Rollback verification (failed write reverts both HashMap + SQLite)
- [ ] Concurrent agent spawning doesn't cause corruption

### AC2: Database Hygiene ✅
- [ ] Auto-vacuum enabled (PRAGMA auto_vacuum = INCREMENTAL)
- [ ] Database size <10MB after migration (down from 153MB)
- [ ] Incremental vacuum runs on idle connections
- [ ] Full vacuum migration successful (one-time)

### AC3: Performance ✅
- [ ] Parallel spawning implemented (3 agents simultaneously)
- [ ] Spawn time <70ms (down from 150ms, target 50ms ±20ms variance)
- [ ] Batch SQLite writes in single transaction
- [ ] No performance regression on single-agent operations

### AC4: Cleanup Automation ✅
- [ ] Cron/scheduler configured for daily 2am execution
- [ ] Platform-specific installers (Linux systemd, macOS launchd, Windows Task Scheduler)
- [ ] Cleanup deletes records >30 days old
- [ ] Logs record cleanup operations with timestamps

### AC5: Documentation ✅
- [ ] Event sourcing rejection documented (`docs/decisions/933-event-sourcing-rejection.md`)
- [ ] SPEC-931F findings summarized (NO-GO rationale)
- [ ] Trigger conditions for revisiting documented

---

## 4. Technical Implementation

### Phase 1: Transaction Infrastructure (Week 1 - 24-36h)

**Files to Modify**:
- `codex-tui/src/chatwidget/spec_kit/handler.rs` (agent state updates)
- `codex-tui/src/chatwidget/spec_kit/consensus.rs` (artifact storage)
- `codex-tui/src/chatwidget/spec_kit/quality_gate_handler.rs` (quality gate transitions)
- `codex-core/src/consensus_db.rs` (add transaction support)

**New Code** (~800-1000 LOC):
```rust
// consensus_db.rs - Transaction wrapper
pub struct DbTransaction<'conn> {
    tx: rusqlite::Transaction<'conn>,
    agent_manager_lock: MutexGuard<'static, HashMap<String, AgentState>>,
}

impl<'conn> DbTransaction<'conn> {
    pub fn begin(conn: &'conn Connection) -> Result<Self> {
        let tx = conn.transaction()?;
        let lock = AGENT_MANAGER.lock().unwrap();
        Ok(DbTransaction { tx, agent_manager_lock: lock })
    }

    pub fn update_agent_state(&mut self, agent_id: &str, state: AgentState) -> Result<()> {
        // Update HashMap
        self.agent_manager_lock.insert(agent_id.to_string(), state.clone());

        // Update SQLite
        self.tx.execute(
            "UPDATE agent_executions SET state = ?, completed_at = ? WHERE agent_id = ?",
            params![state.to_string(), chrono::Utc::now().to_rfc3339(), agent_id],
        )?;

        Ok(())
    }

    pub fn commit(self) -> Result<()> {
        // Commit SQLite first
        self.tx.commit()?;
        // Release HashMap lock (implicitly via Drop)
        Ok(())
    }

    pub fn rollback(self) -> Result<()> {
        self.tx.rollback()?;
        // HashMap changes rolled back via Drop
        Ok(())
    }
}
```

**Testing**:
- Unit tests: Transaction commit/rollback logic
- Integration tests: Crash simulation (spawn test process, kill -9 mid-transaction)
- Concurrency tests: Multiple agents updating simultaneously

---

### Phase 2: Auto-Vacuum Migration (Week 1-2 - 6-8h)

**Migration Script** (`scripts/migrate_to_auto_vacuum.sh`):
```bash
#!/bin/bash
set -e

echo "Starting auto-vacuum migration..."

# Backup database
cp ~/.code/consensus_artifacts.db ~/.code/consensus_artifacts.db.backup

# Full vacuum (one-time, requires exclusive lock)
sqlite3 ~/.code/consensus_artifacts.db "VACUUM;"

# Enable incremental auto-vacuum
sqlite3 ~/.code/consensus_artifacts.db "PRAGMA auto_vacuum = INCREMENTAL;"

# Verify
SIZE_BEFORE=153000000  # 153MB
SIZE_AFTER=$(stat -f%z ~/.code/consensus_artifacts.db 2>/dev/null || stat -c%s ~/.code/consensus_artifacts.db)

echo "Before: ${SIZE_BEFORE} bytes (153MB)"
echo "After: ${SIZE_AFTER} bytes"
echo "Reduction: $(( (SIZE_BEFORE - SIZE_AFTER) * 100 / SIZE_BEFORE ))%"

if [ $SIZE_AFTER -lt 10000000 ]; then
    echo "✅ Migration successful (database < 10MB)"
else
    echo "⚠️  Warning: Database larger than expected ($SIZE_AFTER bytes)"
fi
```

**Application Code** (`consensus_db.rs`):
```rust
pub fn open_with_auto_vacuum(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;

    // Enable incremental auto-vacuum
    conn.execute_batch("PRAGMA auto_vacuum = INCREMENTAL;")?;

    // Run incremental vacuum on idle connections
    conn.execute("PRAGMA incremental_vacuum(100);", [])?;

    Ok(conn)
}
```

---

### Phase 3: Parallel Spawning (Week 2 - 10-15h)

**Files to Modify**:
- `codex-tui/src/chatwidget/spec_kit/quality_gate_handler.rs` (spawn logic)
- `codex-tui/src/chatwidget/spec_kit/agent_orchestrator.rs` (parallel spawn)

**New Code** (~400-600 LOC):
```rust
// quality_gate_handler.rs
async fn spawn_quality_gate_agents_parallel(
    checkpoint: QualityCheckpoint,
    config: &Config,
) -> Result<Vec<AgentHandle>> {
    let agents = checkpoint.required_agents();

    // Spawn all agents in parallel
    let spawn_futures: Vec<_> = agents
        .iter()
        .map(|agent_name| {
            let agent = agent_name.clone();
            let cfg = config.clone();
            tokio::spawn(async move {
                spawn_single_quality_gate_agent(&agent, &cfg).await
            })
        })
        .collect();

    // Wait for all spawns to complete
    let spawn_results = join_all(spawn_futures).await;

    // Batch insert into SQLite with transaction
    let mut tx = DbTransaction::begin(&db)?;
    for (idx, result) in spawn_results.iter().enumerate() {
        match result {
            Ok(Ok(handle)) => {
                tx.insert_agent_execution(&handle)?;
            }
            Ok(Err(e)) => return Err(e.clone()),
            Err(join_err) => return Err(format!("Agent {} join error: {:?}", agents[idx], join_err).into()),
        }
    }
    tx.commit()?;

    Ok(spawn_results.into_iter().filter_map(|r| r.ok().and_then(|r| r.ok())).collect())
}
```

**Performance Testing**:
```rust
#[tokio::test]
async fn test_parallel_spawn_performance() {
    let config = test_config();

    // Measure sequential spawn
    let start = Instant::now();
    for agent in ["gemini", "claude", "code"] {
        spawn_single_agent(agent, &config).await.unwrap();
    }
    let sequential_time = start.elapsed();

    // Measure parallel spawn
    let start = Instant::now();
    spawn_quality_gate_agents_parallel(&config).await.unwrap();
    let parallel_time = start.elapsed();

    // Verify 2-4× speedup (target 3×, allow variance)
    assert!(parallel_time < sequential_time / 2);
    assert!(parallel_time.as_millis() < 100);  // <100ms target
}
```

---

### Phase 4: Cleanup Automation (Week 2-3 - 8-12h)

**CLI Command** (`codex-tui/src/main.rs`):
```rust
#[derive(Parser)]
struct Cli {
    #[arg(long)]
    cleanup_old_executions: Option<u32>,  // Days threshold

    #[arg(long)]
    cleanup_old_consensus: Option<u32>,  // Days threshold

    #[arg(long)]
    cleanup_now: bool,  // Run all cleanup immediately
}

fn main() {
    let cli = Cli::parse();

    if let Some(days) = cli.cleanup_old_executions {
        cleanup::delete_old_agent_executions(days)?;
    }

    if let Some(days) = cli.cleanup_old_consensus {
        cleanup::delete_old_consensus_artifacts(days)?;
    }

    if cli.cleanup_now {
        cleanup::run_full_cleanup()?;
    }

    // Normal TUI startup
    run_tui()?;
}
```

**Cleanup Logic** (`codex-core/src/cleanup.rs`):
```rust
pub fn delete_old_agent_executions(days: u32) -> Result<usize> {
    let threshold = chrono::Utc::now() - chrono::Duration::days(days as i64);

    let conn = Connection::open(db_path())?;
    let deleted = conn.execute(
        "DELETE FROM agent_executions WHERE spawned_at < ?",
        params![threshold.to_rfc3339()],
    )?;

    tracing::info!("Deleted {} agent_executions older than {} days", deleted, days);
    Ok(deleted)
}

pub fn delete_old_consensus_artifacts(days: u32) -> Result<usize> {
    let threshold = chrono::Utc::now() - chrono::Duration::days(days as i64);

    let conn = Connection::open(db_path())?;
    let deleted = conn.execute(
        "DELETE FROM consensus_artifacts WHERE created_at < ?",
        params![threshold.to_rfc3339()],
    )?;

    tracing::info!("Deleted {} consensus_artifacts older than {} days", deleted, days);
    Ok(deleted)
}
```

**Installer Scripts**:
- `scripts/install_cleanup_cron_linux.sh` (systemd timer)
- `scripts/install_cleanup_cron_macos.sh` (launchd)
- `scripts/install_cleanup_cron_windows.ps1` (Task Scheduler)

---

## 5. Success Metrics

### Correctness Metrics
- **Crash Recovery**: 100% pass rate (0% data corruption in 100 crash tests)
- **Transaction Atomicity**: 100% rollback success (failed writes revert both HashMap + SQLite)

### Performance Metrics
- **Database Size**: 153MB → <5MB (96%+ reduction)
- **Spawn Time**: 150ms → 50-70ms (2-3× speedup)
- **Transaction Overhead**: <5ms added latency (acceptable)

### Operational Metrics
- **Cleanup Success Rate**: 100% (daily cron executes successfully)
- **Database Growth**: <1MB/month (stable size with cleanup)
- **Auto-Vacuum Effectiveness**: 0% bloat accumulation

---

## 6. Risk Analysis

### Risk 1: Transaction Deadlocks (MEDIUM)
**Scenario**: Multiple agents updating simultaneously cause lock contention.
**Mitigation**:
- Use `PRAGMA busy_timeout = 5000` (5s wait before error)
- Implement exponential backoff retry (3 attempts)
- Monitor deadlock rate in telemetry

**Likelihood**: Low (quality gates only spawn 3 agents, sequential in practice)

---

### Risk 2: Full Vacuum Migration Downtime (HIGH)
**Scenario**: Initial `VACUUM` requires exclusive lock, blocks operations for ~10-60s.
**Mitigation**:
- Run migration during maintenance window (user notification)
- Provide rollback script (`cp backup → original`)
- Document expected downtime in migration guide

**Likelihood**: High (unavoidable), but one-time only

---

### Risk 3: Platform-Specific Cron Failures (MEDIUM)
**Scenario**: Cron installation fails on Windows/macOS due to permissions or platform differences.
**Mitigation**:
- Provide manual installation instructions
- Fallback: Document manual cleanup command
- CI tests for all 3 platforms

**Likelihood**: Medium (platform diversity)

---

## 7. Open Questions

### Q1: Should we use WAL mode for better concurrency?
**Context**: SQLite WAL (Write-Ahead Logging) allows concurrent readers during writes.
**Trade-off**: Better concurrency vs larger disk footprint (3 files instead of 1).
**Decision**: DEFER - Implement transactions first, add WAL if contention issues emerge.

---

### Q2: What's the retention policy for consensus artifacts?
**Context**: Cleanup deletes >30 days, but should we keep some artifacts longer (auditing, compliance)?
**Decision**: Start with 30 days, make configurable via `CODEX_RETENTION_DAYS`. Document override for compliance use cases.

---

### Q3: Should transaction scope include filesystem evidence writes?
**Context**: Some operations write both SQLite + filesystem evidence. Should transactions coordinate both?
**Decision**: NO - Filesystem is eventually consistent, SQLite is source of truth. Don't over-scope transactions.

---

## 8. Implementation Strategy

### Week 1: Transaction Infrastructure (36h)
- **Mon-Tue**: Design DbTransaction API, update consensus_db.rs
- **Wed**: Integrate into handler.rs (agent state updates)
- **Thu**: Integrate into consensus.rs (artifact storage)
- **Fri**: Integration tests (crash recovery, rollback)

### Week 2: Auto-Vacuum + Parallel (22h)
- **Mon**: Migration script, manual testing
- **Tue**: Auto-vacuum integration into application
- **Wed**: Parallel spawning implementation
- **Thu**: Performance testing, benchmarks
- **Fri**: Bug fixes, optimization

### Week 3: Cleanup + Documentation (18h)
- **Mon**: CLI cleanup commands
- **Tue**: Platform-specific cron installers
- **Wed**: Testing all 3 platforms
- **Thu**: Event sourcing rejection docs
- **Fri**: PR preparation, code review

**Total**: 76h (within 65-96h estimate)

---

## 9. Deliverables

1. **Code Changes**:
   - `codex-core/src/consensus_db.rs` - Transaction support
   - `codex-core/src/cleanup.rs` - Cleanup logic
   - `codex-tui/src/chatwidget/spec_kit/*.rs` - Transaction integration
   - `codex-tui/src/main.rs` - CLI commands

2. **Scripts**:
   - `scripts/migrate_to_auto_vacuum.sh` - One-time migration
   - `scripts/install_cleanup_cron_*.sh` - Platform-specific installers
   - `scripts/codex-cleanup.sh` - Daily cleanup script

3. **Documentation**:
   - `docs/decisions/933-event-sourcing-rejection.md` - NO-GO rationale
   - `docs/database-maintenance.md` - Auto-vacuum + cleanup guide
   - `docs/transactions.md` - Transaction usage guide

4. **Tests**:
   - Integration tests (crash recovery, rollback)
   - Performance tests (parallel spawn benchmarks)
   - Platform tests (cron installation verification)

---

## 10. Validation Plan

### Unit Tests (30 tests)
- Transaction commit/rollback logic
- Cleanup date filtering
- Auto-vacuum pragma execution

### Integration Tests (15 tests)
- Crash recovery simulation
- Concurrent agent updates
- Parallel spawning coordination

### Performance Tests (5 benchmarks)
- Sequential vs parallel spawn time
- Transaction overhead measurement
- Database size before/after migration

### Platform Tests (3 platforms)
- Linux systemd timer installation
- macOS launchd installation
- Windows Task Scheduler installation

**Total**: 53 tests

---

## 11. Conclusion

SPEC-933 addresses critical data integrity, database hygiene, and performance issues through ACID transactions, auto-vacuum, parallel spawning, and automated cleanup. **Estimated effort: 65-96 hours over 2-3 weeks.**

**Key Benefits**:
- ✅ Eliminates data corruption risk (CRITICAL)
- ✅ Reduces database 96% (153MB → <5MB)
- ✅ 3× faster agent spawning (150ms → 50ms)
- ✅ Prevents indefinite growth (automated cleanup)

**Next Steps**:
1. Review and approve SPEC-933
2. Schedule Week 1 kickoff (transaction infrastructure)
3. Plan maintenance window for auto-vacuum migration
4. Coordinate with SPEC-934 (storage consolidation) for testing
