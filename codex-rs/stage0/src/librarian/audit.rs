//! Librarian audit trail for tracking sweep operations
//!
//! SPEC-KIT-103 P98: SQLite-backed audit trail for all librarian sweeps.
//!
//! ## Tables
//!
//! - `librarian_sweeps`: Metadata for each sweep run
//! - `librarian_changes`: Per-memory changes (retype/template)
//! - `librarian_edges`: Causal relationship edges
//!
//! ## Usage
//!
//! ```rust,ignore
//! let db = OverlayDb::connect_and_init(&config)?;
//! let audit = LibrarianAudit::new(&db);
//!
//! let sweep_id = audit.start_sweep("LRB-20251202-001", &sweep_config)?;
//! audit.log_change(sweep_id, &change)?;
//! audit.complete_sweep(sweep_id, &summary)?;
//! ```

use crate::errors::{Result, Stage0Error};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

/// Status of a librarian sweep
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SweepStatus {
    Running,
    Completed,
    Failed,
}

impl SweepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// Type of change applied to a memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Retype,
    Template,
    Both,
}

impl ChangeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Retype => "retype",
            Self::Template => "template",
            Self::Both => "both",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "retype" => Some(Self::Retype),
            "template" => Some(Self::Template),
            "both" => Some(Self::Both),
            _ => None,
        }
    }
}

/// A recorded sweep from the audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepRecord {
    pub id: i64,
    pub run_id: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub args_json: String,
    pub stats_json: Option<String>,
    pub status: SweepStatus,
}

/// A recorded change from the audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub id: i64,
    pub sweep_id: i64,
    pub memory_id: String,
    pub change_type: ChangeType,
    pub old_type: Option<String>,
    pub new_type: Option<String>,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub confidence: Option<f64>,
    pub applied: bool,
    pub created_at: DateTime<Utc>,
}

/// A recorded causal edge from the audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeRecord {
    pub id: i64,
    pub sweep_id: i64,
    pub from_id: String,
    pub to_id: String,
    pub relation_type: String,
    pub reason: Option<String>,
    pub applied: bool,
    pub created_at: DateTime<Utc>,
}

/// Input for logging a memory change
#[derive(Debug, Clone)]
pub struct ChangeInput {
    pub memory_id: String,
    pub change_type: ChangeType,
    pub old_type: Option<String>,
    pub new_type: Option<String>,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub confidence: Option<f64>,
    pub applied: bool,
}

/// Input for logging a causal edge
#[derive(Debug, Clone)]
pub struct EdgeInput {
    pub from_id: String,
    pub to_id: String,
    pub relation_type: String,
    pub reason: Option<String>,
    pub applied: bool,
}

/// Librarian audit trail operations
///
/// Wraps the SQLite connection and provides methods for audit trail CRUD.
pub struct LibrarianAudit<'a> {
    conn: &'a Connection,
}

impl<'a> LibrarianAudit<'a> {
    /// Create a new audit trail wrapper
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Sweep operations
    // ─────────────────────────────────────────────────────────────────────────

    /// Start a new sweep and return its database ID
    ///
    /// # Arguments
    /// * `run_id` - Unique run identifier (e.g., "LRB-20251202-001")
    /// * `args_json` - Serialized SweepConfig
    pub fn start_sweep(&self, run_id: &str, args_json: &str) -> Result<i64> {
        let now = Utc::now().to_rfc3339();

        self.conn
            .execute(
                r#"
                INSERT INTO librarian_sweeps (run_id, started_at, args_json, status)
                VALUES (?1, ?2, ?3, 'running')
                "#,
                params![run_id, now, args_json],
            )
            .map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to start librarian sweep", e)
            })?;

        let sweep_id = self.conn.last_insert_rowid();

        tracing::info!(
            run_id = run_id,
            sweep_id = sweep_id,
            "Started librarian sweep"
        );

        Ok(sweep_id)
    }

    /// Complete a sweep with summary statistics
    pub fn complete_sweep(&self, sweep_id: i64, stats_json: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        self.conn
            .execute(
                r#"
                UPDATE librarian_sweeps
                SET finished_at = ?2,
                    stats_json = ?3,
                    status = 'completed'
                WHERE id = ?1
                "#,
                params![sweep_id, now, stats_json],
            )
            .map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to complete librarian sweep", e)
            })?;

        tracing::info!(sweep_id = sweep_id, "Completed librarian sweep");

        Ok(())
    }

    /// Mark a sweep as failed
    pub fn fail_sweep(&self, sweep_id: i64, error: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        // Store error in stats_json as JSON
        let error_json = serde_json::json!({ "error": error }).to_string();

        self.conn
            .execute(
                r#"
                UPDATE librarian_sweeps
                SET finished_at = ?2,
                    stats_json = ?3,
                    status = 'failed'
                WHERE id = ?1
                "#,
                params![sweep_id, now, error_json],
            )
            .map_err(|e| {
                Stage0Error::overlay_db_with_source("failed to mark sweep as failed", e)
            })?;

        tracing::warn!(sweep_id = sweep_id, error = error, "Librarian sweep failed");

        Ok(())
    }

    /// Get a sweep by run_id
    pub fn get_sweep_by_run_id(&self, run_id: &str) -> Result<Option<SweepRecord>> {
        self.conn
            .query_row(
                r#"
                SELECT id, run_id, started_at, finished_at, args_json, stats_json, status
                FROM librarian_sweeps
                WHERE run_id = ?1
                "#,
                params![run_id],
                |row| {
                    Ok(SweepRecord {
                        id: row.get(0)?,
                        run_id: row.get(1)?,
                        started_at: parse_datetime(row.get::<_, String>(2)?),
                        finished_at: row.get::<_, Option<String>>(3)?.map(parse_datetime),
                        args_json: row.get(4)?,
                        stats_json: row.get(5)?,
                        status: SweepStatus::parse(&row.get::<_, String>(6)?)
                            .unwrap_or(SweepStatus::Running),
                    })
                },
            )
            .optional()
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to get sweep", e))
    }

    /// Get a sweep by database ID
    pub fn get_sweep(&self, sweep_id: i64) -> Result<Option<SweepRecord>> {
        self.conn
            .query_row(
                r#"
                SELECT id, run_id, started_at, finished_at, args_json, stats_json, status
                FROM librarian_sweeps
                WHERE id = ?1
                "#,
                params![sweep_id],
                |row| {
                    Ok(SweepRecord {
                        id: row.get(0)?,
                        run_id: row.get(1)?,
                        started_at: parse_datetime(row.get::<_, String>(2)?),
                        finished_at: row.get::<_, Option<String>>(3)?.map(parse_datetime),
                        args_json: row.get(4)?,
                        stats_json: row.get(5)?,
                        status: SweepStatus::parse(&row.get::<_, String>(6)?)
                            .unwrap_or(SweepStatus::Running),
                    })
                },
            )
            .optional()
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to get sweep", e))
    }

    /// List recent sweeps
    pub fn list_sweeps(&self, limit: usize) -> Result<Vec<SweepRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT id, run_id, started_at, finished_at, args_json, stats_json, status
                FROM librarian_sweeps
                ORDER BY started_at DESC
                LIMIT ?1
                "#,
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to prepare query", e))?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(SweepRecord {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    started_at: parse_datetime(row.get::<_, String>(2)?),
                    finished_at: row.get::<_, Option<String>>(3)?.map(parse_datetime),
                    args_json: row.get(4)?,
                    stats_json: row.get(5)?,
                    status: SweepStatus::parse(&row.get::<_, String>(6)?)
                        .unwrap_or(SweepStatus::Running),
                })
            })
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to query sweeps", e))?;

        let mut sweeps = Vec::new();
        for row in rows {
            sweeps.push(
                row.map_err(|e| Stage0Error::overlay_db_with_source("failed to read sweep", e))?,
            );
        }
        Ok(sweeps)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Change operations
    // ─────────────────────────────────────────────────────────────────────────

    /// Log a memory change
    pub fn log_change(&self, sweep_id: i64, input: &ChangeInput) -> Result<i64> {
        let now = Utc::now().to_rfc3339();

        self.conn
            .execute(
                r#"
                INSERT INTO librarian_changes
                    (sweep_id, memory_id, change_type, old_type, new_type,
                     old_content, new_content, confidence, applied, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                params![
                    sweep_id,
                    input.memory_id,
                    input.change_type.as_str(),
                    input.old_type,
                    input.new_type,
                    input.old_content,
                    input.new_content,
                    input.confidence,
                    if input.applied { 1 } else { 0 },
                    now,
                ],
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to log change", e))?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get changes for a sweep
    pub fn get_changes(&self, sweep_id: i64) -> Result<Vec<ChangeRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT id, sweep_id, memory_id, change_type, old_type, new_type,
                       old_content, new_content, confidence, applied, created_at
                FROM librarian_changes
                WHERE sweep_id = ?1
                ORDER BY id
                "#,
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to prepare query", e))?;

        let rows = stmt
            .query_map(params![sweep_id], |row| {
                Ok(ChangeRecord {
                    id: row.get(0)?,
                    sweep_id: row.get(1)?,
                    memory_id: row.get(2)?,
                    change_type: ChangeType::parse(&row.get::<_, String>(3)?)
                        .unwrap_or(ChangeType::Retype),
                    old_type: row.get(4)?,
                    new_type: row.get(5)?,
                    old_content: row.get(6)?,
                    new_content: row.get(7)?,
                    confidence: row.get(8)?,
                    applied: row.get::<_, i32>(9)? != 0,
                    created_at: parse_datetime(row.get::<_, String>(10)?),
                })
            })
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to query changes", e))?;

        let mut changes = Vec::new();
        for row in rows {
            changes.push(
                row.map_err(|e| Stage0Error::overlay_db_with_source("failed to read change", e))?,
            );
        }
        Ok(changes)
    }

    /// Count changes for a sweep
    pub fn count_changes(&self, sweep_id: i64) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM librarian_changes WHERE sweep_id = ?1",
                params![sweep_id],
                |row| row.get(0),
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to count changes", e))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Edge operations
    // ─────────────────────────────────────────────────────────────────────────

    /// Log a causal edge
    pub fn log_edge(&self, sweep_id: i64, input: &EdgeInput) -> Result<i64> {
        let now = Utc::now().to_rfc3339();

        self.conn
            .execute(
                r#"
                INSERT INTO librarian_edges
                    (sweep_id, from_id, to_id, relation_type, reason, applied, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    sweep_id,
                    input.from_id,
                    input.to_id,
                    input.relation_type,
                    input.reason,
                    if input.applied { 1 } else { 0 },
                    now,
                ],
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to log edge", e))?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get edges for a sweep
    pub fn get_edges(&self, sweep_id: i64) -> Result<Vec<EdgeRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT id, sweep_id, from_id, to_id, relation_type, reason, applied, created_at
                FROM librarian_edges
                WHERE sweep_id = ?1
                ORDER BY id
                "#,
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to prepare query", e))?;

        let rows = stmt
            .query_map(params![sweep_id], |row| {
                Ok(EdgeRecord {
                    id: row.get(0)?,
                    sweep_id: row.get(1)?,
                    from_id: row.get(2)?,
                    to_id: row.get(3)?,
                    relation_type: row.get(4)?,
                    reason: row.get(5)?,
                    applied: row.get::<_, i32>(6)? != 0,
                    created_at: parse_datetime(row.get::<_, String>(7)?),
                })
            })
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to query edges", e))?;

        let mut edges = Vec::new();
        for row in rows {
            edges.push(
                row.map_err(|e| Stage0Error::overlay_db_with_source("failed to read edge", e))?,
            );
        }
        Ok(edges)
    }

    /// Count edges for a sweep
    pub fn count_edges(&self, sweep_id: i64) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM librarian_edges WHERE sweep_id = ?1",
                params![sweep_id],
                |row| row.get(0),
            )
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to count edges", e))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Run ID generation
    // ─────────────────────────────────────────────────────────────────────────

    /// Generate a unique run ID for today
    ///
    /// Format: LRB-YYYYMMDD-NNN where NNN is a sequential number
    pub fn generate_run_id(&self) -> Result<String> {
        let today = Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("LRB-{today}-");

        // Find the highest sequence number for today
        let max_seq: Option<i32> = self
            .conn
            .query_row(
                r#"
                SELECT MAX(CAST(SUBSTR(run_id, 14) AS INTEGER))
                FROM librarian_sweeps
                WHERE run_id LIKE ?1
                "#,
                params![format!("{}%", prefix)],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| Stage0Error::overlay_db_with_source("failed to query run_id", e))?
            .flatten();

        let next_seq = max_seq.unwrap_or(0) + 1;
        Ok(format!("{prefix}{next_seq:03}"))
    }
}

/// Parse RFC3339 datetime string, falling back to current time on error
fn parse_datetime(s: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open");
        // Apply the schema
        let schema = include_str!("../../STAGE0_SCHEMA.sql");
        conn.execute_batch(schema).expect("schema");
        conn
    }

    #[test]
    fn test_sweep_lifecycle() {
        let conn = setup_db();
        let audit = LibrarianAudit::new(&conn);

        // Start sweep
        let sweep_id = audit
            .start_sweep("LRB-20251202-001", r#"{"dry_run":true}"#)
            .expect("start");
        assert!(sweep_id > 0);

        // Check status
        let sweep = audit.get_sweep(sweep_id).expect("get").expect("exists");
        assert_eq!(sweep.status, SweepStatus::Running);
        assert_eq!(sweep.run_id, "LRB-20251202-001");

        // Complete sweep
        audit
            .complete_sweep(sweep_id, r#"{"memories_scanned":10}"#)
            .expect("complete");

        let sweep = audit.get_sweep(sweep_id).expect("get").expect("exists");
        assert_eq!(sweep.status, SweepStatus::Completed);
        assert!(sweep.finished_at.is_some());
    }

    #[test]
    fn test_sweep_failure() {
        let conn = setup_db();
        let audit = LibrarianAudit::new(&conn);

        let sweep_id = audit.start_sweep("LRB-20251202-002", "{}").expect("start");
        audit
            .fail_sweep(sweep_id, "MCP connection failed")
            .expect("fail");

        let sweep = audit.get_sweep(sweep_id).expect("get").expect("exists");
        assert_eq!(sweep.status, SweepStatus::Failed);
        assert!(
            sweep
                .stats_json
                .as_ref()
                .unwrap()
                .contains("MCP connection failed")
        );
    }

    #[test]
    fn test_log_change() {
        let conn = setup_db();
        let audit = LibrarianAudit::new(&conn);

        let sweep_id = audit.start_sweep("LRB-20251202-003", "{}").expect("start");

        let input = ChangeInput {
            memory_id: "mem-001".to_string(),
            change_type: ChangeType::Retype,
            old_type: None,
            new_type: Some("pattern".to_string()),
            old_content: None,
            new_content: None,
            confidence: Some(0.85),
            applied: false,
        };

        let change_id = audit.log_change(sweep_id, &input).expect("log");
        assert!(change_id > 0);

        let changes = audit.get_changes(sweep_id).expect("get");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].memory_id, "mem-001");
        assert_eq!(changes[0].change_type, ChangeType::Retype);
        assert_eq!(changes[0].new_type, Some("pattern".to_string()));
        assert!(!changes[0].applied);
    }

    #[test]
    fn test_log_edge() {
        let conn = setup_db();
        let audit = LibrarianAudit::new(&conn);

        let sweep_id = audit.start_sweep("LRB-20251202-004", "{}").expect("start");

        let input = EdgeInput {
            from_id: "mem-001".to_string(),
            to_id: "mem-002".to_string(),
            relation_type: "causes".to_string(),
            reason: Some("Cache bug caused memory leak".to_string()),
            applied: false,
        };

        let edge_id = audit.log_edge(sweep_id, &input).expect("log");
        assert!(edge_id > 0);

        let edges = audit.get_edges(sweep_id).expect("get");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from_id, "mem-001");
        assert_eq!(edges[0].to_id, "mem-002");
        assert_eq!(edges[0].relation_type, "causes");
    }

    #[test]
    fn test_generate_run_id() {
        let conn = setup_db();
        let audit = LibrarianAudit::new(&conn);

        let run_id1 = audit.generate_run_id().expect("gen1");
        assert!(run_id1.starts_with("LRB-"));
        assert!(run_id1.ends_with("-001"));

        // Start a sweep with that ID
        audit.start_sweep(&run_id1, "{}").expect("start");

        // Next run_id should be 002
        let run_id2 = audit.generate_run_id().expect("gen2");
        assert!(run_id2.ends_with("-002"));
    }

    #[test]
    fn test_list_sweeps() {
        let conn = setup_db();
        let audit = LibrarianAudit::new(&conn);

        audit.start_sweep("LRB-20251202-001", "{}").expect("start");
        audit.start_sweep("LRB-20251202-002", "{}").expect("start");
        audit.start_sweep("LRB-20251202-003", "{}").expect("start");

        let sweeps = audit.list_sweeps(2).expect("list");
        assert_eq!(sweeps.len(), 2);
        // Most recent first
        assert_eq!(sweeps[0].run_id, "LRB-20251202-003");
    }

    #[test]
    fn test_get_sweep_by_run_id() {
        let conn = setup_db();
        let audit = LibrarianAudit::new(&conn);

        audit.start_sweep("LRB-20251202-001", "{}").expect("start");

        let sweep = audit
            .get_sweep_by_run_id("LRB-20251202-001")
            .expect("get")
            .expect("exists");
        assert_eq!(sweep.run_id, "LRB-20251202-001");

        let none = audit.get_sweep_by_run_id("nonexistent").expect("get");
        assert!(none.is_none());
    }

    #[test]
    fn test_count_changes_and_edges() {
        let conn = setup_db();
        let audit = LibrarianAudit::new(&conn);

        let sweep_id = audit.start_sweep("LRB-20251202-001", "{}").expect("start");

        assert_eq!(audit.count_changes(sweep_id).expect("count"), 0);
        assert_eq!(audit.count_edges(sweep_id).expect("count"), 0);

        let change = ChangeInput {
            memory_id: "mem-001".to_string(),
            change_type: ChangeType::Retype,
            old_type: None,
            new_type: Some("pattern".to_string()),
            old_content: None,
            new_content: None,
            confidence: Some(0.85),
            applied: false,
        };
        audit.log_change(sweep_id, &change).expect("log");
        audit.log_change(sweep_id, &change).expect("log");

        let edge = EdgeInput {
            from_id: "mem-001".to_string(),
            to_id: "mem-002".to_string(),
            relation_type: "causes".to_string(),
            reason: None,
            applied: false,
        };
        audit.log_edge(sweep_id, &edge).expect("log");

        assert_eq!(audit.count_changes(sweep_id).expect("count"), 2);
        assert_eq!(audit.count_edges(sweep_id).expect("count"), 1);
    }
}
