//! Async database wrapper for bridging sync SQLite with async Tokio runtime
//!
//! SPEC-945B: Async Integration Patterns (Section 2.3)
//!
//! This module provides async-friendly wrappers for sync SQLite operations using
//! `tokio::task::spawn_blocking` to avoid blocking the async runtime.
//!
//! ## Key Pattern
//! SQLite operations are inherently synchronous, but Tokio runtime is async.
//! We use `spawn_blocking` to run CPU-bound SQLite operations on a dedicated
//! thread pool, preventing blocking of async tasks.
//!
//! ## Usage Example
//! ```rust,no_run
//! use codex_core::db::async_wrapper::with_connection;
//! use codex_core::db::initialize_pool;
//!
//! #[tokio::main]
//! async fn main() -> codex_core::db::Result<()> {
//!     let pool = initialize_pool("test.db")?;
//!
//!     let result = with_connection(&pool, |conn| {
//!         conn.execute("INSERT INTO test (value) VALUES (?1)", [42])?;
//!         Ok(())
//!     }).await?;
//!
//!     Ok(())
//! }
//! ```

use super::{DbError, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

/// Execute sync database operation in async context using spawn_blocking
///
/// This is the core async wrapper that bridges sync SQLite operations with
/// the async Tokio runtime. It uses `tokio::task::spawn_blocking` to run
/// the operation on a dedicated thread pool for blocking operations.
///
/// # SPEC-945B Requirements
/// - Use spawn_blocking for all SQLite operations in async context
/// - Preserve connection pool thread-safety (r2d2 provides this)
/// - Maintain error context across async boundary
///
/// # Arguments
/// * `pool` - Connection pool reference
/// * `f` - Closure that performs sync database operation
///
/// # Returns
/// Result from the database operation, or error with context
///
/// # Example
/// ```rust,no_run
/// # use codex_core::db::async_wrapper::with_connection;
/// # use codex_core::db::initialize_pool;
/// # async fn example() -> codex_core::db::Result<()> {
/// let pool = initialize_pool("test.db")?;
///
/// let count: i64 = with_connection(&pool, |conn| {
///     let mut stmt = conn.prepare("SELECT COUNT(*) FROM users")?;
///     let count: i64 = stmt.query_row([], |row| row.get(0))?;
///     Ok(count)
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub async fn with_connection<F, T>(pool: &Pool<SqliteConnectionManager>, f: F) -> Result<T>
where
    F: FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        // Get connection from pool
        let mut conn = pool
            .get()
            .map_err(|e| DbError::Pool(format!("Failed to get connection: {}", e)))?;

        // Execute operation
        f(&mut conn)
    })
    .await
    .map_err(|e| DbError::Transaction(format!("Task join error: {}", e)))?
}

/// Store consensus run with async wrapper
///
/// High-level async wrapper for storing consensus results using ACID transactions.
/// This demonstrates the pattern for TUI integration.
///
/// # Arguments
/// * `pool` - Connection pool
/// * `spec_id` - SPEC identifier (e.g., "SPEC-KIT-070")
/// * `stage` - Stage name (e.g., "plan", "implement")
/// * `consensus_ok` - Whether consensus was reached
/// * `degraded` - Whether operation was degraded
/// * `synthesis_json` - Optional JSON synthesis result
///
/// # Returns
/// Row ID of inserted/updated consensus run
///
/// # Example
/// ```rust,no_run
/// # use codex_core::db::async_wrapper::store_consensus_run;
/// # use codex_core::db::initialize_pool;
/// # async fn example() -> codex_core::db::Result<()> {
/// let pool = initialize_pool("test.db")?;
///
/// let run_id = store_consensus_run(
///     &pool,
///     "SPEC-KIT-070",
///     "plan",
///     true,
///     false,
///     Some(r#"{"consensus": "approved"}"#),
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn store_consensus_run(
    pool: &Pool<SqliteConnectionManager>,
    spec_id: &str,
    stage: &str,
    consensus_ok: bool,
    degraded: bool,
    synthesis_json: Option<&str>,
) -> Result<i64> {
    let spec_id = spec_id.to_string();
    let stage = stage.to_string();
    let synthesis_json = synthesis_json.map(|s| s.to_string());

    with_connection(pool, move |conn| {
        use crate::db::transactions::{execute_in_transaction, upsert_consensus_run};
        use rusqlite::TransactionBehavior;

        execute_in_transaction(conn, TransactionBehavior::Immediate, |tx| {
            let run_id = upsert_consensus_run(
                tx,
                &spec_id,
                &stage,
                consensus_ok,
                degraded,
                synthesis_json.as_deref(),
            )?;
            Ok(run_id)
        })
    })
    .await
}

/// Store agent output with async wrapper
///
/// Async wrapper for storing individual agent outputs linked to a consensus run.
///
/// # Arguments
/// * `pool` - Connection pool
/// * `run_id` - Consensus run ID (foreign key)
/// * `agent_name` - Agent identifier (e.g., "gemini-flash", "claude-haiku")
/// * `model_version` - Model version string
/// * `content` - Agent output content
///
/// # Returns
/// Row ID of inserted agent output
///
/// # Example
/// ```rust,no_run
/// # use codex_core::db::async_wrapper::store_agent_output;
/// # use codex_core::db::initialize_pool;
/// # async fn example() -> codex_core::db::Result<()> {
/// let pool = initialize_pool("test.db")?;
///
/// let output_id = store_agent_output(
///     &pool,
///     1,
///     "gemini-flash",
///     "gemini-1.5-flash",
///     "Agent analysis output...",
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn store_agent_output(
    pool: &Pool<SqliteConnectionManager>,
    run_id: i64,
    agent_name: &str,
    model_version: Option<&str>,
    content: &str,
) -> Result<i64> {
    let agent_name = agent_name.to_string();
    let model_version = model_version.map(|s| s.to_string());
    let content = content.to_string();

    with_connection(pool, move |conn| {
        use rusqlite::params;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        conn.execute(
            "INSERT INTO agent_outputs (run_id, agent_name, model_version, content, output_timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![run_id, agent_name, model_version, content, timestamp],
        )?;

        let id = conn.last_insert_rowid();
        Ok(id)
    })
    .await
}

/// Query consensus runs with async wrapper
///
/// Async wrapper for querying consensus runs by SPEC ID and stage.
///
/// # Arguments
/// * `pool` - Connection pool
/// * `spec_id` - SPEC identifier filter
/// * `stage` - Optional stage filter
///
/// # Returns
/// Vector of (run_id, timestamp, consensus_ok, degraded, synthesis_json) tuples
///
/// # Example
/// ```rust,no_run
/// # use codex_core::db::async_wrapper::query_consensus_runs;
/// # use codex_core::db::initialize_pool;
/// # async fn example() -> codex_core::db::Result<()> {
/// let pool = initialize_pool("test.db")?;
///
/// let runs = query_consensus_runs(&pool, "SPEC-KIT-070", Some("plan")).await?;
/// for (run_id, timestamp, consensus_ok, degraded, synthesis) in runs {
///     println!("Run {}: consensus={}, degraded={}", run_id, consensus_ok, degraded);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn query_consensus_runs(
    pool: &Pool<SqliteConnectionManager>,
    spec_id: &str,
    stage: Option<&str>,
) -> Result<Vec<(i64, i64, bool, bool, Option<String>)>> {
    let spec_id = spec_id.to_string();
    let stage = stage.map(|s| s.to_string());

    with_connection(pool, move |conn| {
        use rusqlite::params;

        let mut results = Vec::new();

        if let Some(stage_val) = stage {
            let mut stmt = conn.prepare(
                "SELECT id, run_timestamp, consensus_ok, degraded, synthesis_json
                 FROM consensus_runs
                 WHERE spec_id = ?1 AND stage = ?2
                 ORDER BY run_timestamp DESC",
            )?;

            let rows = stmt.query_map(params![spec_id, stage_val], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?;

            for row in rows {
                results.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, run_timestamp, consensus_ok, degraded, synthesis_json
                 FROM consensus_runs
                 WHERE spec_id = ?1
                 ORDER BY run_timestamp DESC",
            )?;

            let rows = stmt.query_map(params![spec_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?;

            for row in rows {
                results.push(row?);
            }
        }

        Ok(results)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::initialize_pool;
    use crate::db::migrations::migrate_to_latest;
    use std::path::Path;

    #[tokio::test]
    async fn test_with_connection_basic() {
        let pool = initialize_pool(Path::new(":memory:"), 1).expect("Pool creation failed");

        // Migrate to create tables
        {
            let mut conn = pool.get().expect("Failed to get connection");
            migrate_to_latest(&mut conn).expect("Migration failed");
        }

        // Test basic operation
        let result: i64 = with_connection(&pool, |conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY)",
                [],
            )?;
            conn.execute("INSERT INTO test DEFAULT VALUES", [])?;
            let id = conn.last_insert_rowid();
            Ok(id)
        })
        .await
        .expect("Operation failed");

        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_with_connection_error_propagation() {
        let pool = initialize_pool(Path::new(":memory:"), 1).expect("Pool creation failed");

        // Test error propagation
        let result: Result<()> = with_connection(&pool, |conn| {
            conn.execute("SELECT * FROM nonexistent_table", [])?;
            Ok(())
        })
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_consensus_run() {
        let pool = initialize_pool(Path::new(":memory:"), 1).expect("Pool creation failed");

        // Migrate to create tables
        {
            let mut conn = pool.get().expect("Failed to get connection");
            migrate_to_latest(&mut conn).expect("Migration failed");
        }

        // Store consensus run
        let run_id = store_consensus_run(
            &pool,
            "SPEC-KIT-070",
            "plan",
            true,
            false,
            Some(r#"{"consensus": "approved"}"#),
        )
        .await
        .expect("Store failed");

        assert!(run_id > 0);

        // Verify it was stored
        let runs = query_consensus_runs(&pool, "SPEC-KIT-070", Some("plan"))
            .await
            .expect("Query failed");

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].0, run_id);
        assert_eq!(runs[0].2, true); // consensus_ok
        assert_eq!(runs[0].3, false); // degraded
    }

    #[tokio::test]
    async fn test_store_agent_output() {
        let pool = initialize_pool(Path::new(":memory:"), 1).expect("Pool creation failed");

        // Migrate and create run
        {
            let mut conn = pool.get().expect("Failed to get connection");
            migrate_to_latest(&mut conn).expect("Migration failed");
        }

        let run_id = store_consensus_run(&pool, "SPEC-KIT-070", "plan", true, false, None)
            .await
            .expect("Store run failed");

        // Store agent output
        let output_id = store_agent_output(
            &pool,
            run_id,
            "gemini-flash",
            Some("gemini-1.5-flash"),
            "Agent analysis output...",
        )
        .await
        .expect("Store output failed");

        assert!(output_id > 0);

        // Verify it was stored
        let count: i64 = with_connection(&pool, move |conn| {
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM agent_outputs WHERE run_id = ?1")?;
            let count: i64 = stmt.query_row([run_id], |row| row.get(0))?;
            Ok(count)
        })
        .await
        .expect("Query failed");

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let pool = initialize_pool(Path::new(":memory:"), 1).expect("Pool creation failed");

        // Migrate
        {
            let mut conn = pool.get().expect("Failed to get connection");
            migrate_to_latest(&mut conn).expect("Migration failed");
        }

        // Spawn 10 concurrent operations
        let mut handles = Vec::new();
        for i in 0..10 {
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move {
                store_consensus_run(
                    &pool_clone,
                    &format!("SPEC-KIT-{:03}", i),
                    "plan",
                    true,
                    false,
                    None,
                )
                .await
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            let result = handle.await.expect("Task panicked");
            assert!(result.is_ok());
        }

        // Verify all were stored
        let count: i64 = with_connection(&pool, |conn| {
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM consensus_runs")?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            Ok(count)
        })
        .await
        .expect("Query failed");

        assert_eq!(count, 10);
    }
}
