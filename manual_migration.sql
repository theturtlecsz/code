-- Manual SPEC-945B migration test
-- Run with: sqlite3 ~/.code/consensus_artifacts.db < manual_migration.sql

-- Show current state
.print "=== CURRENT STATE ==="
PRAGMA user_version;
PRAGMA journal_mode;
PRAGMA auto_vacuum;
.print ""

-- Try to enable WAL mode
.print "=== ENABLING WAL MODE ==="
PRAGMA journal_mode = WAL;
.print ""

-- Try to enable auto-vacuum
.print "=== ENABLING AUTO-VACUUM ==="
PRAGMA auto_vacuum = INCREMENTAL;
.print ""

-- Apply other pragmas
.print "=== APPLYING OTHER PRAGMAS ==="
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA cache_size = -32000;
PRAGMA temp_store = MEMORY;
PRAGMA mmap_size = 1073741824;
PRAGMA busy_timeout = 5000;
.print ""

-- Create new schema tables (Migration V1)
.print "=== CREATING NEW SCHEMA TABLES ==="
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

CREATE TABLE IF NOT EXISTS agent_outputs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL,
    agent_name TEXT NOT NULL,
    model_version TEXT,
    content TEXT NOT NULL,
    output_timestamp INTEGER NOT NULL,
    FOREIGN KEY(run_id) REFERENCES consensus_runs(id) ON DELETE CASCADE
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_consensus_spec_stage ON consensus_runs(spec_id, stage);
CREATE INDEX IF NOT EXISTS idx_consensus_timestamp ON consensus_runs(run_timestamp);
CREATE INDEX IF NOT EXISTS idx_agent_outputs_run ON agent_outputs(run_id);
CREATE INDEX IF NOT EXISTS idx_agent_outputs_agent ON agent_outputs(agent_name);

-- Update schema version to 1
PRAGMA user_version = 1;

.print "=== VERIFYING NEW STATE ==="
PRAGMA user_version;
PRAGMA journal_mode;
PRAGMA auto_vacuum;
PRAGMA foreign_keys;

.print ""
.print "=== TABLE LIST ==="
.tables

.print ""
.print "Migration complete!"
