-- STAGE0_SCHEMA.sql
-- Overlay DB schema for SPEC-KIT-102 Stage 0 Engine.
-- This DB is owned by your Rust code (NOT by local-memory).

PRAGMA foreign_keys = ON;

BEGIN TRANSACTION;

-- Overlay table mapping local-memory IDs to dynamic metadata.
CREATE TABLE IF NOT EXISTS overlay_memories (
    memory_id        TEXT PRIMARY KEY,
    initial_priority INTEGER,
    usage_count      INTEGER DEFAULT 0,
    last_accessed_at DATETIME,
    dynamic_score    REAL,
    structure_status TEXT,      -- unstructured | pending | structured
    content_raw      TEXT       -- optional, original unstructured content
);

CREATE INDEX IF NOT EXISTS idx_overlay_dynamic_score
    ON overlay_memories(dynamic_score DESC);

CREATE INDEX IF NOT EXISTS idx_overlay_last_accessed
    ON overlay_memories(last_accessed_at);

-- Tier 2 synthesis cache (NotebookLM)
CREATE TABLE IF NOT EXISTS tier2_synthesis_cache (
    input_hash       TEXT PRIMARY KEY,  -- hash(spec_hash + brief_hash)
    spec_hash        TEXT,
    brief_hash       TEXT,
    synthesis_result TEXT NOT NULL,     -- Divine Truth (Markdown)
    suggested_links  TEXT,              -- JSON array of relationships
    created_at       DATETIME NOT NULL,
    hit_count        INTEGER DEFAULT 0,
    last_hit_at      DATETIME
);

-- Dependency mapping from cache entries to memory IDs
CREATE TABLE IF NOT EXISTS cache_memory_dependencies (
    cache_hash TEXT NOT NULL,
    memory_id  TEXT NOT NULL,
    PRIMARY KEY (cache_hash, memory_id)
);

CREATE INDEX IF NOT EXISTS idx_cache_dependency_memory_id
    ON cache_memory_dependencies(memory_id);

-- P89/SPEC-KIT-105: Constitution metadata and versioning
-- Single-row table (enforced by CHECK constraint) for constitution version tracking
CREATE TABLE IF NOT EXISTS constitution_meta (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    version      INTEGER NOT NULL DEFAULT 1,
    updated_at   DATETIME NOT NULL,
    content_hash TEXT     -- SHA-256 hash of constitution content for drift detection
);

-- Index for efficient version lookups
CREATE INDEX IF NOT EXISTS idx_constitution_version
    ON constitution_meta(version);

-- Initialize constitution_meta if empty (single row table)
INSERT OR IGNORE INTO constitution_meta (id, version, updated_at)
VALUES (1, 0, datetime('now'));

-- ─────────────────────────────────────────────────────────────────────────────
-- SPEC-KIT-103 P98: Librarian Audit Trail Tables
-- ─────────────────────────────────────────────────────────────────────────────

-- Sweep metadata: records each librarian sweep run
CREATE TABLE IF NOT EXISTS librarian_sweeps (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id          TEXT NOT NULL UNIQUE,       -- Format: LRB-YYYYMMDD-NNN
    started_at      TEXT NOT NULL,              -- ISO 8601 timestamp
    finished_at     TEXT,                        -- NULL if running/failed
    args_json       TEXT NOT NULL,              -- Serialized SweepConfig
    stats_json      TEXT,                        -- Serialized SweepSummary
    status          TEXT DEFAULT 'running'       -- running/completed/failed
);

CREATE INDEX IF NOT EXISTS idx_librarian_sweeps_run_id
    ON librarian_sweeps(run_id);

CREATE INDEX IF NOT EXISTS idx_librarian_sweeps_status
    ON librarian_sweeps(status);

-- Per-memory changes: records each memory modification proposed/applied
CREATE TABLE IF NOT EXISTS librarian_changes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    sweep_id        INTEGER NOT NULL REFERENCES librarian_sweeps(id),
    memory_id       TEXT NOT NULL,
    change_type     TEXT NOT NULL,              -- retype/template/both
    old_type        TEXT,                        -- Previous type tag
    new_type        TEXT,                        -- Suggested new type
    old_content     TEXT,                        -- Original content (if templated)
    new_content     TEXT,                        -- Templated content
    confidence      REAL,                        -- Classification confidence
    applied         INTEGER NOT NULL DEFAULT 0,  -- 0=dry-run, 1=applied
    created_at      TEXT NOT NULL               -- ISO 8601 timestamp
);

CREATE INDEX IF NOT EXISTS idx_librarian_changes_sweep_id
    ON librarian_changes(sweep_id);

CREATE INDEX IF NOT EXISTS idx_librarian_changes_memory_id
    ON librarian_changes(memory_id);

-- Causal edges: records relationship edges proposed/created
CREATE TABLE IF NOT EXISTS librarian_edges (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    sweep_id        INTEGER NOT NULL REFERENCES librarian_sweeps(id),
    from_id         TEXT NOT NULL,              -- Source memory ID
    to_id           TEXT NOT NULL,              -- Target memory ID
    relation_type   TEXT NOT NULL,              -- causes/blocks/enables/etc.
    reason          TEXT,                        -- Why this relationship
    applied         INTEGER NOT NULL DEFAULT 0,  -- 0=proposed, 1=created
    created_at      TEXT NOT NULL               -- ISO 8601 timestamp
);

CREATE INDEX IF NOT EXISTS idx_librarian_edges_sweep_id
    ON librarian_edges(sweep_id);

CREATE INDEX IF NOT EXISTS idx_librarian_edges_from_id
    ON librarian_edges(from_id);

CREATE INDEX IF NOT EXISTS idx_librarian_edges_to_id
    ON librarian_edges(to_id);

COMMIT;
