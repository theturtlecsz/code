//! SPEC-KIT-978: Reflex Bakeoff Metrics Collection
//!
//! SQLite-based metrics storage for comparing reflex (local inference) vs cloud
//! performance. Enables data-driven decisions about routing thresholds.
//!
//! ## Table: reflex_bakeoff_metrics
//! - timestamp: When the attempt occurred
//! - mode: "reflex" or "cloud"
//! - latency_ms: Request latency in milliseconds
//! - success: Whether the request completed successfully
//! - json_compliant: Whether the response was valid JSON (for structured output)
//! - spec_id: SPEC identifier for correlation
//! - run_id: Pipeline run identifier
//!
//! ## Usage
//! ```rust,ignore
//! let db = ReflexMetricsDb::init_default()?;
//! db.record_reflex_attempt("SPEC-978", "run123", 150, true, true)?;
//! db.record_cloud_attempt("SPEC-978", "run123", 2500, true, true)?;
//! let stats = db.compute_bakeoff_stats(Duration::from_secs(86400))?;
//! ```

use rusqlite::{Connection, Result as SqlResult, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Bakeoff metrics database handle
pub struct ReflexMetricsDb {
    conn: Arc<Mutex<Connection>>,
}

/// Single metric record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BakeoffMetric {
    pub id: i64,
    pub timestamp: String,
    pub mode: String,
    pub latency_ms: u64,
    pub success: bool,
    pub json_compliant: bool,
    pub spec_id: String,
    pub run_id: String,
}

/// Aggregated bakeoff statistics for a single mode (reflex or cloud)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeStats {
    pub mode: String,
    pub total_attempts: u64,
    pub success_count: u64,
    pub success_rate: f64,
    pub json_compliant_count: u64,
    pub json_compliance_rate: f64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
}

/// Complete bakeoff statistics comparing reflex vs cloud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BakeoffStats {
    pub reflex: Option<ModeStats>,
    pub cloud: Option<ModeStats>,
    pub period_start: String,
    pub period_end: String,
    pub total_attempts: u64,
}

impl ReflexMetricsDb {
    /// Initialize database at default location (~/.code/reflex_metrics.db)
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
        Ok(db_dir.join("reflex_metrics.db"))
    }

    /// Initialize database at specific path
    pub fn init(db_path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(db_path)?;

        // Create bakeoff metrics table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS reflex_bakeoff_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                mode TEXT NOT NULL CHECK (mode IN ('reflex', 'cloud')),
                latency_ms INTEGER NOT NULL,
                success INTEGER NOT NULL,
                json_compliant INTEGER NOT NULL,
                spec_id TEXT NOT NULL,
                run_id TEXT NOT NULL
            )",
            [],
        )?;

        // Create indexes for efficient querying
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_bakeoff_timestamp
             ON reflex_bakeoff_metrics(timestamp)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_bakeoff_mode
             ON reflex_bakeoff_metrics(mode, timestamp)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_bakeoff_spec
             ON reflex_bakeoff_metrics(spec_id, run_id)",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Record a reflex inference attempt
    pub fn record_reflex_attempt(
        &self,
        spec_id: &str,
        run_id: &str,
        latency_ms: u64,
        success: bool,
        json_compliant: bool,
    ) -> SqlResult<i64> {
        self.record_attempt(
            "reflex",
            spec_id,
            run_id,
            latency_ms,
            success,
            json_compliant,
        )
    }

    /// Record a cloud inference attempt
    pub fn record_cloud_attempt(
        &self,
        spec_id: &str,
        run_id: &str,
        latency_ms: u64,
        success: bool,
        json_compliant: bool,
    ) -> SqlResult<i64> {
        self.record_attempt(
            "cloud",
            spec_id,
            run_id,
            latency_ms,
            success,
            json_compliant,
        )
    }

    /// Record an inference attempt (internal)
    fn record_attempt(
        &self,
        mode: &str,
        spec_id: &str,
        run_id: &str,
        latency_ms: u64,
        success: bool,
        json_compliant: bool,
    ) -> SqlResult<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        conn.execute(
            "INSERT INTO reflex_bakeoff_metrics (mode, latency_ms, success, json_compliant, spec_id, run_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                mode,
                latency_ms as i64,
                if success { 1 } else { 0 },
                if json_compliant { 1 } else { 0 },
                spec_id,
                run_id,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Compute bakeoff statistics for a given time period
    ///
    /// ## Parameters
    /// - `since`: Duration to look back (e.g., 24 hours)
    ///
    /// ## Returns
    /// Statistics comparing reflex vs cloud performance
    pub fn compute_bakeoff_stats(&self, since: Duration) -> SqlResult<BakeoffStats> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        // Calculate cutoff timestamp
        let seconds = since.as_secs() as i64;
        let cutoff = format!("-{} seconds", seconds);

        // Get period bounds
        let (period_start, period_end): (String, String) = conn.query_row(
            "SELECT
                datetime('now', ?1) as period_start,
                datetime('now') as period_end",
            params![cutoff],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        // Get total attempts
        let total_attempts: u64 = conn.query_row(
            "SELECT COUNT(*) FROM reflex_bakeoff_metrics
             WHERE timestamp >= datetime('now', ?1)",
            params![cutoff],
            |row| row.get::<_, i64>(0).map(|v| v as u64),
        )?;

        // Compute stats for each mode
        let reflex = self.compute_mode_stats(&conn, "reflex", &cutoff)?;
        let cloud = self.compute_mode_stats(&conn, "cloud", &cutoff)?;

        Ok(BakeoffStats {
            reflex,
            cloud,
            period_start,
            period_end,
            total_attempts,
        })
    }

    /// Compute statistics for a specific mode
    fn compute_mode_stats(
        &self,
        conn: &Connection,
        mode: &str,
        cutoff: &str,
    ) -> SqlResult<Option<ModeStats>> {
        // Get basic aggregates
        let result: Option<(i64, i64, i64, f64, i64, i64)> = conn.query_row(
            "SELECT
                COUNT(*) as total,
                SUM(success) as success_count,
                SUM(json_compliant) as json_count,
                AVG(latency_ms) as avg_latency,
                MIN(latency_ms) as min_latency,
                MAX(latency_ms) as max_latency
             FROM reflex_bakeoff_metrics
             WHERE mode = ?1 AND timestamp >= datetime('now', ?2)",
            params![mode, cutoff],
            |row| {
                let total: i64 = row.get(0)?;
                if total == 0 {
                    Ok(None)
                } else {
                    Ok(Some((
                        total,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, f64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                    )))
                }
            },
        )?;

        let (total, success_count, json_count, avg_latency, min_latency, max_latency) = match result
        {
            Some(v) => v,
            None => return Ok(None),
        };

        // Get percentiles (requires sorting all latencies)
        let (p50, p95, p99) = self.compute_percentiles(conn, mode, cutoff)?;

        Ok(Some(ModeStats {
            mode: mode.to_string(),
            total_attempts: total as u64,
            success_count: success_count as u64,
            success_rate: if total > 0 {
                (success_count as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            json_compliant_count: json_count as u64,
            json_compliance_rate: if total > 0 {
                (json_count as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            avg_latency_ms: avg_latency,
            p50_latency_ms: p50,
            p95_latency_ms: p95,
            p99_latency_ms: p99,
            min_latency_ms: min_latency as u64,
            max_latency_ms: max_latency as u64,
        }))
    }

    /// Compute latency percentiles (p50, p95, p99)
    fn compute_percentiles(
        &self,
        conn: &Connection,
        mode: &str,
        cutoff: &str,
    ) -> SqlResult<(u64, u64, u64)> {
        // Get all latencies sorted
        let mut stmt = conn.prepare(
            "SELECT latency_ms FROM reflex_bakeoff_metrics
             WHERE mode = ?1 AND timestamp >= datetime('now', ?2)
             ORDER BY latency_ms ASC",
        )?;

        let latencies: Vec<u64> = stmt
            .query_map(params![mode, cutoff], |row| {
                row.get::<_, i64>(0).map(|v| v as u64)
            })?
            .filter_map(|r| r.ok())
            .collect();

        if latencies.is_empty() {
            return Ok((0, 0, 0));
        }

        let n = latencies.len();
        let p50_idx = (n as f64 * 0.50).ceil() as usize - 1;
        let p95_idx = (n as f64 * 0.95).ceil() as usize - 1;
        let p99_idx = (n as f64 * 0.99).ceil() as usize - 1;

        Ok((
            latencies.get(p50_idx).copied().unwrap_or(0),
            latencies.get(p95_idx.min(n - 1)).copied().unwrap_or(0),
            latencies.get(p99_idx.min(n - 1)).copied().unwrap_or(0),
        ))
    }

    /// Get recent metrics for a specific spec/run
    pub fn get_metrics_for_run(
        &self,
        spec_id: &str,
        run_id: &str,
    ) -> SqlResult<Vec<BakeoffMetric>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        let mut stmt = conn.prepare(
            "SELECT id, timestamp, mode, latency_ms, success, json_compliant, spec_id, run_id
             FROM reflex_bakeoff_metrics
             WHERE spec_id = ?1 AND run_id = ?2
             ORDER BY timestamp DESC",
        )?;

        let metrics = stmt
            .query_map(params![spec_id, run_id], |row| {
                Ok(BakeoffMetric {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    mode: row.get(2)?,
                    latency_ms: row.get::<_, i64>(3)? as u64,
                    success: row.get::<_, i64>(4)? != 0,
                    json_compliant: row.get::<_, i64>(5)? != 0,
                    spec_id: row.get(6)?,
                    run_id: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(metrics)
    }

    /// Clean up old metrics (older than N days)
    pub fn cleanup_old_metrics(&self, days: i64) -> SqlResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        conn.execute(
            "DELETE FROM reflex_bakeoff_metrics
             WHERE timestamp < datetime('now', ?1)",
            params![format!("-{} days", days)],
        )
    }

    /// Check if reflex meets bakeoff thresholds
    ///
    /// ## Parameters
    /// - `since`: Duration to analyze (e.g., 24 hours)
    /// - `min_samples`: Minimum number of samples required
    /// - `p95_threshold_ms`: P95 latency must be below this
    /// - `success_threshold_pct`: Success rate must be above this (0-100)
    /// - `json_threshold_pct`: JSON compliance must be above this (0-100)
    ///
    /// ## Returns
    /// (passes, reason) - Whether thresholds are met and explanation
    pub fn check_thresholds(
        &self,
        since: Duration,
        min_samples: u64,
        p95_threshold_ms: u64,
        success_threshold_pct: u8,
        json_threshold_pct: u8,
    ) -> SqlResult<(bool, String)> {
        let stats = self.compute_bakeoff_stats(since)?;

        let reflex = match &stats.reflex {
            Some(r) => r,
            None => return Ok((false, "No reflex samples available".to_string())),
        };

        // Check minimum samples
        if reflex.total_attempts < min_samples {
            return Ok((
                false,
                format!(
                    "Insufficient samples: {} < {} required",
                    reflex.total_attempts, min_samples
                ),
            ));
        }

        // Check P95 latency
        if reflex.p95_latency_ms > p95_threshold_ms {
            return Ok((
                false,
                format!(
                    "P95 latency too high: {}ms > {}ms threshold",
                    reflex.p95_latency_ms, p95_threshold_ms
                ),
            ));
        }

        // Check success rate
        if reflex.success_rate < success_threshold_pct as f64 {
            return Ok((
                false,
                format!(
                    "Success rate too low: {:.1}% < {}% threshold",
                    reflex.success_rate, success_threshold_pct
                ),
            ));
        }

        // Check JSON compliance
        if reflex.json_compliance_rate < json_threshold_pct as f64 {
            return Ok((
                false,
                format!(
                    "JSON compliance too low: {:.1}% < {}% threshold",
                    reflex.json_compliance_rate, json_threshold_pct
                ),
            ));
        }

        Ok((true, "All thresholds met".to_string()))
    }
}

/// Global singleton for metrics DB (initialized lazily)
static METRICS_DB: std::sync::OnceLock<Option<ReflexMetricsDb>> = std::sync::OnceLock::new();

/// Get or initialize the global metrics database
pub fn get_metrics_db() -> SqlResult<&'static ReflexMetricsDb> {
    let db = METRICS_DB.get_or_init(|| ReflexMetricsDb::init_default().ok());

    db.as_ref()
        .ok_or_else(|| rusqlite::Error::InvalidPath("Failed to initialize metrics DB".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_record_and_query_metrics() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_metrics.db");
        let db = ReflexMetricsDb::init(&db_path).unwrap();

        // Record some reflex attempts
        db.record_reflex_attempt("SPEC-978", "run1", 100, true, true)
            .unwrap();
        db.record_reflex_attempt("SPEC-978", "run1", 150, true, true)
            .unwrap();
        db.record_reflex_attempt("SPEC-978", "run1", 200, true, false)
            .unwrap();

        // Record some cloud attempts
        db.record_cloud_attempt("SPEC-978", "run1", 2000, true, true)
            .unwrap();
        db.record_cloud_attempt("SPEC-978", "run1", 2500, true, true)
            .unwrap();

        // Query metrics for run
        let metrics = db.get_metrics_for_run("SPEC-978", "run1").unwrap();
        assert_eq!(metrics.len(), 5);
    }

    #[test]
    fn test_compute_bakeoff_stats() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_stats.db");
        let db = ReflexMetricsDb::init(&db_path).unwrap();

        // Record reflex attempts with varying latencies
        for latency in [100, 120, 130, 140, 150, 160, 170, 180, 190, 200] {
            db.record_reflex_attempt("SPEC-978", "run1", latency, true, true)
                .unwrap();
        }

        // Record one failed attempt
        db.record_reflex_attempt("SPEC-978", "run1", 500, false, false)
            .unwrap();

        // Record cloud attempts
        for latency in [2000, 2100, 2200, 2300, 2400] {
            db.record_cloud_attempt("SPEC-978", "run1", latency, true, true)
                .unwrap();
        }

        // Compute stats
        let stats = db.compute_bakeoff_stats(Duration::from_secs(3600)).unwrap();

        assert_eq!(stats.total_attempts, 16);

        let reflex = stats.reflex.unwrap();
        assert_eq!(reflex.total_attempts, 11);
        assert_eq!(reflex.success_count, 10);
        assert!(reflex.success_rate > 90.0);
        assert!(reflex.p95_latency_ms <= 500);

        let cloud = stats.cloud.unwrap();
        assert_eq!(cloud.total_attempts, 5);
        assert_eq!(cloud.success_rate, 100.0);
    }

    #[test]
    fn test_threshold_checking() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_threshold.db");
        let db = ReflexMetricsDb::init(&db_path).unwrap();

        // Record good reflex attempts (all passing thresholds)
        for _ in 0..20 {
            db.record_reflex_attempt("SPEC-978", "run1", 150, true, true)
                .unwrap();
        }

        // Check thresholds - should pass
        let (passes, reason) = db
            .check_thresholds(
                Duration::from_secs(3600),
                10,   // min_samples
                2000, // p95_threshold_ms
                85,   // success_threshold_pct
                100,  // json_threshold_pct
            )
            .unwrap();

        assert!(passes, "Expected thresholds to pass: {}", reason);
    }

    #[test]
    fn test_threshold_failure_insufficient_samples() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_samples.db");
        let db = ReflexMetricsDb::init(&db_path).unwrap();

        // Only record 5 samples
        for _ in 0..5 {
            db.record_reflex_attempt("SPEC-978", "run1", 150, true, true)
                .unwrap();
        }

        // Check thresholds - should fail due to insufficient samples
        let (passes, reason) = db
            .check_thresholds(
                Duration::from_secs(3600),
                10, // min_samples (need 10, only have 5)
                2000,
                85,
                100,
            )
            .unwrap();

        assert!(!passes, "Expected threshold check to fail");
        assert!(reason.contains("Insufficient samples"));
    }

    #[test]
    fn test_cleanup_old_metrics() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_cleanup.db");
        let db = ReflexMetricsDb::init(&db_path).unwrap();

        // Record some metrics
        db.record_reflex_attempt("SPEC-978", "run1", 150, true, true)
            .unwrap();

        // Verify metric exists
        let metrics = db.get_metrics_for_run("SPEC-978", "run1").unwrap();
        assert_eq!(metrics.len(), 1);

        // Cleanup with -1 days (in the future) should delete nothing just-inserted
        let deleted = db.cleanup_old_metrics(-1).unwrap();
        // Records inserted now are not older than now, so 0 deleted is expected
        assert_eq!(deleted, 0);

        // Verify still there
        let metrics = db.get_metrics_for_run("SPEC-978", "run1").unwrap();
        assert_eq!(metrics.len(), 1);
    }
}
