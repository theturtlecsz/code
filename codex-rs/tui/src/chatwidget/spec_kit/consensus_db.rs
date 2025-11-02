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

use rusqlite::{params, Connection, Result as SqlResult};
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
pub struct ConsensusDb {
    conn: Arc<Mutex<Connection>>,
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
    pub fn init(db_path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(db_path)?;

        // Create tables if they don't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS consensus_artifacts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                spec_id TEXT NOT NULL,
                stage TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                content_json TEXT NOT NULL,
                response_text TEXT,
                run_id TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Create index for fast lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_spec_stage
             ON consensus_artifacts(spec_id, stage)",
            [],
        )?;

        // Create synthesis table for storing final outputs
        conn.execute(
            "CREATE TABLE IF NOT EXISTS consensus_synthesis (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                spec_id TEXT NOT NULL,
                stage TEXT NOT NULL,
                output_markdown TEXT NOT NULL,
                output_path TEXT,
                status TEXT NOT NULL,
                artifacts_count INTEGER,
                agreements TEXT,
                conflicts TEXT,
                degraded BOOLEAN DEFAULT 0,
                run_id TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_synthesis_spec_stage
             ON consensus_synthesis(spec_id, stage)",
            [],
        )?;

        // Agent execution tracking table (for definitive routing)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS agent_executions (
                agent_id TEXT PRIMARY KEY,
                spec_id TEXT NOT NULL,
                stage TEXT NOT NULL,
                phase_type TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                spawned_at TEXT NOT NULL,
                completed_at TEXT,
                response_text TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_agent_executions_spec
             ON agent_executions(spec_id, stage)",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Store agent artifact (from cached response)
    pub fn store_artifact(
        &self,
        spec_id: &str,
        stage: SpecStage,
        agent_name: &str,
        content_json: &str,
        response_text: Option<&str>,
        run_id: Option<&str>,
    ) -> SqlResult<i64> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO consensus_artifacts
             (spec_id, stage, agent_name, content_json, response_text, run_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                spec_id,
                stage.command_name(),
                agent_name,
                content_json,
                response_text,
                run_id,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Query artifacts for a spec and stage
    pub fn query_artifacts(
        &self,
        spec_id: &str,
        stage: SpecStage,
    ) -> SqlResult<Vec<ConsensusArtifact>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, spec_id, stage, agent_name, content_json, response_text, run_id, created_at
             FROM consensus_artifacts
             WHERE spec_id = ?1 AND stage = ?2
             ORDER BY created_at DESC",
        )?;

        let artifacts = stmt
            .query_map(params![spec_id, stage.command_name()], |row| {
                Ok(ConsensusArtifact {
                    id: row.get(0)?,
                    spec_id: row.get(1)?,
                    stage: row.get(2)?,
                    agent_name: row.get(3)?,
                    content_json: row.get(4)?,
                    response_text: row.get(5)?,
                    run_id: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(artifacts)
    }

    /// Delete all artifacts for a spec (for reset/cleanup)
    pub fn delete_spec_artifacts(&self, spec_id: &str) -> SqlResult<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM consensus_artifacts WHERE spec_id = ?1",
            params![spec_id],
        )
    }

    /// Delete artifacts for a specific spec and stage
    pub fn delete_stage_artifacts(&self, spec_id: &str, stage: SpecStage) -> SqlResult<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM consensus_artifacts WHERE spec_id = ?1 AND stage = ?2",
            params![spec_id, stage.command_name()],
        )
    }

    /// Get artifact count for a spec (for diagnostics)
    pub fn count_artifacts(&self, spec_id: &str) -> SqlResult<i64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM consensus_artifacts WHERE spec_id = ?1",
            params![spec_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// List all SPECs with artifacts (for cleanup/maintenance)
    pub fn list_specs(&self) -> SqlResult<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT spec_id FROM consensus_artifacts ORDER BY spec_id"
        )?;

        let specs = stmt
            .query_map([], |row| row.get(0))?
            .collect::<SqlResult<Vec<String>>>()?;

        Ok(specs)
    }

    /// Get database statistics (for monitoring)
    pub fn get_stats(&self) -> SqlResult<(i64, i64, i64)> {
        let conn = self.conn.lock().unwrap();

        let artifact_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM consensus_artifacts",
            [],
            |row| row.get(0),
        )?;

        let synthesis_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM consensus_synthesis",
            [],
            |row| row.get(0),
        )?;

        let db_size: i64 = conn.query_row(
            "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        Ok((artifact_count, synthesis_count, db_size))
    }

    /// Store consensus synthesis output
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
        let conn = self.conn.lock().unwrap();

        let path_str = output_path.map(|p| p.display().to_string());

        conn.execute(
            "INSERT INTO consensus_synthesis
             (spec_id, stage, output_markdown, output_path, status, artifacts_count, agreements, conflicts, degraded, run_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                spec_id,
                stage.command_name(),
                output_markdown,
                path_str,
                status,
                artifacts_count as i64,
                agreements,
                conflicts,
                degraded,
                run_id,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Query latest synthesis for a spec and stage
    pub fn query_latest_synthesis(
        &self,
        spec_id: &str,
        stage: SpecStage,
    ) -> SqlResult<Option<String>> {
        let conn = self.conn.lock().unwrap();

        let result = conn.query_row(
            "SELECT output_markdown FROM consensus_synthesis
             WHERE spec_id = ?1 AND stage = ?2
             ORDER BY created_at DESC
             LIMIT 1",
            params![spec_id, stage.command_name()],
            |row| row.get(0),
        );

        match result {
            Ok(markdown) => Ok(Some(markdown)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
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
    ) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO agent_executions
             (agent_id, spec_id, stage, phase_type, agent_name, spawned_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            params![
                agent_id,
                spec_id,
                stage.command_name(),
                phase_type,
                agent_name,
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

    /// Update agent completion info
    pub fn record_agent_completion(
        &self,
        agent_id: &str,
        response_text: &str,
    ) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE agent_executions
             SET completed_at = datetime('now'), response_text = ?2
             WHERE agent_id = ?1",
            params![agent_id, response_text],
        )?;

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_initialization() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("test_consensus.db");
        let _ = std::fs::remove_file(&db_path); // Clean up if exists

        let db = ConsensusDb::init(&db_path).unwrap();
        assert!(db_path.exists());

        // Cleanup
        drop(db);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn test_store_and_query_artifacts() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("test_consensus_query.db");
        let _ = std::fs::remove_file(&db_path);

        let db = ConsensusDb::init(&db_path).unwrap();

        // Store artifact
        let id = db
            .store_artifact(
                "SPEC-TEST-001",
                SpecStage::Plan,
                "gemini",
                r#"{"test":"data"}"#,
                Some("Response text"),
                Some("run_123"),
            )
            .unwrap();

        assert!(id > 0);

        // Query artifacts
        let artifacts = db.query_artifacts("SPEC-TEST-001", SpecStage::Plan).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].agent_name, "gemini");
        assert_eq!(artifacts[0].content_json, r#"{"test":"data"}"#);

        // Count
        let count = db.count_artifacts("SPEC-TEST-001").unwrap();
        assert_eq!(count, 1);

        // Cleanup
        drop(db);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn test_delete_artifacts() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("test_consensus_delete.db");
        let _ = std::fs::remove_file(&db_path);

        let db = ConsensusDb::init(&db_path).unwrap();

        // Store multiple artifacts
        db.store_artifact("SPEC-TEST-002", SpecStage::Plan, "gemini", "{}", None, None)
            .unwrap();
        db.store_artifact("SPEC-TEST-002", SpecStage::Tasks, "claude", "{}", None, None)
            .unwrap();

        // Delete by spec
        let deleted = db.delete_spec_artifacts("SPEC-TEST-002").unwrap();
        assert_eq!(deleted, 2);

        let count = db.count_artifacts("SPEC-TEST-002").unwrap();
        assert_eq!(count, 0);

        // Cleanup
        drop(db);
        let _ = std::fs::remove_file(&db_path);
    }
}
