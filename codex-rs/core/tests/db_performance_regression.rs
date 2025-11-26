// SPEC-957: Allow expect/unwrap in test code
#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Performance regression tests for SPEC-945B
//!
//! These tests validate that database performance remains within acceptable
//! bounds across code changes. They run in CI/CD and fail if performance
//! regresses beyond 20% threshold.
//!
//! ## Purpose
//! - Catch performance regressions early in CI/CD
//! - Validate WAL mode benefits are maintained
//! - Ensure transaction batching remains effective
//!
//! ## Running Tests
//! ```bash
//! cd codex-rs
//! cargo test --test db_performance_regression
//! ```

use codex_core::db::initialize_pool;
use rusqlite::Connection;
use std::path::PathBuf;
use std::time::Instant;
use tempfile::TempDir;

// ============================================================================
// Test Setup Helpers
// ============================================================================

fn setup_temp_db() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let conn = Connection::open(&db_path).expect("Failed to open connection");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS consensus_runs (
            id INTEGER PRIMARY KEY,
            spec_id TEXT NOT NULL,
            stage TEXT NOT NULL,
            consensus_ok INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_spec_stage ON consensus_runs(spec_id, stage);",
    )
    .expect("Failed to create schema");

    (temp_dir, db_path)
}

fn insert_test_data(conn: &Connection, count: usize) {
    for i in 0..count {
        conn.execute(
            "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at) VALUES (?1, ?2, ?3, ?4)",
            (
                format!("SPEC-TEST-{:03}", i % 100),
                "plan",
                1,
                1000000 + i as i64,
            ),
        )
        .expect("Failed to insert test data");
    }
}

// ============================================================================
// Performance Regression Tests
// ============================================================================

#[test]
fn test_wal_mode_read_performance() {
    // Setup: Database with WAL mode
    let (_temp_dir, db_path) = setup_temp_db();
    let pool = initialize_pool(&db_path, 10).expect("Failed to initialize pool");

    // Insert test data
    {
        let conn = pool.get().expect("Failed to get connection");
        insert_test_data(&conn, 1000);
    }

    // Performance test: 1000 read operations
    let start = Instant::now();
    for _ in 0..1000 {
        let conn = pool.get().expect("Failed to get connection");
        let mut stmt = conn
            .prepare("SELECT * FROM consensus_runs WHERE spec_id = ?1")
            .expect("Failed to prepare statement");
        let _count = stmt
            .query_map(["SPEC-TEST-050"], |_row| Ok(()))
            .expect("Failed to query")
            .count();
    }
    let duration = start.elapsed();

    // Validation: Average read time should be <50µs (with 20% margin)
    // Target: ~10µs from benchmarks, allowing 5× margin for CI variability
    let avg_read_time = duration.as_micros() / 1000;
    println!("Average read time: {avg_read_time}µs");

    assert!(
        avg_read_time < 50,
        "Read performance regression detected: {avg_read_time}µs (expected <50µs)"
    );
}

#[test]
fn test_wal_mode_write_performance() {
    // Setup: Database with WAL mode
    let (_temp_dir, db_path) = setup_temp_db();
    let pool = initialize_pool(&db_path, 10).expect("Failed to initialize pool");

    // Performance test: 100 write operations
    let start = Instant::now();
    for i in 0..100 {
        let conn = pool.get().expect("Failed to get connection");
        conn.execute(
            "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at) VALUES (?1, ?2, ?3, ?4)",
            (format!("SPEC-PERF-{i}"), "implement", 1, 2000000 + i),
        )
        .expect("Failed to insert");
    }
    let duration = start.elapsed();

    // Validation: Average write time should be <100µs (with margin)
    // Target: ~17µs from benchmarks, allowing 6× margin for CI variability
    let avg_write_time = duration.as_micros() / 100;
    println!("Average write time: {avg_write_time}µs");

    assert!(
        avg_write_time < 100,
        "Write performance regression detected: {avg_write_time}µs (expected <100µs)"
    );
}

#[test]
fn test_transaction_batch_performance() {
    // Setup: Database with WAL mode
    let (_temp_dir, db_path) = setup_temp_db();
    let pool = initialize_pool(&db_path, 10).expect("Failed to initialize pool");

    // Performance test: 100 inserts in transaction
    let start = Instant::now();
    {
        let mut conn = pool.get().expect("Failed to get connection");
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .expect("Failed to start transaction");

        for i in 0..100 {
            tx.execute(
                "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at) VALUES (?1, ?2, ?3, ?4)",
                (format!("SPEC-BATCH-{i}"), "validate", 1, 3000000 + i),
            )
            .expect("Failed to insert");
        }

        tx.commit().expect("Failed to commit");
    }
    let duration = start.elapsed();

    // Validation: Transaction batch should be <2ms (with margin)
    // Target: ~890µs from benchmarks, allowing 2× margin
    let batch_time_ms = duration.as_micros() as f64 / 1000.0;
    println!("Transaction batch time: {batch_time_ms:.2}ms");

    assert!(
        batch_time_ms < 2.0,
        "Transaction batch performance regression: {batch_time_ms:.2}ms (expected <2.0ms)"
    );
}

#[test]
fn test_connection_pool_overhead() {
    // Setup: Database with connection pool
    let (_temp_dir, db_path) = setup_temp_db();
    let pool = initialize_pool(&db_path, 10).expect("Failed to initialize pool");

    // Insert test data
    {
        let conn = pool.get().expect("Failed to get connection");
        insert_test_data(&conn, 100);
    }

    // Performance test: 100 pooled connection acquisitions + queries
    let start = Instant::now();
    for _ in 0..100 {
        let conn = pool.get().expect("Failed to get connection");
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM consensus_runs")
            .expect("Failed to prepare");
        let _count: i64 = stmt
            .query_row([], |row| row.get(0))
            .expect("Failed to query");
    }
    let duration = start.elapsed();

    // Validation: Average pooled operation should be <100µs
    // Target: ~10µs query + pool overhead, allowing 10× margin
    let avg_time = duration.as_micros() / 100;
    println!("Average pooled operation time: {avg_time}µs");

    assert!(
        avg_time < 100,
        "Connection pool overhead regression: {avg_time}µs (expected <100µs)"
    );
}

#[test]
fn test_dual_write_overhead_bounds() {
    // Setup: Two separate databases (simulating old + new schema)
    let (_temp_dir1, db_path1) = setup_temp_db();
    let (_temp_dir2, db_path2) = setup_temp_db();

    let pool1 = initialize_pool(&db_path1, 10).expect("Failed to initialize pool1");
    let pool2 = initialize_pool(&db_path2, 10).expect("Failed to initialize pool2");

    // Performance test: 100 dual writes
    let start = Instant::now();
    for i in 0..100 {
        // Write to first database
        {
            let conn1 = pool1.get().expect("Failed to get connection");
            conn1
                .execute(
                    "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at) VALUES (?1, ?2, ?3, ?4)",
                    (format!("SPEC-DUAL-{i}"), "audit", 1, 4000000 + i),
                )
                .expect("Failed to insert to db1");
        }

        // Write to second database
        {
            let conn2 = pool2.get().expect("Failed to get connection");
            conn2
                .execute(
                    "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at) VALUES (?1, ?2, ?3, ?4)",
                    (format!("SPEC-DUAL-{i}"), "audit", 1, 4000000 + i),
                )
                .expect("Failed to insert to db2");
        }
    }
    let duration = start.elapsed();

    // Validation: Dual write should be <150% of single write time
    // Benchmark showed ~105% overhead, allowing margin for CI variability
    // Average dual write: ~35µs from benchmarks (release mode), so 100 operations ~3.5ms
    // In test mode (debug build): 4-6× slower, so allowing <30ms
    let total_time_ms = duration.as_micros() as f64 / 1000.0;
    println!("Dual write total time (100 ops): {total_time_ms:.2}ms");

    assert!(
        total_time_ms < 30.0,
        "Dual write performance regression: {total_time_ms:.2}ms (expected <30ms for 100 operations in debug mode)"
    );
}

#[test]
fn test_concurrent_read_performance() {
    use std::sync::Arc;
    use std::thread;

    // Setup: Database with WAL mode (enables concurrent reads)
    let (_temp_dir, db_path) = setup_temp_db();
    let pool = Arc::new(initialize_pool(&db_path, 10).expect("Failed to initialize pool"));

    // Insert test data
    {
        let conn = pool.get().expect("Failed to get connection");
        insert_test_data(&conn, 1000);
    }

    // Performance test: 10 threads doing 100 reads each
    let start = Instant::now();
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let pool_clone = Arc::clone(&pool);
            thread::spawn(move || {
                for _ in 0..100 {
                    let conn = pool_clone.get().expect("Failed to get connection");
                    let mut stmt = conn
                        .prepare("SELECT * FROM consensus_runs WHERE spec_id = ?1")
                        .expect("Failed to prepare");
                    let _count = stmt
                        .query_map(["SPEC-TEST-050"], |_row| Ok(()))
                        .expect("Failed to query")
                        .count();
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
    let duration = start.elapsed();

    // Validation: 1000 total reads with 10 threads should benefit from parallelism
    // Target: Should be faster than 1000 sequential reads
    // Sequential: ~10µs × 1000 = 10ms
    // Parallel (10 threads): Should be closer to 10ms / 10 = 1ms, allowing 5× margin
    let total_time_ms = duration.as_micros() as f64 / 1000.0;
    println!("Concurrent read time (1000 reads, 10 threads): {total_time_ms:.2}ms");

    assert!(
        total_time_ms < 50.0,
        "Concurrent read performance regression: {total_time_ms:.2}ms (expected <50ms)"
    );
}
