//! Overlay database for Stage0
//!
//! Maintains a SQLite database separate from local-memory that stores:
//! - Dynamic scores and usage tracking for memories
//! - Tier 2 synthesis cache (NotebookLM results)
//! - Cache-to-memory dependency mappings

use crate::config::Stage0Config;
use crate::errors::{Result, Stage0Error};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

/// Embedded schema SQL from STAGE0_SCHEMA.sql
const SCHEMA_SQL: &str = include_str!("../STAGE0_SCHEMA.sql");

/// Structure status for memories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureStatus {
    /// Raw, unstructured content
    Unstructured,
    /// Queued for Template Guardian processing
    Pending,
    /// Successfully restructured
    Structured,
}

impl StructureStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unstructured => "unstructured",
            Self::Pending => "pending",
            Self::Structured => "structured",
        }
    }

    /// Parse from string representation
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "unstructured" => Some(Self::Unstructured),
            "pending" => Some(Self::Pending),
            "structured" => Some(Self::Structured),
            _ => None,
        }
    }
}

/// A row from overlay_memories table
#[derive(Debug, Clone)]
pub struct OverlayMemory {
    pub memory_id: String,
    pub initial_priority: i32,
    pub usage_count: i32,
    pub last_accessed_at: Option<DateTime<Utc>>,
    pub dynamic_score: Option<f64>,
    pub structure_status: Option<StructureStatus>,
    pub content_raw: Option<String>,
}

/// A row from tier2_synthesis_cache table
#[derive(Debug, Clone)]
pub struct Tier2CacheEntry {
    pub input_hash: String,
    pub spec_hash: String,
    pub brief_hash: String,
    pub synthesis_result: String,
    pub suggested_links: Option<String>,
    pub created_at: DateTime<Utc>,
    pub hit_count: i32,
    pub last_hit_at: Option<DateTime<Utc>>,
}

/// Overlay database wrapper
pub struct OverlayDb {
    conn: Connection,
}

impl OverlayDb {
    /// Connect to the overlay database and initialize schema
    ///
    /// Creates the database file if it doesn't exist.
    pub fn connect_and_init(cfg: &Stage0Config) -> Result<Self> {
        let path = cfg.resolved_db_path();
        Self::connect_and_init_at_path(&path)
    }

    /// Connect to a specific database path
    pub fn connect_and_init_at_path(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Stage0Error::overlay_db_with_source(
                    format!("failed to create db directory: {}", parent.display()),
                    e,
                )
            })?;
        }

        let conn = Connection::open(path).map_err(|e| {
            Stage0Error::overlay_db_with_source(
                format!("failed to open db at {}", path.display()),
                e,
            )
        })?;

        Self::apply_schema(&conn)?;

        tracing::debug!(path = %path.display(), "Overlay DB initialized");

        Ok(Self { conn })
    }

    /// Connect to an in-memory database (for testing)
    #[cfg(test)]
    pub fn connect_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to open in-memory db", e))?;

        Self::apply_schema(&conn)?;

        Ok(Self { conn })
    }

    /// Apply the schema to the database
    fn apply_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(SCHEMA_SQL)
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to apply schema", e))?;
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // overlay_memories CRUD
    // ─────────────────────────────────────────────────────────────────────────────

    /// Ensure a memory row exists, creating with defaults if needed
    pub fn ensure_memory_row(&self, memory_id: &str, initial_priority: i32) -> Result<()> {
        self.conn
            .execute(
                r#"
                INSERT OR IGNORE INTO overlay_memories (memory_id, initial_priority, structure_status)
                VALUES (?1, ?2, 'unstructured')
                "#,
                params![memory_id, initial_priority],
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to ensure memory row", e))?;
        Ok(())
    }

    /// Upsert an overlay memory from a GuardedMemory
    ///
    /// Used after guardian processing to record the memory in the overlay.
    /// Creates the row if it doesn't exist, or updates if it does.
    pub fn upsert_overlay_memory(
        &self,
        memory_id: &str,
        kind: crate::guardians::MemoryKind,
        created_at: DateTime<Utc>,
        initial_priority: i32,
        content_raw: &str,
    ) -> Result<()> {
        let created_str = created_at.to_rfc3339();
        let status = StructureStatus::Structured.as_str();

        self.conn
            .execute(
                r#"
                INSERT INTO overlay_memories
                    (memory_id, initial_priority, structure_status, content_raw, last_accessed_at)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(memory_id) DO UPDATE SET
                    initial_priority = ?2,
                    structure_status = ?3,
                    content_raw = ?4,
                    last_accessed_at = ?5
                "#,
                params![
                    memory_id,
                    initial_priority,
                    status,
                    content_raw,
                    created_str
                ],
            )
            .map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to upsert overlay memory", e)
            })?;

        tracing::debug!(
            memory_id = memory_id,
            kind = %kind,
            priority = initial_priority,
            "Upserted overlay memory"
        );

        Ok(())
    }

    /// Get an overlay memory by ID
    pub fn get_memory(&self, memory_id: &str) -> Result<Option<OverlayMemory>> {
        let result = self
            .conn
            .query_row(
                r#"
                SELECT memory_id, initial_priority, usage_count, last_accessed_at,
                       dynamic_score, structure_status, content_raw
                FROM overlay_memories
                WHERE memory_id = ?1
                "#,
                params![memory_id],
                |row| {
                    Ok(OverlayMemory {
                        memory_id: row.get(0)?,
                        initial_priority: row.get(1)?,
                        usage_count: row.get(2)?,
                        last_accessed_at: row.get::<_, Option<String>>(3)?.and_then(|s| {
                            DateTime::parse_from_rfc3339(&s)
                                .ok()
                                .map(|dt| dt.with_timezone(&Utc))
                        }),
                        dynamic_score: row.get(4)?,
                        structure_status: row
                            .get::<_, Option<String>>(5)?
                            .and_then(|s| StructureStatus::parse(&s)),
                        content_raw: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to get memory", e))?;

        Ok(result)
    }

    /// Update dynamic score for a memory
    pub fn update_dynamic_score(&self, memory_id: &str, score: f64) -> Result<()> {
        self.conn
            .execute(
                r#"
                UPDATE overlay_memories
                SET dynamic_score = ?2
                WHERE memory_id = ?1
                "#,
                params![memory_id, score],
            )
            .map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to update dynamic score", e)
            })?;
        Ok(())
    }

    /// Increment usage count and update last_accessed_at
    pub fn record_access(&self, memory_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                r#"
                UPDATE overlay_memories
                SET usage_count = usage_count + 1,
                    last_accessed_at = ?2
                WHERE memory_id = ?1
                "#,
                params![memory_id, now],
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to record access", e))?;
        Ok(())
    }

    /// Update structure status
    pub fn update_structure_status(&self, memory_id: &str, status: StructureStatus) -> Result<()> {
        self.conn
            .execute(
                r#"
                UPDATE overlay_memories
                SET structure_status = ?2
                WHERE memory_id = ?1
                "#,
                params![memory_id, status.as_str()],
            )
            .map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to update structure status", e)
            })?;
        Ok(())
    }

    /// Store raw content for a memory (before structuring)
    pub fn store_content_raw(&self, memory_id: &str, content: &str) -> Result<()> {
        self.conn
            .execute(
                r#"
                UPDATE overlay_memories
                SET content_raw = ?2
                WHERE memory_id = ?1
                "#,
                params![memory_id, content],
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to store raw content", e))?;
        Ok(())
    }

    /// Get all memories ordered by dynamic score (descending)
    pub fn get_memories_by_score(&self, limit: usize) -> Result<Vec<OverlayMemory>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT memory_id, initial_priority, usage_count, last_accessed_at,
                       dynamic_score, structure_status, content_raw
                FROM overlay_memories
                ORDER BY dynamic_score DESC NULLS LAST
                LIMIT ?1
                "#,
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to prepare query", e))?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(OverlayMemory {
                    memory_id: row.get(0)?,
                    initial_priority: row.get(1)?,
                    usage_count: row.get(2)?,
                    last_accessed_at: row.get::<_, Option<String>>(3)?.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    dynamic_score: row.get(4)?,
                    structure_status: row
                        .get::<_, Option<String>>(5)?
                        .and_then(|s| StructureStatus::parse(&s)),
                    content_raw: row.get(6)?,
                })
            })
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to query memories", e))?;

        let mut memories = Vec::new();
        for row in rows {
            memories.push(row.map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to read memory row", e)
            })?);
        }
        Ok(memories)
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // tier2_synthesis_cache CRUD
    // ─────────────────────────────────────────────────────────────────────────────

    /// Look up a Tier 2 cache entry by input hash
    pub fn get_tier2_cache(&self, input_hash: &str) -> Result<Option<Tier2CacheEntry>> {
        let result = self
            .conn
            .query_row(
                r#"
                SELECT input_hash, spec_hash, brief_hash, synthesis_result, suggested_links,
                       created_at, hit_count, last_hit_at
                FROM tier2_synthesis_cache
                WHERE input_hash = ?1
                "#,
                params![input_hash],
                |row| {
                    Ok(Tier2CacheEntry {
                        input_hash: row.get(0)?,
                        spec_hash: row.get(1)?,
                        brief_hash: row.get(2)?,
                        synthesis_result: row.get(3)?,
                        suggested_links: row.get(4)?,
                        created_at: row
                            .get::<_, String>(5)
                            .ok()
                            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(Utc::now),
                        hit_count: row.get(6)?,
                        last_hit_at: row.get::<_, Option<String>>(7)?.and_then(|s| {
                            DateTime::parse_from_rfc3339(&s)
                                .ok()
                                .map(|dt| dt.with_timezone(&Utc))
                        }),
                    })
                },
            )
            .optional()
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to get tier2 cache", e))?;

        Ok(result)
    }

    /// Insert or replace a Tier 2 cache entry
    pub fn upsert_tier2_cache(
        &self,
        input_hash: &str,
        spec_hash: &str,
        brief_hash: &str,
        synthesis_result: &str,
        suggested_links: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                r#"
                INSERT OR REPLACE INTO tier2_synthesis_cache
                (input_hash, spec_hash, brief_hash, synthesis_result, suggested_links, created_at, hit_count, last_hit_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, NULL)
                "#,
                params![input_hash, spec_hash, brief_hash, synthesis_result, suggested_links, now],
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to upsert tier2 cache", e))?;
        Ok(())
    }

    /// Record a cache hit (increment hit_count, update last_hit_at)
    pub fn record_tier2_cache_hit(&self, input_hash: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                r#"
                UPDATE tier2_synthesis_cache
                SET hit_count = hit_count + 1,
                    last_hit_at = ?2
                WHERE input_hash = ?1
                "#,
                params![input_hash, now],
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to record cache hit", e))?;
        Ok(())
    }

    /// Delete stale cache entries older than TTL hours
    pub fn prune_tier2_cache(&self, ttl_hours: u64) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::hours(ttl_hours as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let deleted = self
            .conn
            .execute(
                r#"
                DELETE FROM tier2_synthesis_cache
                WHERE created_at < ?1
                "#,
                params![cutoff_str],
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to prune tier2 cache", e))?;

        Ok(deleted)
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // cache_memory_dependencies CRUD
    // ─────────────────────────────────────────────────────────────────────────────

    /// Add a dependency from a cache entry to a memory
    pub fn add_cache_dependency(&self, cache_hash: &str, memory_id: &str) -> Result<()> {
        self.conn
            .execute(
                r#"
                INSERT OR IGNORE INTO cache_memory_dependencies (cache_hash, memory_id)
                VALUES (?1, ?2)
                "#,
                params![cache_hash, memory_id],
            )
            .map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to add cache dependency", e)
            })?;
        Ok(())
    }

    /// Get all cache hashes that depend on a memory
    pub fn get_dependent_caches(&self, memory_id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT cache_hash
                FROM cache_memory_dependencies
                WHERE memory_id = ?1
                "#,
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to prepare query", e))?;

        let rows = stmt
            .query_map(params![memory_id], |row| row.get(0))
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to query dependencies", e))?;

        let mut hashes = Vec::new();
        for row in rows {
            hashes.push(
                row.map_err(|e| Stage0Error::overlay_db_with_source("failed to read hash", e))?,
            );
        }
        Ok(hashes)
    }

    /// Invalidate cache entries that depend on a memory
    pub fn invalidate_by_memory(&self, memory_id: &str) -> Result<usize> {
        // First get the affected cache hashes
        let hashes = self.get_dependent_caches(memory_id)?;

        if hashes.is_empty() {
            return Ok(0);
        }

        // Delete the cache entries
        let placeholders: Vec<&str> = hashes.iter().map(|_| "?").collect();
        let query = format!(
            "DELETE FROM tier2_synthesis_cache WHERE input_hash IN ({})",
            placeholders.join(", ")
        );

        let mut stmt = self
            .conn
            .prepare(&query)
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to prepare delete", e))?;

        let deleted = stmt
            .execute(rusqlite::params_from_iter(hashes.iter()))
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to invalidate caches", e))?;

        // Clean up the dependency entries
        self.conn
            .execute(
                r#"
                DELETE FROM cache_memory_dependencies
                WHERE memory_id = ?1
                "#,
                params![memory_id],
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to clean dependencies", e))?;

        Ok(deleted)
    }

    /// Get memory count (for metrics/debugging)
    pub fn memory_count(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM overlay_memories", [], |row| {
                row.get(0)
            })
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to count memories", e))
    }

    /// Get cache entry count (for metrics/debugging)
    pub fn cache_count(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM tier2_synthesis_cache", [], |row| {
                row.get(0)
            })
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to count cache entries", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_applies() {
        let db = OverlayDb::connect_in_memory().expect("should connect");
        assert_eq!(db.memory_count().expect("count"), 0);
        assert_eq!(db.cache_count().expect("count"), 0);
    }

    #[test]
    fn test_memory_crud() {
        let db = OverlayDb::connect_in_memory().expect("should connect");

        // Insert
        db.ensure_memory_row("mem-001", 5).expect("insert");
        db.ensure_memory_row("mem-001", 10)
            .expect("insert again should be idempotent");

        // Read
        let mem = db.get_memory("mem-001").expect("get").expect("exists");
        assert_eq!(mem.memory_id, "mem-001");
        assert_eq!(mem.initial_priority, 5); // Should keep original
        assert_eq!(mem.usage_count, 0);
        assert_eq!(mem.structure_status, Some(StructureStatus::Unstructured));

        // Update score
        db.update_dynamic_score("mem-001", 0.85).expect("update");
        let mem = db.get_memory("mem-001").expect("get").expect("exists");
        assert!((mem.dynamic_score.expect("has score") - 0.85).abs() < 0.001);

        // Record access
        db.record_access("mem-001").expect("access");
        let mem = db.get_memory("mem-001").expect("get").expect("exists");
        assert_eq!(mem.usage_count, 1);
        assert!(mem.last_accessed_at.is_some());

        // Update structure status
        db.update_structure_status("mem-001", StructureStatus::Structured)
            .expect("status");
        let mem = db.get_memory("mem-001").expect("get").expect("exists");
        assert_eq!(mem.structure_status, Some(StructureStatus::Structured));
    }

    #[test]
    fn test_tier2_cache_crud() {
        let db = OverlayDb::connect_in_memory().expect("should connect");

        // Insert
        db.upsert_tier2_cache(
            "hash-001",
            "spec-a",
            "brief-b",
            "Divine Truth content",
            Some("[{\"link\":1}]"),
        )
        .expect("upsert");

        // Read
        let entry = db
            .get_tier2_cache("hash-001")
            .expect("get")
            .expect("exists");
        assert_eq!(entry.synthesis_result, "Divine Truth content");
        assert_eq!(entry.hit_count, 0);

        // Record hit
        db.record_tier2_cache_hit("hash-001").expect("hit");
        let entry = db
            .get_tier2_cache("hash-001")
            .expect("get")
            .expect("exists");
        assert_eq!(entry.hit_count, 1);
        assert!(entry.last_hit_at.is_some());

        // Miss
        assert!(db.get_tier2_cache("nonexistent").expect("get").is_none());
    }

    #[test]
    fn test_cache_dependencies() {
        let db = OverlayDb::connect_in_memory().expect("should connect");

        // Setup: cache entry + dependencies
        db.upsert_tier2_cache("cache-001", "spec", "brief", "result", None)
            .expect("upsert");
        db.add_cache_dependency("cache-001", "mem-001")
            .expect("dep");
        db.add_cache_dependency("cache-001", "mem-002")
            .expect("dep");

        // Query dependencies
        let deps = db.get_dependent_caches("mem-001").expect("get");
        assert_eq!(deps, vec!["cache-001"]);

        // Invalidate
        let deleted = db.invalidate_by_memory("mem-001").expect("invalidate");
        assert_eq!(deleted, 1);

        // Cache should be gone
        assert!(db.get_tier2_cache("cache-001").expect("get").is_none());
    }

    #[test]
    fn test_get_memories_by_score() {
        let db = OverlayDb::connect_in_memory().expect("should connect");

        // Insert several memories with different scores
        db.ensure_memory_row("mem-001", 5).expect("insert");
        db.ensure_memory_row("mem-002", 3).expect("insert");
        db.ensure_memory_row("mem-003", 7).expect("insert");

        db.update_dynamic_score("mem-001", 0.5).expect("score");
        db.update_dynamic_score("mem-002", 0.9).expect("score");
        db.update_dynamic_score("mem-003", 0.7).expect("score");

        let mems = db.get_memories_by_score(10).expect("get");
        assert_eq!(mems.len(), 3);
        assert_eq!(mems[0].memory_id, "mem-002"); // highest score
        assert_eq!(mems[1].memory_id, "mem-003");
        assert_eq!(mems[2].memory_id, "mem-001"); // lowest score
    }
}
