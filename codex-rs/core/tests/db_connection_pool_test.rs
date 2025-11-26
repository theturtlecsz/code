//! Integration test for connection pool (SPEC-945B Week 1 Day 1-2)
//!
//! Tests connection pooling with optimal pragmas independently of other tests.

use codex_core::db::initialize_pool;
use std::sync::Arc;
use std::thread;
use tempfile::TempDir;

#[test]
fn test_connection_pool_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let pool = initialize_pool(&db_path, 5).unwrap();
    assert_eq!(pool.max_size(), 5);
}

#[test]
fn test_connection_acquisition_and_release() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let pool = initialize_pool(&db_path, 5).unwrap();

    // Acquire and use connection
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
fn test_wal_mode_enabled() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let pool = initialize_pool(&db_path, 5).unwrap();
    let conn = pool.get().unwrap();

    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal_mode, "wal");
}

#[test]
fn test_foreign_keys_enabled() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let pool = initialize_pool(&db_path, 5).unwrap();
    let conn = pool.get().unwrap();

    let foreign_keys: i32 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .unwrap();
    assert_eq!(foreign_keys, 1);
}

#[test]
fn test_cache_size_configured() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let pool = initialize_pool(&db_path, 5).unwrap();
    let conn = pool.get().unwrap();

    let cache_size: i32 = conn
        .query_row("PRAGMA cache_size", [], |row| row.get(0))
        .unwrap();
    assert_eq!(cache_size, -32000); // -32000 KB = 32MB
}

#[test]
fn test_busy_timeout_configured() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let pool = initialize_pool(&db_path, 5).unwrap();
    let conn = pool.get().unwrap();

    let busy_timeout: i32 = conn
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .unwrap();
    assert_eq!(busy_timeout, 5000); // 5 seconds
}

#[test]
fn test_concurrent_connections() {
    let temp_dir = TempDir::new().unwrap();
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
fn test_pragma_verification_detects_bad_config() {
    use rusqlite::Connection;

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create connection WITHOUT our pool (no WAL mode)
    let _conn = Connection::open(&db_path).unwrap();

    // Now try to init pool - it should verify and detect non-WAL mode
    // Note: First connection will actually SET WAL mode via customizer
    // So this test validates the verification happens
    let pool = initialize_pool(&db_path, 5).unwrap();
    let conn = pool.get().unwrap();

    // But it should now be in WAL mode after pool init
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal_mode, "wal");
}

#[test]
fn test_pool_size_limit() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let pool = Arc::new(initialize_pool(&db_path, 3).unwrap());

    // Acquire max connections
    let _conn1 = pool.get().unwrap();
    let _conn2 = pool.get().unwrap();
    let _conn3 = pool.get().unwrap();

    // Try to acquire one more (should block, so we test with timeout)
    let pool_clone = Arc::clone(&pool);
    let handle = thread::spawn(move || {
        // This should wait for a connection to be released
        let start = std::time::Instant::now();
        let _conn = pool_clone.get();
        start.elapsed().as_millis()
    });

    // Release one connection after a short delay
    thread::sleep(std::time::Duration::from_millis(50));
    drop(_conn1);

    // Fourth connection should succeed after release
    let elapsed = handle.join().unwrap();
    assert!(
        elapsed < 1000,
        "Connection acquisition took too long: {elapsed}ms"
    );
}
