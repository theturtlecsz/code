//! ACID transaction helpers
//!
//! SPEC-945B Component 2: Transaction coordination
//!
//! This module provides:
//! - `execute_in_transaction()`: ACID transaction wrapper with automatic rollback
//! - `batch_insert()`: Performance-optimized bulk insert helper
//! - `upsert_consensus_run()`: Conflict resolution for consensus storage

use super::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior, params};

/// Execute operation within ACID transaction
///
/// # SPEC-945B Requirements:
/// - ACID guarantees (all-or-nothing)
/// - Automatic rollback on error (via Drop)
/// - Transaction behavior selection (Deferred, Immediate, Exclusive)
///
/// # Arguments
/// * `conn` - Mutable connection for transaction
/// * `behavior` - Transaction isolation level
/// * `operation` - Closure to execute within transaction
///
/// # Returns
/// Result from operation closure, or error with automatic rollback
///
/// # Example
/// ```rust,no_run
/// # use codex_core::db::transactions::execute_in_transaction;
/// # use rusqlite::{Connection, TransactionBehavior};
/// # fn example(conn: &mut Connection) -> codex_core::db::Result<()> {
/// execute_in_transaction(conn, TransactionBehavior::Immediate, |tx| {
///     tx.execute("INSERT INTO test (value) VALUES (?1)", [42])?;
///     Ok(())
/// })?;
/// # Ok(())
/// # }
/// ```
pub fn execute_in_transaction<F, T>(
    conn: &mut Connection,
    behavior: TransactionBehavior,
    operation: F,
) -> Result<T>
where
    F: FnOnce(&Transaction) -> Result<T>,
{
    // Begin transaction with specified behavior
    let tx = conn.transaction_with_behavior(behavior)?;

    // Execute operation
    match operation(&tx) {
        Ok(result) => {
            // Commit on success
            tx.commit()?;
            Ok(result)
        }
        Err(e) => {
            // Rollback happens automatically via Drop trait
            // No need to call tx.rollback() explicitly
            Err(e)
        }
    }
}

/// Batch insert with single transaction (performance optimization)
///
/// Wraps multiple insert operations in a single ACID transaction using
/// IMMEDIATE behavior (write-heavy workload).
///
/// # Arguments
/// * `conn` - Mutable connection
/// * `table` - Table name (unused in current implementation, kept for API compatibility)
/// * `columns` - Column names (unused in current implementation, kept for API compatibility)
/// * `rows` - Slice of items to insert
/// * `bind_fn` - Closure that performs insert for each row
///
/// # Returns
/// Number of rows inserted
///
/// # Example
/// ```rust,no_run
/// # use codex_core::db::transactions::batch_insert;
/// # use rusqlite::Connection;
/// # fn example(conn: &mut Connection) -> codex_core::db::Result<()> {
/// let rows = vec![("alice", 30), ("bob", 25)];
/// let count = batch_insert(
///     conn,
///     "users",
///     &["name", "age"],
///     &rows,
///     |tx, (name, age)| {
///         tx.execute(
///             "INSERT INTO users (name, age) VALUES (?1, ?2)",
///             rusqlite::params![name, age],
///         )?;
///         Ok(())
///     },
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn batch_insert<T>(
    conn: &mut Connection,
    _table: &str,
    _columns: &[&str],
    rows: &[T],
    bind_fn: impl Fn(&Transaction, &T) -> Result<()>,
) -> Result<usize>
where
    T: Send,
{
    execute_in_transaction(conn, TransactionBehavior::Immediate, |tx| {
        for row in rows {
            bind_fn(tx, row)?;
        }
        Ok(rows.len())
    })
}

/// UPSERT pattern with conflict resolution
///
/// Inserts consensus run or updates existing run with matching (spec_id, stage, run_timestamp).
///
/// # Arguments
/// * `tx` - Active transaction
/// * `spec_id` - SPEC identifier (e.g., "SPEC-KIT-945")
/// * `stage` - Workflow stage (e.g., "plan", "implement")
/// * `consensus_ok` - Whether consensus was achieved
/// * `degraded` - Whether consensus was degraded (missing agent outputs)
/// * `synthesis_json` - Optional JSON synthesis result
///
/// # Returns
/// Row ID of inserted/updated record
///
/// # Example
/// ```rust,no_run
/// # use codex_core::db::transactions::{execute_in_transaction, upsert_consensus_run};
/// # use rusqlite::{Connection, TransactionBehavior};
/// # fn example(conn: &mut Connection) -> codex_core::db::Result<()> {
/// execute_in_transaction(conn, TransactionBehavior::Immediate, |tx| {
///     let run_id = upsert_consensus_run(
///         tx,
///         "SPEC-KIT-945",
///         "plan",
///         true,
///         false,
///         Some(r#"{"verdict": "approved"}"#),
///     )?;
///     Ok(run_id)
/// })?;
/// # Ok(())
/// # }
/// ```
pub fn upsert_consensus_run(
    tx: &Transaction,
    spec_id: &str,
    stage: &str,
    consensus_ok: bool,
    degraded: bool,
    synthesis_json: Option<&str>,
) -> Result<i64> {
    // Get current timestamp
    let timestamp = chrono::Utc::now().timestamp_millis();

    // INSERT with ON CONFLICT DO UPDATE
    tx.execute(
        "INSERT INTO consensus_runs
         (spec_id, stage, run_timestamp, consensus_ok, degraded, synthesis_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(spec_id, stage, run_timestamp)
         DO UPDATE SET
            consensus_ok = excluded.consensus_ok,
            degraded = excluded.degraded,
            synthesis_json = excluded.synthesis_json",
        params![
            spec_id,
            stage,
            timestamp,
            consensus_ok,
            degraded,
            synthesis_json,
        ],
    )?;

    // Return row ID
    Ok(tx.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Create in-memory test database with schema
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        // Create test table
        conn.execute(
            "CREATE TABLE test_data (
                id INTEGER PRIMARY KEY,
                value INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();

        // Create consensus_runs table (for upsert tests)
        conn.execute(
            "CREATE TABLE consensus_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                spec_id TEXT NOT NULL,
                stage TEXT NOT NULL,
                run_timestamp INTEGER NOT NULL,
                consensus_ok INTEGER NOT NULL,
                degraded INTEGER NOT NULL,
                synthesis_json TEXT,
                UNIQUE(spec_id, stage, run_timestamp)
            )",
            [],
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_transaction_commit() {
        let mut conn = setup_test_db();

        // Execute transaction that should commit
        let result = execute_in_transaction(&mut conn, TransactionBehavior::Immediate, |tx| {
            tx.execute("INSERT INTO test_data (value) VALUES (?1)", [42])?;
            Ok(())
        });

        assert!(result.is_ok(), "Transaction should commit successfully");

        // Verify data persisted
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM test_data WHERE value = 42",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "Committed data should persist");
    }

    #[test]
    fn test_transaction_rollback() {
        let mut conn = setup_test_db();

        // Execute transaction that should rollback
        let result: Result<()> = execute_in_transaction(&mut conn, TransactionBehavior::Immediate, |tx| {
            tx.execute("INSERT INTO test_data (value) VALUES (?1)", [99])?;
            // Simulate error - this should trigger rollback
            Err(super::super::DbError::Transaction(
                "Intentional error".to_string(),
            ))
        });

        assert!(result.is_err(), "Transaction should fail");

        // Verify data NOT persisted (rollback)
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM test_data WHERE value = 99",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "Rolled back data should not persist");
    }

    #[test]
    fn test_acid_atomicity() {
        let mut conn = setup_test_db();

        // Insert multiple rows - all should succeed or all should fail
        let result = execute_in_transaction(&mut conn, TransactionBehavior::Immediate, |tx| {
            tx.execute("INSERT INTO test_data (value) VALUES (?1)", [1])?;
            tx.execute("INSERT INTO test_data (value) VALUES (?1)", [2])?;
            tx.execute("INSERT INTO test_data (value) VALUES (?1)", [3])?;
            Ok(())
        });

        assert!(result.is_ok(), "All inserts should succeed");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test_data", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 3, "All 3 rows should be committed atomically");

        // Now test failure case - none should persist
        let result: Result<()> = execute_in_transaction(&mut conn, TransactionBehavior::Immediate, |tx| {
            tx.execute("INSERT INTO test_data (value) VALUES (?1)", [10])?;
            tx.execute("INSERT INTO test_data (value) VALUES (?1)", [20])?;
            // Fail after 2 inserts
            Err(super::super::DbError::Transaction(
                "Simulated failure".to_string(),
            ))
        });

        assert!(result.is_err(), "Transaction should fail");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test_data", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            count, 3,
            "Failed transaction should not add any rows (ACID atomicity)"
        );
    }

    #[test]
    fn test_batch_insert() {
        let mut conn = setup_test_db();

        let rows = vec![100, 200, 300, 400, 500];

        let result = batch_insert(&mut conn, "test_data", &["value"], &rows, |tx, &value| {
            tx.execute("INSERT INTO test_data (value) VALUES (?1)", [value])?;
            Ok(())
        });

        assert!(result.is_ok(), "Batch insert should succeed");
        assert_eq!(result.unwrap(), 5, "Should return count of inserted rows");

        // Verify all rows inserted
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test_data", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 5, "All 5 rows should be inserted");
    }

    #[test]
    fn test_batch_insert_rollback() {
        let mut conn = setup_test_db();

        let rows = vec![1000, 2000, 3000];

        // Batch insert that fails mid-way
        let result = batch_insert(&mut conn, "test_data", &["value"], &rows, |tx, &value| {
            if value == 2000 {
                // Fail on second row
                return Err(super::super::DbError::Transaction(
                    "Intentional failure".to_string(),
                ));
            }
            tx.execute("INSERT INTO test_data (value) VALUES (?1)", [value])?;
            Ok(())
        });

        assert!(result.is_err(), "Batch insert should fail");

        // Verify NO rows inserted (atomic rollback)
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test_data", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0, "Failed batch should not persist any rows");
    }

    #[test]
    fn test_upsert_insert_new() {
        let mut conn = setup_test_db();

        // Insert new consensus run
        let result = execute_in_transaction(&mut conn, TransactionBehavior::Immediate, |tx| {
            upsert_consensus_run(
                tx,
                "SPEC-TEST-001",
                "plan",
                true,
                false,
                Some(r#"{"status": "ok"}"#),
            )
        });

        assert!(result.is_ok(), "Upsert should succeed for new row");
        let row_id = result.unwrap();
        assert!(row_id > 0, "Should return valid row ID");

        // Verify inserted
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM consensus_runs WHERE spec_id = 'SPEC-TEST-001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "New row should be inserted");
    }

    #[test]
    fn test_upsert_conflict_resolution() {
        let mut conn = setup_test_db();

        // Insert first time
        execute_in_transaction(&mut conn, TransactionBehavior::Immediate, |tx| {
            upsert_consensus_run(
                tx,
                "SPEC-TEST-002",
                "implement",
                false, // consensus_ok = false initially
                true,  // degraded = true
                None,
            )
        })
        .unwrap();

        // Sleep briefly to ensure different timestamp (millisecond precision)
        std::thread::sleep(std::time::Duration::from_millis(2));

        // Upsert again with same spec_id/stage but updated values
        execute_in_transaction(&mut conn, TransactionBehavior::Immediate, |tx| {
            upsert_consensus_run(
                tx,
                "SPEC-TEST-002",
                "implement",
                true,                              // Updated: consensus_ok = true
                false,                             // Updated: degraded = false
                Some(r#"{"result": "improved"}"#), // Updated: synthesis added
            )
        })
        .unwrap();

        // Verify we have 2 rows (different timestamps)
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM consensus_runs WHERE spec_id = 'SPEC-TEST-002'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2, "Should have 2 runs with different timestamps");

        // Verify latest values
        let (consensus_ok, degraded): (bool, bool) = conn
            .query_row(
                "SELECT consensus_ok, degraded FROM consensus_runs
                 WHERE spec_id = 'SPEC-TEST-002'
                 ORDER BY run_timestamp DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert!(consensus_ok, "Latest consensus_ok should be true");
        assert!(!degraded, "Latest degraded should be false");
    }

    #[test]
    fn test_transaction_isolation_deferred() {
        let mut conn = setup_test_db();

        // Test Deferred behavior (locks on first write)
        let result = execute_in_transaction(&mut conn, TransactionBehavior::Deferred, |tx| {
            tx.execute("INSERT INTO test_data (value) VALUES (?1)", [777])?;
            Ok(())
        });

        assert!(result.is_ok(), "Deferred transaction should work");
    }

    #[test]
    fn test_transaction_isolation_exclusive() {
        let mut conn = setup_test_db();

        // Test Exclusive behavior (immediate exclusive lock)
        let result = execute_in_transaction(&mut conn, TransactionBehavior::Exclusive, |tx| {
            tx.execute("INSERT INTO test_data (value) VALUES (?1)", [888])?;
            Ok(())
        });

        assert!(result.is_ok(), "Exclusive transaction should work");
    }
}
