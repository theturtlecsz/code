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

COMMIT;
