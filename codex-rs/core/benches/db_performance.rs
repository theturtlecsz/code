// Benchmarks use expect/unwrap for simplicity - test code, not production
#![allow(clippy::expect_used, clippy::unwrap_used)]

//! SPEC-945B: Database Performance Benchmarks
//!
//! Week 2 Day 4: Benchmarking & Performance Validation
//!
//! This benchmark suite validates:
//! 1. Connection pool vs single connection (target: 6.6× read speedup)
//! 2. Dual-write overhead (target: <10% overhead)
//! 3. WAL mode benefits (target: 6.6× read, 2.3× write)
//! 4. Transaction performance (IMMEDIATE vs DEFERRED)
//!
//! ## Running Benchmarks
//! ```bash
//! cd codex-rs
//! cargo bench --bench db_performance
//! ```
//!
//! ## Performance Targets (from SPEC-945B)
//! - Before: 850µs/read, 2.1ms/write, 78ms/100-read batch
//! - After: 129µs/read, 0.9ms/write, 12ms/100-read batch
//! - Improvement: 6.6× read, 2.3× write, 6.5× batch

use codex_core::db::initialize_pool;
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ============================================================================
// Benchmark Setup Helpers
// ============================================================================

/// Create temporary database with schema
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

/// Create connection pool with WAL mode
fn setup_pool(db_path: &Path) -> Pool<SqliteConnectionManager> {
    initialize_pool(db_path, 10).expect("Failed to initialize pool")
}

/// Create single connection with WAL mode
fn setup_single_connection_wal(db_path: &Path) -> Connection {
    let conn = Connection::open(db_path).expect("Failed to open connection");
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA cache_size = -32000;",
    )
    .expect("Failed to set pragmas");
    conn
}

/// Create single connection with DELETE mode (no WAL)
fn setup_single_connection_delete(db_path: &PathBuf) -> Connection {
    let conn = Connection::open(db_path).expect("Failed to open connection");
    conn.execute_batch(
        "PRAGMA journal_mode = DELETE;
         PRAGMA synchronous = FULL;
         PRAGMA cache_size = -32000;",
    )
    .expect("Failed to set pragmas");
    conn
}

/// Insert test data
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
// Benchmark #1: Connection Pool vs Single Connection
// ============================================================================

fn benchmark_connection_pool_vs_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_pool_vs_single");

    // Setup: Create database with test data
    let (_temp_dir, db_path) = setup_temp_db();
    let pool = setup_pool(&db_path);

    // Insert 1000 test records
    {
        let conn = pool.get().expect("Failed to get connection");
        insert_test_data(&conn, 1000);
    }

    // Benchmark: Pooled connection reads
    group.bench_function("pooled_connection_read", |b| {
        b.iter(|| {
            let conn = pool.get().expect("Failed to get connection");
            let mut stmt = conn
                .prepare("SELECT * FROM consensus_runs WHERE spec_id = ?1")
                .expect("Failed to prepare statement");
            let _count = stmt
                .query_map(["SPEC-TEST-050"], |_row| Ok(()))
                .expect("Failed to query")
                .count();
            black_box(_count);
        });
    });

    // Benchmark: Single connection reads (reused connection)
    group.bench_function("single_connection_read", |b| {
        let conn = setup_single_connection_wal(&db_path);
        b.iter(|| {
            let mut stmt = conn
                .prepare("SELECT * FROM consensus_runs WHERE spec_id = ?1")
                .expect("Failed to prepare statement");
            let _count = stmt
                .query_map(["SPEC-TEST-050"], |_row| Ok(()))
                .expect("Failed to query")
                .count();
            black_box(_count);
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark #2: Dual-Write Overhead
// ============================================================================

fn benchmark_dual_write_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("dual_write_overhead");

    // Setup: Two databases (old schema and new schema)
    let (_temp_dir, db_path) = setup_temp_db();
    let (_temp_dir2, db_path2) = setup_temp_db();

    let pool = setup_pool(&db_path);
    let pool2 = setup_pool(&db_path2);

    // Benchmark: Single write (old schema only)
    group.bench_function("single_write", |b| {
        b.iter(|| {
            let conn = pool.get().expect("Failed to get connection");
            conn.execute(
                "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at) VALUES (?1, ?2, ?3, ?4)",
                ("SPEC-TEST-999", "implement", 1, 2000000),
            )
            .expect("Failed to insert");
        });
    });

    // Benchmark: Dual write (old + new schema)
    group.bench_function("dual_write", |b| {
        b.iter(|| {
            // Write to old schema
            let conn = pool.get().expect("Failed to get connection");
            conn.execute(
                "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at) VALUES (?1, ?2, ?3, ?4)",
                ("SPEC-TEST-999", "implement", 1, 2000000),
            )
            .expect("Failed to insert old");

            // Write to new schema
            let conn2 = pool2.get().expect("Failed to get connection");
            conn2
                .execute(
                    "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at) VALUES (?1, ?2, ?3, ?4)",
                    ("SPEC-TEST-999", "implement", 1, 2000000),
                )
                .expect("Failed to insert new");
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark #3: WAL Mode Performance
// ============================================================================

fn benchmark_wal_mode(c: &mut Criterion) {
    let mut group = c.benchmark_group("wal_mode_performance");

    // Setup: Two databases (DELETE mode vs WAL mode)
    let (_temp_dir_delete, db_path_delete) = setup_temp_db();
    let (_temp_dir_wal, db_path_wal) = setup_temp_db();

    let conn_delete = setup_single_connection_delete(&db_path_delete);
    let conn_wal = setup_single_connection_wal(&db_path_wal);

    // Insert test data
    insert_test_data(&conn_delete, 1000);
    insert_test_data(&conn_wal, 1000);

    // Benchmark: DELETE mode read
    group.bench_function("delete_mode_read", |b| {
        b.iter(|| {
            let mut stmt = conn_delete
                .prepare("SELECT * FROM consensus_runs WHERE spec_id = ?1")
                .expect("Failed to prepare");
            let _count = stmt
                .query_map(["SPEC-TEST-050"], |_row| Ok(()))
                .expect("Failed to query")
                .count();
            black_box(_count);
        });
    });

    // Benchmark: WAL mode read
    group.bench_function("wal_mode_read", |b| {
        b.iter(|| {
            let mut stmt = conn_wal
                .prepare("SELECT * FROM consensus_runs WHERE spec_id = ?1")
                .expect("Failed to prepare");
            let _count = stmt
                .query_map(["SPEC-TEST-050"], |_row| Ok(()))
                .expect("Failed to query")
                .count();
            black_box(_count);
        });
    });

    // Benchmark: DELETE mode write
    group.bench_function("delete_mode_write", |b| {
        b.iter(|| {
            conn_delete
                .execute(
                    "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at) VALUES (?1, ?2, ?3, ?4)",
                    ("SPEC-TEST-WRITE", "plan", 1, 3000000),
                )
                .expect("Failed to insert");
        });
    });

    // Benchmark: WAL mode write
    group.bench_function("wal_mode_write", |b| {
        b.iter(|| {
            conn_wal
                .execute(
                    "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at) VALUES (?1, ?2, ?3, ?4)",
                    ("SPEC-TEST-WRITE", "plan", 1, 3000000),
                )
                .expect("Failed to insert");
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark #4: Transaction Performance
// ============================================================================

fn benchmark_transaction_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("transaction_performance");
    group.throughput(Throughput::Elements(100));

    let (_temp_dir, db_path) = setup_temp_db();
    let pool = setup_pool(&db_path);

    // Benchmark: IMMEDIATE transaction (100 inserts)
    group.bench_function("immediate_transaction_batch", |b| {
        b.iter(|| {
            let mut conn = pool.get().expect("Failed to get connection");
            let tx = conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .expect("Failed to start transaction");

            for i in 0..100 {
                tx.execute(
                    "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at) VALUES (?1, ?2, ?3, ?4)",
                    (format!("SPEC-BATCH-{i}"), "validate", 1, 4000000 + i),
                )
                .expect("Failed to insert");
            }

            tx.commit().expect("Failed to commit");
        });
    });

    // Benchmark: DEFERRED transaction (100 inserts)
    group.bench_function("deferred_transaction_batch", |b| {
        b.iter(|| {
            let mut conn = pool.get().expect("Failed to get connection");
            let tx = conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
                .expect("Failed to start transaction");

            for i in 0..100 {
                tx.execute(
                    "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at) VALUES (?1, ?2, ?3, ?4)",
                    (format!("SPEC-BATCH-{i}"), "validate", 1, 5000000 + i),
                )
                .expect("Failed to insert");
            }

            tx.commit().expect("Failed to commit");
        });
    });

    // Benchmark: Individual inserts (no transaction)
    group.bench_function("no_transaction_batch", |b| {
        b.iter(|| {
            let conn = pool.get().expect("Failed to get connection");
            for i in 0..100 {
                conn.execute(
                    "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at) VALUES (?1, ?2, ?3, ?4)",
                    (format!("SPEC-NOTX-{i}"), "audit", 1, 6000000 + i),
                )
                .expect("Failed to insert");
            }
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    benchmark_connection_pool_vs_single,
    benchmark_dual_write_overhead,
    benchmark_wal_mode,
    benchmark_transaction_performance
);
criterion_main!(benches);
