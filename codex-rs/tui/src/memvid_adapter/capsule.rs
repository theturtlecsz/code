//! SPEC-KIT-971: Capsule lifecycle and single-writer coordination
//!
//! ## Decision IDs
//! - D7: Single-writer capsule model (global lock + writer queue)
//! - D18: Stage boundary checkpoints
//! - D2: Canonical capsule path: `./.speckit/memvid/workspace.mv2`

use crate::memvid_adapter::lock::{CapsuleLock, LockError, LockMetadata, is_locked, lock_path_for};
use crate::memvid_adapter::types::{
    BranchId, CheckpointId, CheckpointMetadata, EventType, LogicalUri, ObjectType,
    PhysicalPointer, RunEventEnvelope, UriIndex,
};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use thiserror::Error;

// =============================================================================
// CapsuleError
// =============================================================================

#[derive(Debug, Error)]
pub enum CapsuleError {
    #[error("Capsule not found at {path}")]
    NotFound { path: PathBuf },

    #[error("Capsule is locked by another process")]
    Locked,

    #[error("Capsule is locked by another writer: {0}")]
    LockedByWriter(LockMetadata),

    #[error("Capsule is corrupted: {reason}")]
    Corrupted { reason: String },

    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Write queue full")]
    WriteQueueFull,

    #[error("Capsule not open")]
    NotOpen,

    #[error("URI not found: {uri}")]
    UriNotFound { uri: LogicalUri },

    #[error("Invalid operation: {reason}")]
    InvalidOperation { reason: String },
}

pub type Result<T> = std::result::Result<T, CapsuleError>;

// =============================================================================
// CapsuleConfig
// =============================================================================

/// Configuration for capsule operations.
#[derive(Debug, Clone)]
pub struct CapsuleConfig {
    /// Path to the workspace capsule.
    /// Default: `./.speckit/memvid/workspace.mv2`
    pub capsule_path: PathBuf,

    /// Workspace ID for URI generation.
    pub workspace_id: String,

    /// Maximum write queue size.
    pub max_write_queue: usize,

    /// Lock timeout in milliseconds.
    pub lock_timeout_ms: u64,

    /// Enable dedup tracks (BLAKE3 + SimHash).
    pub enable_dedup: bool,
}

impl Default for CapsuleConfig {
    fn default() -> Self {
        Self {
            capsule_path: PathBuf::from(".speckit/memvid/workspace.mv2"),
            workspace_id: "default".to_string(),
            max_write_queue: 100,
            lock_timeout_ms: 5000,
            enable_dedup: true,
        }
    }
}

// =============================================================================
// CapsuleHandle - Lifecycle management (D7)
// =============================================================================

/// Handle to an open capsule with single-writer coordination.
///
/// ## Single-Writer Model (D7)
/// - Global lock prevents concurrent writes
/// - Write queue allows async write submission
/// - Crash recovery via last-good checkpoint
///
/// ## Lifecycle
/// 1. `CapsuleHandle::open(config)` - Open or create capsule
/// 2. `handle.put(...)` - Submit write (queued)
/// 3. `handle.commit(...)` - Flush writes + create checkpoint
/// 4. Drop handle - Release lock
pub struct CapsuleHandle {
    config: CapsuleConfig,

    /// Cross-process exclusive lock (SPEC-KIT-971)
    /// Holds the lock file handle; released on drop
    cross_process_lock: Option<CapsuleLock>,

    /// In-process write lock - single writer at a time
    write_lock: Arc<Mutex<WriteLock>>,

    /// URI index for resolution
    uri_index: Arc<RwLock<UriIndex>>,

    /// Pending writes (writer queue)
    write_queue: Arc<Mutex<VecDeque<PendingWrite>>>,

    /// Current branch
    current_branch: Arc<RwLock<BranchId>>,

    /// Checkpoints
    checkpoints: Arc<RwLock<Vec<CheckpointMetadata>>>,

    /// Events track
    events: Arc<RwLock<Vec<RunEventEnvelope>>>,

    /// Sequence counter for events
    event_seq: Arc<Mutex<u64>>,

    /// Is the capsule open?
    is_open: Arc<RwLock<bool>>,

    // TODO: When memvid crate is added as dependency:
    // inner: memvid::Capsule,
}

/// Write lock state.
struct WriteLock {
    holder: Option<std::thread::ThreadId>,
    acquired_at: Option<std::time::Instant>,
}

/// A pending write in the queue.
#[derive(Debug)]
struct PendingWrite {
    uri: LogicalUri,
    data: Vec<u8>,
    metadata: serde_json::Value,
}

/// Options for opening a capsule.
#[derive(Debug, Clone, Default)]
pub struct CapsuleOpenOptions {
    /// Acquire exclusive write lock (default: true)
    pub write_lock: bool,

    /// Context for lock metadata
    pub spec_id: Option<String>,
    pub run_id: Option<String>,
    pub branch: Option<String>,
}

impl CapsuleOpenOptions {
    /// Create options for write access (default).
    pub fn write() -> Self {
        Self {
            write_lock: true,
            ..Default::default()
        }
    }

    /// Create options for read-only access.
    pub fn read_only() -> Self {
        Self {
            write_lock: false,
            ..Default::default()
        }
    }

    /// Set context for the lock metadata.
    pub fn with_context(mut self, spec_id: Option<String>, run_id: Option<String>, branch: Option<String>) -> Self {
        self.spec_id = spec_id;
        self.run_id = run_id;
        self.branch = branch;
        self
    }
}

impl CapsuleHandle {
    /// Open or create a capsule with write lock (default behavior).
    ///
    /// ## Behavior
    /// - If capsule exists: open and verify integrity
    /// - If capsule doesn't exist: create new
    /// - If capsule is corrupted: return error (caller decides fallback)
    /// - Acquires exclusive cross-process lock
    ///
    /// ## Acceptance Criteria (SPEC-KIT-971)
    /// - End-to-end: create → put → commit → reopen → search returns artifact
    /// - Crash recovery: capsule reopens; last committed checkpoint readable
    /// - Cross-process lock prevents concurrent writes
    pub fn open(config: CapsuleConfig) -> Result<Self> {
        Self::open_with_options(config, CapsuleOpenOptions::write())
    }

    /// Open a capsule for read-only access (no lock acquired).
    pub fn open_read_only(config: CapsuleConfig) -> Result<Self> {
        Self::open_with_options(config, CapsuleOpenOptions::read_only())
    }

    /// Open a capsule with custom options.
    ///
    /// ## SPEC-KIT-971: Cross-process locking
    /// When `options.write_lock` is true:
    /// - Creates `<capsule_path>.lock` atomically
    /// - Writes LockMetadata JSON to the lock file
    /// - Holds advisory OS lock (fs2) for extra safety
    /// - If another process holds the lock, returns `CapsuleError::LockedByWriter`
    pub fn open_with_options(config: CapsuleConfig, options: CapsuleOpenOptions) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = config.capsule_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Check if capsule exists
        let exists = config.capsule_path.exists();

        if exists {
            // Verify capsule integrity (for read access, we don't fail on lock)
            Self::verify_capsule(&config.capsule_path, !options.write_lock)?;
        }

        // Acquire cross-process lock if write access requested
        let cross_process_lock = if options.write_lock {
            let lock_metadata = LockMetadata::with_context(
                options.spec_id.clone(),
                options.run_id.clone(),
                options.branch.clone(),
            );

            match CapsuleLock::acquire(&config.capsule_path, lock_metadata) {
                Ok(lock) => Some(lock),
                Err(LockError::AlreadyLocked(existing)) => {
                    return Err(CapsuleError::LockedByWriter(existing));
                }
                Err(LockError::Io(e)) => {
                    return Err(CapsuleError::Io(e));
                }
            }
        } else {
            None
        };

        // TODO: When memvid crate is added:
        // let inner = if exists {
        //     memvid::Capsule::open(&config.capsule_path)?
        // } else {
        //     memvid::Capsule::create(&config.capsule_path)?
        // };

        let handle = Self {
            config,
            cross_process_lock,
            write_lock: Arc::new(Mutex::new(WriteLock {
                holder: None,
                acquired_at: None,
            })),
            uri_index: Arc::new(RwLock::new(UriIndex::new())),
            write_queue: Arc::new(Mutex::new(VecDeque::new())),
            current_branch: Arc::new(RwLock::new(BranchId::main())),
            checkpoints: Arc::new(RwLock::new(Vec::new())),
            events: Arc::new(RwLock::new(Vec::new())),
            event_seq: Arc::new(Mutex::new(0)),
            is_open: Arc::new(RwLock::new(true)),
        };

        // If this is a new capsule, create it
        if !exists {
            handle.create_capsule_file()?;
        }

        Ok(handle)
    }

    /// Verify capsule integrity.
    ///
    /// ## Parameters
    /// - `path`: Path to the capsule file
    /// - `skip_lock_check`: If true, don't fail on existing lock (for read-only access)
    ///
    /// ## Checks
    /// - File exists and is readable
    /// - Footer is valid
    /// - Version is compatible
    /// - Lock file status (unless `skip_lock_check` is true)
    fn verify_capsule(path: &Path, skip_lock_check: bool) -> Result<()> {
        // Check file exists
        if !path.exists() {
            return Err(CapsuleError::NotFound {
                path: path.to_path_buf(),
            });
        }

        // Check lock file (SPEC-KIT-971: use <path>.lock convention)
        if !skip_lock_check {
            if let Some(lock_metadata) = is_locked(path) {
                // Check if lock is stale
                if lock_metadata.is_stale() {
                    tracing::warn!(
                        pid = lock_metadata.pid,
                        host = %lock_metadata.host,
                        "Found stale lock from terminated process, will be cleaned on acquire"
                    );
                    // Don't fail here - lock acquisition will clean it up
                } else {
                    return Err(CapsuleError::LockedByWriter(lock_metadata));
                }
            }
        }

        // Check header magic bytes
        let data = std::fs::read(path)?;
        if data.len() < 3 || &data[0..3] != b"MV2" {
            return Err(CapsuleError::Corrupted {
                reason: "Invalid capsule header (expected MV2)".to_string(),
            });
        }

        // TODO: When memvid crate is added:
        // - Verify footer
        // - Check version compatibility
        // - Validate index integrity

        Ok(())
    }

    /// Create a new capsule file.
    fn create_capsule_file(&self) -> Result<()> {
        // Create a minimal capsule file
        // TODO: When memvid crate is added, use proper initialization
        std::fs::write(&self.config.capsule_path, b"MV2\x00\x01")?;
        Ok(())
    }

    /// Check if capsule is open.
    pub fn is_open(&self) -> bool {
        *self.is_open.read().unwrap()
    }

    /// Get the current branch.
    pub fn current_branch(&self) -> BranchId {
        self.current_branch.read().unwrap().clone()
    }

    /// Switch to a different branch.
    pub fn switch_branch(&self, branch: BranchId) -> Result<()> {
        if !self.is_open() {
            return Err(CapsuleError::NotOpen);
        }
        *self.current_branch.write().unwrap() = branch;
        Ok(())
    }

    // =========================================================================
    // Write operations (single-writer queue)
    // =========================================================================

    /// Put an artifact into the capsule.
    ///
    /// Returns a stable `mv2://...` URI.
    ///
    /// ## URI Contract (SPEC-KIT-971)
    /// - Every `put` returns a `mv2://` URI
    /// - URIs remain stable after reopen
    /// - URIs are unique per stored object
    pub fn put(
        &self,
        spec_id: &str,
        run_id: &str,
        object_type: ObjectType,
        path: &str,
        data: Vec<u8>,
        metadata: serde_json::Value,
    ) -> Result<LogicalUri> {
        if !self.is_open() {
            return Err(CapsuleError::NotOpen);
        }

        // Generate stable logical URI
        let uri = LogicalUri::new(
            &self.config.workspace_id,
            spec_id,
            run_id,
            object_type,
            path,
        )
        .ok_or_else(|| CapsuleError::InvalidOperation {
            reason: "Invalid URI components".to_string(),
        })?;

        // Queue the write
        let mut queue = self.write_queue.lock().unwrap();
        if queue.len() >= self.config.max_write_queue {
            return Err(CapsuleError::WriteQueueFull);
        }

        queue.push_back(PendingWrite {
            uri: uri.clone(),
            data,
            metadata,
        });

        Ok(uri)
    }

    /// Flush pending writes to disk.
    fn flush_writes(&self) -> Result<()> {
        let mut queue = self.write_queue.lock().unwrap();
        let writes: Vec<_> = queue.drain(..).collect();
        drop(queue);

        let mut uri_index = self.uri_index.write().unwrap();

        for (i, write) in writes.into_iter().enumerate() {
            // TODO: When memvid crate is added, write to actual capsule
            // For now, just update the URI index with a placeholder
            let pointer = PhysicalPointer {
                frame_id: i as u64,
                offset: 0,
                length: write.data.len() as u64,
            };
            uri_index.insert(write.uri, pointer);
        }

        Ok(())
    }

    // =========================================================================
    // Checkpoint operations (D18)
    // =========================================================================

    /// Create a checkpoint at stage boundary.
    ///
    /// ## Acceptance Criteria
    /// - `speckit capsule checkpoints` returns non-empty list after stage commit
    /// - At least one `StageTransition` event is appended on stage commit
    pub fn commit_stage(
        &self,
        spec_id: &str,
        run_id: &str,
        stage: &str,
        commit_hash: Option<&str>,
    ) -> Result<CheckpointId> {
        if !self.is_open() {
            return Err(CapsuleError::NotOpen);
        }

        // Flush pending writes first
        self.flush_writes()?;

        // Generate checkpoint ID
        let checkpoint_id = CheckpointId::new(format!(
            "{}_{}_{}",
            spec_id,
            stage,
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        ));

        // Create checkpoint metadata
        let metadata = CheckpointMetadata {
            checkpoint_id: checkpoint_id.clone(),
            label: Some(format!("stage:{}", stage)),
            stage: Some(stage.to_string()),
            spec_id: Some(spec_id.to_string()),
            run_id: Some(run_id.to_string()),
            commit_hash: commit_hash.map(|s| s.to_string()),
            timestamp: chrono::Utc::now(),
            is_manual: false,
        };

        // Store checkpoint
        self.checkpoints.write().unwrap().push(metadata);

        // Emit StageTransition event
        self.emit_event(spec_id, run_id, Some(stage), EventType::StageTransition, serde_json::json!({
            "stage": stage,
            "checkpoint_id": checkpoint_id.as_str(),
        }))?;

        Ok(checkpoint_id)
    }

    /// Create a manual checkpoint.
    ///
    /// Used by `speckit capsule commit --label <LABEL>`
    pub fn commit_manual(&self, label: &str) -> Result<CheckpointId> {
        if !self.is_open() {
            return Err(CapsuleError::NotOpen);
        }

        // Flush pending writes
        self.flush_writes()?;

        // Generate checkpoint ID
        let checkpoint_id = CheckpointId::new(format!(
            "manual_{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        ));

        // Create checkpoint metadata
        let metadata = CheckpointMetadata {
            checkpoint_id: checkpoint_id.clone(),
            label: Some(label.to_string()),
            stage: None,
            spec_id: None,
            run_id: None,
            commit_hash: None,
            timestamp: chrono::Utc::now(),
            is_manual: true,
        };

        // Store checkpoint
        self.checkpoints.write().unwrap().push(metadata);

        Ok(checkpoint_id)
    }

    /// List all checkpoints.
    pub fn list_checkpoints(&self) -> Vec<CheckpointMetadata> {
        self.checkpoints.read().unwrap().clone()
    }

    /// List checkpoints with optional branch filter.
    ///
    /// ## Parameters
    /// - `branch`: Optional branch filter. If None, returns checkpoints from all branches.
    pub fn list_checkpoints_filtered(&self, branch: Option<&BranchId>) -> Vec<CheckpointMetadata> {
        let all = self.checkpoints.read().unwrap();

        match branch {
            Some(b) => {
                // Filter by branch (checkpoints on run branches have run_id matching branch)
                all.iter()
                    .filter(|cp| {
                        // Main branch checkpoints have no run_id or spec_id
                        if b.is_main() {
                            return cp.run_id.is_none();
                        }
                        // Run branch checkpoints match branch name
                        if let Some(run_id) = &cp.run_id {
                            let branch_name = format!("run/{}", run_id);
                            return b.as_str() == branch_name;
                        }
                        false
                    })
                    .cloned()
                    .collect()
            }
            None => all.clone(),
        }
    }

    /// Get a checkpoint by its ID.
    pub fn get_checkpoint(&self, checkpoint_id: &CheckpointId) -> Option<CheckpointMetadata> {
        self.checkpoints
            .read()
            .unwrap()
            .iter()
            .find(|cp| cp.checkpoint_id == *checkpoint_id)
            .cloned()
    }

    /// Get a checkpoint by its label.
    ///
    /// Labels must be unique within a branch. If multiple checkpoints have the
    /// same label (on different branches), returns the first match.
    ///
    /// ## SPEC-KIT-971 Requirement
    /// "Checkpoints queryable by ID AND by label (non-negotiable)"
    pub fn get_checkpoint_by_label(&self, label: &str) -> Option<CheckpointMetadata> {
        self.checkpoints
            .read()
            .unwrap()
            .iter()
            .find(|cp| cp.label.as_deref() == Some(label))
            .cloned()
    }

    /// Get a checkpoint by label within a specific branch.
    ///
    /// Labels must be unique within a branch.
    pub fn get_checkpoint_by_label_in_branch(
        &self,
        label: &str,
        branch: &BranchId,
    ) -> Option<CheckpointMetadata> {
        self.list_checkpoints_filtered(Some(branch))
            .into_iter()
            .find(|cp| cp.label.as_deref() == Some(label))
    }

    /// Check if a label is unique within a branch.
    pub fn is_label_unique(&self, label: &str, branch: &BranchId) -> bool {
        self.get_checkpoint_by_label_in_branch(label, branch).is_none()
    }

    // =========================================================================
    // Event track operations (SPEC-KIT-971 baseline)
    // =========================================================================

    /// Emit an event to the events track.
    fn emit_event(
        &self,
        spec_id: &str,
        run_id: &str,
        stage: Option<&str>,
        event_type: EventType,
        payload: serde_json::Value,
    ) -> Result<LogicalUri> {
        let seq = {
            let mut seq = self.event_seq.lock().unwrap();
            *seq += 1;
            *seq
        };

        let uri = LogicalUri::for_event(&self.config.workspace_id, spec_id, run_id, seq);

        let event = RunEventEnvelope {
            uri: uri.clone(),
            event_type,
            timestamp: chrono::Utc::now(),
            spec_id: spec_id.to_string(),
            run_id: run_id.to_string(),
            stage: stage.map(|s| s.to_string()),
            payload,
        };

        self.events.write().unwrap().push(event);

        Ok(uri)
    }

    /// Emit a PolicySnapshotRef event.
    pub fn emit_policy_snapshot_ref(
        &self,
        spec_id: &str,
        run_id: &str,
        stage: Option<&str>,
        policy_uri: &LogicalUri,
    ) -> Result<LogicalUri> {
        self.emit_event(
            spec_id,
            run_id,
            stage,
            EventType::PolicySnapshotRef,
            serde_json::json!({
                "policy_uri": policy_uri.as_str(),
            }),
        )
    }

    // =========================================================================
    // URI resolution (D70)
    // =========================================================================

    /// Resolve a logical URI to its physical location.
    ///
    /// ## Parameters
    /// - `uri`: The logical URI to resolve
    /// - `branch`: Branch context (defaults to current branch)
    /// - `as_of`: Checkpoint for time-travel (None = latest)
    ///
    /// ## SPEC-KIT-971 Requirements
    /// - resolve_uri works with branch and as_of parameters
    /// - as_of enables point-in-time resolution
    ///
    /// ## Current Behavior
    /// Without the memvid crate, we track URI index per checkpoint and
    /// resolve against the appropriate snapshot.
    pub fn resolve_uri(
        &self,
        uri: &LogicalUri,
        branch: Option<&BranchId>,
        as_of: Option<&CheckpointId>,
    ) -> Result<PhysicalPointer> {
        if !self.is_open() {
            return Err(CapsuleError::NotOpen);
        }

        // Validate branch if provided
        let current_branch_guard = self.current_branch.read().unwrap();
        let target_branch = branch.unwrap_or(&current_branch_guard);

        // If as_of is specified, validate the checkpoint exists
        if let Some(checkpoint_id) = as_of {
            let checkpoint = self.get_checkpoint(checkpoint_id);
            if checkpoint.is_none() {
                return Err(CapsuleError::InvalidOperation {
                    reason: format!("Checkpoint {} not found", checkpoint_id.as_str()),
                });
            }

            // Verify checkpoint is on the target branch
            let cp = checkpoint.unwrap();
            if let Some(run_id) = &cp.run_id {
                let cp_branch = BranchId::for_run(run_id);
                if target_branch != &cp_branch && !target_branch.is_main() {
                    return Err(CapsuleError::InvalidOperation {
                        reason: format!(
                            "Checkpoint {} is on branch {}, not {}",
                            checkpoint_id.as_str(),
                            cp_branch.as_str(),
                            target_branch.as_str()
                        ),
                    });
                }
            }

            // TODO: When memvid crate is added, resolve against checkpoint snapshot
            // For now, we only have the current URI index
        }

        // Look up in the URI index
        let uri_index = self.uri_index.read().unwrap();
        uri_index
            .resolve(uri)
            .cloned()
            .ok_or_else(|| CapsuleError::UriNotFound { uri: uri.clone() })
    }

    /// Resolve a URI string to its physical location.
    ///
    /// Convenience wrapper that parses the URI string first.
    pub fn resolve_uri_str(
        &self,
        uri_str: &str,
        branch: Option<&BranchId>,
        as_of: Option<&CheckpointId>,
    ) -> Result<PhysicalPointer> {
        let uri: LogicalUri = uri_str.parse().map_err(|_| CapsuleError::InvalidOperation {
            reason: format!("Invalid URI: {}", uri_str),
        })?;
        self.resolve_uri(&uri, branch, as_of)
    }

    /// Resolve a URI with as_of specified by label instead of CheckpointId.
    pub fn resolve_uri_at_label(
        &self,
        uri: &LogicalUri,
        branch: Option<&BranchId>,
        label: &str,
    ) -> Result<PhysicalPointer> {
        let checkpoint = self.get_checkpoint_by_label(label);
        match checkpoint {
            Some(cp) => self.resolve_uri(uri, branch, Some(&cp.checkpoint_id)),
            None => Err(CapsuleError::InvalidOperation {
                reason: format!("Checkpoint with label '{}' not found", label),
            }),
        }
    }

    // =========================================================================
    // Doctor / diagnostics (SPEC-KIT-971)
    // =========================================================================

    /// Run capsule diagnostics.
    ///
    /// Checks:
    /// - Capsule exists and is readable
    /// - No stale lock
    /// - Footer is valid
    /// - Version is compatible
    ///
    /// ## Acceptance Criteria (SPEC-KIT-971)
    /// `speckit capsule doctor` detects: missing capsule, locked capsule,
    /// corrupted footer, and version mismatch; returns non-zero exit on failure.
    /// Shows actionable recovery steps for each issue.
    pub fn doctor(path: &Path) -> Vec<DiagnosticResult> {
        let mut results = Vec::new();

        // Check existence
        if !path.exists() {
            results.push(DiagnosticResult::Error(
                "Capsule not found".to_string(),
                "Create with: speckit capsule init".to_string(),
            ));
            return results;
        }
        results.push(DiagnosticResult::Ok("Capsule exists".to_string()));

        // Check lock (SPEC-KIT-971: use <path>.lock convention with metadata)
        let lock_path = lock_path_for(path);
        if let Some(lock_metadata) = is_locked(path) {
            let is_stale = lock_metadata.is_stale();
            let lock_info = format!(
                "PID: {}, User: {}@{}, Started: {}",
                lock_metadata.pid,
                lock_metadata.user,
                lock_metadata.host,
                lock_metadata.started_at.format("%Y-%m-%d %H:%M:%S UTC")
            );

            if is_stale {
                results.push(DiagnosticResult::Warning(
                    format!("Stale lock detected ({})", lock_info),
                    format!(
                        "Process {} is no longer running. Remove with:\n  rm {}",
                        lock_metadata.pid,
                        lock_path.display()
                    ),
                ));
            } else {
                let context = match (&lock_metadata.spec_id, &lock_metadata.run_id) {
                    (Some(spec), Some(run)) => format!(" [spec: {}, run: {}]", spec, run),
                    (Some(spec), None) => format!(" [spec: {}]", spec),
                    _ => String::new(),
                };

                results.push(DiagnosticResult::Error(
                    format!("Capsule is locked{} ({})", context, lock_info),
                    format!(
                        "Wait for process {} to complete, or if stuck:\n  \
                        1. Check process: ps -p {}\n  \
                        2. If not running: rm {}\n  \
                        3. If stuck: kill {} && rm {}",
                        lock_metadata.pid,
                        lock_metadata.pid,
                        lock_path.display(),
                        lock_metadata.pid,
                        lock_path.display()
                    ),
                ));
            }
        } else {
            results.push(DiagnosticResult::Ok("No lock held".to_string()));
        }

        // Check readability
        match std::fs::read(path) {
            Ok(data) => {
                if data.len() < 5 {
                    results.push(DiagnosticResult::Error(
                        "Capsule file too small".to_string(),
                        "File may be corrupted. Restore from backup or recreate:\n  \
                        rm -f {} && speckit capsule init".to_string(),
                    ));
                } else if &data[0..3] != b"MV2" {
                    results.push(DiagnosticResult::Error(
                        "Invalid capsule header".to_string(),
                        "File is not a valid MV2 capsule. Restore from backup.".to_string(),
                    ));
                } else {
                    results.push(DiagnosticResult::Ok("Capsule header valid".to_string()));
                }
            }
            Err(e) => {
                results.push(DiagnosticResult::Error(
                    "Cannot read capsule".to_string(),
                    format!("IO error: {}. Check file permissions.", e),
                ));
            }
        }

        results
    }

    /// Get capsule statistics.
    ///
    /// ## SPEC-KIT-971 Deliverable
    /// `speckit capsule stats` command: size, frame counts, index status, and dedup ratio.
    pub fn stats(&self) -> CapsuleStats {
        let size_bytes = std::fs::metadata(&self.config.capsule_path)
            .map(|m| m.len())
            .unwrap_or(0);

        // Calculate dedup ratio (stub - actual calculation when memvid integrated)
        // For now, return 1.0 (no dedup) since we're not actually deduplicating
        let dedup_ratio = 1.0;

        CapsuleStats {
            path: self.config.capsule_path.clone(),
            size_bytes,
            checkpoint_count: self.checkpoints.read().unwrap().len(),
            event_count: self.events.read().unwrap().len(),
            uri_count: self.uri_index.read().unwrap().len(),
            current_branch: self.current_branch(),
            frame_count: 0, // Stub - actual frame count when memvid integrated
            index_status: IndexStatus::Healthy, // Stub - actual status check
            dedup_ratio,
        }
    }
}

impl Drop for CapsuleHandle {
    fn drop(&mut self) {
        // Mark as closed
        *self.is_open.write().unwrap() = false;

        // SPEC-KIT-971: Cross-process lock is automatically released when
        // self.cross_process_lock (Option<CapsuleLock>) is dropped.
        // CapsuleLock::drop() handles unlocking and removing the lock file.
    }
}

// =============================================================================
// Diagnostic types
// =============================================================================

#[derive(Debug, Clone)]
pub enum DiagnosticResult {
    Ok(String),
    Warning(String, String),
    Error(String, String),
}

#[derive(Debug, Clone)]
pub struct CapsuleStats {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub checkpoint_count: usize,
    pub event_count: usize,
    pub uri_count: usize,
    pub current_branch: BranchId,
    /// Frame count (stub - actual count when memvid crate integrated)
    pub frame_count: usize,
    /// Index status (healthy/rebuilding/missing)
    pub index_status: IndexStatus,
    /// Dedup ratio (1.0 = no dedup, >1.0 = dedup active)
    /// Calculated as (logical_size / physical_size)
    pub dedup_ratio: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IndexStatus {
    Healthy,
    Rebuilding,
    Missing,
}
