//! Integration examples for async database wrapper with TUI
//!
//! SPEC-945B Week 2 Day 3-4: Async Database Wrapper & TUI Integration
//!
//! This module demonstrates integration patterns for using the async database
//! wrapper in TUI components. These examples show:
//! - Consensus artifact storage with transactions
//! - Agent state updates using ACID guarantees
//! - Error handling patterns for async operations
//!
//! ## Integration Strategy
//! The async wrapper is designed to work alongside existing synchronous code.
//! Gradual migration allows testing and validation before full replacement.

use super::{DbError, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::TransactionBehavior;

/// Example: Store consensus run with agent outputs using async wrapper
///
/// Demonstrates ACID transaction pattern for storing consensus results
/// with multiple agent outputs in a single atomic operation.
///
/// # Example Usage in TUI
/// ```rust,no_run
/// # use codex_core::db::integration_examples::store_consensus_with_agents;
/// # use codex_core::db::initialize_pool;
/// # async fn example() -> codex_core::db::Result<()> {
/// let pool = initialize_pool("~/.code/consensus_artifacts.db")?;
///
/// // Agent outputs from multi-agent consensus
/// let agents = vec![
///     ("gemini-flash", "gemini-1.5-flash", "Agent 1 output..."),
///     ("claude-haiku", "claude-3-haiku", "Agent 2 output..."),
///     ("gpt-5-medium", "gpt-5", "Agent 3 output..."),
/// ];
///
/// let run_id = store_consensus_with_agents(
///     &pool,
///     "SPEC-KIT-070",
///     "plan",
///     true,
///     false,
///     Some(r#"{"consensus": "approved", "confidence": 0.95}"#),
///     &agents,
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn store_consensus_with_agents(
    pool: &Pool<SqliteConnectionManager>,
    spec_id: &str,
    stage: &str,
    consensus_ok: bool,
    degraded: bool,
    synthesis_json: Option<&str>,
    agent_outputs: &[(&str, &str, &str)], // (agent_name, model_version, content)
) -> Result<i64> {
    use crate::db::async_wrapper::with_connection;
    use crate::db::transactions::{execute_in_transaction, upsert_consensus_run};

    let spec_id = spec_id.to_string();
    let stage = stage.to_string();
    let synthesis_json = synthesis_json.map(|s| s.to_string());
    let agent_outputs: Vec<(String, String, String)> = agent_outputs
        .iter()
        .map(|(name, version, content)| {
            (name.to_string(), version.to_string(), content.to_string())
        })
        .collect();

    with_connection(pool, move |conn| {
        execute_in_transaction(conn, TransactionBehavior::Immediate, |tx| {
            // 1. Store consensus run
            let run_id = upsert_consensus_run(
                tx,
                &spec_id,
                &stage,
                consensus_ok,
                degraded,
                synthesis_json.as_deref(),
            )?;

            // 2. Store all agent outputs atomically
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            for (agent_name, model_version, content) in agent_outputs {
                tx.execute(
                    "INSERT INTO agent_outputs (run_id, agent_name, model_version, content, output_timestamp)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![run_id, agent_name, model_version, content, timestamp],
                )?;
            }

            Ok(run_id)
        })
    })
    .await
}

/// Example: Update agent state with transaction safety
///
/// Demonstrates atomic agent state update pattern for TUI widgets.
/// Ensures state changes are ACID-compliant and rollback on error.
///
/// # Example Usage in TUI
/// ```rust,no_run
/// # use codex_core::db::integration_examples::update_agent_state;
/// # use codex_core::db::initialize_pool;
/// # async fn example() -> codex_core::db::Result<()> {
/// let pool = initialize_pool("~/.code/consensus_artifacts.db")?;
///
/// // Update agent state atomically
/// update_agent_state(
///     &pool,
///     "SPEC-KIT-070",
///     "plan",
///     "gemini-flash",
///     "running",
///     Some("Processing request..."),
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn update_agent_state(
    pool: &Pool<SqliteConnectionManager>,
    spec_id: &str,
    stage: &str,
    agent_name: &str,
    status: &str,
    status_message: Option<&str>,
) -> Result<()> {
    use crate::db::async_wrapper::with_connection;
    use crate::db::transactions::execute_in_transaction;

    let spec_id = spec_id.to_string();
    let stage = stage.to_string();
    let agent_name = agent_name.to_string();
    let status = status.to_string();
    let status_message = status_message.map(|s| s.to_string());

    with_connection(pool, move |conn| {
        execute_in_transaction(conn, TransactionBehavior::Immediate, |tx| {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            // Create agent_state table if not exists (example schema)
            tx.execute(
                "CREATE TABLE IF NOT EXISTS agent_state (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    spec_id TEXT NOT NULL,
                    stage TEXT NOT NULL,
                    agent_name TEXT NOT NULL,
                    status TEXT NOT NULL,
                    status_message TEXT,
                    updated_at INTEGER NOT NULL,
                    UNIQUE(spec_id, stage, agent_name)
                )",
                [],
            )?;

            // Upsert agent state
            tx.execute(
                "INSERT INTO agent_state (spec_id, stage, agent_name, status, status_message, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(spec_id, stage, agent_name)
                 DO UPDATE SET status = ?4, status_message = ?5, updated_at = ?6",
                rusqlite::params![
                    spec_id,
                    stage,
                    agent_name,
                    status,
                    status_message,
                    timestamp
                ],
            )?;

            Ok(())
        })
    })
    .await
}

/// Example: Query agent states for TUI display
///
/// Demonstrates async query pattern for retrieving agent states.
///
/// # Example Usage in TUI
/// ```rust,no_run
/// # use codex_core::db::integration_examples::query_agent_states;
/// # use codex_core::db::initialize_pool;
/// # async fn example() -> codex_core::db::Result<()> {
/// let pool = initialize_pool("~/.code/consensus_artifacts.db")?;
///
/// let states = query_agent_states(&pool, "SPEC-KIT-070", "plan").await?;
/// for (agent, status, message, updated_at) in states {
///     println!("{}: {} - {:?} ({})", agent, status, message, updated_at);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn query_agent_states(
    pool: &Pool<SqliteConnectionManager>,
    spec_id: &str,
    stage: &str,
) -> Result<Vec<(String, String, Option<String>, i64)>> {
    use crate::db::async_wrapper::with_connection;

    let spec_id = spec_id.to_string();
    let stage = stage.to_string();

    with_connection(pool, move |conn| {
        // Check if table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='agent_state')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !table_exists {
            return Ok(Vec::new());
        }

        let mut stmt = conn.prepare(
            "SELECT agent_name, status, status_message, updated_at
             FROM agent_state
             WHERE spec_id = ?1 AND stage = ?2
             ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map(rusqlite::params![spec_id, stage], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    })
    .await
}

/// Example: Batch store agent outputs (performance optimization)
///
/// Demonstrates batch insert pattern for storing multiple agent outputs
/// in a single transaction for better performance.
///
/// # Example Usage in TUI
/// ```rust,no_run
/// # use codex_core::db::integration_examples::batch_store_agent_outputs;
/// # use codex_core::db::initialize_pool;
/// # async fn example() -> codex_core::db::Result<()> {
/// let pool = initialize_pool("~/.code/consensus_artifacts.db")?;
///
/// let outputs = vec![
///     (1, "gemini-flash", Some("gemini-1.5-flash"), "Output 1"),
///     (1, "claude-haiku", Some("claude-3-haiku"), "Output 2"),
///     (1, "gpt-5-medium", Some("gpt-5"), "Output 3"),
/// ];
///
/// let count = batch_store_agent_outputs(&pool, &outputs).await?;
/// println!("Stored {} agent outputs", count);
/// # Ok(())
/// # }
/// ```
pub async fn batch_store_agent_outputs(
    pool: &Pool<SqliteConnectionManager>,
    outputs: &[(i64, &str, Option<&str>, &str)], // (run_id, agent_name, model_version, content)
) -> Result<usize> {
    use crate::db::async_wrapper::with_connection;
    use crate::db::transactions::batch_insert;

    let outputs: Vec<(i64, String, Option<String>, String)> = outputs
        .iter()
        .map(|(run_id, name, version, content)| {
            (
                *run_id,
                name.to_string(),
                version.map(|s| s.to_string()),
                content.to_string(),
            )
        })
        .collect();

    with_connection(pool, move |conn| {
        batch_insert(
            conn,
            "agent_outputs",
            &["run_id", "agent_name", "model_version", "content", "output_timestamp"],
            &outputs,
            |tx, (run_id, agent_name, model_version, content)| {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;

                tx.execute(
                    "INSERT INTO agent_outputs (run_id, agent_name, model_version, content, output_timestamp)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![run_id, agent_name, model_version, content, timestamp],
                )?;
                Ok(())
            },
        )
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::initialize_pool;
    use crate::db::migrations::migrate_to_latest;

    #[tokio::test]
    async fn test_store_consensus_with_agents() {
        let pool = initialize_pool(":memory:").expect("Pool creation failed");

        // Migrate to create tables
        {
            let mut conn = pool.get().expect("Failed to get connection");
            migrate_to_latest(&mut conn).expect("Migration failed");
        }

        // Store consensus with multiple agents
        let agents = vec![
            ("gemini-flash", "gemini-1.5-flash", "Agent 1 analysis..."),
            ("claude-haiku", "claude-3-haiku", "Agent 2 analysis..."),
            ("gpt-5-medium", "gpt-5", "Agent 3 analysis..."),
        ];

        let run_id = store_consensus_with_agents(
            &pool,
            "SPEC-KIT-070",
            "plan",
            true,
            false,
            Some(r#"{"consensus": "approved"}"#),
            &agents,
        )
        .await
        .expect("Store failed");

        assert!(run_id > 0);

        // Verify all agent outputs were stored
        use crate::db::async_wrapper::with_connection;
        let count: i64 = with_connection(&pool, move |conn| {
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM agent_outputs WHERE run_id = ?1")?;
            let count: i64 = stmt.query_row([run_id], |row| row.get(0))?;
            Ok(count)
        })
        .await
        .expect("Query failed");

        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_update_agent_state() {
        let pool = initialize_pool(":memory:").expect("Pool creation failed");

        // Update agent state
        update_agent_state(
            &pool,
            "SPEC-KIT-070",
            "plan",
            "gemini-flash",
            "running",
            Some("Processing..."),
        )
        .await
        .expect("Update failed");

        // Query agent states
        let states = query_agent_states(&pool, "SPEC-KIT-070", "plan")
            .await
            .expect("Query failed");

        assert_eq!(states.len(), 1);
        assert_eq!(states[0].0, "gemini-flash");
        assert_eq!(states[0].1, "running");
        assert_eq!(states[0].2, Some("Processing...".to_string()));
    }

    #[tokio::test]
    async fn test_batch_store_agent_outputs() {
        let pool = initialize_pool(":memory:").expect("Pool creation failed");

        // Migrate to create tables
        {
            let mut conn = pool.get().expect("Failed to get connection");
            migrate_to_latest(&mut conn).expect("Migration failed");
        }

        // Create a run first
        use crate::db::async_wrapper::store_consensus_run;
        let run_id = store_consensus_run(&pool, "SPEC-KIT-070", "plan", true, false, None)
            .await
            .expect("Store run failed");

        // Batch store outputs
        let outputs = vec![
            (run_id, "agent1", Some("v1"), "output1"),
            (run_id, "agent2", Some("v2"), "output2"),
            (run_id, "agent3", Some("v3"), "output3"),
        ];

        let count = batch_store_agent_outputs(&pool, &outputs)
            .await
            .expect("Batch store failed");

        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_concurrent_agent_updates() {
        let pool = initialize_pool(":memory:").expect("Pool creation failed");

        // Spawn 5 concurrent agent state updates
        let mut handles = Vec::new();
        for i in 0..5 {
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move {
                update_agent_state(
                    &pool_clone,
                    "SPEC-KIT-070",
                    "plan",
                    &format!("agent-{}", i),
                    "completed",
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
        let states = query_agent_states(&pool, "SPEC-KIT-070", "plan")
            .await
            .expect("Query failed");

        assert_eq!(states.len(), 5);
    }
}
