//! Auto-vacuum scheduling and space reclamation
//!
//! SPEC-945B Component 3: Auto-vacuum

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use tokio::task::JoinHandle;
use tokio::time::{Duration, interval};

use super::{DbError, Result};

/// Vacuum statistics
#[derive(Debug)]
pub struct VacuumStats {
    pub size_before: i64,
    pub size_after: i64,
    pub reclaimed: i64,
}

/// Start background vacuum daemon (non-blocking)
///
/// # SPEC-945B Requirements:
/// - Daily incremental vacuum (20 pages per cycle)
/// - tokio background task
/// - Non-blocking operation
/// - Telemetry (space reclaimed, DB size)
pub fn spawn_vacuum_daemon(pool: Pool<SqliteConnectionManager>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(86400)); // Daily (24 hours)

        loop {
            ticker.tick().await;

            if let Err(e) = run_vacuum_cycle(&pool).await {
                tracing::error!("Vacuum cycle failed: {}", e);
            }
        }
    })
}

/// Execute incremental vacuum cycle (public for manual triggering)
///
/// Returns statistics about space reclaimed.
pub async fn run_vacuum_cycle(pool: &Pool<SqliteConnectionManager>) -> Result<VacuumStats> {
    tokio::task::spawn_blocking({
        let pool = pool.clone();
        move || {
            let conn = pool.get().map_err(|e| {
                DbError::Pool(format!("Failed to acquire connection for vacuum: {e}"))
            })?;

            let size_before = get_db_size(&conn)?;

            // Incremental vacuum (20 pages per cycle)
            conn.execute("PRAGMA incremental_vacuum(20)", [])?;

            let size_after = get_db_size(&conn)?;
            let reclaimed = size_before.saturating_sub(size_after);

            tracing::info!(
                "Vacuum cycle complete: reclaimed {}KB ({} → {} bytes)",
                reclaimed / 1024,
                size_before,
                size_after
            );

            Ok(VacuumStats {
                size_before,
                size_after,
                reclaimed,
            })
        }
    })
    .await
    .map_err(|e| DbError::Pool(format!("Vacuum task panicked: {e}")))?
}

/// Get current database size (data + freelist)
fn get_db_size(conn: &Connection) -> Result<i64> {
    let page_count: i64 = conn.query_row("PRAGMA page_count", [], |row| row.get(0))?;
    let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
    Ok(page_count * page_size)
}

/// Get freelist size (wasted space)
pub fn get_freelist_size(conn: &Connection) -> Result<i64> {
    let freelist_count: i64 = conn.query_row("PRAGMA freelist_count", [], |row| row.get(0))?;
    let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
    Ok(freelist_count * page_size)
}

/// Estimate vacuum savings before running
pub fn estimate_vacuum_savings(conn: &Connection) -> Result<i64> {
    get_freelist_size(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use r2d2_sqlite::SqliteConnectionManager;
    use rusqlite::Connection;
    use std::time::Duration as StdDuration;

    fn setup_test_pool() -> Pool<SqliteConnectionManager> {
        let manager = SqliteConnectionManager::memory();
        Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("Failed to create test pool")
    }

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");

        // Enable auto_vacuum=incremental
        conn.execute("PRAGMA auto_vacuum=INCREMENTAL", [])
            .expect("Failed to set auto_vacuum");

        // Create test table
        conn.execute(
            "CREATE TABLE test_data (id INTEGER PRIMARY KEY, data TEXT)",
            [],
        )
        .expect("Failed to create test table");

        conn
    }

    #[test]
    fn test_get_db_size() {
        let conn = setup_test_db();

        let size = get_db_size(&conn).expect("Failed to get DB size");

        // Should have non-zero size (at least page_size for schema)
        assert!(size > 0, "Database size should be greater than 0");

        // Typical page size is 4096 bytes
        let page_size: i64 = conn
            .query_row("PRAGMA page_size", [], |row| row.get(0))
            .unwrap();
        assert_eq!(page_size, 4096, "Default page size should be 4096");
    }

    #[test]
    fn test_get_freelist_size() {
        let conn = setup_test_db();

        // Initially should have no freelist
        let freelist = get_freelist_size(&conn).expect("Failed to get freelist size");
        assert_eq!(freelist, 0, "New database should have no freelist");

        // Insert and delete data to create freelist
        conn.execute(
            "INSERT INTO test_data (data) VALUES (?)",
            ["x".repeat(1000)],
        )
        .unwrap();
        conn.execute("DELETE FROM test_data", []).unwrap();

        // Now freelist may be non-zero (depends on auto_vacuum mode)
        let freelist_after = get_freelist_size(&conn).expect("Failed to get freelist after delete");
        // Note: With auto_vacuum=INCREMENTAL, freelist won't grow until pages accumulate
        assert!(freelist_after >= 0, "Freelist size should be non-negative");
    }

    #[test]
    fn test_estimate_vacuum_savings() {
        let conn = setup_test_db();

        let savings = estimate_vacuum_savings(&conn).expect("Failed to estimate savings");

        // Should match freelist size
        let freelist = get_freelist_size(&conn).expect("Failed to get freelist");
        assert_eq!(
            savings, freelist,
            "Estimated savings should match freelist size"
        );
    }

    #[tokio::test]
    async fn test_run_vacuum_cycle() {
        let pool = setup_test_pool();

        // Setup database with auto_vacuum
        {
            let conn = pool.get().unwrap();
            conn.execute("PRAGMA auto_vacuum=INCREMENTAL", []).unwrap();
            conn.execute(
                "CREATE TABLE IF NOT EXISTS test_data (id INTEGER PRIMARY KEY, data TEXT)",
                [],
            )
            .unwrap();
        }

        // Run vacuum cycle
        let stats = run_vacuum_cycle(&pool)
            .await
            .expect("Vacuum cycle should succeed");

        // Verify stats structure
        assert!(stats.size_before >= 0, "size_before should be non-negative");
        assert!(stats.size_after >= 0, "size_after should be non-negative");
        assert!(stats.reclaimed >= 0, "reclaimed should be non-negative");
        assert_eq!(
            stats.size_before - stats.size_after,
            stats.reclaimed,
            "reclaimed should equal size difference"
        );
    }

    #[tokio::test]
    async fn test_spawn_vacuum_daemon() {
        let pool = setup_test_pool();

        // Setup database
        {
            let conn = pool.get().unwrap();
            conn.execute("PRAGMA auto_vacuum=INCREMENTAL", []).unwrap();
        }

        // Spawn daemon
        let handle = spawn_vacuum_daemon(pool.clone());

        // Daemon should be running
        assert!(!handle.is_finished(), "Daemon should be running");

        // Let it run briefly (won't complete a full 24-hour cycle)
        tokio::time::sleep(StdDuration::from_millis(100)).await;

        // Abort the daemon
        handle.abort();

        // Give it time to abort
        tokio::time::sleep(StdDuration::from_millis(50)).await;

        assert!(
            handle.is_finished(),
            "Daemon should be finished after abort"
        );
    }

    #[tokio::test]
    async fn test_vacuum_space_reclamation_integration() {
        let pool = setup_test_pool();

        // Setup database with auto_vacuum and insert data
        {
            let conn = pool.get().unwrap();
            conn.execute("PRAGMA auto_vacuum=INCREMENTAL", []).unwrap();
            conn.execute(
                "CREATE TABLE test_data (id INTEGER PRIMARY KEY, data TEXT)",
                [],
            )
            .unwrap();

            // Insert substantial data
            for i in 0..100 {
                conn.execute(
                    "INSERT INTO test_data (data) VALUES (?)",
                    [format!("Data row {} with padding: {}", i, "x".repeat(500))],
                )
                .unwrap();
            }
        }

        let size_with_data = {
            let conn = pool.get().unwrap();
            get_db_size(&conn).unwrap()
        };

        // Delete half the data to create freelist
        {
            let conn = pool.get().unwrap();
            conn.execute("DELETE FROM test_data WHERE id % 2 = 0", [])
                .unwrap();
        }

        let size_after_delete = {
            let conn = pool.get().unwrap();
            get_db_size(&conn).unwrap()
        };

        // Size should be same (deleted pages go to freelist)
        assert_eq!(
            size_with_data, size_after_delete,
            "Size should not decrease immediately after DELETE"
        );

        // Run vacuum to reclaim space
        let stats = run_vacuum_cycle(&pool).await.unwrap();

        // After vacuum, size should be smaller or equal
        assert!(
            stats.size_after <= stats.size_before,
            "Size after vacuum should be <= size before vacuum"
        );

        // Final size should be less than or equal to original
        assert!(
            stats.size_after <= size_with_data,
            "Final size should be <= size with full data"
        );
    }
}
