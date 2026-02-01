//! SPEC-KIT-971: Cross-process single-writer lock for capsules
//!
//! ## Design (D7: Single-writer capsule model)
//! - Opening capsule for write creates an exclusive lock
//! - Lock file contains metadata: pid, host, user, timestamps, context
//! - If another process holds the lock, open fails with actionable error
//! - Lock lifetime matches CapsuleHandle lifetime (via Drop)
//!
//! Note: `result_large_err` is allowed because LockError::AlreadyLocked
//! intentionally contains full LockMetadata for debugging contention issues.
#![allow(clippy::result_large_err)]

//! ## Lock File Format
//! Path: `<capsule_path>.lock` (e.g., `workspace.mv2.lock`)
//! Contents: JSON with LockMetadata

use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

// =============================================================================
// LockMetadata
// =============================================================================

/// Metadata stored in the lock file for diagnostics and recovery.
///
/// When a lock conflict occurs, this metadata helps identify:
/// - Which process holds the lock (pid, host, user)
/// - When it was acquired (started_at)
/// - What it's doing (spec_id, run_id, branch)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockMetadata {
    /// Process ID of the lock holder
    pub pid: u32,

    /// Hostname where the lock was acquired
    pub host: String,

    /// Username of the lock holder
    pub user: String,

    /// When the lock was acquired
    pub started_at: DateTime<Utc>,

    /// Optional: spec ID being processed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_id: Option<String>,

    /// Optional: run ID for the current operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,

    /// Optional: git branch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Lock file schema version (for forward compatibility)
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

fn default_schema_version() -> u32 {
    1
}

impl std::fmt::Display for LockMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_summary())
    }
}

impl LockMetadata {
    /// Create lock metadata for the current process.
    pub fn current() -> Self {
        Self {
            pid: std::process::id(),
            host: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            user: whoami::username(),
            started_at: Utc::now(),
            spec_id: None,
            run_id: None,
            branch: None,
            schema_version: 1,
        }
    }

    /// Create lock metadata with context.
    pub fn with_context(
        spec_id: Option<String>,
        run_id: Option<String>,
        branch: Option<String>,
    ) -> Self {
        let mut meta = Self::current();
        meta.spec_id = spec_id;
        meta.run_id = run_id;
        meta.branch = branch;
        meta
    }

    /// Check if this lock appears to be stale (process no longer running).
    ///
    /// This is a best-effort check - it only works reliably on the same host.
    pub fn is_stale(&self) -> bool {
        // Check if we're on the same host
        let current_host = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_default();

        if self.host != current_host {
            // Can't reliably check cross-host, assume not stale
            return false;
        }

        // Check if process is still running
        !is_process_running(self.pid)
    }

    /// Format for display in error messages.
    pub fn display_summary(&self) -> String {
        let age = Utc::now().signed_duration_since(self.started_at);
        let age_str = if age.num_hours() > 0 {
            format!("{}h {}m ago", age.num_hours(), age.num_minutes() % 60)
        } else if age.num_minutes() > 0 {
            format!("{}m {}s ago", age.num_minutes(), age.num_seconds() % 60)
        } else {
            format!("{}s ago", age.num_seconds())
        };

        let context = match (&self.spec_id, &self.run_id) {
            (Some(spec), Some(run)) => format!(" (spec: {}, run: {})", spec, run),
            (Some(spec), None) => format!(" (spec: {})", spec),
            _ => String::new(),
        };

        format!(
            "PID {} on {}@{} started {}{}",
            self.pid, self.user, self.host, age_str, context
        )
    }
}

/// Check if a process with the given PID is running.
#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    // Use kill(pid, 0) to check if process exists
    // Returns 0 if process exists, -1 with ESRCH if not
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(windows)]
fn is_process_running(pid: u32) -> bool {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_INFORMATION,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
        if handle == null_mut() {
            return false;
        }
        let mut exit_code: u32 = 0;
        let result = GetExitCodeProcess(handle, &mut exit_code);
        CloseHandle(handle);
        result != 0 && exit_code == STILL_ACTIVE
    }
}

#[cfg(not(any(unix, windows)))]
fn is_process_running(_pid: u32) -> bool {
    // Can't check on this platform, assume running
    true
}

// =============================================================================
// CapsuleLock - File-based lock with advisory OS lock
// =============================================================================

/// A held lock on a capsule file.
///
/// The lock is released when this struct is dropped.
pub struct CapsuleLock {
    /// The lock file handle (kept open to maintain lock)
    file: File,

    /// Path to the lock file
    path: PathBuf,

    /// Metadata written to the lock file
    metadata: LockMetadata,
}

impl CapsuleLock {
    /// Attempt to acquire an exclusive lock on the capsule.
    ///
    /// ## Parameters
    /// - `capsule_path`: Path to the capsule file (lock file will be `<path>.lock`)
    /// - `metadata`: Lock metadata to write
    ///
    /// ## Returns
    /// - `Ok(CapsuleLock)`: Lock acquired successfully
    /// - `Err(LockError::AlreadyLocked(metadata))`: Another process holds the lock
    /// - `Err(LockError::Io(e))`: IO error
    pub fn acquire(capsule_path: &Path, metadata: LockMetadata) -> Result<Self, LockError> {
        let lock_path = lock_path_for(capsule_path);

        // Try to create the lock file atomically (O_CREAT | O_EXCL)
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                // Got the file, now try to get an advisory OS lock for extra safety
                if let Err(e) = file.try_lock_exclusive() {
                    // Clean up the file we just created
                    let _ = std::fs::remove_file(&lock_path);
                    return Err(LockError::Io(e));
                }

                // Write metadata to the lock file
                let json = serde_json::to_string_pretty(&metadata)
                    .map_err(|e| LockError::Io(std::io::Error::other(e)))?;
                file.write_all(json.as_bytes())?;
                file.sync_all()?;

                Ok(CapsuleLock {
                    file,
                    path: lock_path,
                    metadata,
                })
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Lock file exists - try to read the metadata
                match read_lock_metadata(&lock_path) {
                    Ok(existing) => {
                        // Check if the lock is stale
                        if existing.is_stale() {
                            // Try to clean up stale lock and retry
                            tracing::warn!(
                                pid = existing.pid,
                                host = %existing.host,
                                "Removing stale lock from terminated process"
                            );
                            if std::fs::remove_file(&lock_path).is_ok() {
                                // Retry acquisition
                                return Self::acquire(capsule_path, metadata);
                            }
                        }
                        Err(LockError::AlreadyLocked(existing))
                    }
                    Err(_) => {
                        // Can't read metadata, but file exists - return generic locked error
                        Err(LockError::AlreadyLocked(LockMetadata {
                            pid: 0,
                            host: "unknown".to_string(),
                            user: "unknown".to_string(),
                            started_at: Utc::now(),
                            spec_id: None,
                            run_id: None,
                            branch: None,
                            schema_version: 1,
                        }))
                    }
                }
            }
            Err(e) => Err(LockError::Io(e)),
        }
    }

    /// Get the lock metadata.
    pub fn metadata(&self) -> &LockMetadata {
        &self.metadata
    }

    /// Release the lock (called automatically on drop).
    fn release(&mut self) {
        // Unlock the file
        let _ = self.file.unlock();

        // Remove the lock file
        let _ = std::fs::remove_file(&self.path);
    }
}

impl Drop for CapsuleLock {
    fn drop(&mut self) {
        self.release();
    }
}

// =============================================================================
// LockError
// =============================================================================

/// Errors that can occur during lock operations.
#[derive(Debug)]
pub enum LockError {
    /// Lock is held by another process
    AlreadyLocked(LockMetadata),

    /// IO error during lock operation
    Io(std::io::Error),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::AlreadyLocked(meta) => {
                write!(f, "Capsule is locked by: {}", meta.display_summary())
            }
            LockError::Io(e) => write!(f, "Lock IO error: {}", e),
        }
    }
}

impl std::error::Error for LockError {}

impl From<std::io::Error> for LockError {
    fn from(e: std::io::Error) -> Self {
        LockError::Io(e)
    }
}

// =============================================================================
// Helper functions
// =============================================================================

/// Get the lock file path for a capsule.
pub fn lock_path_for(capsule_path: &Path) -> PathBuf {
    // Per SPEC-KIT-971: lockfile path is "<capsule_path>.lock"
    let mut lock_path = capsule_path.as_os_str().to_owned();
    lock_path.push(".lock");
    PathBuf::from(lock_path)
}

/// Read lock metadata from an existing lock file.
pub fn read_lock_metadata(lock_path: &Path) -> Result<LockMetadata, std::io::Error> {
    let mut file = File::open(lock_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    serde_json::from_str(&contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Check if a capsule is locked (without acquiring the lock).
pub fn is_locked(capsule_path: &Path) -> Option<LockMetadata> {
    let lock_path = lock_path_for(capsule_path);
    if lock_path.exists() {
        read_lock_metadata(&lock_path).ok()
    } else {
        None
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_lock_acquire_release() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        // Create a dummy capsule file
        std::fs::write(&capsule_path, b"MV2\x00\x01").unwrap();

        // Acquire lock
        let metadata = LockMetadata::current();
        let lock = CapsuleLock::acquire(&capsule_path, metadata.clone()).unwrap();

        // Verify lock file exists
        let lock_path = lock_path_for(&capsule_path);
        assert!(lock_path.exists());

        // Verify metadata was written
        let read_meta = read_lock_metadata(&lock_path).unwrap();
        assert_eq!(read_meta.pid, metadata.pid);

        // Drop the lock
        drop(lock);

        // Verify lock file was removed
        assert!(!lock_path.exists());
    }

    #[test]
    fn test_lock_conflict() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        // Create a dummy capsule file
        std::fs::write(&capsule_path, b"MV2\x00\x01").unwrap();

        // Acquire first lock
        let metadata1 = LockMetadata::with_context(
            Some("SPEC-001".to_string()),
            Some("run-123".to_string()),
            Some("main".to_string()),
        );
        let _lock1 = CapsuleLock::acquire(&capsule_path, metadata1).unwrap();

        // Try to acquire second lock - should fail
        let metadata2 = LockMetadata::current();
        let result = CapsuleLock::acquire(&capsule_path, metadata2);

        match result {
            Err(LockError::AlreadyLocked(existing)) => {
                assert_eq!(existing.spec_id, Some("SPEC-001".to_string()));
                assert_eq!(existing.run_id, Some("run-123".to_string()));
            }
            _ => panic!("Expected AlreadyLocked error"),
        }
    }

    #[test]
    fn test_lock_metadata_display() {
        let metadata = LockMetadata {
            pid: 12345,
            host: "myhost".to_string(),
            user: "testuser".to_string(),
            started_at: Utc::now() - chrono::Duration::minutes(5),
            spec_id: Some("SPEC-KIT-971".to_string()),
            run_id: Some("run-abc".to_string()),
            branch: Some("main".to_string()),
            schema_version: 1,
        };

        let display = metadata.display_summary();
        assert!(display.contains("12345"));
        assert!(display.contains("testuser"));
        assert!(display.contains("myhost"));
        assert!(display.contains("SPEC-KIT-971"));
    }

    #[test]
    fn test_is_locked() {
        let temp_dir = TempDir::new().unwrap();
        let capsule_path = temp_dir.path().join("test.mv2");

        // Not locked initially
        assert!(is_locked(&capsule_path).is_none());

        // Acquire lock
        std::fs::write(&capsule_path, b"MV2\x00\x01").unwrap();
        let lock = CapsuleLock::acquire(&capsule_path, LockMetadata::current()).unwrap();

        // Now locked
        assert!(is_locked(&capsule_path).is_some());

        // Release
        drop(lock);

        // Not locked anymore
        assert!(is_locked(&capsule_path).is_none());
    }
}
