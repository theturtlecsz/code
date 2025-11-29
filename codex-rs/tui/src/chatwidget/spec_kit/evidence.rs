//! Evidence repository abstraction for spec-kit
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit multi-agent automation framework
//!
//! This module breaks the hard-coded filesystem coupling and enables
//! testing with mock implementations. File locking (ARCH-007) prevents
//! concurrent write corruption.

#![allow(dead_code)] // Extended evidence features pending integration

use super::error::{Result, SpecKitError};
use crate::spec_prompts::SpecStage;
use codex_spec_kit::retry::strategy::{RetryConfig, execute_with_backoff_sync};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// MAINT-7: Centralized evidence path constants
/// Default evidence base directory
pub const DEFAULT_EVIDENCE_BASE: &str = "docs/SPEC-OPS-004-integrated-coder-hooks/evidence";

/// Helper: Build consensus directory path
pub fn consensus_dir(cwd: &Path) -> PathBuf {
    cwd.join(DEFAULT_EVIDENCE_BASE).join("consensus")
}

/// Helper: Build commands directory path
pub fn commands_dir(cwd: &Path) -> PathBuf {
    cwd.join(DEFAULT_EVIDENCE_BASE).join("commands")
}

/// Evidence storage operations abstraction
///
/// This trait allows swapping between filesystem, in-memory, or other
/// storage backends for testing and flexibility.
pub trait EvidenceRepository: Send + Sync {
    /// Get the base evidence directory for a spec
    fn evidence_dir(&self, spec_id: &str, category: EvidenceCategory) -> PathBuf;

    /// Read the latest telemetry file matching a stage prefix
    fn read_latest_telemetry(&self, spec_id: &str, stage: SpecStage) -> Result<(PathBuf, Value)>;

    /// Read latest consensus synthesis for a stage
    fn read_latest_consensus(&self, spec_id: &str, stage: SpecStage) -> Result<Option<Value>>;

    /// Write consensus verdict to filesystem
    fn write_consensus_verdict(
        &self,
        spec_id: &str,
        stage: SpecStage,
        verdict: &Value,
    ) -> Result<PathBuf>;

    /// Write telemetry bundle
    fn write_telemetry_bundle(
        &self,
        spec_id: &str,
        stage: SpecStage,
        telemetry: &Value,
    ) -> Result<PathBuf>;

    /// Write consensus synthesis
    fn write_consensus_synthesis(
        &self,
        spec_id: &str,
        stage: SpecStage,
        synthesis: &Value,
    ) -> Result<PathBuf>;

    /// List all files in a directory matching a pattern
    fn list_files(&self, directory: &Path, pattern: &str) -> Result<Vec<PathBuf>>;

    /// Check if evidence exists for a spec/stage
    fn has_evidence(&self, spec_id: &str, stage: SpecStage, category: EvidenceCategory) -> bool;

    /// Write quality gate checkpoint telemetry
    fn write_quality_checkpoint_telemetry(
        &self,
        spec_id: &str,
        checkpoint: super::state::QualityCheckpoint,
        telemetry: &Value,
    ) -> Result<PathBuf>;
}

/// Evidence category (commands vs consensus)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceCategory {
    /// Guardrail command telemetry
    Commands,
    /// Multi-agent consensus artifacts
    Consensus,
}

/// Filesystem-based evidence repository (production implementation)
pub struct FilesystemEvidence {
    cwd: PathBuf,
    base_path: String,
}

impl FilesystemEvidence {
    /// Create a new filesystem evidence repository
    ///
    /// # Arguments
    /// * `cwd` - Current working directory (project root)
    /// * `base_path` - Base evidence path (default: "docs/SPEC-OPS-004-integrated-coder-hooks/evidence")
    pub fn new(cwd: PathBuf, base_path: Option<String>) -> Self {
        Self {
            cwd,
            base_path: base_path.unwrap_or_else(|| DEFAULT_EVIDENCE_BASE.to_string()),
        }
    }

    /// ARCH-007: Write file with exclusive lock to prevent concurrent corruption
    ///
    /// Acquires per-spec lock before writing to prevent races between
    /// guardrail scripts and spec-kit consensus checks.
    fn write_with_lock(&self, spec_id: &str, target_path: &PathBuf, content: &str) -> Result<()> {
        use fs2::FileExt;
        use std::fs::OpenOptions;

        let lock_dir = self.cwd.join(&self.base_path).join(".locks");
        std::fs::create_dir_all(&lock_dir).map_err(|e| SpecKitError::DirectoryCreate {
            path: lock_dir.clone(),
            source: e,
        })?;

        let lock_file_path = lock_dir.join(format!("{}.lock", spec_id));
        let lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&lock_file_path)
            .map_err(|e| SpecKitError::FileWrite {
                path: lock_file_path.clone(),
                source: e,
            })?;

        // Acquire exclusive lock (blocks if another writer active)
        lock_file
            .lock_exclusive()
            .map_err(|e| SpecKitError::FileWrite {
                path: lock_file_path.clone(),
                source: e,
            })?;

        // SPEC-945C Day 4-5: Write with retry logic to handle transient I/O errors
        let retry_config = RetryConfig {
            max_attempts: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 5_000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.5,
            max_elapsed_ms: None, // Use max_attempts only
        };

        // Lock auto-released when lock_file drops (RAII)
        execute_with_backoff_sync(
            || {
                std::fs::write(target_path, content).map_err(|e| SpecKitError::FileWrite {
                    path: target_path.clone(),
                    source: e,
                })
            },
            &retry_config,
        )
        .map_err(|_| SpecKitError::from_string("Evidence write failed after retries"))
    }

    /// Get category subdirectory name
    fn category_dir(&self, category: EvidenceCategory) -> &'static str {
        match category {
            EvidenceCategory::Commands => "commands",
            EvidenceCategory::Consensus => "consensus",
        }
    }

    /// Get stage prefix for filename matching
    fn stage_prefix(&self, stage: SpecStage) -> &'static str {
        super::state::spec_ops_stage_prefix(stage)
    }
}

impl EvidenceRepository for FilesystemEvidence {
    fn evidence_dir(&self, spec_id: &str, category: EvidenceCategory) -> PathBuf {
        self.cwd
            .join(&self.base_path)
            .join(self.category_dir(category))
            .join(spec_id)
    }

    fn read_latest_telemetry(&self, spec_id: &str, stage: SpecStage) -> Result<(PathBuf, Value)> {
        let evidence_dir = self.evidence_dir(spec_id, EvidenceCategory::Commands);
        let prefix = self.stage_prefix(stage);

        let entries =
            std::fs::read_dir(&evidence_dir).map_err(|e| SpecKitError::DirectoryRead {
                path: evidence_dir.clone(),
                source: e,
            })?;

        let mut latest: Option<(PathBuf, SystemTime)> = None;
        for entry_res in entries {
            let entry = entry_res.map_err(|e| SpecKitError::DirectoryRead {
                path: evidence_dir.clone(),
                source: e,
            })?;

            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }

            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            if !name.starts_with(prefix) {
                continue;
            }

            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);

            if latest
                .as_ref()
                .map(|(_, ts)| modified > *ts)
                .unwrap_or(true)
            {
                latest = Some((path.clone(), modified));
            }
        }

        let (path, _) = latest.ok_or_else(|| SpecKitError::NoTelemetryFound {
            spec_id: spec_id.to_string(),
            stage: stage.command_name().to_string(),
            pattern: format!("{}*", prefix),
            directory: evidence_dir.clone(),
        })?;

        let mut file = std::fs::File::open(&path).map_err(|e| SpecKitError::FileRead {
            path: path.clone(),
            source: e,
        })?;

        let mut buf = String::new();
        std::io::Read::read_to_string(&mut file, &mut buf).map_err(|e| SpecKitError::FileRead {
            path: path.clone(),
            source: e,
        })?;

        let value: Value = serde_json::from_str(&buf).map_err(|e| SpecKitError::JsonParse {
            path: path.clone(),
            source: e,
        })?;

        Ok((path, value))
    }

    fn read_latest_consensus(&self, spec_id: &str, stage: SpecStage) -> Result<Option<Value>> {
        let consensus_dir = self.evidence_dir(spec_id, EvidenceCategory::Consensus);
        let prefix = self.stage_prefix(stage);

        let entries = match std::fs::read_dir(&consensus_dir) {
            Ok(e) => e,
            Err(_) => return Ok(None), // Directory doesn't exist yet
        };

        let mut latest: Option<(PathBuf, SystemTime)> = None;
        for entry_res in entries {
            let Ok(entry) = entry_res else { continue };
            let path = entry.path();

            if !path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(prefix) && n.ends_with("_synthesis.json"))
                .unwrap_or(false)
            {
                continue;
            }

            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);

            if latest
                .as_ref()
                .map(|(_, ts)| modified > *ts)
                .unwrap_or(true)
            {
                latest = Some((path.clone(), modified));
            }
        }

        let Some((path, _)) = latest else {
            return Ok(None);
        };

        let contents = std::fs::read_to_string(&path).map_err(|e| SpecKitError::FileRead {
            path: path.clone(),
            source: e,
        })?;

        let value: Value =
            serde_json::from_str(&contents).map_err(|e| SpecKitError::JsonParse {
                path: path.clone(),
                source: e,
            })?;

        Ok(Some(value))
    }

    fn write_consensus_verdict(
        &self,
        spec_id: &str,
        stage: SpecStage,
        verdict: &Value,
    ) -> Result<PathBuf> {
        let consensus_dir = self.evidence_dir(spec_id, EvidenceCategory::Consensus);

        std::fs::create_dir_all(&consensus_dir).map_err(|e| SpecKitError::DirectoryCreate {
            path: consensus_dir.clone(),
            source: e,
        })?;

        let filename = format!("{}_{}_verdict.json", spec_id, stage.command_name());
        let path = consensus_dir.join(filename);

        let json = serde_json::to_string_pretty(verdict)
            .map_err(|e| SpecKitError::JsonSerialize { source: e })?;

        // ARCH-007: Use locking to prevent concurrent write corruption
        self.write_with_lock(spec_id, &path, &json)?;

        Ok(path)
    }

    fn write_telemetry_bundle(
        &self,
        spec_id: &str,
        stage: SpecStage,
        telemetry: &Value,
    ) -> Result<PathBuf> {
        // FORK-SPECIFIC (just-every/code): SPEC-KIT-069 telemetry path fix
        // Lifecycle telemetry should go to commands/ not consensus/
        let commands_dir = self.evidence_dir(spec_id, EvidenceCategory::Commands);

        std::fs::create_dir_all(&commands_dir).map_err(|e| SpecKitError::DirectoryCreate {
            path: commands_dir.clone(),
            source: e,
        })?;

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!(
            "{}_{}_telemetry_{}.json",
            spec_id,
            stage.command_name(),
            timestamp
        );
        let path = commands_dir.join(filename);

        let json = serde_json::to_string_pretty(telemetry)
            .map_err(|e| SpecKitError::JsonSerialize { source: e })?;

        // ARCH-007: Use locking to prevent concurrent write corruption
        self.write_with_lock(spec_id, &path, &json)?;

        Ok(path)
    }

    fn write_consensus_synthesis(
        &self,
        spec_id: &str,
        stage: SpecStage,
        synthesis: &Value,
    ) -> Result<PathBuf> {
        let consensus_dir = self.evidence_dir(spec_id, EvidenceCategory::Consensus);

        std::fs::create_dir_all(&consensus_dir).map_err(|e| SpecKitError::DirectoryCreate {
            path: consensus_dir.clone(),
            source: e,
        })?;

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!(
            "{}_{}_synthesis_{}.json",
            spec_id,
            stage.command_name(),
            timestamp
        );
        let path = consensus_dir.join(filename);

        let json = serde_json::to_string_pretty(synthesis)
            .map_err(|e| SpecKitError::JsonSerialize { source: e })?;

        // ARCH-007: Use locking to prevent concurrent write corruption
        self.write_with_lock(spec_id, &path, &json)?;

        Ok(path)
    }

    fn list_files(&self, directory: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
        let entries = match std::fs::read_dir(directory) {
            Ok(e) => e,
            Err(_) => return Ok(Vec::new()), // Directory doesn't exist
        };

        let mut files = Vec::new();
        for entry_res in entries {
            let Ok(entry) = entry_res else { continue };
            let path = entry.path();

            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.contains(pattern)
            {
                files.push(path);
            }
        }

        Ok(files)
    }

    fn has_evidence(&self, spec_id: &str, stage: SpecStage, category: EvidenceCategory) -> bool {
        let dir = self.evidence_dir(spec_id, category);
        let prefix = self.stage_prefix(stage);

        std::fs::read_dir(&dir)
            .ok()
            .and_then(|entries| {
                entries.filter_map(|e| e.ok()).find(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .map(|n| n.starts_with(prefix))
                        .unwrap_or(false)
                })
            })
            .is_some()
    }

    fn write_quality_checkpoint_telemetry(
        &self,
        spec_id: &str,
        checkpoint: super::state::QualityCheckpoint,
        telemetry: &Value,
    ) -> Result<PathBuf> {
        let evidence_dir = self.evidence_dir(spec_id, EvidenceCategory::Consensus);

        std::fs::create_dir_all(&evidence_dir).map_err(|e| SpecKitError::DirectoryCreate {
            path: evidence_dir.clone(),
            source: e,
        })?;

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!(
            "{}_quality-gate-{}_{}.json",
            spec_id,
            checkpoint.name(),
            timestamp
        );
        let path = evidence_dir.join(filename);

        let json = serde_json::to_string_pretty(telemetry)
            .map_err(|e| SpecKitError::JsonSerialize { source: e })?;

        // ARCH-007: Use locking to prevent concurrent write corruption
        self.write_with_lock(spec_id, &path, &json)?;

        Ok(path)
    }
}

impl Default for FilesystemEvidence {
    fn default() -> Self {
        Self::new(
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            None,
        )
    }
}

// === AUTOMATIC EVIDENCE EXPORT (SPEC-KIT-900 Session 3) ===

/// Automatically export stage evidence after synthesis completes
///
/// CRITICAL: Called immediately after db.store_synthesis() to ensure evidence
/// directory is ALWAYS populated for checklist compliance.
///
/// Exports:
/// - <stage>_synthesis.json (from consensus_runs.synthesis_json column)
/// - <stage>_verdict.json (from agent_outputs table)
///
/// Does NOT fail pipeline if export fails (logs warning instead).
pub fn auto_export_stage_evidence(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
    run_id: Option<&str>,
) {
    let evidence_root = cwd.join(DEFAULT_EVIDENCE_BASE);
    let consensus_dir = evidence_root.join("consensus").join(spec_id);

    // Create directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&consensus_dir) {
        tracing::warn!("Failed to create consensus directory: {}", e);
        return;
    }

    let stage_name = stage.display_name().to_lowercase();
    let stage_command = stage.command_name();

    tracing::info!("📤 Auto-exporting evidence for {} stage", stage_name);

    // Export synthesis record
    match export_synthesis_record(&consensus_dir, spec_id, stage_command, &stage_name) {
        Ok(path) => {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            tracing::info!("  ✓ {}_synthesis.json ({} bytes)", stage_name, size);
        }
        Err(e) => {
            tracing::warn!("  ✗ Failed to export synthesis for {}: {}", stage_name, e);
        }
    }

    // Export verdict (agent proposals)
    match export_verdict_record(&consensus_dir, spec_id, stage_command, &stage_name, run_id) {
        Ok(path) => {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            tracing::info!("  ✓ {}_verdict.json ({} bytes)", stage_name, size);
        }
        Err(e) => {
            tracing::warn!("  ✗ Failed to export verdict for {}: {}", stage_name, e);
        }
    }
}

fn export_synthesis_record(
    consensus_dir: &Path,
    spec_id: &str,
    stage_command: &str,
    stage_name: &str,
) -> Result<PathBuf> {
    let db_path = dirs::home_dir()
        .ok_or_else(|| SpecKitError::from_string("Cannot determine home directory"))?
        .join(".code")
        .join("consensus_artifacts.db");

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| SpecKitError::from_string(format!("Failed to open database: {}", e)))?;

    // Query new schema (consensus_runs.synthesis_json column)
    let synthesis_json_str: String = conn
        .query_row(
            "SELECT synthesis_json FROM consensus_runs
             WHERE spec_id = ?1 AND stage = ?2 AND synthesis_json IS NOT NULL
             ORDER BY run_timestamp DESC
             LIMIT 1",
            rusqlite::params![spec_id, stage_command],
            |row| row.get(0),
        )
        .map_err(|e| SpecKitError::from_string(format!("Query failed: {}", e)))?;

    // Parse synthesis JSON and add metadata
    let mut synthesis_data: serde_json::Value = serde_json::from_str(&synthesis_json_str)
        .map_err(|e| SpecKitError::from_string(format!("JSON parse failed: {}", e)))?;

    // Add spec_id and stage metadata if not present
    if let Some(obj) = synthesis_data.as_object_mut() {
        obj.insert(
            "spec_id".to_string(),
            serde_json::Value::String(spec_id.to_string()),
        );
        obj.insert(
            "stage".to_string(),
            serde_json::Value::String(stage_command.to_string()),
        );
    }

    let synthesis_file = consensus_dir.join(format!("{}_synthesis.json", stage_name));
    let json_str = serde_json::to_string_pretty(&synthesis_data)
        .map_err(|e| SpecKitError::from_string(format!("JSON serialization failed: {}", e)))?;

    std::fs::write(&synthesis_file, json_str)
        .map_err(|e| SpecKitError::from_string(format!("Write failed: {}", e)))?;

    Ok(synthesis_file)
}

fn export_verdict_record(
    consensus_dir: &Path,
    spec_id: &str,
    stage_command: &str,
    stage_name: &str,
    run_id: Option<&str>,
) -> Result<PathBuf> {
    let db_path = dirs::home_dir()
        .ok_or_else(|| SpecKitError::from_string("Cannot determine home directory"))?
        .join(".code")
        .join("consensus_artifacts.db");

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| SpecKitError::from_string(format!("Failed to open database: {}", e)))?;

    // Get agent proposals for this stage from new schema
    let proposals: Vec<Value> = if let Some(rid) = run_id {
        // Query with run_id filter - join agent_outputs with consensus_runs
        let mut stmt = conn
            .prepare(
                "SELECT ao.agent_name, ao.content, ao.output_timestamp
             FROM agent_outputs ao
             JOIN consensus_runs cr ON ao.run_id = cr.id
             WHERE cr.spec_id = ?1 AND cr.stage = ?2 AND ao.run_id = ?3
             ORDER BY ao.output_timestamp",
            )
            .map_err(|e| SpecKitError::from_string(format!("Prepare failed: {}", e)))?;

        stmt.query_map(rusqlite::params![spec_id, stage_command, rid], |row| {
            let agent_name: String = row.get(0)?;
            let content_json: String = row.get(1)?;
            let timestamp: i64 = row.get(2)?;

            let content = serde_json::from_str::<Value>(&content_json)
                .unwrap_or_else(|_| Value::String(content_json));

            // Format timestamp as ISO 8601
            let datetime =
                chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0).unwrap_or_default();
            let created_at = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

            Ok(serde_json::json!({
                "agent_name": agent_name,
                "content": content,
                "created_at": created_at,
            }))
        })
        .map_err(|e| SpecKitError::from_string(format!("Query failed: {}", e)))?
        .filter_map(std::result::Result::ok)
        .collect()
    } else {
        // Query without run_id filter
        let mut stmt = conn
            .prepare(
                "SELECT ao.agent_name, ao.content, ao.output_timestamp
             FROM agent_outputs ao
             JOIN consensus_runs cr ON ao.run_id = cr.id
             WHERE cr.spec_id = ?1 AND cr.stage = ?2
             ORDER BY ao.output_timestamp",
            )
            .map_err(|e| SpecKitError::from_string(format!("Prepare failed: {}", e)))?;

        stmt.query_map(rusqlite::params![spec_id, stage_command], |row| {
            let agent_name: String = row.get(0)?;
            let content_json: String = row.get(1)?;
            let timestamp: i64 = row.get(2)?;

            let content = serde_json::from_str::<Value>(&content_json)
                .unwrap_or_else(|_| Value::String(content_json));

            // Format timestamp as ISO 8601
            let datetime =
                chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0).unwrap_or_default();
            let created_at = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

            Ok(serde_json::json!({
                "agent_name": agent_name,
                "content": content,
                "created_at": created_at,
            }))
        })
        .map_err(|e| SpecKitError::from_string(format!("Query failed: {}", e)))?
        .filter_map(std::result::Result::ok)
        .collect()
    };

    let verdict_data = serde_json::json!({
        "spec_id": spec_id,
        "stage": stage_name,
        "proposals": proposals,
        "run_id": run_id,
        "exported_at": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    });

    let verdict_file = consensus_dir.join(format!("{}_verdict.json", stage_name));
    let json_str = serde_json::to_string_pretty(&verdict_data)
        .map_err(|e| SpecKitError::from_string(format!("JSON serialization failed: {}", e)))?;

    std::fs::write(&verdict_file, json_str)
        .map_err(|e| SpecKitError::from_string(format!("Write failed: {}", e)))?;

    Ok(verdict_file)
}

// === NATIVE EVIDENCE STATS (SPEC-KIT-902) ===

/// Evidence size limits (SPEC-KIT-909)
pub const EVIDENCE_WARN_MB: f64 = 40.0;
pub const EVIDENCE_LIMIT_MB: f64 = 50.0;

/// Result of evidence size calculation
#[derive(Debug, Clone)]
pub struct EvidenceStats {
    /// Total size in bytes
    pub total_size_bytes: u64,
    /// Human-readable size (e.g., "15.2 MB")
    pub total_size_human: String,
    /// Size by SPEC ID: (spec_id, bytes)
    pub spec_sizes: Vec<(String, u64)>,
    /// True if total exceeds warning threshold (40 MB)
    pub exceeds_warning: bool,
    /// True if total exceeds hard limit (50 MB)
    pub exceeds_limit: bool,
}

impl EvidenceStats {
    /// Format bytes as human-readable string
    fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

/// Calculate evidence statistics for a SPEC or all SPECs
///
/// # Arguments
/// * `cwd` - Project root directory
/// * `spec_id` - Optional SPEC ID to filter by (None = all SPECs)
///
/// # Returns
/// Evidence statistics including sizes and limit checks
pub fn calculate_evidence_stats(cwd: &Path, spec_id: Option<&str>) -> Result<EvidenceStats> {
    let evidence_root = cwd.join(DEFAULT_EVIDENCE_BASE);

    if !evidence_root.exists() {
        return Ok(EvidenceStats {
            total_size_bytes: 0,
            total_size_human: "0 B".to_string(),
            spec_sizes: Vec::new(),
            exceeds_warning: false,
            exceeds_limit: false,
        });
    }

    let consensus_dir = evidence_root.join("consensus");
    let commands_dir = evidence_root.join("commands");

    let mut spec_sizes: Vec<(String, u64)> = Vec::new();

    // Collect all SPEC IDs from consensus and commands directories
    let mut spec_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for dir in [&consensus_dir, &commands_dir] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        // Skip hidden directories and locks
                        if !name.starts_with('.') {
                            spec_ids.insert(name.to_string());
                        }
                    }
                }
            }
        }
    }

    // Filter by spec_id if provided
    if let Some(filter_id) = spec_id {
        spec_ids.retain(|id| id == filter_id);
    }

    // Calculate sizes for each SPEC
    for id in &spec_ids {
        let mut size: u64 = 0;

        // Consensus directory
        let spec_consensus = consensus_dir.join(id);
        if spec_consensus.exists() {
            size += calculate_dir_size(&spec_consensus);
        }

        // Commands directory
        let spec_commands = commands_dir.join(id);
        if spec_commands.exists() {
            size += calculate_dir_size(&spec_commands);
        }

        spec_sizes.push((id.clone(), size));
    }

    // Sort by size (largest first)
    spec_sizes.sort_by(|a, b| b.1.cmp(&a.1));

    let total_size_bytes: u64 = spec_sizes.iter().map(|(_, s)| s).sum();
    let total_mb = total_size_bytes as f64 / (1024.0 * 1024.0);

    Ok(EvidenceStats {
        total_size_bytes,
        total_size_human: EvidenceStats::format_size(total_size_bytes),
        spec_sizes,
        exceeds_warning: total_mb > EVIDENCE_WARN_MB,
        exceeds_limit: total_mb > EVIDENCE_LIMIT_MB,
    })
}

/// Calculate total size of a directory recursively
fn calculate_dir_size(dir: &Path) -> u64 {
    let mut total: u64 = 0;

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                total += path.metadata().map(|m| m.len()).unwrap_or(0);
            } else if path.is_dir() {
                total += calculate_dir_size(&path);
            }
        }
    }

    total
}

/// Check if a SPEC's evidence exceeds the size limit (50 MB)
///
/// This is the native replacement for calling evidence_stats.sh
///
/// # Returns
/// Ok(()) if within limits, Err with message if exceeds 50 MB
pub fn check_spec_evidence_limit(cwd: &Path, spec_id: &str) -> Result<()> {
    let stats = calculate_evidence_stats(cwd, Some(spec_id))?;

    if stats.exceeds_limit {
        let size_mb = stats.total_size_bytes as f64 / (1024.0 * 1024.0);
        return Err(SpecKitError::from_string(format!(
            "{} evidence is {:.1} MB (exceeds {:.0} MB limit). Archive consensus artifacts before continuing.",
            spec_id, size_mb, EVIDENCE_LIMIT_MB
        )));
    }

    if stats.exceeds_warning {
        let size_mb = stats.total_size_bytes as f64 / (1024.0 * 1024.0);
        tracing::warn!(
            "⚠️  {} evidence is {:.1} MB (above {:.0} MB warning threshold)",
            spec_id,
            size_mb,
            EVIDENCE_WARN_MB
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Mock evidence repository for testing
    struct MockEvidence {
        telemetry: Mutex<std::collections::HashMap<String, Value>>,
        consensus: Mutex<std::collections::HashMap<String, Value>>,
    }

    impl MockEvidence {
        fn new() -> Self {
            Self {
                telemetry: Mutex::new(std::collections::HashMap::new()),
                consensus: Mutex::new(std::collections::HashMap::new()),
            }
        }

        fn insert_telemetry(&self, key: String, value: Value) {
            self.telemetry.lock().unwrap().insert(key, value);
        }
    }

    impl EvidenceRepository for MockEvidence {
        fn evidence_dir(&self, spec_id: &str, category: EvidenceCategory) -> PathBuf {
            PathBuf::from(format!("/mock/{:?}/{}", category, spec_id))
        }

        fn read_latest_telemetry(
            &self,
            spec_id: &str,
            stage: SpecStage,
        ) -> Result<(PathBuf, Value)> {
            let key = format!("{}_{}", spec_id, stage.command_name());
            let telemetry = self.telemetry.lock().unwrap();

            let value =
                telemetry
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| SpecKitError::NoTelemetryFound {
                        spec_id: spec_id.to_string(),
                        stage: stage.command_name().to_string(),
                        pattern: format!("{}*", stage.command_name()),
                        directory: PathBuf::from("/mock"),
                    })?;

            Ok((PathBuf::from(format!("/mock/{}.json", key)), value))
        }

        fn read_latest_consensus(&self, spec_id: &str, stage: SpecStage) -> Result<Option<Value>> {
            let key = format!("{}_{}", spec_id, stage.command_name());
            let consensus = self.consensus.lock().unwrap();
            Ok(consensus.get(&key).cloned())
        }

        fn write_consensus_verdict(
            &self,
            spec_id: &str,
            stage: SpecStage,
            verdict: &Value,
        ) -> Result<PathBuf> {
            let key = format!("{}_{}", spec_id, stage.command_name());
            self.consensus
                .lock()
                .unwrap()
                .insert(key.clone(), verdict.clone());
            Ok(PathBuf::from(format!("/mock/{}_verdict.json", key)))
        }

        fn write_telemetry_bundle(
            &self,
            spec_id: &str,
            stage: SpecStage,
            telemetry: &Value,
        ) -> Result<PathBuf> {
            let key = format!("{}_{}", spec_id, stage.command_name());
            self.telemetry
                .lock()
                .unwrap()
                .insert(key.clone(), telemetry.clone());
            Ok(PathBuf::from(format!("/mock/{}_telemetry.json", key)))
        }

        fn write_consensus_synthesis(
            &self,
            spec_id: &str,
            stage: SpecStage,
            synthesis: &Value,
        ) -> Result<PathBuf> {
            let key = format!("{}_{}", spec_id, stage.command_name());
            self.consensus
                .lock()
                .unwrap()
                .insert(key.clone(), synthesis.clone());
            Ok(PathBuf::from(format!("/mock/{}_synthesis.json", key)))
        }

        fn list_files(&self, _directory: &Path, _pattern: &str) -> Result<Vec<PathBuf>> {
            Ok(Vec::new())
        }

        fn has_evidence(
            &self,
            spec_id: &str,
            stage: SpecStage,
            category: EvidenceCategory,
        ) -> bool {
            let key = format!("{}_{}", spec_id, stage.command_name());
            match category {
                EvidenceCategory::Commands => self.telemetry.lock().unwrap().contains_key(&key),
                EvidenceCategory::Consensus => self.consensus.lock().unwrap().contains_key(&key),
            }
        }

        fn write_quality_checkpoint_telemetry(
            &self,
            spec_id: &str,
            checkpoint: crate::chatwidget::spec_kit::state::QualityCheckpoint,
            telemetry: &Value,
        ) -> Result<PathBuf> {
            let key = format!("{}_{}", spec_id, checkpoint.name());
            self.consensus
                .lock()
                .unwrap()
                .insert(key.clone(), telemetry.clone());
            Ok(PathBuf::from(format!("/mock/quality-gate-{}.json", key)))
        }
    }

    #[test]
    fn test_filesystem_evidence_paths() {
        let repo = FilesystemEvidence::new(PathBuf::from("/project"), None);

        let commands_dir = repo.evidence_dir("SPEC-KIT-065", EvidenceCategory::Commands);
        assert_eq!(
            commands_dir,
            PathBuf::from(
                "/project/docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-065"
            )
        );

        let consensus_dir = repo.evidence_dir("SPEC-KIT-065", EvidenceCategory::Consensus);
        assert_eq!(
            consensus_dir,
            PathBuf::from(
                "/project/docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-065"
            )
        );
    }

    #[test]
    fn test_filesystem_evidence_custom_base() {
        let repo = FilesystemEvidence::new(
            PathBuf::from("/project"),
            Some("custom/evidence".to_string()),
        );

        let dir = repo.evidence_dir("SPEC-TEST", EvidenceCategory::Commands);
        assert_eq!(
            dir,
            PathBuf::from("/project/custom/evidence/commands/SPEC-TEST")
        );
    }

    #[test]
    fn test_mock_evidence_read_write() {
        let mock = MockEvidence::new();

        // Insert test telemetry (key format: SPEC-ID_stage)
        let test_value = serde_json::json!({"test": "data"});
        let key = format!("SPEC-TEST_{}", SpecStage::Plan.command_name());
        mock.insert_telemetry(key.clone(), test_value.clone());

        // Read it back
        let result = mock.read_latest_telemetry("SPEC-TEST", SpecStage::Plan);
        assert!(
            result.is_ok(),
            "Failed to read telemetry: {:?}",
            result.err()
        );

        let (path, value) = result.unwrap();
        assert_eq!(value, test_value);
        assert!(path.to_str().unwrap().contains("SPEC-TEST"));
    }

    #[test]
    fn test_mock_evidence_missing_telemetry() {
        let mock = MockEvidence::new();

        let result = mock.read_latest_telemetry("SPEC-MISSING", SpecStage::Plan);
        assert!(result.is_err());

        match result {
            Err(SpecKitError::NoTelemetryFound { spec_id, .. }) => {
                assert_eq!(spec_id, "SPEC-MISSING");
            }
            _ => panic!("Expected NoTelemetryFound error"),
        }
    }

    #[test]
    fn test_mock_evidence_write_verdict() {
        let mock = MockEvidence::new();

        let verdict = serde_json::json!({"status": "approved"});
        let result = mock.write_consensus_verdict("SPEC-TEST", SpecStage::Plan, &verdict);
        assert!(result.is_ok());

        // Verify it was stored
        assert!(mock.has_evidence("SPEC-TEST", SpecStage::Plan, EvidenceCategory::Consensus));
    }

    #[test]
    fn test_evidence_category() {
        let repo = FilesystemEvidence::default();

        assert_eq!(repo.category_dir(EvidenceCategory::Commands), "commands");
        assert_eq!(repo.category_dir(EvidenceCategory::Consensus), "consensus");
    }
}
