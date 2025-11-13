//! Connection pooling and pragma configuration
//!
//! SPEC-945B Component 1: Connection pool with optimal pragmas

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::Path;

use super::{DbError, Result};

/// Initialize connection pool with optimal pragmas
///
/// # SPEC-945B Requirements:
/// - r2d2 connection pooling (10 connections)
/// - WAL mode (6.6× read speedup)
/// - Optimized pragmas (cache_size, synchronous, mmap_size)
/// - Foreign key enforcement
/// - Auto-vacuum (incremental)
///
/// # Arguments
/// * `db_path` - Path to SQLite database file
/// * `pool_size` - Maximum number of connections (recommended: 10)
///
/// # Pragmas Applied
/// - `journal_mode = WAL`: 6.6× read speedup (per SPEC-945B benchmarks)
/// - `synchronous = NORMAL`: 2-3× write speedup (safe with WAL)
/// - `foreign_keys = ON`: Referential integrity enforcement
/// - `cache_size = -32000`: 32MB page cache
/// - `temp_store = MEMORY`: In-memory temp tables
/// - `auto_vacuum = INCREMENTAL`: Prevent unbounded growth
/// - `mmap_size = 1073741824`: 1GB memory-mapped I/O
/// - `busy_timeout = 5000`: 5s deadlock wait
///
/// # Performance Impact (per SPEC-945B Section 1.3)
/// - Before: 850µs/read, 2.1ms/write, 78ms/100-read batch
/// - After: 129µs/read, 0.9ms/write, 12ms/100-read batch
/// - Overall: 6.6× read improvement, 2.3× write improvement
pub fn initialize_pool(db_path: &Path, pool_size: u32) -> Result<Pool<SqliteConnectionManager>> {
    let manager = SqliteConnectionManager::file(db_path);

    let pool = Pool::builder()
        .max_size(pool_size)
        .min_idle(Some(2)) // Keep 2 warm connections
        .connection_customizer(Box::new(ConnectionCustomizer))
        .test_on_check_out(true) // Health check before returning
        .build(manager)
        .map_err(|e| DbError::Pool(format!("Failed to create connection pool: {e}")))?;

    // Verify pragmas on initial connection
    let conn = pool
        .get()
        .map_err(|e| DbError::Pool(format!("Failed to get initial connection: {e}")))?;
    verify_pragmas(&conn)?;

    Ok(pool)
}

/// Apply optimal pragmas to each connection
#[derive(Debug)]
struct ConnectionCustomizer;

impl r2d2::CustomizeConnection<Connection, rusqlite::Error> for ConnectionCustomizer {
    fn on_acquire(&self, conn: &mut Connection) -> std::result::Result<(), rusqlite::Error> {
        // Apply performance pragmas to each connection
        // Comments inline would be parsed as Rust, so documented here:
        // - cache_size = -32000: 32MB page cache
        // - mmap_size = 1073741824: 1GB memory-mapped I/O
        // - busy_timeout = 5000: 5s deadlock wait
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA cache_size = -32000;
             PRAGMA temp_store = MEMORY;
             PRAGMA auto_vacuum = INCREMENTAL;
             PRAGMA mmap_size = 1073741824;
             PRAGMA busy_timeout = 5000;",
        )
    }
}

/// Verify critical pragmas are applied
///
/// Checks that WAL mode and foreign keys are enabled, which are
/// critical for performance and correctness.
fn verify_pragmas(conn: &Connection) -> Result<()> {
    let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;

    if journal_mode != "wal" {
        return Err(DbError::Pool(format!(
            "WAL mode not enabled (got: {journal_mode})"
        )));
    }

    let foreign_keys: i32 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;

    if foreign_keys != 1 {
        return Err(DbError::Pool(
            "Foreign key enforcement not enabled".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_initialize_pool_succeeds() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = initialize_pool(&db_path, 5).unwrap();
        assert_eq!(pool.max_size(), 5);
    }

    #[test]
    fn test_connection_acquisition() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = initialize_pool(&db_path, 5).unwrap();

        // Acquire and release connection
        {
            let conn = pool.get().unwrap();
            conn.execute("CREATE TABLE test (id INTEGER)", []).unwrap();
        }

        // Acquire again (should succeed after release)
        let conn = pool.get().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_pragmas_applied() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = initialize_pool(&db_path, 5).unwrap();
        let conn = pool.get().unwrap();

        // Verify WAL mode
        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(journal_mode, "wal");

        // Verify foreign keys
        let foreign_keys: i32 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(foreign_keys, 1);

        // Verify cache size (negative value means KB)
        let cache_size: i32 = conn
            .query_row("PRAGMA cache_size", [], |row| row.get(0))
            .unwrap();
        assert_eq!(cache_size, -32000);

        // Verify busy timeout
        let busy_timeout: i32 = conn
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();
        assert_eq!(busy_timeout, 5000);
    }

    #[test]
    fn test_concurrent_connections() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = Arc::new(initialize_pool(&db_path, 10).unwrap());

        // Create table first
        {
            let conn = pool.get().unwrap();
            conn.execute("CREATE TABLE test (id INTEGER, value TEXT)", [])
                .unwrap();
        }

        // Spawn 10 threads, each acquiring a connection
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let pool = Arc::clone(&pool);
                thread::spawn(move || {
                    let conn = pool.get().unwrap();
                    conn.execute(
                        "INSERT INTO test (id, value) VALUES (?1, ?2)",
                        rusqlite::params![i, format!("value_{}", i)],
                    )
                    .unwrap();
                })
            })
            .collect();

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all inserts succeeded
        let conn = pool.get().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 10);
    }

    #[test]
    fn test_verify_pragmas_detects_bad_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create connection without WAL mode
        let conn = Connection::open(&db_path).unwrap();

        // Should fail verification (not in WAL mode)
        let result = verify_pragmas(&conn);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("WAL mode not enabled")
        );
    }
}
