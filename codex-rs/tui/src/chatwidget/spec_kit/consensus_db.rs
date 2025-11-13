//! SQLite-based consensus artifact storage (SPEC-KIT-072)
//!
//! Replaces local-memory MCP for agent outputs and consensus synthesis.
//! Eliminates knowledge base pollution with transient workflow artifacts.
//!
//! Benefits:
//! - Proper data lifecycle (workflow artifacts vs curated knowledge)
//! - Fast SQL queries vs MCP overhead
//! - Schema validation and indexing
//! - No reset conflicts (delete SQLite rows, not memories)
//!
//! SPEC-945B Phase 1 Week 2 Day 3: Dual-Write Implementation
//! Writes to BOTH old schema (consensus_artifacts) and new schema (consensus_runs)
//! for gradual migration without breaking existing functionality.

use codex_core::db::DbError;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, Result as SqlResult, params};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::spec_prompts::SpecStage;

/// Consensus artifact stored in SQLite
#[derive(Debug, Clone)]
pub struct ConsensusArtifact {
    pub id: i64,
    pub spec_id: String,
    pub stage: String,
    pub agent_name: String,
    pub content_json: String,
    pub response_text: Option<String>,
    pub run_id: Option<String>,
    pub created_at: String,
}

/// Thread-safe database connection pool
///
/// SPEC-945B Dual-Write: Contains both old (single connection) and new (connection pool)
/// for gradual migration. During Phase 2 (dual-write), both are active.
pub struct ConsensusDb {
    conn: Arc<Mutex<Connection>>,
    pool: Option<Pool<SqliteConnectionManager>>,
}

impl ConsensusDb {
    /// Initialize database at default location (~/.code/consensus_artifacts.db)
    pub fn init_default() -> SqlResult<Self> {
        let db_path = Self::default_db_path()?;
        Self::init(&db_path)
    }

    /// Get default database path
    fn default_db_path() -> SqlResult<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| {
            rusqlite::Error::InvalidPath("Cannot determine home directory".into())
        })?;
        let db_dir = home.join(".code");
        std::fs::create_dir_all(&db_dir).map_err(|e| {
            rusqlite::Error::InvalidPath(format!("Cannot create .code dir: {}", e).into())
        })?;
        Ok(db_dir.join("consensus_artifacts.db"))
    }

    /// Initialize database at specific path
    ///
    /// SPEC-945B Phase 1 Complete: Uses new schema only (consensus_runs + agent_outputs).
    /// Connection pool with WAL mode provides optimized concurrent access.
    pub fn init(db_path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(db_path)?;

        // Agent execution tracking table (for definitive routing)
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

        // Migrations for existing databases (errors are OK if columns already exist)
        let _ = conn.execute("ALTER TABLE agent_executions ADD COLUMN run_id TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE agent_executions ADD COLUMN extraction_error TEXT",
            [],
        );

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_agent_executions_spec
             ON agent_executions(spec_id, stage)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_agent_executions_run
             ON agent_executions(run_id)",
            [],
        )?;

        // SPEC-945B: Initialize new schema connection pool
        // Use codex_core::db for connection pooling with optimal pragmas
        let pool = Self::initialize_new_schema_pool(db_path);

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            pool,
        })
    }

    /// Initialize connection pool for new schema (SPEC-945B)
    ///
    /// Creates connection pool with WAL mode and optimal pragmas for the new schema.
    /// Returns None if initialization fails (graceful degradation - old schema still works).
    fn initialize_new_schema_pool(db_path: &Path) -> Option<Pool<SqliteConnectionManager>> {
        // Try to initialize pool with codex_core::db
        match codex_core::db::initialize_pool(db_path, 10) {
            Ok(pool) => {
                // Migrate to latest schema (creates new tables if needed)
                match pool.get() {
                    Ok(mut conn) => {
                        if let Err(e) = codex_core::db::migrations::migrate_to_latest(&mut conn) {
                            eprintln!("Warning: Failed to migrate new schema: {}", e);
                            return None;
                        }
                        Some(pool)
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to get connection from pool: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to initialize new schema pool: {}", e);
                None
            }
        }
    }

    /// Store agent artifact (from cached response)
    ///
    /// SPEC-945B Phase 1 Complete: Writes to new schema (consensus_runs + agent_outputs).
    /// Uses connection pool with WAL mode for optimized concurrent access.
    ///
    /// Returns new schema output ID (agent_outputs.id).
    pub fn store_artifact(
        &self,
        spec_id: &str,
        stage: SpecStage,
        agent_name: &str,
        content_json: &str,
        response_text: Option<&str>,
        run_id: Option<&str>,
    ) -> SqlResult<i64> {
        // Ensure connection pool is available
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| rusqlite::Error::InvalidQuery)?;

        // Clone data for move into async closure
        let pool = pool.clone();
        let spec_id = spec_id.to_string();
        let stage = stage.command_name().to_string();
        let agent_name = agent_name.to_string();
        let content_json = content_json.to_string();

        // Write to NEW schema using async wrapper
        // Use Runtime::new() to avoid nested runtime issues
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        runtime
            .block_on(async {
                use codex_core::db::async_wrapper::{store_agent_output, store_consensus_run};

                // 1. Store/update consensus run
                let run_id = store_consensus_run(
                    &pool,
                    &spec_id,
                    &stage,
                    true,  // consensus_ok (artifact exists)
                    false, // degraded
                    None,  // synthesis_json (not available at artifact stage)
                )
                .await
                .map_err(|_| rusqlite::Error::InvalidQuery)?;

                // 2. Store agent output
                let output_id = store_agent_output(
                    &pool,
                    run_id,
                    &agent_name,
                    None, // model_version (not available in old schema)
                    &content_json,
                )
                .await
                .map_err(|_| rusqlite::Error::InvalidQuery)?;

                Ok(output_id)
            })
            .map_err(|_: rusqlite::Error| rusqlite::Error::InvalidQuery)
    }

    // === Read-Path Migration (SPEC-945B Week 2 Day 5) ===

    /// Query artifacts from NEW schema (consensus_runs + agent_outputs)
    ///
    /// Reads from optimized schema with connection pool. Returns artifacts
    /// in the same format as old schema for backward compatibility.
    fn query_artifacts_new_schema(
        &self,
        spec_id: &str,
        stage: &str,
    ) -> Result<Vec<ConsensusArtifact>, String> {
        let pool = match &self.pool {
            Some(p) => p,
            None => return Err("Connection pool not initialized".to_string()),
        };

        let pool = pool.clone();
        let spec_id = spec_id.to_string();
        let stage = stage.to_string();

        // Use Runtime::new() to avoid nested runtime issues
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;

        runtime.block_on(async {
            use codex_core::db::async_wrapper::with_connection;

            with_connection(&pool, move |conn| {
                use rusqlite::params;

                let mut stmt = conn.prepare(
                    "SELECT ao.id, cr.spec_id, cr.stage, ao.agent_name, ao.content,
                            NULL as response_text, NULL as run_id, ao.output_timestamp
                     FROM consensus_runs cr
                     JOIN agent_outputs ao ON cr.id = ao.run_id
                     WHERE cr.spec_id = ?1 AND cr.stage = ?2
                     ORDER BY ao.output_timestamp DESC",
                )?;

                let artifacts = stmt
                    .query_map(params![spec_id, stage], |row| {
                        Ok(ConsensusArtifact {
                            id: row.get(0)?,
                            spec_id: row.get(1)?,
                            stage: row.get(2)?,
                            agent_name: row.get(3)?,
                            content_json: row.get(4)?,
                            response_text: row.get(5)?,
                            run_id: row.get(6)?,
                            created_at: Self::format_timestamp(row.get::<_, i64>(7)?),
                        })
                    })?
                    .collect::<SqlResult<Vec<_>>>()?;

                Ok(artifacts)
            })
            .await
            .map_err(|e| format!("Failed to query new schema: {}", e))
        })
    }

    /// Format Unix timestamp to ISO 8601 string for backward compatibility
    fn format_timestamp(timestamp: i64) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = std::time::Duration::from_secs(timestamp as u64);
        let system_time = UNIX_EPOCH + duration;

        // Format as ISO 8601 (YYYY-MM-DD HH:MM:SS)
        let datetime = chrono::DateTime::<chrono::Utc>::from(system_time);
        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    /// Query synthesis from NEW schema (consensus_runs.synthesis_json)
    ///
    /// Reads from optimized schema with connection pool.
    fn query_synthesis_new_schema(
        &self,
        spec_id: &str,
        stage: &str,
    ) -> Result<Option<String>, String> {
        let pool = match &self.pool {
            Some(p) => p,
            None => return Err("Connection pool not initialized".to_string()),
        };

        let pool = pool.clone();
        let spec_id = spec_id.to_string();
        let stage = stage.to_string();

        // Use Runtime::new() to avoid nested runtime issues
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;

        runtime.block_on(async {
            use codex_core::db::async_wrapper::with_connection;

            with_connection(&pool, move |conn| {
                use rusqlite::params;

                let result = conn.query_row(
                    "SELECT synthesis_json FROM consensus_runs
                     WHERE spec_id = ?1 AND stage = ?2
                     ORDER BY run_timestamp DESC
                     LIMIT 1",
                    params![spec_id, stage],
                    |row| row.get::<_, Option<String>>(0),
                );

                match result {
                    Ok(synthesis) => Ok(synthesis),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(DbError::Sqlite(e)),
                }
            })
            .await
            .map_err(|e| format!("Failed to query new schema: {}", e))
        })
    }

    /// Query artifacts for a spec and stage
    ///
    /// SPEC-945B Phase 1 Complete: Queries new schema only (consensus_runs + agent_outputs).
    /// Uses connection pool with WAL mode for optimized concurrent access.
    pub fn query_artifacts(
        &self,
        spec_id: &str,
        stage: SpecStage,
    ) -> SqlResult<Vec<ConsensusArtifact>> {
        let stage_name = stage.command_name();

        // Query new schema
        self.query_artifacts_new_schema(spec_id, stage_name)
            .map_err(|e| rusqlite::Error::InvalidQuery)
    }

    /// Store consensus synthesis output
    ///
    /// SPEC-945B Phase 1 Complete: Writes to new schema (consensus_runs.synthesis_json).
    /// Uses connection pool with WAL mode for optimized concurrent access.
    ///
    /// Returns new schema run ID (consensus_runs.id).
    pub fn store_synthesis(
        &self,
        spec_id: &str,
        stage: SpecStage,
        output_markdown: &str,
        output_path: Option<&Path>,
        status: &str,
        artifacts_count: usize,
        agreements: Option<&str>,
        conflicts: Option<&str>,
        degraded: bool,
        run_id: Option<&str>,
    ) -> SqlResult<i64> {
        // Ensure connection pool is available
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| rusqlite::Error::InvalidQuery)?;

        // Build synthesis JSON for new schema
        let synthesis_json = serde_json::json!({
            "output_markdown": output_markdown,
            "output_path": output_path.map(|p| p.display().to_string()),
            "status": status,
            "artifacts_count": artifacts_count,
            "agreements": agreements,
            "conflicts": conflicts,
        })
        .to_string();

        // Clone data for move into async closure
        let pool = pool.clone();
        let spec_id = spec_id.to_string();
        let stage = stage.command_name().to_string();

        // Write to NEW schema using async wrapper
        // Use Runtime::new() to avoid nested runtime issues
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        runtime
            .block_on(async {
                use codex_core::db::async_wrapper::store_consensus_run;

                // Store/update consensus run with synthesis
                let run_id = store_consensus_run(
                    &pool,
                    &spec_id,
                    &stage,
                    true, // consensus_ok (synthesis exists)
                    degraded,
                    Some(&synthesis_json),
                )
                .await
                .map_err(|_| rusqlite::Error::InvalidQuery)?;

                Ok(run_id)
            })
            .map_err(|_: rusqlite::Error| rusqlite::Error::InvalidQuery)
    }

    /// Query latest synthesis for a spec and stage
    ///
    /// SPEC-945B Phase 1 Complete: Queries new schema only (consensus_runs.synthesis_json).
    /// Uses connection pool with WAL mode for optimized concurrent access.
    pub fn query_latest_synthesis(
        &self,
        spec_id: &str,
        stage: SpecStage,
    ) -> SqlResult<Option<String>> {
        let stage_name = stage.command_name();

        // Query new schema
        self.query_synthesis_new_schema(spec_id, stage_name)
            .map_err(|e| rusqlite::Error::InvalidQuery)
    }

    // === Agent Execution Tracking (for definitive routing) ===

    /// Record agent spawn (called when agents are launched)
    pub fn record_agent_spawn(
        &self,
        agent_id: &str,
        spec_id: &str,
        stage: SpecStage,
        phase_type: &str, // "quality_gate" | "regular_stage"
        agent_name: &str,
        run_id: Option<&str>,
    ) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO agent_executions
             (agent_id, spec_id, stage, phase_type, agent_name, run_id, spawned_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
            params![
                agent_id,
                spec_id,
                stage.command_name(),
                phase_type,
                agent_name,
                run_id,
            ],
        )?;

        Ok(())
    }

    /// Get agent spawn info (called at completion to route correctly)
    pub fn get_agent_spawn_info(&self, agent_id: &str) -> SqlResult<Option<(String, String)>> {
        let conn = self.conn.lock().unwrap();

        let result = conn.query_row(
            "SELECT phase_type, stage FROM agent_executions WHERE agent_id = ?1",
            params![agent_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        );

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get expected agent name for an agent_id (for collection with correct names)
    pub fn get_agent_name(&self, agent_id: &str) -> SqlResult<Option<String>> {
        let conn = self.conn.lock().unwrap();

        let result = conn.query_row(
            "SELECT agent_name FROM agent_executions WHERE agent_id = ?1",
            params![agent_id],
            |row| row.get::<_, String>(0),
        );

        match result {
            Ok(name) => Ok(Some(name)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Update agent completion info
    pub fn record_agent_completion(&self, agent_id: &str, response_text: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE agent_executions
             SET completed_at = datetime('now'), response_text = ?2
             WHERE agent_id = ?1",
            params![agent_id, response_text],
        )?;

        Ok(())
    }

    /// Record extraction failure with raw output for debugging
    ///
    /// Stores raw agent output even when JSON extraction fails.
    /// Enables post-mortem debugging of malformed output.
    pub fn record_extraction_failure(
        &self,
        agent_id: &str,
        raw_output: &str,
        error: &str,
    ) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE agent_executions
             SET completed_at = datetime('now'),
                 response_text = ?2,
                 extraction_error = ?3
             WHERE agent_id = ?1",
            params![agent_id, raw_output, error],
        )?;

        Ok(())
    }

    /// Query extraction failures for debugging
    ///
    /// Returns (agent_id, agent_name, error, raw_output_preview) for failed extractions.
    pub fn query_extraction_failures(
        &self,
        spec_id: &str,
    ) -> SqlResult<Vec<(String, String, String, String)>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT agent_id, agent_name, extraction_error, substr(response_text, 1, 1000)
             FROM agent_executions
             WHERE spec_id = ?1 AND extraction_error IS NOT NULL
             ORDER BY spawned_at DESC",
        )?;

        let rows = stmt.query_map(params![spec_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Clean up old agent execution records (older than N days)
    pub fn cleanup_old_executions(&self, days: i64) -> SqlResult<usize> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "DELETE FROM agent_executions
             WHERE spawned_at < datetime('now', ?1)",
            params![format!("-{} days", days)],
        )
    }
}

// Unit tests removed - covered by integration tests in tests/read_path_migration.rs
// and tests/write_path_cutover.rs which test the new schema (consensus_runs + agent_outputs)
//
// Old unit tests were testing deprecated functionality:
// - count_artifacts() - removed
// - delete_spec_artifacts() - removed
// - list_specs() - removed
// - get_stats() - removed
//
// Integration tests provide comprehensive coverage of new schema behavior.

#[cfg(test)]
mod tests {
    // Unit tests removed - comprehensive test coverage provided by integration tests:
    // - tests/read_path_migration.rs
    // - tests/write_path_cutover.rs
    //
    // These integration tests cover all new schema functionality (consensus_runs + agent_outputs)
}
