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

// ─────────────────────────────────────────────────────────────────────────────
// P89/SPEC-KIT-105: Constitution Types
// ─────────────────────────────────────────────────────────────────────────────

/// Constitution memory types with associated priority levels
///
/// Used for domain:constitution memories. Each type maps to a specific
/// initial_priority value that ensures constitution content ranks highly
/// in DCC selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstitutionType {
    /// Hard constraints that must never be violated (priority: 10)
    Guardrail,
    /// Architectural values and design principles (priority: 9)
    Principle,
    /// Mid-term objectives and success criteria (priority: 8)
    Goal,
    /// Explicit exclusions - what we don't build (priority: 8)
    NonGoal,
}

impl ConstitutionType {
    /// Get the initial_priority for this constitution type
    ///
    /// Returns:
    /// - Guardrail: 10 (highest - hard constraints)
    /// - Principle: 9 (architectural values)
    /// - Goal/NonGoal: 8 (objectives and exclusions)
    pub fn priority(&self) -> i32 {
        match self {
            Self::Guardrail => 10,
            Self::Principle => 9,
            Self::Goal | Self::NonGoal => 8,
        }
    }

    /// Get the tag string for this constitution type
    ///
    /// Returns tag in format "type:guardrail", "type:principle", etc.
    pub fn as_tag(&self) -> &'static str {
        match self {
            Self::Guardrail => "type:guardrail",
            Self::Principle => "type:principle",
            Self::Goal => "type:goal",
            Self::NonGoal => "type:non-goal",
        }
    }

    /// Parse constitution type from a tag string
    ///
    /// Accepts both "type:guardrail" and "guardrail" formats
    pub fn parse(tag: &str) -> Option<Self> {
        let normalized = tag.strip_prefix("type:").unwrap_or(tag);
        match normalized {
            "guardrail" => Some(Self::Guardrail),
            "principle" => Some(Self::Principle),
            "goal" => Some(Self::Goal),
            "non-goal" | "nongoal" => Some(Self::NonGoal),
            _ => None,
        }
    }
}

/// The domain identifier for constitution memories
pub const CONSTITUTION_DOMAIN: &str = "constitution";

/// Minimum number of constitution memories to always include in TASK_BRIEF
pub const CONSTITUTION_MIN_COUNT: usize = 3;

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

    /// Get memory IDs ordered by dynamic score for vector indexing
    ///
    /// V2.5b: Used by `/stage0.index` to determine which memories to index.
    /// Returns memory IDs ordered by dynamic_score DESC.
    ///
    /// # Arguments
    /// * `max_memories` - Maximum number to return (0 = no limit)
    pub fn get_memory_ids_for_indexing(&self, max_memories: usize) -> Result<Vec<String>> {
        let limit = if max_memories == 0 {
            i64::MAX
        } else {
            max_memories as i64
        };

        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT memory_id
                FROM overlay_memories
                ORDER BY dynamic_score DESC NULLS LAST, initial_priority DESC
                LIMIT ?1
                "#,
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to prepare query", e))?;

        let rows = stmt
            .query_map(params![limit], |row| row.get(0))
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to query memory ids", e))?;

        let mut ids = Vec::new();
        for row in rows {
            ids.push(
                row.map_err(|e| {
                    Stage0Error::overlay_db_with_source("failed to read memory id", e)
                })?,
            );
        }
        Ok(ids)
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
        self.upsert_tier2_cache_at(
            input_hash,
            spec_hash,
            brief_hash,
            synthesis_result,
            suggested_links,
            Utc::now(),
        )
    }

    /// Insert or replace a Tier 2 cache entry with specific created_at timestamp
    ///
    /// Used by tests to control cache timing. For production, use `upsert_tier2_cache`.
    pub fn upsert_tier2_cache_at(
        &self,
        input_hash: &str,
        spec_hash: &str,
        brief_hash: &str,
        synthesis_result: &str,
        suggested_links: Option<&str>,
        created_at: DateTime<Utc>,
    ) -> Result<()> {
        let created_str = created_at.to_rfc3339();
        self.conn
            .execute(
                r#"
                INSERT OR REPLACE INTO tier2_synthesis_cache
                (input_hash, spec_hash, brief_hash, synthesis_result, suggested_links, created_at, hit_count, last_hit_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, NULL)
                "#,
                params![input_hash, spec_hash, brief_hash, synthesis_result, suggested_links, created_str],
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

    /// Get tier2 cache entry with TTL check
    ///
    /// Returns None if:
    /// - Entry doesn't exist
    /// - Entry is older than TTL hours
    ///
    /// Automatically records a cache hit if entry is valid.
    pub fn get_tier2_cache_with_ttl(
        &self,
        input_hash: &str,
        ttl_hours: u64,
        now: DateTime<Utc>,
    ) -> Result<Option<Tier2CacheEntry>> {
        let entry = self.get_tier2_cache(input_hash)?;

        if let Some(ref e) = entry {
            let cutoff = now - chrono::Duration::hours(ttl_hours as i64);
            if e.created_at < cutoff {
                // Entry is stale, treat as cache miss
                tracing::debug!(
                    input_hash = input_hash,
                    created_at = %e.created_at,
                    cutoff = %cutoff,
                    "Tier2 cache entry stale (TTL expired)"
                );
                return Ok(None);
            }

            // Valid entry - record hit
            self.record_tier2_cache_hit(input_hash)?;
        }

        Ok(entry)
    }

    /// Store tier2 cache entry with suggested links
    ///
    /// Links are serialized to JSON for storage.
    pub fn store_tier2_cache_with_links(
        &self,
        input_hash: &str,
        spec_hash: &str,
        brief_hash: &str,
        synthesis_result: &str,
        links: &[crate::tier2::CausalLinkSuggestion],
    ) -> Result<()> {
        let links_json =
            if links.is_empty() {
                None
            } else {
                Some(serde_json::to_string(links).map_err(|e| {
                    Stage0Error::overlay_db_with_source("failed to serialize links", e)
                })?)
            };

        self.upsert_tier2_cache(
            input_hash,
            spec_hash,
            brief_hash,
            synthesis_result,
            links_json.as_deref(),
        )
    }

    /// Store cache dependencies for multiple memories
    pub fn store_cache_dependencies(&self, cache_hash: &str, memory_ids: &[String]) -> Result<()> {
        for memory_id in memory_ids {
            self.add_cache_dependency(cache_hash, memory_id)?;
        }
        Ok(())
    }

    /// Parse suggested links from cached JSON
    pub fn parse_cached_links(json_str: Option<&str>) -> Vec<crate::tier2::CausalLinkSuggestion> {
        match json_str {
            Some(s) if !s.is_empty() => serde_json::from_str(s).unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to parse cached links JSON");
                vec![]
            }),
            _ => vec![],
        }
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

    // ─────────────────────────────────────────────────────────────────────────────
    // V1.3: Scoring-aware usage recording
    // ─────────────────────────────────────────────────────────────────────────────

    /// Record memory usage and recalculate dynamic score
    ///
    /// This is the primary method for DCC integration. When a memory is selected
    /// for inclusion in a TASK_BRIEF, call this to:
    /// 1. Increment usage_count
    /// 2. Update last_accessed_at
    /// 3. Recalculate dynamic_score using the scoring formula
    ///
    /// If the memory doesn't exist in the overlay, it's created with defaults.
    ///
    /// # Arguments
    /// * `memory_id` - The local-memory ID
    /// * `initial_priority` - Priority to use if creating new row (1-10)
    /// * `created_at` - Memory creation time (from local-memory)
    /// * `scoring_config` - Scoring weights and parameters
    pub fn record_memory_usage(
        &self,
        memory_id: &str,
        initial_priority: i32,
        created_at: DateTime<Utc>,
        scoring_config: &crate::config::ScoringConfig,
    ) -> Result<f64> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        // Ensure the row exists
        self.ensure_memory_row(memory_id, initial_priority)?;

        // Get current state
        let mem = self
            .get_memory(memory_id)?
            .ok_or_else(|| Stage0Error::overlay_db("memory row not found after ensure"))?;

        // Calculate new score with incremented usage
        let new_usage_count = mem.usage_count as u32 + 1;
        let scoring_input = crate::scoring::ScoringInput::new(
            new_usage_count,
            mem.initial_priority,
            Some(now), // last_accessed_at will be now
            created_at,
        );
        let new_score = crate::scoring::calculate_score(&scoring_input, scoring_config, now);

        // Atomic update
        self.conn
            .execute(
                r#"
                UPDATE overlay_memories
                SET usage_count = ?2,
                    last_accessed_at = ?3,
                    dynamic_score = ?4
                WHERE memory_id = ?1
                "#,
                params![memory_id, new_usage_count as i32, now_str, new_score],
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to record memory usage", e))?;

        tracing::debug!(
            memory_id = memory_id,
            usage_count = new_usage_count,
            new_score = new_score,
            "Recorded memory usage"
        );

        Ok(new_score)
    }

    /// Batch record usage for multiple memories
    ///
    /// More efficient than calling record_memory_usage in a loop when
    /// multiple memories are selected in a single DCC run.
    ///
    /// Returns a map of memory_id -> new_score.
    pub fn record_batch_usage(
        &self,
        memories: &[(String, i32, DateTime<Utc>)], // (memory_id, priority, created_at)
        scoring_config: &crate::config::ScoringConfig,
    ) -> Result<Vec<(String, f64)>> {
        let mut results = Vec::with_capacity(memories.len());

        for (memory_id, priority, created_at) in memories {
            let score =
                self.record_memory_usage(memory_id, *priority, *created_at, scoring_config)?;
            results.push((memory_id.clone(), score));
        }

        Ok(results)
    }

    /// Recalculate dynamic score for a memory without recording usage
    ///
    /// Useful for background score refresh or when re-scoring after config changes.
    pub fn recalculate_score(
        &self,
        memory_id: &str,
        created_at: DateTime<Utc>,
        scoring_config: &crate::config::ScoringConfig,
    ) -> Result<Option<f64>> {
        let now = Utc::now();

        let Some(mem) = self.get_memory(memory_id)? else {
            return Ok(None);
        };

        let scoring_input = crate::scoring::ScoringInput::new(
            mem.usage_count as u32,
            mem.initial_priority,
            mem.last_accessed_at,
            created_at,
        );
        let new_score = crate::scoring::calculate_score(&scoring_input, scoring_config, now);

        self.update_dynamic_score(memory_id, new_score)?;

        Ok(Some(new_score))
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // P89/SPEC-KIT-105: Constitution Methods
    // ─────────────────────────────────────────────────────────────────────────────

    /// Upsert a constitution memory with type-specific priority
    ///
    /// This is the primary method for storing constitution entries. It:
    /// 1. Uses the ConstitutionType to determine initial_priority
    /// 2. Creates/updates the overlay memory row
    /// 3. Does NOT increment version (caller should use increment_constitution_version)
    ///
    /// # Arguments
    /// * `memory_id` - The local-memory ID
    /// * `constitution_type` - Type determines priority: guardrail=10, principle=9, goal/non-goal=8
    /// * `content` - Raw constitution text content
    pub fn upsert_constitution_memory(
        &self,
        memory_id: &str,
        constitution_type: ConstitutionType,
        content: &str,
    ) -> Result<()> {
        let priority = constitution_type.priority();
        let now = Utc::now().to_rfc3339();
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
                params![memory_id, priority, status, content, now],
            )
            .map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to upsert constitution memory", e)
            })?;

        tracing::debug!(
            memory_id = memory_id,
            constitution_type = ?constitution_type,
            priority = priority,
            "Upserted constitution memory"
        );

        Ok(())
    }

    /// Get the current constitution version
    ///
    /// Returns 0 if no constitution has been defined yet.
    pub fn get_constitution_version(&self) -> Result<u32> {
        let version: i32 = self
            .conn
            .query_row(
                "SELECT version FROM constitution_meta WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to get constitution version", e)
            })?;

        Ok(version as u32)
    }

    /// Increment the constitution version and optionally update the content hash
    ///
    /// Call this after modifying constitution memories (add/update/delete).
    /// Returns the new version number.
    pub fn increment_constitution_version(&self, content_hash: Option<&str>) -> Result<u32> {
        let now = Utc::now().to_rfc3339();

        if let Some(hash) = content_hash {
            self.conn.execute(
                r#"
                UPDATE constitution_meta
                SET version = version + 1,
                    updated_at = ?1,
                    content_hash = ?2
                WHERE id = 1
                "#,
                params![now, hash],
            )
        } else {
            self.conn.execute(
                r#"
                UPDATE constitution_meta
                SET version = version + 1,
                    updated_at = ?1
                WHERE id = 1
                "#,
                params![now],
            )
        }
        .map_err(|e| {
            Stage0Error::overlay_db_with_source("failed to increment constitution version", e)
        })?;

        let new_version = self.get_constitution_version()?;

        tracing::info!(
            version = new_version,
            hash = content_hash,
            "Incremented constitution version"
        );

        Ok(new_version)
    }

    /// Get constitution metadata (version, hash, updated_at)
    ///
    /// Returns (version, content_hash, updated_at)
    pub fn get_constitution_meta(&self) -> Result<(u32, Option<String>, Option<DateTime<Utc>>)> {
        let result = self
            .conn
            .query_row(
                r#"
                SELECT version, content_hash, updated_at
                FROM constitution_meta
                WHERE id = 1
                "#,
                [],
                |row| {
                    let version: i32 = row.get(0)?;
                    let hash: Option<String> = row.get(1)?;
                    let updated_str: Option<String> = row.get(2)?;
                    let updated_at = updated_str.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    });
                    Ok((version as u32, hash, updated_at))
                },
            )
            .map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to get constitution meta", e)
            })?;

        Ok(result)
    }

    /// Get all constitution memories ordered by priority (descending)
    ///
    /// Returns memories that have domain=constitution based on tag pattern.
    /// Used by DCC for the separate always-on constitution pass.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of constitution memories to return
    pub fn get_constitution_memories(&self, limit: usize) -> Result<Vec<OverlayMemory>> {
        // Constitution memories have priority >= 8 (goal/non-goal/principle/guardrail)
        // We identify them by high priority as a proxy for domain
        // Full domain filtering happens in DCC via local-memory search
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT memory_id, initial_priority, usage_count, last_accessed_at,
                       dynamic_score, structure_status, content_raw
                FROM overlay_memories
                WHERE initial_priority >= 8
                ORDER BY initial_priority DESC, dynamic_score DESC NULLS LAST
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
            .map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to query constitution memories", e)
            })?;

        let mut memories = Vec::new();
        for row in rows {
            memories.push(row.map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to read constitution memory row", e)
            })?);
        }
        Ok(memories)
    }

    /// Count constitution memories in the overlay
    ///
    /// Returns count of memories with priority >= 8 (constitution range)
    pub fn constitution_memory_count(&self) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM overlay_memories WHERE initial_priority >= 8",
                [],
                |row| row.get(0),
            )
            .map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to count constitution memories", e)
            })
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Metrics
    // ─────────────────────────────────────────────────────────────────────────────

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

    // V1.3: Scoring tests
    #[test]
    fn test_record_memory_usage() {
        use crate::config::ScoringConfig;

        let db = OverlayDb::connect_in_memory().expect("should connect");
        let config = ScoringConfig::default();
        let created_at = Utc::now();

        // Record usage for a new memory (creates row)
        let score1 = db
            .record_memory_usage("mem-new", 8, created_at, &config)
            .expect("record");
        assert!(score1 > 0.0);

        let mem = db.get_memory("mem-new").expect("get").expect("exists");
        assert_eq!(mem.usage_count, 1);
        assert!(mem.last_accessed_at.is_some());
        assert!(mem.dynamic_score.is_some());

        // Record again - usage should increment
        let score2 = db
            .record_memory_usage("mem-new", 8, created_at, &config)
            .expect("record");
        let mem = db.get_memory("mem-new").expect("get").expect("exists");
        assert_eq!(mem.usage_count, 2);

        // Score should change (typically decrease due to less novelty)
        assert_ne!(score1, score2);
    }

    #[test]
    fn test_record_batch_usage() {
        use crate::config::ScoringConfig;

        let db = OverlayDb::connect_in_memory().expect("should connect");
        let config = ScoringConfig::default();
        let created_at = Utc::now();

        let memories = vec![
            ("mem-a".to_string(), 5, created_at),
            ("mem-b".to_string(), 8, created_at),
            ("mem-c".to_string(), 3, created_at),
        ];

        let results = db.record_batch_usage(&memories, &config).expect("batch");
        assert_eq!(results.len(), 3);

        // All should have scores > 0
        for (_, score) in &results {
            assert!(*score > 0.0);
        }

        // Higher priority should generally yield higher score
        let score_b = results.iter().find(|(id, _)| id == "mem-b").unwrap().1;
        let score_c = results.iter().find(|(id, _)| id == "mem-c").unwrap().1;
        assert!(score_b > score_c);
    }

    #[test]
    fn test_recalculate_score() {
        use crate::config::ScoringConfig;

        let db = OverlayDb::connect_in_memory().expect("should connect");
        let config = ScoringConfig::default();
        let created_at = Utc::now();

        // Setup: create memory with some usage
        db.ensure_memory_row("mem-recalc", 7).expect("insert");
        db.record_access("mem-recalc").expect("access");
        db.record_access("mem-recalc").expect("access");

        // Recalculate
        let new_score = db
            .recalculate_score("mem-recalc", created_at, &config)
            .expect("recalc")
            .expect("exists");

        assert!(new_score > 0.0);

        // Verify stored in DB
        let mem = db.get_memory("mem-recalc").expect("get").expect("exists");
        assert!((mem.dynamic_score.unwrap() - new_score).abs() < 0.0001);
    }

    #[test]
    fn test_recalculate_score_nonexistent() {
        use crate::config::ScoringConfig;

        let db = OverlayDb::connect_in_memory().expect("should connect");
        let config = ScoringConfig::default();

        let result = db
            .recalculate_score("nonexistent", Utc::now(), &config)
            .expect("recalc");

        assert!(result.is_none());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // P84: TTL-aware cache tests with fixed timestamps (no wall-clock dependency)
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_tier2_cache_ttl_with_fixed_timestamps() {
        use chrono::TimeZone;

        let db = OverlayDb::connect_in_memory().expect("should connect");

        // Fixed base time: 2025-01-01T00:00:00Z
        let base_time = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let ttl_hours = 24u64;

        // Insert cache entry with known created_at
        db.upsert_tier2_cache_at(
            "hash-ttl-test",
            "spec",
            "brief",
            "test content",
            None,
            base_time,
        )
        .expect("insert");

        // Case 1: Query at base_time + 23 hours (within TTL) -> expect Some
        let within_ttl = base_time + chrono::Duration::hours(23);
        let result = db
            .get_tier2_cache_with_ttl("hash-ttl-test", ttl_hours, within_ttl)
            .expect("lookup");
        assert!(
            result.is_some(),
            "Entry should be valid 23h after creation (TTL=24h)"
        );

        // Case 2: Query at base_time + 25 hours (past TTL) -> expect None
        let past_ttl = base_time + chrono::Duration::hours(25);
        let result = db
            .get_tier2_cache_with_ttl("hash-ttl-test", ttl_hours, past_ttl)
            .expect("lookup");
        assert!(
            result.is_none(),
            "Entry should be stale 25h after creation (TTL=24h)"
        );

        // Case 3: Query at exact TTL boundary (24h) -> expect Some (boundary is inclusive)
        let at_boundary = base_time + chrono::Duration::hours(24);
        let result = db
            .get_tier2_cache_with_ttl("hash-ttl-test", ttl_hours, at_boundary)
            .expect("lookup");
        assert!(
            result.is_some(),
            "Entry should still be valid at exact TTL boundary"
        );
    }

    #[test]
    fn test_tier2_cache_ttl_hit_count_increment() {
        use chrono::TimeZone;

        let db = OverlayDb::connect_in_memory().expect("should connect");
        let base_time = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();

        // Insert cache entry
        db.upsert_tier2_cache_at("hash-hits", "spec", "brief", "content", None, base_time)
            .expect("insert");

        let query_time = base_time + chrono::Duration::hours(1);

        // First TTL-checked read increments hit_count in DB
        let _entry1 = db
            .get_tier2_cache_with_ttl("hash-hits", 24, query_time)
            .expect("lookup")
            .expect("exists");

        // Verify hit was recorded by querying raw entry
        let raw1 = db
            .get_tier2_cache("hash-hits")
            .expect("lookup")
            .expect("exists");
        assert_eq!(
            raw1.hit_count, 1,
            "First TTL-checked read should record hit"
        );

        // Second read should increment again
        let _entry2 = db
            .get_tier2_cache_with_ttl("hash-hits", 24, query_time)
            .expect("lookup")
            .expect("exists");

        let raw2 = db
            .get_tier2_cache("hash-hits")
            .expect("lookup")
            .expect("exists");
        assert_eq!(
            raw2.hit_count, 2,
            "Second TTL-checked read should record hit"
        );
    }

    #[test]
    fn test_tier2_cache_ttl_nonexistent_returns_none() {
        let db = OverlayDb::connect_in_memory().expect("should connect");
        let now = Utc::now();

        let result = db
            .get_tier2_cache_with_ttl("nonexistent-hash", 24, now)
            .expect("lookup");
        assert!(result.is_none(), "Nonexistent entry should return None");
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // P89/SPEC-KIT-105: Constitution tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_constitution_type_priority() {
        assert_eq!(ConstitutionType::Guardrail.priority(), 10);
        assert_eq!(ConstitutionType::Principle.priority(), 9);
        assert_eq!(ConstitutionType::Goal.priority(), 8);
        assert_eq!(ConstitutionType::NonGoal.priority(), 8);
    }

    #[test]
    fn test_constitution_type_tags() {
        assert_eq!(ConstitutionType::Guardrail.as_tag(), "type:guardrail");
        assert_eq!(ConstitutionType::Principle.as_tag(), "type:principle");
        assert_eq!(ConstitutionType::Goal.as_tag(), "type:goal");
        assert_eq!(ConstitutionType::NonGoal.as_tag(), "type:non-goal");
    }

    #[test]
    fn test_constitution_type_parse() {
        // Full format
        assert_eq!(
            ConstitutionType::parse("type:guardrail"),
            Some(ConstitutionType::Guardrail)
        );
        assert_eq!(
            ConstitutionType::parse("type:principle"),
            Some(ConstitutionType::Principle)
        );
        assert_eq!(
            ConstitutionType::parse("type:goal"),
            Some(ConstitutionType::Goal)
        );
        assert_eq!(
            ConstitutionType::parse("type:non-goal"),
            Some(ConstitutionType::NonGoal)
        );

        // Short format
        assert_eq!(
            ConstitutionType::parse("guardrail"),
            Some(ConstitutionType::Guardrail)
        );
        assert_eq!(
            ConstitutionType::parse("nongoal"),
            Some(ConstitutionType::NonGoal)
        );

        // Unknown
        assert_eq!(ConstitutionType::parse("type:unknown"), None);
        assert_eq!(ConstitutionType::parse("pattern"), None);
    }

    #[test]
    fn test_upsert_constitution_memory() {
        let db = OverlayDb::connect_in_memory().expect("should connect");

        // Insert a guardrail (priority 10)
        db.upsert_constitution_memory(
            "const-001",
            ConstitutionType::Guardrail,
            "Never store secrets in plain text",
        )
        .expect("upsert guardrail");

        let mem = db.get_memory("const-001").expect("get").expect("exists");
        assert_eq!(mem.initial_priority, 10);
        assert_eq!(mem.structure_status, Some(StructureStatus::Structured));

        // Insert a principle (priority 9)
        db.upsert_constitution_memory(
            "const-002",
            ConstitutionType::Principle,
            "Developer ergonomics first",
        )
        .expect("upsert principle");

        let mem = db.get_memory("const-002").expect("get").expect("exists");
        assert_eq!(mem.initial_priority, 9);

        // Insert a goal (priority 8)
        db.upsert_constitution_memory(
            "const-003",
            ConstitutionType::Goal,
            "Support 3 cloud providers by Q3",
        )
        .expect("upsert goal");

        let mem = db.get_memory("const-003").expect("get").expect("exists");
        assert_eq!(mem.initial_priority, 8);
    }

    #[test]
    fn test_constitution_version_starts_at_zero() {
        let db = OverlayDb::connect_in_memory().expect("should connect");

        let version = db.get_constitution_version().expect("get version");
        assert_eq!(version, 0, "Constitution version should start at 0");
    }

    #[test]
    fn test_constitution_version_increment() {
        let db = OverlayDb::connect_in_memory().expect("should connect");

        // Initial version
        let v0 = db.get_constitution_version().expect("get");
        assert_eq!(v0, 0);

        // Increment without hash
        let v1 = db.increment_constitution_version(None).expect("increment");
        assert_eq!(v1, 1);

        // Increment with hash
        let v2 = db
            .increment_constitution_version(Some("sha256:abc123"))
            .expect("increment");
        assert_eq!(v2, 2);

        // Verify stored in DB
        let (version, hash, updated_at) = db.get_constitution_meta().expect("meta");
        assert_eq!(version, 2);
        assert_eq!(hash, Some("sha256:abc123".to_string()));
        assert!(updated_at.is_some());
    }

    #[test]
    fn test_get_constitution_memories() {
        let db = OverlayDb::connect_in_memory().expect("should connect");

        // Insert some constitution memories (high priority)
        db.upsert_constitution_memory("const-g1", ConstitutionType::Guardrail, "Guardrail 1")
            .expect("insert");
        db.upsert_constitution_memory("const-p1", ConstitutionType::Principle, "Principle 1")
            .expect("insert");
        db.upsert_constitution_memory("const-goal", ConstitutionType::Goal, "Goal 1")
            .expect("insert");

        // Insert some regular memories (low priority)
        db.ensure_memory_row("regular-1", 5).expect("insert");
        db.ensure_memory_row("regular-2", 3).expect("insert");

        // Get constitution memories only
        let constitution = db.get_constitution_memories(10).expect("get");

        // Should have 3 constitution memories (priority >= 8)
        assert_eq!(constitution.len(), 3);

        // Should be ordered by priority (guardrail=10, principle=9, goal=8)
        assert_eq!(constitution[0].initial_priority, 10);
        assert_eq!(constitution[1].initial_priority, 9);
        assert_eq!(constitution[2].initial_priority, 8);
    }

    #[test]
    fn test_constitution_memory_count() {
        let db = OverlayDb::connect_in_memory().expect("should connect");

        // No constitution memories initially
        let count = db.constitution_memory_count().expect("count");
        assert_eq!(count, 0);

        // Add constitution memories
        db.upsert_constitution_memory("const-1", ConstitutionType::Guardrail, "G1")
            .expect("insert");
        db.upsert_constitution_memory("const-2", ConstitutionType::Principle, "P1")
            .expect("insert");

        let count = db.constitution_memory_count().expect("count");
        assert_eq!(count, 2);

        // Add regular memory (should not be counted)
        db.ensure_memory_row("regular", 5).expect("insert");

        let count = db.constitution_memory_count().expect("count");
        assert_eq!(count, 2, "Regular memories should not be counted");
    }
}
