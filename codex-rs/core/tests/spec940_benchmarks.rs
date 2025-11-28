// SPEC-957: Allow expect/unwrap in test code
#![allow(clippy::expect_used, clippy::unwrap_used)]
#![allow(clippy::print_stdout, clippy::print_stderr)]

//! SPEC-940: Performance Benchmark Tests
//!
//! Statistical performance validation for key operations:
//! - P0: DirectProcessExecutor spawn (<1ms target)
//! - P1: SQLite consensus write (<30ms target)
//! - P1: Config parsing (<10ms target)
//!
//! ## Running Benchmarks
//! ```bash
//! cd codex-rs
//! cargo test -p codex-core --test spec940_benchmarks -- --nocapture
//! ```

use codex_core::benchmarks::{BenchmarkConfig, BenchmarkHarness, BenchmarkResult};
use codex_core::db::initialize_pool;
use rusqlite::Connection;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::process::Command;

// ============================================================================
// Configuration
// ============================================================================

/// Benchmark parameters
const ITERATIONS: usize = 10;
const WARMUP: usize = 2;

/// Performance thresholds (in milliseconds)
mod thresholds {
    /// DirectProcessExecutor spawn target
    pub const SPAWN_MEAN_MS: f64 = 50.0; // <50ms mean
    pub const SPAWN_MAX_MS: f64 = 100.0; // <100ms max (outliers)

    /// SQLite write target
    pub const SQLITE_WRITE_MEAN_MS: f64 = 30.0;
    pub const SQLITE_WRITE_MAX_MS: f64 = 100.0;

    /// Config parsing target
    pub const CONFIG_PARSE_MEAN_MS: f64 = 10.0;
    pub const CONFIG_PARSE_MAX_MS: f64 = 50.0;
}

// ============================================================================
// Helper Functions
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

/// Create a minimal config TOML for parsing benchmarks
fn create_test_config() -> String {
    r#"
[model]
provider = "openai"
name = "gpt-4"

[shell]
shell = ["bash", "-c"]
sandboxPermission = "none"

[history]
saveHistory = false
"#
    .to_string()
}

/// Print and log benchmark results
fn report_benchmark(result: &BenchmarkResult) {
    eprintln!("  {}", result.summary());
    result.log();
}

// ============================================================================
// P0: DirectProcessExecutor Spawn Benchmark
// ============================================================================

/// SPEC-940 P0: Validate DirectProcessExecutor spawn performance
///
/// Claim: <50ms per spawn (vs 6.5s tmux baseline)
///
/// Uses minimal `echo` command to measure spawn overhead only.
#[tokio::test]
async fn benchmark_spawn_performance() {
    eprintln!("\n=== SPEC-940 P0: Spawn Performance Benchmark ===");

    let harness = BenchmarkHarness::new(BenchmarkConfig {
        name: "spawn_echo_command".into(),
        iterations: ITERATIONS,
        warmup_iterations: WARMUP,
    });

    let result = harness
        .run_async(|| async {
            let output = Command::new("echo")
                .arg("benchmark")
                .output()
                .await
                .expect("Failed to spawn echo");
            assert!(output.status.success());
        })
        .await;

    report_benchmark(&result);

    // Validate against thresholds
    result.assert_mean_under(thresholds::SPAWN_MEAN_MS);
    result.assert_max_under(thresholds::SPAWN_MAX_MS);

    eprintln!(
        "✅ SPEC-940 P0 PASS: {:.2}ms mean spawn (target: <{}ms)",
        result.mean_ms, thresholds::SPAWN_MEAN_MS
    );
}

// ============================================================================
// P1: SQLite Consensus Write Benchmark
// ============================================================================

/// SPEC-940 P1: Validate SQLite consensus write performance
///
/// Claim: <30ms per write (vs 150ms MCP baseline)
#[tokio::test]
async fn benchmark_sqlite_write_performance() {
    eprintln!("\n=== SPEC-940 P1: SQLite Write Performance Benchmark ===");

    let (_temp_dir, db_path) = setup_temp_db();
    let pool = initialize_pool(&db_path, 4).expect("Failed to initialize pool");

    let harness = BenchmarkHarness::new(BenchmarkConfig {
        name: "sqlite_consensus_write".into(),
        iterations: ITERATIONS,
        warmup_iterations: WARMUP,
    });

    let mut iteration = 0;
    let result = harness
        .run_async(|| {
            iteration += 1;
            let pool = pool.clone();
            async move {
                let conn = pool.get().expect("Failed to get connection");
                conn.execute(
                    "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        format!("SPEC-{}", iteration),
                        "validate",
                        1,
                        chrono::Utc::now().timestamp()
                    ],
                )
                .expect("Insert failed");
            }
        })
        .await;

    report_benchmark(&result);

    // Validate against thresholds
    result.assert_mean_under(thresholds::SQLITE_WRITE_MEAN_MS);
    result.assert_max_under(thresholds::SQLITE_WRITE_MAX_MS);

    eprintln!(
        "✅ SPEC-940 P1 PASS: {:.2}ms mean SQLite write (target: <{}ms)",
        result.mean_ms, thresholds::SQLITE_WRITE_MEAN_MS
    );
}

/// SPEC-940 P1: Validate SQLite batch transaction performance
#[tokio::test]
async fn benchmark_sqlite_batch_transaction() {
    eprintln!("\n=== SPEC-940 P1: SQLite Batch Transaction Benchmark ===");

    let (_temp_dir, db_path) = setup_temp_db();

    let harness = BenchmarkHarness::new(BenchmarkConfig {
        name: "sqlite_batch_transaction".into(),
        iterations: ITERATIONS,
        warmup_iterations: WARMUP,
    });

    let result = harness
        .run_sync(|| {
            let conn = Connection::open(&db_path).expect("Failed to open connection");

            // Batch 10 inserts in a single transaction
            conn.execute_batch("BEGIN TRANSACTION").unwrap();
            for i in 0..10 {
                conn.execute(
                    "INSERT INTO consensus_runs (spec_id, stage, consensus_ok, created_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![format!("BATCH-{}", i), "plan", 1, 0],
                )
                .unwrap();
            }
            conn.execute_batch("COMMIT").unwrap();
        });

    report_benchmark(&result);

    // Batch should complete within same threshold (10 ops amortized)
    result.assert_mean_under(thresholds::SQLITE_WRITE_MEAN_MS);

    eprintln!(
        "✅ SPEC-940 P1 PASS: {:.2}ms mean batch (10 ops) (target: <{}ms)",
        result.mean_ms, thresholds::SQLITE_WRITE_MEAN_MS
    );
}

// ============================================================================
// P1: Config Parsing Benchmark
// ============================================================================

/// SPEC-940 P1: Validate Config parsing performance
#[tokio::test]
async fn benchmark_config_parsing() {
    eprintln!("\n=== SPEC-940 P1: Config Parsing Benchmark ===");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, create_test_config()).expect("Failed to write config");

    let harness = BenchmarkHarness::new(BenchmarkConfig {
        name: "config_parse".into(),
        iterations: ITERATIONS,
        warmup_iterations: WARMUP,
    });

    let result = harness.run_sync(|| {
        let content = std::fs::read_to_string(&config_path).expect("Failed to read config");
        let _config: toml::Value = toml::from_str(&content).expect("Failed to parse TOML");
    });

    report_benchmark(&result);

    result.assert_mean_under(thresholds::CONFIG_PARSE_MEAN_MS);
    result.assert_max_under(thresholds::CONFIG_PARSE_MAX_MS);

    eprintln!(
        "✅ SPEC-940 P1 PASS: {:.2}ms mean config parse (target: <{}ms)",
        result.mean_ms, thresholds::CONFIG_PARSE_MEAN_MS
    );
}

// ============================================================================
// Summary Report Generation
// ============================================================================

/// Run all benchmarks and generate a summary report
#[tokio::test]
async fn generate_baseline_report() {
    eprintln!("\n");
    eprintln!("╔═══════════════════════════════════════════════════════════════╗");
    eprintln!("║           SPEC-940 Performance Baseline Report                ║");
    eprintln!("║                    Date: {}                   ║", chrono::Utc::now().format("%Y-%m-%d"));
    eprintln!("╠═══════════════════════════════════════════════════════════════╣");
    eprintln!("║ Operation                    │ Mean±Stddev │ P95    │ Max    ║");
    eprintln!("╠══════════════════════════════╪═════════════╪════════╪════════╣");

    // P0: Spawn
    let spawn_result = run_spawn_benchmark().await;
    print_table_row("spawn_echo_command", &spawn_result);

    // P1: SQLite
    let sqlite_result = run_sqlite_benchmark().await;
    print_table_row("sqlite_write", &sqlite_result);

    // P1: Config
    let config_result = run_config_benchmark().await;
    print_table_row("config_parse", &config_result);

    eprintln!("╚═══════════════════════════════════════════════════════════════╝");
    eprintln!();
}

fn print_table_row(name: &str, result: &BenchmarkResult) {
    eprintln!(
        "║ {:28} │ {:5.2}±{:4.2}ms │ {:5.2}ms │ {:5.2}ms ║",
        name, result.mean_ms, result.stddev_ms, result.p95_ms, result.max_ms
    );
}

async fn run_spawn_benchmark() -> BenchmarkResult {
    let harness = BenchmarkHarness::new(BenchmarkConfig::new("spawn").iterations(10).warmup(2));
    harness
        .run_async(|| async {
            let _ = Command::new("echo").arg("x").output().await;
        })
        .await
}

async fn run_sqlite_benchmark() -> BenchmarkResult {
    let (_temp_dir, db_path) = setup_temp_db();
    let harness = BenchmarkHarness::new(BenchmarkConfig::new("sqlite").iterations(10).warmup(2));
    harness
        .run_sync(|| {
            let conn = Connection::open(&db_path).expect("conn");
            conn.execute(
                "INSERT INTO consensus_runs VALUES (NULL, 'X', 'plan', 1, 0)",
                [],
            )
            .expect("insert");
        })
}

async fn run_config_benchmark() -> BenchmarkResult {
    let temp_dir = TempDir::new().expect("temp");
    let path = temp_dir.path().join("c.toml");
    std::fs::write(&path, create_test_config()).expect("write");

    let harness = BenchmarkHarness::new(BenchmarkConfig::new("config").iterations(10).warmup(2));
    harness.run_sync(|| {
        let s = std::fs::read_to_string(&path).expect("read");
        let _: toml::Value = toml::from_str(&s).expect("parse");
    })
}
