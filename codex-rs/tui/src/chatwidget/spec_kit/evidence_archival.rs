//! Evidence archival module (E.3 capability)
//!
//! Implements evidence retention policy from docs/spec-kit/evidence-policy.md §4-6:
//! - Archive evidence >30 days old
//! - Purge evidence >180 days old (safety margin)
//! - Exempt in-progress SPECs (files modified within 7 days)
//!
//! Uses injectable Clock trait for deterministic testing.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// ============================================================================
// Clock Abstraction (for deterministic testing)
// ============================================================================

/// Clock abstraction for injectable time source.
///
/// Enables deterministic testing of time-based archival logic.
pub trait Clock: Send + Sync {
    /// Returns the current time.
    fn now(&self) -> DateTime<Utc>;
}

/// System clock implementation (production use).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Mock clock for testing with controlled timestamps.
#[derive(Debug, Clone)]
pub struct MockClock {
    pub current_time: DateTime<Utc>,
}

impl MockClock {
    /// Create a new mock clock at the specified time.
    pub fn new(time: DateTime<Utc>) -> Self {
        Self { current_time: time }
    }

    /// Create a mock clock at a fixed test time (2026-01-15 12:00:00 UTC).
    pub fn fixed() -> Self {
        Self {
            current_time: DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
                .expect("valid fixed time")
                .with_timezone(&Utc),
        }
    }

    /// Advance the clock by the specified duration.
    pub fn advance(&mut self, duration: Duration) {
        self.current_time += duration;
    }
}

impl Clock for MockClock {
    fn now(&self) -> DateTime<Utc> {
        self.current_time
    }
}

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for evidence archival operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalConfig {
    /// Days before evidence is eligible for archival (default: 30).
    pub archive_after_days: u32,
    /// Days before archived evidence is eligible for purge (default: 180).
    pub purge_after_days: u32,
    /// Activity window for in-progress exemption (default: 7 days).
    /// SPECs with files modified within this window are exempt from archival.
    pub activity_window_days: u32,
    /// Enable gzip compression when archiving (default: true).
    pub compression_enabled: bool,
    /// Warning threshold in MB (default: 45).
    pub warning_threshold_mb: u32,
    /// Hard limit in MB (default: 50).
    pub hard_limit_mb: u32,
    /// Dry-run mode - report without executing (default: false).
    pub dry_run: bool,
}

impl Default for ArchivalConfig {
    fn default() -> Self {
        Self {
            archive_after_days: 30,
            purge_after_days: 180,
            activity_window_days: 7,
            compression_enabled: true,
            warning_threshold_mb: 45,
            hard_limit_mb: 50,
            dry_run: false,
        }
    }
}

// ============================================================================
// Types
// ============================================================================

/// Classification of a SPEC's archival status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchivalStatus {
    /// SPEC is active (within activity window or recent).
    Active,
    /// SPEC is eligible for archival (>archive_after_days, no recent activity).
    Archivable,
    /// SPEC is eligible for purge (>purge_after_days since archival).
    Purgeable,
}

/// Information about a SPEC's archival eligibility.
#[derive(Debug, Clone)]
pub struct SpecArchivalInfo {
    /// SPEC identifier.
    pub spec_id: String,
    /// Path to the SPEC's evidence directory.
    pub evidence_path: PathBuf,
    /// Archival status classification.
    pub status: ArchivalStatus,
    /// Most recent file modification time.
    pub last_modified: DateTime<Utc>,
    /// Total size in bytes.
    pub size_bytes: u64,
    /// Number of files.
    pub file_count: usize,
    /// Days since last modification.
    pub days_since_modified: i64,
}

/// Result of an archival operation.
#[derive(Debug, Clone)]
pub struct ArchiveResult {
    /// SPEC identifier.
    pub spec_id: String,
    /// Path to the created archive.
    pub archive_path: PathBuf,
    /// Original size in bytes.
    pub original_size: u64,
    /// Compressed size in bytes.
    pub compressed_size: u64,
    /// Compression ratio (0.0-1.0).
    pub compression_ratio: f64,
    /// Number of files archived.
    pub files_archived: usize,
    /// SHA256 checksum of the archive.
    pub checksum: String,
}

/// Result of a cleanup run.
#[derive(Debug, Clone, Default)]
pub struct CleanupSummary {
    /// Number of SPECs archived.
    pub specs_archived: usize,
    /// Number of SPECs purged.
    pub specs_purged: usize,
    /// Total bytes archived.
    pub bytes_archived: u64,
    /// Total bytes purged.
    pub bytes_purged: u64,
    /// SPECs that were exempt (in-progress).
    pub specs_exempt: Vec<String>,
    /// Warnings encountered.
    pub warnings: Vec<String>,
    /// Whether this was a dry run.
    pub dry_run: bool,
}

// ============================================================================
// Core Functions
// ============================================================================

/// Get the last modification time of a file or directory (recursively).
fn get_last_modified(path: &Path) -> io::Result<DateTime<Utc>> {
    let metadata = fs::metadata(path)?;
    let modified = metadata.modified()?;
    let mut latest: DateTime<Utc> = modified.into();

    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_modified = get_last_modified(&entry.path())?;
            if entry_modified > latest {
                latest = entry_modified;
            }
        }
    }

    Ok(latest)
}

/// Calculate the total size of a directory (recursively).
fn get_directory_size(path: &Path) -> io::Result<u64> {
    let mut total = 0;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                total += get_directory_size(&entry_path)?;
            } else {
                total += entry.metadata()?.len();
            }
        }
    } else {
        total = fs::metadata(path)?.len();
    }
    Ok(total)
}

/// Count files in a directory (recursively).
fn count_files(path: &Path) -> io::Result<usize> {
    let mut count = 0;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                count += count_files(&entry_path)?;
            } else {
                count += 1;
            }
        }
    }
    Ok(count)
}

/// Get archival information for all SPECs in the evidence directory.
///
/// # Arguments
/// * `clock` - Time source for determining age
/// * `config` - Archival configuration
/// * `evidence_root` - Root evidence directory (e.g., `docs/SPEC-OPS-004-integrated-coder-hooks/evidence`)
///
/// # Returns
/// Vector of `SpecArchivalInfo` for each SPEC found.
pub fn get_spec_archival_info(
    clock: &dyn Clock,
    config: &ArchivalConfig,
    evidence_root: &Path,
) -> io::Result<Vec<SpecArchivalInfo>> {
    let mut results = Vec::new();
    let now = clock.now();

    // Scan commands/ and consensus/ directories
    for category in ["commands", "consensus"] {
        let category_path = evidence_root.join(category);
        if !category_path.exists() {
            continue;
        }

        for entry in fs::read_dir(&category_path)? {
            let entry = entry?;
            let spec_path = entry.path();

            if !spec_path.is_dir() {
                continue;
            }

            let spec_id = match spec_path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => continue,
            };

            // Skip .locks directory
            if spec_id.starts_with('.') {
                continue;
            }

            // Check if we already processed this SPEC (from the other category)
            if results
                .iter()
                .any(|r: &SpecArchivalInfo| r.spec_id == spec_id)
            {
                continue;
            }

            // Gather combined info from both commands/ and consensus/
            let commands_path = evidence_root.join("commands").join(&spec_id);
            let consensus_path = evidence_root.join("consensus").join(&spec_id);

            let mut last_modified = DateTime::<Utc>::MIN_UTC;
            let mut total_size = 0u64;
            let mut total_files = 0usize;

            for p in [&commands_path, &consensus_path] {
                if p.exists() {
                    if let Ok(modified) = get_last_modified(p) {
                        if modified > last_modified {
                            last_modified = modified;
                        }
                    }
                    if let Ok(size) = get_directory_size(p) {
                        total_size += size;
                    }
                    if let Ok(count) = count_files(p) {
                        total_files += count;
                    }
                }
            }

            if last_modified == DateTime::<Utc>::MIN_UTC {
                continue; // No valid modification time
            }

            let days_since_modified = (now - last_modified).num_days();

            // Determine archival status
            let status = if days_since_modified <= i64::from(config.activity_window_days) {
                ArchivalStatus::Active
            } else if days_since_modified > i64::from(config.purge_after_days) {
                ArchivalStatus::Purgeable
            } else if days_since_modified > i64::from(config.archive_after_days) {
                ArchivalStatus::Archivable
            } else {
                ArchivalStatus::Active
            };

            results.push(SpecArchivalInfo {
                spec_id,
                evidence_path: spec_path,
                status,
                last_modified,
                size_bytes: total_size,
                file_count: total_files,
                days_since_modified,
            });
        }
    }

    Ok(results)
}

/// Get SPECs that are eligible for archival.
pub fn get_archivable_specs(
    clock: &dyn Clock,
    config: &ArchivalConfig,
    evidence_root: &Path,
) -> io::Result<Vec<SpecArchivalInfo>> {
    let all_specs = get_spec_archival_info(clock, config, evidence_root)?;
    Ok(all_specs
        .into_iter()
        .filter(|s| s.status == ArchivalStatus::Archivable)
        .collect())
}

/// Get SPECs that are eligible for purge.
pub fn get_purgeable_specs(
    clock: &dyn Clock,
    config: &ArchivalConfig,
    evidence_root: &Path,
) -> io::Result<Vec<SpecArchivalInfo>> {
    let all_specs = get_spec_archival_info(clock, config, evidence_root)?;
    Ok(all_specs
        .into_iter()
        .filter(|s| s.status == ArchivalStatus::Purgeable)
        .collect())
}

/// Check if a SPEC is in-progress (exempt from archival).
///
/// A SPEC is considered in-progress if any evidence file was modified
/// within the activity window (default: 7 days).
pub fn is_spec_in_progress(
    clock: &dyn Clock,
    config: &ArchivalConfig,
    evidence_root: &Path,
    spec_id: &str,
) -> io::Result<bool> {
    let now = clock.now();

    for category in ["commands", "consensus"] {
        let spec_path = evidence_root.join(category).join(spec_id);
        if spec_path.exists() {
            let last_modified = get_last_modified(&spec_path)?;
            let days_since = (now - last_modified).num_days();
            if days_since <= i64::from(config.activity_window_days) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Validate that archival happens before purge (policy constraint).
///
/// Returns an error if a SPEC would be purged without first being archived.
/// This enforces the 30d archive -> 180d purge order from evidence-policy.md.
pub fn validate_archive_before_purge(
    clock: &dyn Clock,
    config: &ArchivalConfig,
    evidence_root: &Path,
) -> io::Result<Result<(), Vec<String>>> {
    // Check that purge threshold is greater than archive threshold
    if config.purge_after_days <= config.archive_after_days {
        return Ok(Err(vec![format!(
            "Invalid config: purge_after_days ({}) must be > archive_after_days ({})",
            config.purge_after_days, config.archive_after_days
        )]));
    }

    let all_specs = get_spec_archival_info(clock, config, evidence_root)?;
    let violations: Vec<String> = all_specs
        .iter()
        .filter(|s| s.status == ArchivalStatus::Purgeable)
        .filter_map(|s| {
            // Check if archive exists for this SPEC
            let _archive_pattern = evidence_root
                .join("archives")
                .join(format!("{}-*.tar.gz", s.spec_id));
            // Simple check: if archives/ dir exists and contains this spec's archive
            let archives_dir = evidence_root.join("archives");
            if archives_dir.exists() {
                if let Ok(entries) = fs::read_dir(&archives_dir) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with(&format!("{}-", s.spec_id)) && name.ends_with(".tar.gz")
                        {
                            return None; // Archive exists, OK to purge
                        }
                    }
                }
            }
            Some(format!(
                "SPEC {} is purgeable ({} days old) but no archive exists",
                s.spec_id, s.days_since_modified
            ))
        })
        .collect();

    if violations.is_empty() {
        Ok(Ok(()))
    } else {
        Ok(Err(violations))
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;
    use tempfile::TempDir;

    fn create_test_evidence(temp_dir: &TempDir, spec_id: &str, age_days: i64) -> PathBuf {
        let evidence_root = temp_dir.path().to_path_buf();
        let commands_dir = evidence_root.join("commands").join(spec_id);
        let consensus_dir = evidence_root.join("consensus").join(spec_id);

        fs::create_dir_all(&commands_dir).unwrap();
        fs::create_dir_all(&consensus_dir).unwrap();

        // Create test files
        let telemetry = commands_dir.join("telemetry.json");
        fs::write(&telemetry, r#"{"test": "data"}"#).unwrap();

        let synthesis = consensus_dir.join("synthesis.json");
        fs::write(&synthesis, r#"{"synthesis": "test"}"#).unwrap();

        // Set modification time to specified age
        if age_days > 0 {
            let mtime = filetime::FileTime::from_unix_time(
                (Utc::now() - Duration::days(age_days)).timestamp(),
                0,
            );
            filetime::set_file_mtime(&telemetry, mtime).unwrap();
            filetime::set_file_mtime(&synthesis, mtime).unwrap();
            filetime::set_file_mtime(&commands_dir, mtime).unwrap();
            filetime::set_file_mtime(&consensus_dir, mtime).unwrap();
        }

        evidence_root
    }

    #[test]
    fn test_mock_clock_fixed_time() {
        let clock = MockClock::fixed();
        let now = clock.now();
        assert_eq!(now.year(), 2026);
        assert_eq!(now.month(), 1);
        assert_eq!(now.day(), 15);
    }

    #[test]
    fn test_mock_clock_advance() {
        let mut clock = MockClock::fixed();
        let initial = clock.now();
        clock.advance(Duration::days(5));
        let advanced = clock.now();
        assert_eq!((advanced - initial).num_days(), 5);
    }

    #[test]
    fn test_default_archival_config() {
        let config = ArchivalConfig::default();
        assert_eq!(config.archive_after_days, 30);
        assert_eq!(config.purge_after_days, 180);
        assert_eq!(config.activity_window_days, 7);
        assert!(config.compression_enabled);
        assert!(!config.dry_run);
    }

    #[test]
    fn test_active_spec_not_archivable() {
        let temp_dir = TempDir::new().unwrap();
        let evidence_root = create_test_evidence(&temp_dir, "SPEC-TEST-001", 0);

        let clock = MockClock::fixed();
        let config = ArchivalConfig::default();

        let specs = get_spec_archival_info(&clock, &config, &evidence_root).unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].status, ArchivalStatus::Active);
    }

    #[test]
    fn test_in_progress_exemption() {
        let temp_dir = TempDir::new().unwrap();
        let evidence_root = create_test_evidence(&temp_dir, "SPEC-TEST-002", 3); // 3 days old

        let clock = MockClock::fixed();
        let config = ArchivalConfig::default(); // 7 day activity window

        let is_in_progress =
            is_spec_in_progress(&clock, &config, &evidence_root, "SPEC-TEST-002").unwrap();
        assert!(is_in_progress);
    }

    #[test]
    fn test_archive_before_purge_config_validation() {
        let temp_dir = TempDir::new().unwrap();
        let evidence_root = temp_dir.path().to_path_buf();
        fs::create_dir_all(evidence_root.join("commands")).unwrap();

        let clock = MockClock::fixed();
        let mut config = ArchivalConfig::default();

        // Invalid: purge <= archive
        config.archive_after_days = 30;
        config.purge_after_days = 30;

        let result = validate_archive_before_purge(&clock, &config, &evidence_root).unwrap();
        assert!(result.is_err());
    }
}
