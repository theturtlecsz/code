//! Schema versioning and migrations
//!
//! SPEC-945B Component: Migration system

use super::{DbError, Result};
use rusqlite::{Connection, TransactionBehavior};
use tracing::info;

/// Current schema version
pub const SCHEMA_VERSION: i32 = 2;

/// Apply all migrations to bring DB to current version
///
/// # SPEC-945B Requirements:
/// - Forward-only migrations
/// - Version tracking via PRAGMA user_version
/// - Idempotent migrations (IF NOT EXISTS)
/// - Create new tables: consensus_runs, agent_outputs
/// - Preserve existing tables (dual-schema phase)
///
/// # Implementation: Week 1, Day 3-4
pub fn migrate_to_latest(conn: &mut Connection) -> Result<()> {
    let current_version = get_schema_version(conn)?;

    if current_version == SCHEMA_VERSION {
        info!("Schema already at version {}", SCHEMA_VERSION);
        return Ok(());
    }

    if current_version > SCHEMA_VERSION {
        return Err(DbError::Migration(format!(
            "Database schema version {} is newer than application version {}. \
             Please update the application.",
            current_version, SCHEMA_VERSION
        )));
    }

    info!(
        "Migrating schema from version {} to {}",
        current_version, SCHEMA_VERSION
    );

    // Apply migrations sequentially within a transaction
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Exclusive)
        .map_err(|e| DbError::Migration(format!("Failed to begin migration transaction: {}", e)))?;

    for version in (current_version + 1)..=SCHEMA_VERSION {
        info!("Applying migration to version {}", version);
        apply_migration(&tx, version)?;
    }

    // Update schema version
    tx.execute(&format!("PRAGMA user_version = {}", SCHEMA_VERSION), [])
        .map_err(|e| DbError::Migration(format!("Failed to update schema version: {}", e)))?;

    tx.commit()
        .map_err(|e| DbError::Migration(format!("Failed to commit migration: {}", e)))?;

    info!("Schema migration complete: version {}", SCHEMA_VERSION);
    Ok(())
}

/// Get current schema version
///
/// # Implementation: Week 1, Day 3
fn get_schema_version(conn: &Connection) -> Result<i32> {
    let version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| DbError::Migration(format!("Failed to query schema version: {}", e)))?;

    Ok(version)
}

/// Apply specific migration version
///
/// # Implementation: Week 1, Day 3-4
fn apply_migration(conn: &Connection, version: i32) -> Result<()> {
    match version {
        1 => migration_v1(conn),
        2 => migration_v2(conn),
        _ => Err(DbError::Migration(format!(
            "Unknown migration version: {}",
            version
        ))),
    }
}

/// Migration V1: Create new normalized schema
///
/// Creates:
/// - consensus_runs table (workflow orchestration)
/// - agent_outputs table (individual agent results)
/// - Indexes for performance
///
/// # Implementation: Week 1, Day 3-4
fn migration_v1(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        -- Consensus runs table (workflow orchestration)
        CREATE TABLE IF NOT EXISTS consensus_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            spec_id TEXT NOT NULL,
            stage TEXT NOT NULL,
            run_timestamp INTEGER NOT NULL,
            consensus_ok BOOLEAN NOT NULL,
            degraded BOOLEAN DEFAULT 0,
            synthesis_json TEXT,
            UNIQUE(spec_id, stage, run_timestamp)
        );

        -- Agent outputs table (individual agent results)
        CREATE TABLE IF NOT EXISTS agent_outputs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id INTEGER NOT NULL,
            agent_name TEXT NOT NULL,
            model_version TEXT,
            content TEXT NOT NULL,
            output_timestamp INTEGER NOT NULL,
            FOREIGN KEY(run_id) REFERENCES consensus_runs(id) ON DELETE CASCADE
        );

        -- Indexes for performance
        CREATE INDEX IF NOT EXISTS idx_consensus_spec_stage
            ON consensus_runs(spec_id, stage);

        CREATE INDEX IF NOT EXISTS idx_consensus_timestamp
            ON consensus_runs(run_timestamp);

        CREATE INDEX IF NOT EXISTS idx_agent_outputs_run
            ON agent_outputs(run_id);

        CREATE INDEX IF NOT EXISTS idx_agent_outputs_agent
            ON agent_outputs(agent_name);
        ",
    )
    .map_err(|e| DbError::Migration(format!("Failed to execute migration V1: {}", e)))?;

    info!("Migration V1 complete: created consensus_runs and agent_outputs tables");
    Ok(())
}

/// Migration V2: Remove old schema tables
///
/// Drops:
/// - consensus_artifacts table (replaced by agent_outputs)
/// - consensus_synthesis table (replaced by consensus_runs.synthesis_json)
/// - Associated indexes
///
/// # Implementation: SPEC-945B Phase 1 Complete (Week 3)
fn migration_v2(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        -- Drop old schema tables and indexes
        -- These tables were used during dual-schema migration phase
        -- Now deprecated in favor of consensus_runs + agent_outputs

        DROP TABLE IF EXISTS consensus_artifacts;
        DROP TABLE IF EXISTS consensus_synthesis;

        -- Drop old indexes if they exist
        DROP INDEX IF EXISTS idx_spec_stage;
        DROP INDEX IF EXISTS idx_synthesis_spec_stage;
        ",
    )
    .map_err(|e| DbError::Migration(format!("Failed to execute migration V2: {}", e)))?;

    info!(
        "Migration V2 complete: removed old schema tables (consensus_artifacts, consensus_synthesis)"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Helper: Create in-memory database for testing
    fn create_test_db() -> Connection {
        Connection::open_in_memory().expect("Failed to create test database")
    }

    #[test]
    fn test_get_schema_version_default_is_zero() {
        let conn = create_test_db();
        let version = get_schema_version(&conn).expect("Failed to get version");
        assert_eq!(version, 0, "Default schema version should be 0");
    }

    #[test]
    fn test_migration_v1_creates_tables() {
        let conn = create_test_db();

        migration_v1(&conn).expect("Migration V1 failed");

        // Verify consensus_runs table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='consensus_runs'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to query table existence");
        assert_eq!(count, 1, "consensus_runs table should exist");

        // Verify agent_outputs table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='agent_outputs'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to query table existence");
        assert_eq!(count, 1, "agent_outputs table should exist");
    }

    #[test]
    fn test_migration_v1_creates_indexes() {
        let conn = create_test_db();

        migration_v1(&conn).expect("Migration V1 failed");

        // Verify indexes exist
        let index_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count indexes");

        assert_eq!(index_count, 4, "Should create 4 indexes");
    }

    #[test]
    fn test_migration_v1_idempotent() {
        let conn = create_test_db();

        // Run migration twice
        migration_v1(&conn).expect("First migration failed");
        migration_v1(&conn).expect("Second migration should not fail");

        // Verify only one table created (not duplicated)
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='consensus_runs'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to query table count");
        assert_eq!(count, 1, "Should only have one consensus_runs table");
    }

    #[test]
    fn test_migrate_to_latest_from_v0() {
        let mut conn = create_test_db();

        // Initial version should be 0
        let version = get_schema_version(&conn).expect("Failed to get version");
        assert_eq!(version, 0);

        // Run migration
        migrate_to_latest(&mut conn).expect("Migration failed");

        // Verify version updated
        let version = get_schema_version(&conn).expect("Failed to get version");
        assert_eq!(version, SCHEMA_VERSION);

        // Verify tables exist
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND (name='consensus_runs' OR name='agent_outputs')",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count tables");
        assert_eq!(count, 2, "Both tables should exist");
    }

    #[test]
    fn test_migrate_to_latest_idempotent() {
        let mut conn = create_test_db();

        // Run migration twice
        migrate_to_latest(&mut conn).expect("First migration failed");
        migrate_to_latest(&mut conn).expect("Second migration should succeed without changes");

        // Verify version still correct
        let version = get_schema_version(&conn).expect("Failed to get version");
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn test_migrate_rejects_newer_schema() {
        let mut conn = create_test_db();

        // Manually set version higher than current
        conn.execute(&format!("PRAGMA user_version = {}", SCHEMA_VERSION + 1), [])
            .expect("Failed to set version");

        // Migration should fail
        let result = migrate_to_latest(&mut conn);
        assert!(
            result.is_err(),
            "Should reject database with newer schema version"
        );

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("newer than application version"),
            "Error should mention version mismatch"
        );
    }

    #[test]
    fn test_foreign_key_constraint() {
        let mut conn = create_test_db();

        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("Failed to enable foreign keys");

        migrate_to_latest(&mut conn).expect("Migration failed");

        // Insert consensus run
        conn.execute(
            "INSERT INTO consensus_runs (spec_id, stage, run_timestamp, consensus_ok) VALUES (?, ?, ?, ?)",
            ["SPEC-TEST", "plan", "1234567890", "1"],
        )
        .expect("Failed to insert consensus run");

        let run_id: i64 = conn.last_insert_rowid();

        // Insert agent output
        conn.execute(
            "INSERT INTO agent_outputs (run_id, agent_name, content, output_timestamp) VALUES (?, ?, ?, ?)",
            [&run_id.to_string(), "test-agent", "test content", "1234567890"],
        )
        .expect("Failed to insert agent output");

        // Delete consensus run (should cascade)
        conn.execute("DELETE FROM consensus_runs WHERE id = ?", [run_id])
            .expect("Failed to delete consensus run");

        // Verify agent output also deleted
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_outputs WHERE run_id = ?",
                [run_id],
                |row| row.get(0),
            )
            .expect("Failed to count agent outputs");

        assert_eq!(count, 0, "Agent outputs should be deleted via CASCADE");
    }
}
