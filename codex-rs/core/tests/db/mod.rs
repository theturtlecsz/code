//! Database module tests
//!
//! SPEC-945B: Unit and integration tests for database layer

#[cfg(test)]
mod connection_tests {
    // TODO: Unit tests for connection pooling, pragma verification
}

#[cfg(test)]
mod transaction_tests {
    // TODO: Unit tests for ACID transactions, rollback, batch operations
}

#[cfg(test)]
mod migration_tests {
    // Unit tests are in migrations.rs module
}

#[cfg(test)]
mod vacuum_tests {
    // TODO: Unit tests for vacuum scheduling, space reclamation
}

#[cfg(test)]
mod integration_tests {
    use codex_core::db::migrations::{migrate_to_latest, SCHEMA_VERSION};
    use codex_core::db::initialize_pool;
    use rusqlite::Connection;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper: Create temporary database directory
    fn create_temp_db() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join("test.db");
        (temp_dir, db_path)
    }

    #[test]
    fn test_migration_with_real_database_file() {
        let (_temp_dir, db_path) = create_temp_db();

        // Open connection to real file-based database
        let mut conn = Connection::open(&db_path).expect("Failed to create database file");

        // Verify initial version is 0
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("Failed to query version");
        assert_eq!(version, 0, "Initial version should be 0");

        // Run migration
        migrate_to_latest(&mut conn).expect("Migration failed");

        // Verify version updated
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("Failed to query version");
        assert_eq!(version, SCHEMA_VERSION, "Version should be updated");

        // Verify consensus_runs table exists with correct schema
        let table_info: Vec<(String, String)> = conn
            .prepare("PRAGMA table_info(consensus_runs)")
            .expect("Failed to prepare pragma")
            .query_map([], |row| Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
            .expect("Failed to query table info")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect table info");

        assert!(table_info.len() >= 6, "consensus_runs should have at least 6 columns");

        // Verify key columns exist
        let columns: Vec<String> = table_info.iter().map(|(name, _)| name.clone()).collect();
        assert!(columns.contains(&"spec_id".to_string()), "spec_id column missing");
        assert!(columns.contains(&"stage".to_string()), "stage column missing");
        assert!(columns.contains(&"run_timestamp".to_string()), "run_timestamp column missing");
        assert!(columns.contains(&"consensus_ok".to_string()), "consensus_ok column missing");

        // Verify agent_outputs table exists with foreign key
        let table_info: Vec<(String, String)> = conn
            .prepare("PRAGMA table_info(agent_outputs)")
            .expect("Failed to prepare pragma")
            .query_map([], |row| Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
            .expect("Failed to query table info")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect table info");

        assert!(table_info.len() >= 5, "agent_outputs should have at least 5 columns");

        // Verify indexes were created
        let indexes: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%'")
            .expect("Failed to prepare index query")
            .query_map([], |row| row.get(0))
            .expect("Failed to query indexes")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect indexes");

        assert_eq!(indexes.len(), 4, "Should have 4 indexes");
        assert!(indexes.contains(&"idx_consensus_spec_stage".to_string()));
        assert!(indexes.contains(&"idx_consensus_timestamp".to_string()));
        assert!(indexes.contains(&"idx_agent_outputs_run".to_string()));
        assert!(indexes.contains(&"idx_agent_outputs_agent".to_string()));
    }

    #[test]
    fn test_migration_with_connection_pool() {
        let (_temp_dir, db_path) = create_temp_db();

        // Initialize connection pool (from Week 1 Day 1-2)
        let pool = initialize_pool(&db_path, 5).expect("Failed to create pool");

        // Get connection from pool
        let mut conn = pool.get().expect("Failed to get connection from pool");

        // Run migration
        migrate_to_latest(&mut conn).expect("Migration failed");

        // Verify pragmas are still applied (from connection pool)
        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("Failed to query journal_mode");
        assert_eq!(journal_mode, "wal", "Journal mode should be WAL");

        let foreign_keys: i32 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("Failed to query foreign_keys");
        assert_eq!(foreign_keys, 1, "Foreign keys should be enabled");

        // Verify migration succeeded
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("Failed to query version");
        assert_eq!(version, SCHEMA_VERSION);

        // Test basic insert to verify schema works
        conn.execute(
            "INSERT INTO consensus_runs (spec_id, stage, run_timestamp, consensus_ok) VALUES (?, ?, ?, ?)",
            ["TEST-001", "plan", "1699564800000", "1"],
        )
        .expect("Failed to insert test data");

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM consensus_runs", [], |row| row.get(0))
            .expect("Failed to count rows");
        assert_eq!(count, 1, "Should have one row");
    }

    #[test]
    fn test_migration_preserves_existing_tables() {
        let (_temp_dir, db_path) = create_temp_db();
        let mut conn = Connection::open(&db_path).expect("Failed to create database");

        // Create a mock "old" table before migration
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS old_consensus_artifacts (
                id INTEGER PRIMARY KEY,
                data TEXT
            );"
        ).expect("Failed to create old table");

        // Insert test data into old table
        conn.execute(
            "INSERT INTO old_consensus_artifacts (data) VALUES (?)",
            ["test_data"],
        ).expect("Failed to insert into old table");

        // Run migration
        migrate_to_latest(&mut conn).expect("Migration failed");

        // Verify old table still exists and data preserved
        let data: String = conn
            .query_row(
                "SELECT data FROM old_consensus_artifacts WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .expect("Failed to query old table");
        assert_eq!(data, "test_data", "Old table data should be preserved");

        // Verify new tables also exist
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('consensus_runs', 'agent_outputs')",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count new tables");
        assert_eq!(count, 2, "New tables should exist");
    }
}
