# Database Layer

SQLite storage with optimized performance for consensus artifacts.

---

## Overview

**Database**: SQLite (embedded, ACID transactions)
**Primary Use**: Consensus artifact storage (agent outputs, synthesis)
**Performance**: **6.6× read speedup**, **2.3× write speedup** (via optimizations)
**Location**: `codex-rs/core/src/db/`, `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs`

---

## Architecture

```
Application
    ↓
R2D2 Connection Pool (2-8 connections)
    ├→ SQLite Connection (WAL mode)
    ├→ SQLite Connection
    └→ SQLite Connection
    ↓
Database File (consensus_artifacts.db)
├─ consensus_runs table
├─ agent_outputs table
└─ consensus_artifacts table
```

---

## Connection Pooling

### R2D2 Configuration

**Location**: `codex-rs/core/src/db/connection.rs:39-105`

```rust
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

pub fn initialize_pool(
    db_path: &Path,
    pool_size: u32,
) -> Result<Pool<SqliteConnectionManager>> {
    let manager = SqliteConnectionManager::file(db_path);

    let pool = Pool::builder()
        .max_size(pool_size)           // Max connections (default: 8)
        .min_idle(Some(2))             // Keep 2 connections warm
        .connection_customizer(Box::new(ConnectionCustomizer))
        .test_on_check_out(true)       // Health check before use
        .build(manager)?;

    // Verify pragmas on first connection
    let conn = pool.get()?;
    verify_pragmas(&conn)?;

    Ok(pool)
}
```

**Benefits**:
- **Connection reuse**: Eliminate open/close overhead
- **Concurrency**: Multiple connections for parallel access
- **Health checks**: Detect broken connections before use
- **Warm pool**: Keep minimum connections ready

---

### Connection Customizer (Pragma Optimization)

```rust
struct ConnectionCustomizer;

impl r2d2::CustomizeConnection<Connection, rusqlite::Error> for ConnectionCustomizer {
    fn on_acquire(&self, conn: &mut Connection) -> Result<(), rusqlite::Error> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;              -- Write-Ahead Logging (6.6× read speedup)
             PRAGMA synchronous = NORMAL;           -- 2-3× write speedup (safe with WAL)
             PRAGMA foreign_keys = ON;              -- Referential integrity
             PRAGMA cache_size = -32000;            -- 32MB page cache
             PRAGMA temp_store = MEMORY;            -- In-memory temporary tables
             PRAGMA auto_vacuum = INCREMENTAL;      -- Prevent unbounded growth
             PRAGMA mmap_size = 1073741824;         -- 1GB memory-mapped I/O
             PRAGMA busy_timeout = 5000;            -- 5s deadlock wait
            "
        )
    }
}
```

**Pragma Explanations**:

| Pragma | Value | Effect |
|--------|-------|--------|
| `journal_mode` | `WAL` | Write-Ahead Logging: Concurrent reads during writes |
| `synchronous` | `NORMAL` | Fewer fsync calls (safe with WAL) |
| `foreign_keys` | `ON` | Enforce foreign key constraints |
| `cache_size` | `-32000` | 32MB in-memory page cache |
| `temp_store` | `MEMORY` | Temporary tables in RAM |
| `auto_vacuum` | `INCREMENTAL` | Gradual space reclamation |
| `mmap_size` | `1GB` | Memory-mapped I/O for reads |
| `busy_timeout` | `5s` | Retry on lock for 5 seconds |

---

### Performance Impact

**Before optimizations**:
```
Single read:  850µs
Single write: 2.1ms
100-read batch: 78ms
```

**After optimizations**:
```
Single read:  129µs  (6.6× faster)
Single write: 0.9ms  (2.3× faster)
100-read batch: 12ms  (6.5× faster)
```

**Total improvement**: 6.6× read, 2.3× write

**Benchmark**: `codex-rs/core/tests/db_benchmark.rs`

---

## Schema

### consensus_runs

**Purpose**: Track workflow execution metadata

```sql
CREATE TABLE IF NOT EXISTS consensus_runs (
    run_id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    consensus_ok BOOLEAN NOT NULL,
    degraded BOOLEAN NOT NULL,
    synthesis_json TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(spec_id, stage)
);

CREATE INDEX IF NOT EXISTS idx_consensus_runs_spec
ON consensus_runs(spec_id, stage);
```

**Columns**:
- `run_id`: Auto-increment primary key
- `spec_id`: SPEC identifier (e.g., "SPEC-KIT-065")
- `stage`: Workflow stage ("plan", "tasks", "implement", etc.)
- `consensus_ok`: Consensus achieved (true/false)
- `degraded`: Some agents failed (true/false)
- `synthesis_json`: Synthesized consensus result (JSON)
- `created_at`: Timestamp

**Constraint**: One run per (spec_id, stage) pair (UPSERT semantics)

---

### agent_outputs

**Purpose**: Store individual agent responses

```sql
CREATE TABLE IF NOT EXISTS agent_outputs (
    output_id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL,
    agent_name TEXT NOT NULL,
    agent_version TEXT,
    content_json TEXT NOT NULL,
    response_text TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (run_id) REFERENCES consensus_runs(run_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_agent_outputs_run
ON agent_outputs(run_id);
```

**Columns**:
- `output_id`: Auto-increment primary key
- `run_id`: Foreign key to consensus_runs
- `agent_name`: Agent identifier ("gemini", "claude", "code")
- `agent_version`: Model version ("gemini-flash-1.5", "claude-sonnet-4-5")
- `content_json`: Structured agent output (JSON)
- `response_text`: Raw response text
- `created_at`: Timestamp

**Relationship**: Many agent_outputs per consensus_run (1:N)

---

### consensus_artifacts (legacy)

**Purpose**: Old schema (being phased out)

```sql
CREATE TABLE IF NOT EXISTS consensus_artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    spec_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    content_json TEXT NOT NULL,
    response_text TEXT,
    run_id TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**Status**: Deprecated, replaced by consensus_runs + agent_outputs

---

## Transaction Handling

### Transaction Behavior

```rust
use rusqlite::TransactionBehavior;

pub enum TransactionBehavior {
    Deferred,     // Lock on first read/write
    Immediate,    // Write lock immediately
    Exclusive,    // Exclusive lock (blocks all)
}
```

**Recommendation**: Use `IMMEDIATE` for writes (avoid write-write deadlock)

---

### Transaction Helpers

**Location**: `codex-rs/core/src/db/transactions.rs:40-119`

```rust
pub fn execute_in_transaction<F, T>(
    conn: &mut Connection,
    behavior: TransactionBehavior,
    operation: F,
) -> Result<T>
where
    F: FnOnce(&Transaction) -> Result<T>,
{
    let tx = conn.transaction_with_behavior(behavior)?;

    match operation(&tx) {
        Ok(result) => {
            tx.commit()?;
            Ok(result)
        },
        Err(e) => {
            // Automatic rollback via Drop trait
            Err(e)
        },
    }
}
```

**Usage**:
```rust
execute_in_transaction(conn, TransactionBehavior::Immediate, |tx| {
    tx.execute("INSERT INTO consensus_runs (...) VALUES (?)", params)?;
    tx.execute("INSERT INTO agent_outputs (...) VALUES (?)", params)?;
    Ok(())
})?;
```

**ACID guarantees**:
- **Atomicity**: All or nothing (rollback on error)
- **Consistency**: Foreign keys enforced
- **Isolation**: IMMEDIATE locks prevent conflicts
- **Durability**: WAL ensures persistence

---

### Batch Operations

```rust
pub fn batch_insert<T>(
    conn: &mut Connection,
    _table: &str,
    _columns: &[&str],
    rows: &[T],
    bind_fn: impl Fn(&Transaction, &T) -> Result<()>,
) -> Result<usize> {
    execute_in_transaction(conn, TransactionBehavior::Immediate, |tx| {
        for row in rows {
            bind_fn(tx, row)?;
        }
        Ok(rows.len())
    })
}
```

**Performance**: 100 inserts in single transaction ~12ms (vs ~2s for 100 individual commits)

---

## Async Wrapper

**Problem**: SQLite is synchronous, Tokio is async

**Solution**: `tokio::task::spawn_blocking` wrapper

**Location**: `codex-rs/core/src/db/async_wrapper.rs:69-150`

```rust
pub async fn with_connection<F, T>(
    pool: &Pool<SqliteConnectionManager>,
    f: F,
) -> Result<T>
where
    F: FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let pool = pool.clone();

    // Run synchronous SQLite code in blocking thread pool
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        f(&mut conn)
    })
    .await?
}
```

**Usage**:
```rust
// Async function calling sync SQLite
pub async fn store_consensus(
    pool: &Pool<SqliteConnectionManager>,
    spec_id: &str,
    stage: &str,
    synthesis: &str,
) -> Result<i64> {
    with_connection(pool, move |conn| {
        execute_in_transaction(conn, TransactionBehavior::Immediate, |tx| {
            upsert_consensus_run(tx, spec_id, stage, synthesis)
        })
    }).await
}
```

**Key Points**:
- **Doesn't block Tokio runtime**: Runs on separate thread pool
- **Connection pooling**: Still benefits from R2D2 pool
- **Error propagation**: Propagates SQLite errors to async context

---

## Consensus Database (Spec-Kit)

### ConsensusDb

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs:34-150`

```rust
pub struct ConsensusDb {
    conn: Arc<Mutex<Connection>>,                       // Legacy single connection
    pool: Option<Pool<SqliteConnectionManager>>,        // New pooled connections
}

impl ConsensusDb {
    pub fn init(db_path: &Path) -> Result<Self> {
        // Open single connection (legacy)
        let conn = Connection::open(db_path)?;

        // Create schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS agent_executions (
                agent_id TEXT PRIMARY KEY,
                spec_id TEXT NOT NULL,
                stage TEXT NOT NULL,
                phase_type TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                run_id TEXT,
                spawned_at TEXT NOT NULL,
                completed_at TEXT,
                response_text TEXT,
                extraction_error TEXT
            )",
            [],
        )?;

        // Create indices
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_agent_executions_spec
             ON agent_executions(spec_id, stage)",
            [],
        )?;

        // Initialize new schema pool (SPEC-945B)
        let pool = initialize_pool(db_path, 8)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            pool: Some(pool),
        })
    }

    pub fn upsert_consensus_run(
        &self,
        spec_id: &str,
        stage: &str,
        consensus_ok: bool,
        degraded: bool,
        synthesis_json: Option<&str>,
    ) -> Result<i64> {
        let pool = self.pool.as_ref().ok_or(Error::PoolNotInitialized)?;
        let conn = pool.get()?;

        execute_in_transaction(&mut conn, TransactionBehavior::Immediate, |tx| {
            tx.execute(
                "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, degraded, synthesis_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(spec_id, stage) DO UPDATE SET
                     consensus_ok = excluded.consensus_ok,
                     degraded = excluded.degraded,
                     synthesis_json = excluded.synthesis_json,
                     created_at = CURRENT_TIMESTAMP",
                params![spec_id, stage, consensus_ok, degraded, synthesis_json],
            )?;

            let run_id = tx.last_insert_rowid();
            Ok(run_id)
        })
    }

    pub fn insert_agent_output(
        &self,
        run_id: i64,
        agent_name: &str,
        agent_version: Option<&str>,
        content_json: &str,
        response_text: Option<&str>,
    ) -> Result<i64> {
        let pool = self.pool.as_ref().ok_or(Error::PoolNotInitialized)?;
        let conn = pool.get()?;

        conn.execute(
            "INSERT INTO agent_outputs (run_id, agent_name, agent_version, content_json, response_text)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![run_id, agent_name, agent_version, content_json, response_text],
        )?;

        Ok(conn.last_insert_rowid())
    }
}
```

---

### Dual-Schema Migration (SPEC-945B)

**Phase 1**: Old schema only (agent_executions)
**Phase 2**: Dual-write to both schemas (current)
**Phase 3**: New schema only (consensus_runs + agent_outputs)

**Current state**: Dual-write active

```rust
// Write to both old and new schema
pub fn store_consensus(&self, spec_id: &str, stage: &str) -> Result<()> {
    // Old schema (legacy)
    self.conn.lock().unwrap().execute(
        "INSERT INTO agent_executions (...) VALUES (?)",
        params![...],
    )?;

    // New schema (pooled)
    if let Some(pool) = &self.pool {
        self.upsert_consensus_run(spec_id, stage, true, false, None)?;
    }

    Ok(())
}
```

---

## Auto-Vacuum Strategy

### Incremental Auto-Vacuum

**Purpose**: Prevent unbounded database growth

**Configuration**: `PRAGMA auto_vacuum = INCREMENTAL;`

**How it works**:
- Database tracks free pages internally
- On `PRAGMA incremental_vacuum(N)`, reclaim N pages
- No blocking full-vacuum required

**Usage**:
```rust
pub fn compact_database(conn: &Connection, max_pages: u32) -> Result<()> {
    conn.execute(&format!("PRAGMA incremental_vacuum({})", max_pages), [])?;
    Ok(())
}
```

**Result**: 99.95% size reduction after cleanup (multi-GB → few MB)

**Evidence**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/database-cleanup.log`

---

## Retry Logic (Database Operations)

### Sync Retry

**Location**: `codex-rs/spec-kit/src/retry/strategy.rs`

```rust
pub fn execute_with_backoff_sync<F, T, E>(
    mut operation: F,
    config: &RetryConfig,
) -> Result<T>
where
    F: FnMut() -> Result<T, E>,
    E: Error + RetryClassifiable,
{
    let mut attempts = 0;
    let mut backoff_ms = config.initial_backoff_ms;

    loop {
        attempts += 1;

        match operation() {
            Ok(value) => return Ok(value),
            Err(err) if err.error_code() == Some(rusqlite::ErrorCode::DatabaseBusy) => {
                // Database locked, retry with backoff
                if attempts >= config.max_attempts {
                    return Err(RetryError::MaxAttemptsExceeded(attempts));
                }

                std::thread::sleep(Duration::from_millis(backoff_ms));
                backoff_ms = (backoff_ms as f64 * config.backoff_multiplier) as u64;
                backoff_ms = backoff_ms.min(config.max_backoff_ms);
            },
            Err(err) => {
                // Permanent error, don't retry
                return Err(RetryError::PermanentError(err.to_string()));
            },
        }
    }
}
```

**Usage**:
```rust
let result = execute_with_backoff_sync(
    || conn.execute("INSERT INTO ...", params),
    &RetryConfig::default(),
)?;
```

---

## Error Handling

### Database Errors

```rust
pub enum DbError {
    // Connection errors
    PoolExhausted,
    ConnectionFailed(r2d2::Error),

    // SQLite errors
    DatabaseBusy,              // Retryable
    DatabaseLocked,            // Retryable
    ConstraintViolation(String), // Not retryable
    SchemaError(String),       // Not retryable

    // Application errors
    InvalidSchema(String),
    MigrationFailed(String),
}
```

---

### Error Classification

```rust
impl RetryClassifiable for DbError {
    fn is_retryable(&self) -> bool {
        match self {
            DbError::DatabaseBusy => true,
            DbError::DatabaseLocked => true,
            DbError::PoolExhausted => true,

            DbError::ConstraintViolation(_) => false,
            DbError::SchemaError(_) => false,
            DbError::InvalidSchema(_) => false,

            _ => false,
        }
    }
}
```

---

## Schema Migrations

### Migration System

**Location**: `codex-rs/core/src/db/migrations.rs:9-87`

```rust
pub const SCHEMA_VERSION: i32 = 2;

pub fn migrate_to_latest(conn: &mut Connection) -> Result<()> {
    let current_version = get_schema_version(conn)?;

    if current_version == SCHEMA_VERSION {
        return Ok(());  // Already up-to-date
    }

    let tx = conn.transaction_with_behavior(TransactionBehavior::Exclusive)?;

    for version in (current_version + 1)..=SCHEMA_VERSION {
        apply_migration(&tx, version)?;
    }

    // Update schema version
    tx.execute(&format!("PRAGMA user_version = {}", SCHEMA_VERSION), [])?;
    tx.commit()?;

    Ok(())
}

fn get_schema_version(conn: &Connection) -> Result<i32> {
    let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(version)
}

fn apply_migration(conn: &Connection, version: i32) -> Result<()> {
    match version {
        1 => migration_v1(conn),  // Create consensus_runs, agent_outputs
        2 => migration_v2(conn),  // Add indices
        _ => Err(Error::UnknownMigrationVersion(version)),
    }
}
```

**Migration v1**:
```rust
fn migration_v1(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS consensus_runs (...);
         CREATE TABLE IF NOT EXISTS agent_outputs (...);
         CREATE INDEX IF NOT EXISTS idx_consensus_runs_spec ON consensus_runs(spec_id, stage);
         CREATE INDEX IF NOT EXISTS idx_agent_outputs_run ON agent_outputs(run_id);"
    )?;
    Ok(())
}
```

**Forward-only**: Migrations never rollback (destructive changes prohibited)

---

## Summary

**Database Layer Highlights**:

1. **Performance**: 6.6× read speedup, 2.3× write speedup (WAL + pragmas)
2. **Connection Pooling**: R2D2 with 2-8 connections
3. **Async Wrapper**: `tokio::task::spawn_blocking` for Tokio integration
4. **ACID Transactions**: IMMEDIATE mode for write consistency
5. **Schema**: consensus_runs + agent_outputs (normalized)
6. **Auto-Vacuum**: Incremental strategy (99.95% size reduction)
7. **Retry Logic**: Exponential backoff for database busy errors
8. **Migrations**: Forward-only schema evolution

**Next Steps**:
- [Configuration System](configuration-system.md) - Hot-reload and 5-tier precedence

---

**File References**:
- Connection pool: `codex-rs/core/src/db/connection.rs:39-105`
- Transactions: `codex-rs/core/src/db/transactions.rs:40-119`
- Async wrapper: `codex-rs/core/src/db/async_wrapper.rs:69-150`
- Consensus DB: `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs:34-150`
- Migrations: `codex-rs/core/src/db/migrations.rs:9-87`
- Retry logic: `codex-rs/spec-kit/src/retry/strategy.rs`
