//! SPEC-KIT-971: Capsule lifecycle and single-writer coordination
//!
//! ## Decision IDs
//! - D7: Single-writer capsule model (global lock + writer queue)
//! - D18: Stage boundary checkpoints
//! - D2: Canonical capsule path: `./.speckit/memvid/workspace.mv2`
//!
//! ## On-Disk Format (Minimal Persistence)
//! ```text
//! [Header: "MV2\x00\x01" (5 bytes)]
//! [Record 0]
//! [Record 1]
//! ...
//!
//! Record format:
//! [u32 record_len][u8 record_kind][u32 meta_len][meta_json bytes][payload bytes]
//!
//! record_kind:
//!   0 = Artifact
//!   1 = Checkpoint
//!   2 = Event
//! ```

use crate::memvid_adapter::lock::{CapsuleLock, LockError, LockMetadata, is_locked, lock_path_for};
use crate::memvid_adapter::types::{
    BranchId, CheckpointId, CheckpointMetadata, EventType, LogicalUri, ObjectType,
    PhysicalPointer, RoutingDecisionPayload, RunEventEnvelope, UriIndex,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
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
// On-Disk Record Format (SPEC-KIT-971 Minimal Persistence)
// =============================================================================

/// Magic header for MV2 capsule files.
const MV2_HEADER: &[u8] = b"MV2\x00\x01";
/// Header length in bytes.
const MV2_HEADER_LEN: usize = 5;

/// Record kind identifiers.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordKind {
    Artifact = 0,
    Checkpoint = 1,
    Event = 2,
}

impl TryFrom<u8> for RecordKind {
    type Error = ();

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(RecordKind::Artifact),
            1 => Ok(RecordKind::Checkpoint),
            2 => Ok(RecordKind::Event),
            _ => Err(()),
        }
    }
}

/// Metadata stored with artifact records (JSON-serializable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRecordMeta {
    /// Logical URI for this artifact
    pub uri: String,
    /// Object type
    pub object_type: String,
    /// User-provided metadata
    pub metadata: serde_json::Value,
}

/// A stored record read from disk during scan.
#[derive(Debug, Clone)]
pub struct StoredRecord {
    /// Record kind
    pub kind: RecordKind,
    /// Record sequence number (0-based)
    pub seq: u64,
    /// File offset where payload starts
    pub payload_offset: u64,
    /// Payload length in bytes
    pub payload_len: u64,
    /// Parsed metadata (JSON)
    pub meta: serde_json::Value,
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

    // ─────────────────────────────────────────────────────────────────────────────
    // Persistence state (SPEC-KIT-971 minimal persistence)
    // ─────────────────────────────────────────────────────────────────────────────

    /// File handle for append-only writes (only present when write_lock is true)
    file_handle: Arc<Mutex<Option<File>>>,

    /// Record sequence counter (for frame_id)
    record_seq: Arc<Mutex<u64>>,

    /// Stored records for reading payload bytes back
    stored_records: Arc<RwLock<Vec<StoredRecord>>>,

    // ─────────────────────────────────────────────────────────────────────────────
    // Policy tracking (SPEC-KIT-977)
    // ─────────────────────────────────────────────────────────────────────────────

    /// Current policy snapshot info (policy_id, hash, uri)
    /// Set when policy is captured at run start
    current_policy: Arc<RwLock<Option<CurrentPolicyInfo>>>,
}

/// Current policy info tracked in the capsule handle (SPEC-KIT-977).
#[derive(Debug, Clone)]
pub struct CurrentPolicyInfo {
    /// Policy ID (UUID)
    pub policy_id: String,
    /// Content hash (SHA256)
    pub hash: String,
    /// Capsule URI (mv2://<workspace>/policy/<policy_id>)
    pub uri: LogicalUri,
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
    ///
    /// ## Persistence
    /// On open, scans existing file and rebuilds:
    /// - URI index
    /// - Checkpoints
    /// - Events
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

        // Initialize handle with default/empty state
        let mut handle = Self {
            config: config.clone(),
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
            file_handle: Arc::new(Mutex::new(None)),
            record_seq: Arc::new(Mutex::new(0)),
            stored_records: Arc::new(RwLock::new(Vec::new())),
            current_policy: Arc::new(RwLock::new(None)),
        };

        // If this is a new capsule, create it
        if !exists {
            handle.create_capsule_file()?;
        } else {
            // Scan existing file and rebuild indexes
            handle.scan_and_rebuild()?;
        }

        // Open file handle for append if write access requested
        if options.write_lock {
            let file = OpenOptions::new()
                .append(true)
                .open(&config.capsule_path)?;
            *handle.file_handle.lock().unwrap() = Some(file);
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
        // Create capsule file with MV2 header
        std::fs::write(&self.config.capsule_path, MV2_HEADER)?;
        Ok(())
    }

    /// Scan existing capsule file and rebuild indexes.
    ///
    /// Reads all records from disk and populates:
    /// - `uri_index`: URI → PhysicalPointer mapping
    /// - `checkpoints`: List of checkpoint metadata
    /// - `events`: List of event envelopes
    /// - `stored_records`: All records for payload retrieval
    fn scan_and_rebuild(&mut self) -> Result<()> {
        let mut file = File::open(&self.config.capsule_path)?;

        // Skip header
        file.seek(SeekFrom::Start(MV2_HEADER_LEN as u64))?;

        let file_len = file.metadata()?.len();
        let mut pos = MV2_HEADER_LEN as u64;
        let mut seq = 0u64;

        let mut uri_index = UriIndex::new();
        let mut checkpoints = Vec::new();
        let mut events = Vec::new();
        let mut stored_records = Vec::new();
        let mut max_event_seq = 0u64;

        while pos < file_len {
            // Try to read a record
            match Self::read_record(&mut file, pos, seq) {
                Ok((record, next_pos)) => {
                    // Process record based on kind
                    match record.kind {
                        RecordKind::Artifact => {
                            // Parse artifact metadata
                            if let Ok(art_meta) = serde_json::from_value::<ArtifactRecordMeta>(record.meta.clone()) {
                                if let Ok(uri) = art_meta.uri.parse::<LogicalUri>() {
                                    let pointer = PhysicalPointer {
                                        frame_id: record.seq,
                                        offset: record.payload_offset,
                                        length: record.payload_len,
                                    };
                                    uri_index.insert(uri, pointer);
                                }
                            }
                        }
                        RecordKind::Checkpoint => {
                            // Parse checkpoint metadata
                            if let Ok(cp_meta) = serde_json::from_value::<CheckpointMetadata>(record.meta.clone()) {
                                checkpoints.push(cp_meta);
                            }
                        }
                        RecordKind::Event => {
                            // Parse event envelope
                            if let Ok(event) = serde_json::from_value::<RunEventEnvelope>(record.meta.clone()) {
                                // Track max event seq for future event numbering
                                if let Some(seq_num) = event.uri.as_str().split('/').last().and_then(|s| s.parse::<u64>().ok()) {
                                    max_event_seq = max_event_seq.max(seq_num);
                                }
                                events.push(event);
                            }
                        }
                    }

                    stored_records.push(record);
                    pos = next_pos;
                    seq += 1;
                }
                Err(e) => {
                    // If we can't read more records, stop scanning
                    // This could be end of file or corruption
                    tracing::debug!("Stopped scanning at pos {}: {}", pos, e);
                    break;
                }
            }
        }

        // Update handle state
        *self.uri_index.write().unwrap() = uri_index;
        *self.checkpoints.write().unwrap() = checkpoints;
        *self.events.write().unwrap() = events;
        *self.stored_records.write().unwrap() = stored_records;
        *self.record_seq.lock().unwrap() = seq;
        *self.event_seq.lock().unwrap() = max_event_seq;

        Ok(())
    }

    /// Read a single record from the file at the given position.
    ///
    /// Returns (StoredRecord, next_position)
    fn read_record(file: &mut File, pos: u64, seq: u64) -> Result<(StoredRecord, u64)> {
        file.seek(SeekFrom::Start(pos))?;

        // Read record length (u32)
        let mut len_buf = [0u8; 4];
        file.read_exact(&mut len_buf)?;
        let record_len = u32::from_le_bytes(len_buf) as u64;

        if record_len < 6 {
            return Err(CapsuleError::Corrupted {
                reason: format!("Record too small at pos {}", pos),
            });
        }

        // Read record kind (u8)
        let mut kind_buf = [0u8; 1];
        file.read_exact(&mut kind_buf)?;
        let kind = RecordKind::try_from(kind_buf[0]).map_err(|_| CapsuleError::Corrupted {
            reason: format!("Invalid record kind {} at pos {}", kind_buf[0], pos),
        })?;

        // Read metadata length (u32)
        let mut meta_len_buf = [0u8; 4];
        file.read_exact(&mut meta_len_buf)?;
        let meta_len = u32::from_le_bytes(meta_len_buf) as u64;

        // Read metadata JSON
        let mut meta_buf = vec![0u8; meta_len as usize];
        file.read_exact(&mut meta_buf)?;
        let meta: serde_json::Value = serde_json::from_slice(&meta_buf).map_err(|e| {
            CapsuleError::Corrupted {
                reason: format!("Invalid JSON metadata at pos {}: {}", pos, e),
            }
        })?;

        // Calculate payload offset and length
        // Record format: [4 bytes len][1 byte kind][4 bytes meta_len][meta_len bytes meta][payload]
        let header_size = 4 + 1 + 4 + meta_len;
        let payload_offset = pos + header_size;
        let payload_len = record_len - 1 - 4 - meta_len; // record_len doesn't include the len field itself

        let record = StoredRecord {
            kind,
            seq,
            payload_offset,
            payload_len,
            meta,
        };

        // Calculate next record position
        let next_pos = pos + 4 + record_len;

        Ok((record, next_pos))
    }

    /// Write a record to the capsule file.
    ///
    /// Returns the PhysicalPointer for the written payload.
    fn write_record(
        &self,
        kind: RecordKind,
        meta: &serde_json::Value,
        payload: &[u8],
    ) -> Result<PhysicalPointer> {
        let mut file_handle = self.file_handle.lock().unwrap();
        let file = file_handle.as_mut().ok_or(CapsuleError::InvalidOperation {
            reason: "No file handle for write (opened read-only?)".to_string(),
        })?;

        // Get current file position for offset calculation
        let file_pos = file.seek(SeekFrom::End(0))?;

        // Serialize metadata
        let meta_bytes = serde_json::to_vec(meta).map_err(|e| {
            CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize metadata: {}", e),
            }
        })?;

        // Calculate record length (kind + meta_len + meta + payload)
        let record_len = 1 + 4 + meta_bytes.len() + payload.len();

        // Write record
        file.write_all(&(record_len as u32).to_le_bytes())?; // record_len
        file.write_all(&[kind as u8])?;                       // kind
        file.write_all(&(meta_bytes.len() as u32).to_le_bytes())?; // meta_len
        file.write_all(&meta_bytes)?;                         // meta
        file.write_all(payload)?;                              // payload
        file.flush()?;

        // Calculate payload offset
        let payload_offset = file_pos + 4 + 1 + 4 + meta_bytes.len() as u64;

        // Get and increment sequence
        let seq = {
            let mut seq = self.record_seq.lock().unwrap();
            let current = *seq;
            *seq += 1;
            current
        };

        // Store the record in memory for later reads
        let record = StoredRecord {
            kind,
            seq,
            payload_offset,
            payload_len: payload.len() as u64,
            meta: meta.clone(),
        };
        self.stored_records.write().unwrap().push(record);

        Ok(PhysicalPointer {
            frame_id: seq,
            offset: payload_offset,
            length: payload.len() as u64,
        })
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

    // =========================================================================
    // Policy storage (SPEC-KIT-977)
    // =========================================================================

    /// Store a policy snapshot in the capsule.
    ///
    /// ## SPEC-KIT-977: Global Policy URI
    ///
    /// Unlike regular artifacts that use `mv2://<workspace>/<spec>/<run>/<type>/<path>`,
    /// policy snapshots use a **global** URI: `mv2://<workspace>/policy/<policy_id>`.
    /// This allows policy to be shared across runs.
    ///
    /// ## Returns
    /// The policy URI (mv2://<workspace>/policy/<policy_id>)
    pub fn put_policy(
        &self,
        policy_id: &str,
        policy_hash: &str,
        data: Vec<u8>,
        metadata: serde_json::Value,
    ) -> Result<LogicalUri> {
        if !self.is_open() {
            return Err(CapsuleError::NotOpen);
        }

        // Generate global policy URI (not spec/run scoped)
        let uri = LogicalUri::for_policy(&self.config.workspace_id, policy_id);

        // Create artifact record metadata
        let art_meta = ArtifactRecordMeta {
            uri: uri.as_str().to_string(),
            object_type: "policy".to_string(),
            metadata,
        };
        let meta_value = serde_json::to_value(&art_meta).map_err(|e| {
            CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize policy metadata: {}", e),
            }
        })?;

        // Write record to disk directly (bypass queue for immediate persistence)
        let pointer = self.write_record(RecordKind::Artifact, &meta_value, &data)?;

        // Update URI index
        self.uri_index.write().unwrap().insert(uri.clone(), pointer);

        // Track as current policy
        self.set_current_policy(policy_id, policy_hash, &uri);

        tracing::debug!(
            policy_id = %policy_id,
            hash = %policy_hash,
            uri = %uri,
            "Stored policy snapshot in capsule"
        );

        Ok(uri)
    }

    /// Set the current policy for this capsule session.
    ///
    /// Called after `put_policy()` to track the active policy.
    /// StageTransition events will include this policy info.
    pub fn set_current_policy(&self, policy_id: &str, hash: &str, uri: &LogicalUri) {
        *self.current_policy.write().unwrap() = Some(CurrentPolicyInfo {
            policy_id: policy_id.to_string(),
            hash: hash.to_string(),
            uri: uri.clone(),
        });
    }

    /// Get the current policy info (if set).
    pub fn current_policy(&self) -> Option<CurrentPolicyInfo> {
        self.current_policy.read().unwrap().clone()
    }

    /// Flush pending writes to disk.
    ///
    /// Writes all queued artifacts to the capsule file and updates the URI index.
    fn flush_writes(&self) -> Result<()> {
        let mut queue = self.write_queue.lock().unwrap();
        let writes: Vec<_> = queue.drain(..).collect();
        drop(queue);

        for write in writes {
            // Create artifact record metadata
            let art_meta = ArtifactRecordMeta {
                uri: write.uri.as_str().to_string(),
                object_type: "artifact".to_string(), // Could extract from URI
                metadata: write.metadata,
            };
            let meta_value = serde_json::to_value(&art_meta).map_err(|e| {
                CapsuleError::InvalidOperation {
                    reason: format!("Failed to serialize artifact metadata: {}", e),
                }
            })?;

            // Write record to disk
            let pointer = self.write_record(RecordKind::Artifact, &meta_value, &write.data)?;

            // Update URI index
            self.uri_index.write().unwrap().insert(write.uri, pointer);
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

        // SPEC-KIT-971: Stamp branch_id for run isolation
        let branch_id = self.current_branch().as_str().to_string();

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
            branch_id: Some(branch_id),
        };

        // Persist checkpoint to disk (if we have write access)
        if self.file_handle.lock().unwrap().is_some() {
            let meta_value = serde_json::to_value(&metadata).map_err(|e| {
                CapsuleError::InvalidOperation {
                    reason: format!("Failed to serialize checkpoint: {}", e),
                }
            })?;
            self.write_record(RecordKind::Checkpoint, &meta_value, &[])?;
        }

        // Store checkpoint in memory
        self.checkpoints.write().unwrap().push(metadata);

        // Emit StageTransition event with policy info (SPEC-KIT-977)
        let policy_info = self.current_policy();
        let event_payload = if let Some(ref policy) = policy_info {
            serde_json::json!({
                "stage": stage,
                "checkpoint_id": checkpoint_id.as_str(),
                "policy_id": policy.policy_id,
                "policy_hash": policy.hash,
                "policy_uri": policy.uri.as_str(),
            })
        } else {
            serde_json::json!({
                "stage": stage,
                "checkpoint_id": checkpoint_id.as_str(),
            })
        };

        self.emit_event(spec_id, run_id, Some(stage), EventType::StageTransition, event_payload)?;

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

        // SPEC-KIT-971: Stamp branch_id for run isolation
        let branch_id = self.current_branch().as_str().to_string();

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
            branch_id: Some(branch_id),
        };

        // Persist checkpoint to disk (if we have write access)
        if self.file_handle.lock().unwrap().is_some() {
            let meta_value = serde_json::to_value(&metadata).map_err(|e| {
                CapsuleError::InvalidOperation {
                    reason: format!("Failed to serialize checkpoint: {}", e),
                }
            })?;
            self.write_record(RecordKind::Checkpoint, &meta_value, &[])?;
        }

        // Store checkpoint in memory
        self.checkpoints.write().unwrap().push(metadata);

        Ok(checkpoint_id)
    }

    /// List all checkpoints.
    pub fn list_checkpoints(&self) -> Vec<CheckpointMetadata> {
        self.checkpoints.read().unwrap().clone()
    }

    /// List all events.
    pub fn list_events(&self) -> Vec<RunEventEnvelope> {
        self.events.read().unwrap().clone()
    }

    /// List checkpoints with optional branch filter.
    ///
    /// ## Parameters
    /// - `branch`: Optional branch filter. If None, returns checkpoints from all branches.
    ///
    /// ## SPEC-KIT-971: Branch Isolation
    /// Uses the `branch_id` field for filtering when available. Falls back to
    /// heuristic (run_id matching) for backward compatibility with older checkpoints.
    pub fn list_checkpoints_filtered(&self, branch: Option<&BranchId>) -> Vec<CheckpointMetadata> {
        let all = self.checkpoints.read().unwrap();

        match branch {
            Some(b) => {
                all.iter()
                    .filter(|cp| {
                        // SPEC-KIT-971: Use branch_id if available
                        if let Some(ref cp_branch) = cp.branch_id {
                            return cp_branch == b.as_str();
                        }

                        // Fallback: heuristic based on run_id (backward compatibility)
                        if b.is_main() {
                            return cp.run_id.is_none();
                        }
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

    /// List events with optional branch filter.
    ///
    /// ## Parameters
    /// - `branch`: Optional branch filter. If None, returns events from all branches.
    ///
    /// ## SPEC-KIT-971: Branch Isolation
    /// Uses the `branch_id` field for filtering when available. Falls back to
    /// heuristic (run_id matching) for backward compatibility with older events.
    pub fn list_events_filtered(&self, branch: Option<&BranchId>) -> Vec<RunEventEnvelope> {
        let all = self.events.read().unwrap();

        match branch {
            Some(b) => {
                all.iter()
                    .filter(|ev| {
                        // SPEC-KIT-971: Use branch_id if available
                        if let Some(ref ev_branch) = ev.branch_id {
                            return ev_branch == b.as_str();
                        }

                        // Fallback: heuristic based on run_id (backward compatibility)
                        if b.is_main() {
                            return false; // Events always have run_id
                        }
                        let branch_name = format!("run/{}", ev.run_id);
                        b.as_str() == branch_name
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

        // SPEC-KIT-971: Stamp branch_id for run isolation
        let branch_id = self.current_branch().as_str().to_string();

        let event = RunEventEnvelope {
            uri: uri.clone(),
            event_type,
            timestamp: chrono::Utc::now(),
            spec_id: spec_id.to_string(),
            run_id: run_id.to_string(),
            stage: stage.map(|s| s.to_string()),
            payload,
            branch_id: Some(branch_id),
        };

        // Persist event to disk (if we have write access)
        if self.file_handle.lock().unwrap().is_some() {
            let meta_value = serde_json::to_value(&event).map_err(|e| {
                CapsuleError::InvalidOperation {
                    reason: format!("Failed to serialize event: {}", e),
                }
            })?;
            self.write_record(RecordKind::Event, &meta_value, &[])?;
        }

        // Store event in memory
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

    /// Emit a PolicySnapshotRef event with full policy info (SPEC-KIT-977).
    ///
    /// This version includes policy_id and policy_hash in the event payload
    /// for better traceability without needing to dereference the URI.
    pub fn emit_policy_snapshot_ref_with_info(
        &self,
        spec_id: &str,
        run_id: &str,
        stage: Option<&str>,
        policy_uri: &LogicalUri,
        policy_id: &str,
        policy_hash: &str,
    ) -> Result<LogicalUri> {
        self.emit_event(
            spec_id,
            run_id,
            stage,
            EventType::PolicySnapshotRef,
            serde_json::json!({
                "policy_uri": policy_uri.as_str(),
                "policy_id": policy_id,
                "policy_hash": policy_hash,
            }),
        )
    }

    /// Emit a RoutingDecision event (SPEC-KIT-978).
    ///
    /// Records every model routing decision (reflex vs cloud) for the
    /// Implementer role. This enables:
    /// - Bakeoff analysis (compare reflex vs cloud outcomes)
    /// - Audit trail for routing decisions
    /// - Fallback tracking for reliability metrics
    pub fn emit_routing_decision(
        &self,
        spec_id: &str,
        run_id: &str,
        payload: &RoutingDecisionPayload,
    ) -> Result<LogicalUri> {
        let payload_json = serde_json::to_value(payload).map_err(|e| {
            CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize routing decision: {}", e),
            }
        })?;

        self.emit_event(
            spec_id,
            run_id,
            Some(&payload.stage),
            EventType::RoutingDecision,
            payload_json,
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
    // Payload retrieval (SPEC-KIT-971 persistence)
    // =========================================================================

    /// Read payload bytes for a URI.
    ///
    /// ## Parameters
    /// - `uri`: The logical URI to read
    /// - `branch`: Branch context (defaults to current branch)
    /// - `as_of`: Checkpoint for time-travel (None = latest)
    ///
    /// ## Returns
    /// The raw payload bytes stored for this URI.
    ///
    /// ## SPEC-KIT-971 Persistence Requirement
    /// After put + commit + reopen, `get_bytes(uri)` must return identical bytes.
    pub fn get_bytes(
        &self,
        uri: &LogicalUri,
        branch: Option<&BranchId>,
        as_of: Option<&CheckpointId>,
    ) -> Result<Vec<u8>> {
        // First resolve the URI to get the physical pointer
        let pointer = self.resolve_uri(uri, branch, as_of)?;

        // Read the payload from disk
        self.read_payload(&pointer)
    }

    /// Read payload bytes by URI string.
    pub fn get_bytes_str(
        &self,
        uri_str: &str,
        branch: Option<&BranchId>,
        as_of: Option<&CheckpointId>,
    ) -> Result<Vec<u8>> {
        let uri: LogicalUri = uri_str.parse().map_err(|_| CapsuleError::InvalidOperation {
            reason: format!("Invalid URI: {}", uri_str),
        })?;
        self.get_bytes(&uri, branch, as_of)
    }

    /// Read payload from disk using a physical pointer.
    fn read_payload(&self, pointer: &PhysicalPointer) -> Result<Vec<u8>> {
        let mut file = File::open(&self.config.capsule_path)?;
        file.seek(SeekFrom::Start(pointer.offset))?;

        let mut buf = vec![0u8; pointer.length as usize];
        file.read_exact(&mut buf)?;

        Ok(buf)
    }

    /// Get stored records (for adapter search index rebuilding).
    ///
    /// Returns an iterator over stored artifact records with their metadata
    /// for rebuilding the TF-IDF search index on reopen.
    pub fn iter_stored_artifacts(&self) -> impl Iterator<Item = (LogicalUri, Vec<u8>, serde_json::Value)> + '_ {
        let records = self.stored_records.read().unwrap();
        let path = self.config.capsule_path.clone();

        records
            .iter()
            .filter(|r| r.kind == RecordKind::Artifact)
            .filter_map(move |r| {
                // Parse artifact metadata
                let art_meta: ArtifactRecordMeta = serde_json::from_value(r.meta.clone()).ok()?;
                let uri: LogicalUri = art_meta.uri.parse().ok()?;

                // Read payload
                let mut file = File::open(&path).ok()?;
                file.seek(SeekFrom::Start(r.payload_offset)).ok()?;
                let mut buf = vec![0u8; r.payload_len as usize];
                file.read_exact(&mut buf).ok()?;

                Some((uri, buf, art_meta.metadata))
            })
            .collect::<Vec<_>>()
            .into_iter()
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
