//! SPEC-KIT-971: Capsule lifecycle and single-writer coordination
//!
//! ## Decision IDs
//! - D7: Single-writer capsule model (global lock + writer queue)
//! - D18: Stage boundary checkpoints
//! - D2: Canonical capsule path: `./.speckit/memvid/workspace.mv2`
//!
//! Note: `result_large_err` is allowed because CapsuleError::LockedByWriter
//! intentionally contains full LockMetadata for debugging contention issues.
#![allow(clippy::result_large_err)]

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
//!   3 = UriIndexSnapshot (SPEC-KIT-971: time-travel resolution)
//!   4 = Manifest (metadata-only; not indexed)
//! ```

use crate::memvid_adapter::lock::{CapsuleLock, LockError, LockMetadata, is_locked, lock_path_for};
use crate::memvid_adapter::types::{
    BranchId, CapsuleExportedPayload, CapsuleImportedPayload, CheckpointId, CheckpointMetadata,
    ErrorEventPayload, EventType, GateDecisionPayload, IntakeCompletedPayload, LogicalUri,
    MergeMode, ModelCallEnvelopePayload, ObjectType, PatchApplyPayload, PhysicalPointer,
    RetrievalRequestPayload, RetrievalResponsePayload, RoutingDecisionPayload, RunEventEnvelope,
    ToolCallPayload, ToolResultPayload, UriIndex, UriIndexSnapshot,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use thiserror::Error;

// S974-003: Encryption imports (secrecy re-exported from age)
// Note: age uses its own secrecy re-export internally

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

    /// SPEC-KIT-971: Duplicate label within branch
    #[error("Label '{label}' already exists on branch '{branch}'. Use --force to override.")]
    DuplicateLabel { label: String, branch: String },

    /// S974-003: Passphrase required but not provided in non-interactive context
    #[error(
        "Passphrase required for encrypted export/import (set SPECKIT_MEMVID_PASSPHRASE or use interactive mode)"
    )]
    PassphraseRequired,

    /// S974-003: Decryption failed due to invalid passphrase
    #[error("Invalid passphrase for encrypted capsule")]
    InvalidPassphrase,

    /// S974-003: Age encryption/decryption error
    #[error("Encryption error: {reason}")]
    EncryptionError { reason: String },

    /// S974-mount: Invalid mount name (path traversal or invalid characters)
    #[error("Invalid mount name '{name}': {reason}")]
    InvalidMountName { name: String, reason: String },

    /// S974-mount: Mount already exists with different content
    #[error(
        "Mount '{name}' already exists with different content. Use --mount-as <different_name> or unmount first."
    )]
    MountHashConflict { name: String },

    /// S974-mount: Registry corruption or parse error
    #[error("Mounts registry error: {reason}")]
    MountsRegistryError { reason: String },
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
    /// SPEC-KIT-971: URI index snapshot for time-travel resolution
    UriIndexSnapshot = 3,
    /// SPEC-KIT-974: Export manifest (metadata only, not indexed)
    Manifest = 4,
}

impl TryFrom<u8> for RecordKind {
    type Error = ();

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(RecordKind::Artifact),
            1 => Ok(RecordKind::Checkpoint),
            2 => Ok(RecordKind::Event),
            3 => Ok(RecordKind::UriIndexSnapshot),
            4 => Ok(RecordKind::Manifest),
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
    // Stored for Drop/RAII - not read directly
    #[allow(dead_code)]
    cross_process_lock: Option<CapsuleLock>,

    /// In-process write lock - single writer at a time
    // Stored for Drop/RAII - not read directly
    #[allow(dead_code)]
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

    // ─────────────────────────────────────────────────────────────────────────────
    // S974-003: Encrypted capsule support
    // ─────────────────────────────────────────────────────────────────────────────
    /// Temp directory for decrypted .mv2e files (cleaned up on drop)
    /// When Some, this handle was opened from an encrypted file
    decrypted_temp_dir: Option<tempfile::TempDir>,
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
// Diagnostic fields - reserved for future lock contention debugging
#[allow(dead_code)]
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
    pub fn with_context(
        mut self,
        spec_id: Option<String>,
        run_id: Option<String>,
        branch: Option<String>,
    ) -> Self {
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

    /// S974-003: Open an encrypted capsule (.mv2e) for read-only access.
    ///
    /// ## Process
    /// 1. Read encrypted file
    /// 2. Decrypt using age passphrase
    /// 3. Write to temp file
    /// 4. Open temp file as read-only capsule
    ///
    /// ## Security
    /// - Wrong passphrase returns `CapsuleError::InvalidPassphrase`
    /// - No partial plaintext written on failure (decryption happens in memory)
    /// - Temp file is cleaned up when handle is dropped
    ///
    /// ## Parameters
    /// - `path`: Path to the encrypted .mv2e file
    /// - `passphrase`: The passphrase to decrypt the file
    pub fn open_encrypted(path: &Path, passphrase: &secrecy::SecretString) -> Result<Self> {
        use secrecy::ExposeSecret;

        // Read encrypted file
        let encrypted_data = std::fs::read(path)?;

        // Decrypt using age
        let decryptor = age::Decryptor::new(&encrypted_data[..]).map_err(|e| {
            CapsuleError::EncryptionError {
                reason: format!("Invalid age format: {}", e),
            }
        })?;

        // Verify it's a passphrase-encrypted file
        if !decryptor.is_scrypt() {
            return Err(CapsuleError::InvalidOperation {
                reason: "Not a passphrase-encrypted file (recipient key detected)".into(),
            });
        }

        // Create passphrase identity for decryption
        let identity = age::scrypt::Identity::new(secrecy::SecretString::from(
            passphrase.expose_secret().to_string(),
        ));

        // Decrypt content (all in memory to avoid partial plaintext on failure)
        let mut decrypted = vec![];
        let mut reader = decryptor
            .decrypt(std::iter::once(&identity as &dyn age::Identity))
            .map_err(|_| CapsuleError::InvalidPassphrase)?;
        std::io::Read::read_to_end(&mut reader, &mut decrypted)?;

        // Write decrypted content to temp file
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().join("decrypted.mv2");
        std::fs::write(&temp_path, &decrypted)?;

        tracing::debug!(
            encrypted_path = %path.display(),
            decrypted_size = decrypted.len(),
            "Decrypted encrypted capsule"
        );

        // Create config pointing to temp file
        let config = CapsuleConfig {
            capsule_path: temp_path,
            workspace_id: "decrypted".to_string(), // Will be overwritten from manifest
            ..Default::default()
        };

        // Open as read-only
        let mut handle = Self::open_with_options(config, CapsuleOpenOptions::read_only())?;

        // Store temp dir to keep it alive (cleaned up on drop)
        handle.decrypted_temp_dir = Some(temp_dir);

        Ok(handle)
    }

    /// S974-003: Open an encrypted capsule with passphrase from environment or prompt.
    ///
    /// Convenience wrapper that handles passphrase acquisition.
    pub fn open_encrypted_interactive(path: &Path, interactive: bool) -> Result<Self> {
        let passphrase = Self::acquire_passphrase(interactive)?;
        Self::open_encrypted(path, &passphrase)
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
            decrypted_temp_dir: None,
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
            let file = OpenOptions::new().append(true).open(&config.capsule_path)?;
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
                            if let Ok(art_meta) =
                                serde_json::from_value::<ArtifactRecordMeta>(record.meta.clone())
                            {
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
                            if let Ok(cp_meta) =
                                serde_json::from_value::<CheckpointMetadata>(record.meta.clone())
                            {
                                checkpoints.push(cp_meta);
                            }
                        }
                        RecordKind::Event => {
                            // Parse event envelope
                            if let Ok(event) =
                                serde_json::from_value::<RunEventEnvelope>(record.meta.clone())
                            {
                                // Track max event seq for future event numbering
                                if let Some(seq_num) = event
                                    .uri
                                    .as_str()
                                    .split('/')
                                    .next_back()
                                    .and_then(|s| s.parse::<u64>().ok())
                                {
                                    max_event_seq = max_event_seq.max(seq_num);
                                }
                                events.push(event);
                            }
                        }
                        RecordKind::UriIndexSnapshot => {
                            // SPEC-KIT-971: Restore URI index snapshot for time-travel
                            if let Ok(snapshot) =
                                serde_json::from_value::<UriIndexSnapshot>(record.meta.clone())
                            {
                                uri_index.import_snapshot(snapshot);
                            }
                        }
                        RecordKind::Manifest => {
                            // SPEC-KIT-974: Export manifest is metadata-only, skip indexing
                            // Manifests contain export provenance info but don't need to be
                            // stored in the in-memory record list.
                            tracing::debug!("Skipping manifest record at seq {}", seq);
                        }
                    }

                    // Store all records except Manifest (which is metadata-only)
                    if record.kind != RecordKind::Manifest {
                        stored_records.push(record);
                    }
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

        // SPEC-KIT-971: Restore branch entries from latest snapshots
        // This ensures resolve_uri(branch, as_of=None) works correctly after reopen
        // by reconstructing the "current state" for each branch from its latest checkpoint.
        uri_index.restore_entries_from_latest_snapshots(&checkpoints);

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
        let meta: serde_json::Value =
            serde_json::from_slice(&meta_buf).map_err(|e| CapsuleError::Corrupted {
                reason: format!("Invalid JSON metadata at pos {}: {}", pos, e),
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
        let meta_bytes = serde_json::to_vec(meta).map_err(|e| CapsuleError::InvalidOperation {
            reason: format!("Failed to serialize metadata: {}", e),
        })?;

        // Calculate record length (kind + meta_len + meta + payload)
        let record_len = 1 + 4 + meta_bytes.len() + payload.len();

        // Write record
        file.write_all(&(record_len as u32).to_le_bytes())?; // record_len
        file.write_all(&[kind as u8])?; // kind
        file.write_all(&(meta_bytes.len() as u32).to_le_bytes())?; // meta_len
        file.write_all(&meta_bytes)?; // meta
        file.write_all(payload)?; // payload
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
        let meta_value =
            serde_json::to_value(&art_meta).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize policy metadata: {}", e),
            })?;

        // Write record to disk directly (bypass queue for immediate persistence)
        let pointer = self.write_record(RecordKind::Artifact, &meta_value, &data)?;

        // Update URI index (branch-aware for SPEC-KIT-971 time-travel)
        let branch = self.current_branch();
        self.uri_index
            .write()
            .unwrap()
            .insert_on_branch(&branch, uri.clone(), pointer);

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

        // Get current branch for branch-aware insert (SPEC-KIT-971)
        let branch = self.current_branch();

        for write in writes {
            // Create artifact record metadata
            let art_meta = ArtifactRecordMeta {
                uri: write.uri.as_str().to_string(),
                object_type: "artifact".to_string(), // Could extract from URI
                metadata: write.metadata,
            };
            let meta_value =
                serde_json::to_value(&art_meta).map_err(|e| CapsuleError::InvalidOperation {
                    reason: format!("Failed to serialize artifact metadata: {}", e),
                })?;

            // Write record to disk
            let pointer = self.write_record(RecordKind::Artifact, &meta_value, &write.data)?;

            // Update URI index (branch-aware for SPEC-KIT-971 time-travel)
            self.uri_index
                .write()
                .unwrap()
                .insert_on_branch(&branch, write.uri, pointer);
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
            let meta_value =
                serde_json::to_value(&metadata).map_err(|e| CapsuleError::InvalidOperation {
                    reason: format!("Failed to serialize checkpoint: {}", e),
                })?;
            self.write_record(RecordKind::Checkpoint, &meta_value, &[])?;
        }

        // Store checkpoint in memory
        self.checkpoints.write().unwrap().push(metadata);

        // SPEC-KIT-971: Create and persist URI index snapshot for time-travel
        let branch = self.current_branch();
        {
            let mut uri_index = self.uri_index.write().unwrap();
            uri_index.snapshot(&branch, &checkpoint_id);

            // Persist snapshot to disk for reopen
            if self.file_handle.lock().unwrap().is_some() {
                if let Some(snapshot) = uri_index.export_snapshot(&branch, &checkpoint_id) {
                    let snapshot_value = serde_json::to_value(&snapshot).map_err(|e| {
                        CapsuleError::InvalidOperation {
                            reason: format!("Failed to serialize URI index snapshot: {}", e),
                        }
                    })?;
                    self.write_record(RecordKind::UriIndexSnapshot, &snapshot_value, &[])?;
                }
            }
        }

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

        self.emit_event(
            spec_id,
            run_id,
            Some(stage),
            EventType::StageTransition,
            event_payload,
        )?;

        Ok(checkpoint_id)
    }

    /// Create a manual checkpoint.
    ///
    /// Used by `speckit capsule commit --label <LABEL>`
    ///
    /// Labels must be unique within the current branch. Use `commit_manual_force`
    /// to override uniqueness check.
    pub fn commit_manual(&self, label: &str) -> Result<CheckpointId> {
        self.commit_manual_with_options(label, false)
    }

    /// Create a manual checkpoint, optionally forcing duplicate labels.
    ///
    /// ## Parameters
    /// - `label`: The checkpoint label
    /// - `force`: If true, allows creating checkpoints with duplicate labels
    ///
    /// ## SPEC-KIT-971 Label Uniqueness
    /// Labels must be unique within the current branch by default.
    /// Use `force=true` to override (e.g., via `--force` CLI flag).
    pub fn commit_manual_with_options(&self, label: &str, force: bool) -> Result<CheckpointId> {
        if !self.is_open() {
            return Err(CapsuleError::NotOpen);
        }

        // SPEC-KIT-971: Enforce label uniqueness within branch (unless forced)
        let branch = self.current_branch();
        if !force && !self.is_label_unique(label, &branch) {
            return Err(CapsuleError::DuplicateLabel {
                label: label.to_string(),
                branch: branch.as_str().to_string(),
            });
        }

        // Flush pending writes
        self.flush_writes()?;

        // Generate checkpoint ID
        let checkpoint_id = CheckpointId::new(format!(
            "manual_{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        ));

        // SPEC-KIT-971: Stamp branch_id for run isolation
        let branch_id = branch.as_str().to_string();

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
            let meta_value =
                serde_json::to_value(&metadata).map_err(|e| CapsuleError::InvalidOperation {
                    reason: format!("Failed to serialize checkpoint: {}", e),
                })?;
            self.write_record(RecordKind::Checkpoint, &meta_value, &[])?;
        }

        // Store checkpoint in memory
        self.checkpoints.write().unwrap().push(metadata);

        // SPEC-KIT-971: Create and persist URI index snapshot for time-travel
        let branch = self.current_branch();
        {
            let mut uri_index = self.uri_index.write().unwrap();
            uri_index.snapshot(&branch, &checkpoint_id);

            // Persist snapshot to disk for reopen
            if self.file_handle.lock().unwrap().is_some() {
                if let Some(snapshot) = uri_index.export_snapshot(&branch, &checkpoint_id) {
                    let snapshot_value = serde_json::to_value(&snapshot).map_err(|e| {
                        CapsuleError::InvalidOperation {
                            reason: format!("Failed to serialize URI index snapshot: {}", e),
                        }
                    })?;
                    self.write_record(RecordKind::UriIndexSnapshot, &snapshot_value, &[])?;
                }
            }
        }

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
                // If filtering for main, also include events from merged branches
                // based on BranchMerged events (which track what was merged and how)
                let merged_branches = if b.is_main() {
                    self.get_merged_branches_info()
                } else {
                    Vec::new()
                };

                all.iter()
                    .filter(|ev| {
                        // SPEC-KIT-971: Use branch_id if available
                        if let Some(ref ev_branch) = ev.branch_id {
                            if ev_branch == b.as_str() {
                                return true;
                            }

                            // For main branch, check if this event is from a merged branch
                            if b.is_main() {
                                for (from_branch, mode) in &merged_branches {
                                    if ev_branch == from_branch.as_str() {
                                        // In curated mode, only include curated-eligible events
                                        // In full mode, include all events
                                        return match mode {
                                            MergeMode::Full => true,
                                            MergeMode::Curated => {
                                                ev.event_type.is_curated_eligible()
                                            }
                                        };
                                    }
                                }
                            }
                            return false;
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

    /// Get list of branches that have been merged to main and their merge modes.
    ///
    /// Returns Vec<(from_branch, MergeMode)> based on BranchMerged events.
    fn get_merged_branches_info(&self) -> Vec<(BranchId, MergeMode)> {
        let all = self.events.read().unwrap();
        all.iter()
            .filter(|ev| ev.event_type == EventType::BranchMerged)
            .filter(|ev| ev.branch_id.as_deref() == Some("main"))
            .filter_map(|ev| {
                // Parse the BranchMerged payload to get from_branch and mode
                let from_branch = ev.payload.get("from_branch")?.as_str()?;
                let mode_str = ev.payload.get("mode")?.as_str()?;
                let mode = match mode_str {
                    "curated" | "Curated" => MergeMode::Curated,
                    "full" | "Full" => MergeMode::Full,
                    _ => return None,
                };
                Some((BranchId::new(from_branch), mode))
            })
            .collect()
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
        self.get_checkpoint_by_label_in_branch(label, branch)
            .is_none()
    }

    // =========================================================================
    // Branch merge operations (SPEC-KIT-971: Merge at Unlock)
    // =========================================================================

    /// Merge a run branch into main.
    ///
    /// ## SPEC-KIT-971 Invariant
    /// - Merge modes are `curated` or `full` only (never squash, ff, or rebase)
    /// - Objects created on run branch become resolvable on main after merge
    /// - A BranchMerged event is emitted
    /// - A merge checkpoint is created
    ///
    /// ## Parameters
    /// - `from`: Source branch (e.g., `BranchId::for_run("run123")`)
    /// - `to`: Target branch (should be `BranchId::main()`)
    /// - `mode`: Merge mode (`Curated` or `Full`)
    /// - `spec_id`: Optional spec ID for event metadata
    /// - `run_id`: Optional run ID for event metadata
    ///
    /// ## Returns
    /// - Checkpoint ID for the merge checkpoint
    pub fn merge_branch(
        &self,
        from: &BranchId,
        to: &BranchId,
        mode: MergeMode,
        spec_id: Option<&str>,
        run_id: Option<&str>,
    ) -> Result<CheckpointId> {
        use crate::memvid_adapter::types::BranchMergedPayload;

        if !self.is_open() {
            return Err(CapsuleError::NotOpen);
        }

        // Validate: target must be main
        if !to.is_main() {
            return Err(CapsuleError::InvalidOperation {
                reason: format!("Merge target must be main branch, got: {}", to.as_str()),
            });
        }

        // Validate: source must be a run branch
        if !from.is_run_branch() {
            return Err(CapsuleError::InvalidOperation {
                reason: format!("Merge source must be a run branch, got: {}", from.as_str()),
            });
        }

        // Merge URI mappings based on mode
        let uris_merged = {
            let mut uri_index = self.uri_index.write().unwrap();
            uri_index.merge_branch(from, to, mode) as u64
        };

        // Count events that would be merged based on mode.
        // Events themselves keep their original branch_id - the merge is tracked
        // via the BranchMerged event, and list_events_filtered uses that to
        // include merged events when filtering for main.
        // - Curated: Only curated-eligible events (StageTransition, PolicySnapshotRef, etc.)
        // - Full: All events
        let events_merged = {
            let events = self.events.read().unwrap();
            events
                .iter()
                .filter(|ev| ev.branch_id.as_deref() == Some(from.as_str()))
                .filter(|ev| match mode {
                    MergeMode::Full => true,
                    MergeMode::Curated => ev.event_type.is_curated_eligible(),
                })
                .count() as u64
        };

        // Create merge checkpoint
        let checkpoint_id = CheckpointId::new(format!(
            "merge_{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        ));

        let checkpoint_metadata = CheckpointMetadata {
            checkpoint_id: checkpoint_id.clone(),
            label: Some(format!("merge:{}", from.as_str())),
            stage: Some("Unlock".to_string()),
            spec_id: spec_id.map(|s| s.to_string()),
            run_id: run_id.map(|r| r.to_string()),
            commit_hash: None,
            timestamp: chrono::Utc::now(),
            is_manual: false,
            branch_id: Some(to.as_str().to_string()),
        };

        // Persist checkpoint to disk
        if self.file_handle.lock().unwrap().is_some() {
            let meta_value = serde_json::to_value(&checkpoint_metadata).map_err(|e| {
                CapsuleError::InvalidOperation {
                    reason: format!("Failed to serialize checkpoint: {}", e),
                }
            })?;
            self.write_record(RecordKind::Checkpoint, &meta_value, &[])?;
        }

        // Store checkpoint in memory
        self.checkpoints.write().unwrap().push(checkpoint_metadata);

        // Create URI index snapshot for the merge checkpoint on main
        {
            let mut uri_index = self.uri_index.write().unwrap();
            uri_index.snapshot(to, &checkpoint_id);

            // Persist snapshot to disk
            if self.file_handle.lock().unwrap().is_some() {
                if let Some(snapshot) = uri_index.export_snapshot(to, &checkpoint_id) {
                    let snapshot_value = serde_json::to_value(&snapshot).map_err(|e| {
                        CapsuleError::InvalidOperation {
                            reason: format!("Failed to serialize URI index snapshot: {}", e),
                        }
                    })?;
                    self.write_record(RecordKind::UriIndexSnapshot, &snapshot_value, &[])?;
                }
            }
        }

        // Emit BranchMerged event
        let payload = BranchMergedPayload {
            from_branch: from.as_str().to_string(),
            to_branch: to.as_str().to_string(),
            mode,
            merge_checkpoint_id: checkpoint_id.as_str().to_string(),
            uris_merged,
            events_merged,
            spec_id: spec_id.map(|s| s.to_string()),
            run_id: run_id.map(|r| r.to_string()),
        };

        let payload_value =
            serde_json::to_value(&payload).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize BranchMerged payload: {}", e),
            })?;

        // Emit on target branch (main) explicitly, not current branch
        self.emit_event_on_branch(
            spec_id.unwrap_or("unknown"),
            run_id.unwrap_or("unknown"),
            Some("Unlock"),
            EventType::BranchMerged,
            payload_value,
            Some(to),
        )?;

        Ok(checkpoint_id)
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
        // Use current branch by default
        self.emit_event_on_branch(spec_id, run_id, stage, event_type, payload, None)
    }

    /// Emit an event to the events track on a specific branch.
    ///
    /// ## Parameters
    /// - `branch`: Target branch. If None, uses current branch.
    fn emit_event_on_branch(
        &self,
        spec_id: &str,
        run_id: &str,
        stage: Option<&str>,
        event_type: EventType,
        payload: serde_json::Value,
        branch: Option<&BranchId>,
    ) -> Result<LogicalUri> {
        let seq = {
            let mut seq = self.event_seq.lock().unwrap();
            *seq += 1;
            *seq
        };

        let uri = LogicalUri::for_event(&self.config.workspace_id, spec_id, run_id, seq);

        // SPEC-KIT-971: Stamp branch_id for run isolation
        let branch_id = branch
            .map(|b| b.as_str().to_string())
            .unwrap_or_else(|| self.current_branch().as_str().to_string());

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
            let meta_value =
                serde_json::to_value(&event).map_err(|e| CapsuleError::InvalidOperation {
                    reason: format!("Failed to serialize event: {}", e),
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

    /// Emit an IntakeCompleted event.
    ///
    /// Records completion of the "Architect-in-a-box" intake flow, including
    /// the capsule URIs and hashes for raw answers + normalized brief.
    pub fn emit_intake_completed(
        &self,
        spec_id: &str,
        run_id: &str,
        payload: &IntakeCompletedPayload,
    ) -> Result<LogicalUri> {
        let payload_json =
            serde_json::to_value(payload).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize IntakeCompleted payload: {}", e),
            })?;

        self.emit_event(
            spec_id,
            run_id,
            None,
            EventType::IntakeCompleted,
            payload_json,
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
        let payload_json =
            serde_json::to_value(payload).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize routing decision: {}", e),
            })?;

        self.emit_event(
            spec_id,
            run_id,
            Some(&payload.stage),
            EventType::RoutingDecision,
            payload_json,
        )
    }

    /// Emit a DebugTrace event.
    ///
    /// DebugTrace events are debug/telemetry events that are excluded from
    /// curated merge. They remain on the run branch for audit purposes but
    /// do not propagate to main branch in curated mode.
    ///
    /// Used for verbose debugging, performance tracing, and other non-essential
    /// observability data that should not clutter the main branch history.
    pub fn emit_debug_trace(
        &self,
        spec_id: &str,
        run_id: &str,
        stage: Option<&str>,
        message: &str,
        context: serde_json::Value,
    ) -> Result<LogicalUri> {
        self.emit_event(
            spec_id,
            run_id,
            stage,
            EventType::DebugTrace,
            serde_json::json!({
                "message": message,
                "context": context,
            }),
        )
    }

    // =========================================================================
    // SPEC-KIT-975: Replayable Audit Event Helpers
    // =========================================================================

    /// Emit a ToolCall event (SPEC-KIT-975).
    ///
    /// Records tool invocations for audit trail and replay.
    pub fn emit_tool_call(
        &self,
        spec_id: &str,
        run_id: &str,
        payload: &ToolCallPayload,
    ) -> Result<LogicalUri> {
        let payload_json =
            serde_json::to_value(payload).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize tool call: {}", e),
            })?;

        self.emit_event(
            spec_id,
            run_id,
            payload.stage.as_deref(),
            EventType::ToolCall,
            payload_json,
        )
    }

    /// Emit a ToolResult event (SPEC-KIT-975).
    ///
    /// Records tool outputs for audit trail and replay.
    pub fn emit_tool_result(
        &self,
        spec_id: &str,
        run_id: &str,
        stage: Option<&str>,
        payload: &ToolResultPayload,
    ) -> Result<LogicalUri> {
        let payload_json =
            serde_json::to_value(payload).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize tool result: {}", e),
            })?;

        self.emit_event(spec_id, run_id, stage, EventType::ToolResult, payload_json)
    }

    /// Emit a RetrievalRequest event (SPEC-KIT-975).
    ///
    /// Records retrieval queries for replay verification.
    pub fn emit_retrieval_request(
        &self,
        spec_id: &str,
        run_id: &str,
        payload: &RetrievalRequestPayload,
    ) -> Result<LogicalUri> {
        let payload_json =
            serde_json::to_value(payload).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize retrieval request: {}", e),
            })?;

        self.emit_event(
            spec_id,
            run_id,
            payload.stage.as_deref(),
            EventType::RetrievalRequest,
            payload_json,
        )
    }

    /// Emit a RetrievalResponse event (SPEC-KIT-975).
    ///
    /// Records retrieval results for replay verification.
    pub fn emit_retrieval_response(
        &self,
        spec_id: &str,
        run_id: &str,
        stage: Option<&str>,
        payload: &RetrievalResponsePayload,
    ) -> Result<LogicalUri> {
        let payload_json =
            serde_json::to_value(payload).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize retrieval response: {}", e),
            })?;

        self.emit_event(
            spec_id,
            run_id,
            stage,
            EventType::RetrievalResponse,
            payload_json,
        )
    }

    /// Emit a PatchApply event (SPEC-KIT-975).
    ///
    /// Records file modifications for audit trail.
    pub fn emit_patch_apply(
        &self,
        spec_id: &str,
        run_id: &str,
        payload: &PatchApplyPayload,
    ) -> Result<LogicalUri> {
        let payload_json =
            serde_json::to_value(payload).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize patch apply: {}", e),
            })?;

        self.emit_event(
            spec_id,
            run_id,
            payload.stage.as_deref(),
            EventType::PatchApply,
            payload_json,
        )
    }

    /// Emit a GateDecision event (SPEC-KIT-975).
    ///
    /// Records governance gate outcomes for compliance audit.
    pub fn emit_gate_decision(
        &self,
        spec_id: &str,
        run_id: &str,
        payload: &GateDecisionPayload,
    ) -> Result<LogicalUri> {
        let payload_json =
            serde_json::to_value(payload).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize gate decision: {}", e),
            })?;

        self.emit_event(
            spec_id,
            run_id,
            Some(&payload.stage),
            EventType::GateDecision,
            payload_json,
        )
    }

    /// Emit an ErrorEvent (SPEC-KIT-975).
    ///
    /// Records errors during run execution.
    pub fn emit_error_event(
        &self,
        spec_id: &str,
        run_id: &str,
        payload: &ErrorEventPayload,
    ) -> Result<LogicalUri> {
        let payload_json =
            serde_json::to_value(payload).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize error event: {}", e),
            })?;

        self.emit_event(
            spec_id,
            run_id,
            payload.stage.as_deref(),
            EventType::ErrorEvent,
            payload_json,
        )
    }

    /// Emit a ModelCallEnvelope event (SPEC-KIT-975).
    ///
    /// Records LLM request/response based on capture mode.
    /// Content fields are populated according to LLMCaptureMode.
    pub fn emit_model_call_envelope(
        &self,
        spec_id: &str,
        run_id: &str,
        payload: &ModelCallEnvelopePayload,
    ) -> Result<LogicalUri> {
        let payload_json =
            serde_json::to_value(payload).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize model call envelope: {}", e),
            })?;

        self.emit_event(
            spec_id,
            run_id,
            payload.stage.as_deref(),
            EventType::ModelCallEnvelope,
            payload_json,
        )
    }

    /// Emit a CapsuleExported event (SPEC-KIT-975).
    ///
    /// Tracks when a capsule is exported for provenance.
    pub fn emit_capsule_exported(
        &self,
        spec_id: &str,
        run_id: &str,
        payload: &CapsuleExportedPayload,
    ) -> Result<LogicalUri> {
        let payload_json =
            serde_json::to_value(payload).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize capsule exported: {}", e),
            })?;

        self.emit_event(
            spec_id,
            run_id,
            None,
            EventType::CapsuleExported,
            payload_json,
        )
    }

    /// Emit a CapsuleImported event (SPEC-KIT-975).
    ///
    /// Tracks when a capsule is imported for provenance.
    pub fn emit_capsule_imported(
        &self,
        spec_id: &str,
        run_id: &str,
        payload: &CapsuleImportedPayload,
    ) -> Result<LogicalUri> {
        let payload_json =
            serde_json::to_value(payload).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize capsule imported: {}", e),
            })?;

        self.emit_event(
            spec_id,
            run_id,
            None,
            EventType::CapsuleImported,
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

            // Verify checkpoint is on the target branch (if checkpoint has branch info)
            let cp = checkpoint.unwrap();
            if let Some(ref cp_branch_str) = cp.branch_id {
                let cp_branch = BranchId::new(cp_branch_str);
                if target_branch != &cp_branch {
                    return Err(CapsuleError::InvalidOperation {
                        reason: format!(
                            "Checkpoint {} is on branch {}, not {}",
                            checkpoint_id.as_str(),
                            cp_branch.as_str(),
                            target_branch.as_str()
                        ),
                    });
                }
            } else if let Some(run_id) = &cp.run_id {
                // Fallback for older checkpoints without explicit branch_id
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
        }

        // SPEC-KIT-971: Time-travel resolution using UriIndex snapshots
        let uri_index = self.uri_index.read().unwrap();
        uri_index
            .resolve_on_branch(uri, target_branch, as_of)
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
        let uri: LogicalUri = uri_str
            .parse()
            .map_err(|_| CapsuleError::InvalidOperation {
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
        let uri: LogicalUri = uri_str
            .parse()
            .map_err(|_| CapsuleError::InvalidOperation {
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
    pub fn iter_stored_artifacts(
        &self,
    ) -> impl Iterator<Item = (LogicalUri, Vec<u8>, serde_json::Value)> + '_ {
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

        // SPEC-KIT-974: Detect encrypted capsules (.mv2e)
        let is_encrypted = path.extension().map(|e| e == "mv2e").unwrap_or(false);

        // Check readability and header
        match std::fs::read(path) {
            Ok(data) => {
                if data.len() < 5 {
                    results.push(DiagnosticResult::Error(
                        "Capsule file too small".to_string(),
                        "File may be corrupted. Restore from backup or recreate:\n  \
                        rm -f {} && speckit capsule init"
                            .to_string(),
                    ));
                } else if is_encrypted {
                    // P0-3: For encrypted capsules, check age header and try to verify with env var
                    // Age encrypted files start with "age-encryption.org" header
                    let has_age_header = data.starts_with(b"age-encryption.org");

                    if !has_age_header {
                        results.push(DiagnosticResult::Error(
                            "Invalid encrypted capsule header".to_string(),
                            "File has .mv2e extension but doesn't have valid age encryption header.".to_string(),
                        ));
                    } else {
                        results.push(DiagnosticResult::Ok(
                            "Encrypted capsule header valid (age)".to_string(),
                        ));

                        // Try to verify decrypted content if passphrase available via env var
                        // (no interactive prompt in doctor - use env var only)
                        if let Ok(pass) = std::env::var("SPECKIT_MEMVID_PASSPHRASE") {
                            if !pass.is_empty() {
                                let passphrase = secrecy::SecretString::from(pass);
                                match Self::open_encrypted(path, &passphrase) {
                                    Ok(_handle) => {
                                        results.push(DiagnosticResult::Ok(
                                            "Decryption verified (passphrase correct)".to_string(),
                                        ));
                                    }
                                    Err(CapsuleError::InvalidPassphrase) => {
                                        results.push(DiagnosticResult::Error(
                                            "Decryption failed - wrong passphrase".to_string(),
                                            "Check SPECKIT_MEMVID_PASSPHRASE value.".to_string(),
                                        ));
                                    }
                                    Err(e) => {
                                        results.push(DiagnosticResult::Warning(
                                            format!("Decryption verification failed: {}", e),
                                            "Capsule may be corrupted or passphrase incorrect."
                                                .to_string(),
                                        ));
                                    }
                                }
                            } else {
                                results.push(DiagnosticResult::Warning(
                                    "Encrypted capsule - cannot fully verify without passphrase"
                                        .to_string(),
                                    "Set SPECKIT_MEMVID_PASSPHRASE to verify decryption."
                                        .to_string(),
                                ));
                            }
                        } else {
                            results.push(DiagnosticResult::Warning(
                                "Encrypted capsule - cannot fully verify without passphrase"
                                    .to_string(),
                                "Set SPECKIT_MEMVID_PASSPHRASE to verify decryption.".to_string(),
                            ));
                        }
                    }
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

    // =========================================================================
    // S974-003: Passphrase Acquisition
    // =========================================================================

    /// Acquire passphrase for encryption/decryption.
    ///
    /// ## Passphrase acquisition order (per S974-003 requirements):
    /// 1. `SPECKIT_MEMVID_PASSPHRASE` environment variable
    /// 2. Interactive prompt (if `interactive` is true)
    /// 3. Return `CapsuleError::PassphraseRequired` if neither available
    ///
    /// ## Security
    /// - Passphrase is stored in `secrecy::SecretString` to prevent accidental logging
    /// - Memory is zeroized on drop
    fn acquire_passphrase(interactive: bool) -> Result<secrecy::SecretString> {
        use secrecy::SecretString;

        // 1. Check environment variable first
        if let Ok(pass) = std::env::var("SPECKIT_MEMVID_PASSPHRASE") {
            if !pass.is_empty() {
                return Ok(SecretString::from(pass));
            }
        }

        // 2. Interactive prompt if allowed
        if interactive {
            match rpassword::prompt_password("Capsule passphrase: ") {
                Ok(pass) if !pass.is_empty() => return Ok(SecretString::from(pass)),
                Ok(_) => {
                    // Empty passphrase entered
                    return Err(CapsuleError::PassphraseRequired);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to read passphrase from terminal");
                    return Err(CapsuleError::PassphraseRequired);
                }
            }
        }

        // 3. No passphrase available
        Err(CapsuleError::PassphraseRequired)
    }

    // =========================================================================
    // SPEC-KIT-974: Capsule Export (.mv2 and .mv2e)
    // =========================================================================

    /// Export a subset of the capsule to a single .mv2 or .mv2e file.
    ///
    /// ## SPEC-KIT-974 MVP Deliverables
    /// - Export produces a single file artifact (.mv2 or .mv2e)
    /// - Includes run artifacts, events, checkpoints, and manifest
    /// - Emits CapsuleExported event into workspace capsule
    ///
    /// ## S974-003: Encryption
    /// - When `options.encrypt` is true, output is .mv2e (age passphrase encrypted)
    /// - Passphrase from SPECKIT_MEMVID_PASSPHRASE env var or interactive prompt
    /// - content_hash is SHA256 of the encrypted file (post-encryption)
    ///
    /// ## Parameters
    /// - `options`: Export configuration (run filter, output path, safe mode, encrypt)
    ///
    /// ## Returns
    /// - `ExportResult` with path, digest, and stats
    ///
    /// ## Acceptance Criteria
    /// - Export produces a single file artifact with no sidecar files
    /// - Every export writes a `CapsuleExported` event into the workspace capsule
    pub fn export(&self, options: &ExportOptions) -> Result<ExportResult> {
        if !self.is_open() {
            return Err(CapsuleError::NotOpen);
        }

        // Determine final output path (adjust extension based on encryption)
        let final_output_path = if options.encrypt {
            let mut path = options.output_path.clone();
            path.set_extension("mv2e");
            path
        } else {
            let mut path = options.output_path.clone();
            // Ensure .mv2 extension for unencrypted
            if path.extension().is_none_or(|e| e != "mv2") {
                path.set_extension("mv2");
            }
            path
        };

        // Validate output path
        if let Some(parent) = final_output_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        // Collect records to export based on filter
        let records_to_export = self.collect_export_records(options)?;
        let checkpoints_to_export = self.collect_export_checkpoints(options);
        let events_to_export = self.collect_export_events(options);

        // Create export manifest
        let manifest = ExportManifest {
            version: "1.0.0".to_string(),
            exported_at: chrono::Utc::now(),
            source_capsule_path: self.config.capsule_path.display().to_string(),
            workspace_id: self.config.workspace_id.clone(),
            filter: ExportFilter {
                spec_id: options.spec_id.clone(),
                run_id: options.run_id.clone(),
                branch: options.branch.as_ref().map(|b| b.as_str().to_string()),
            },
            artifact_count: records_to_export.len() as u64,
            checkpoint_count: checkpoints_to_export.len() as u64,
            event_count: events_to_export.len() as u64,
            safe_mode: options.safe_mode,
            encrypted: options.encrypt,
        };

        // Write export file (encryption handled separately)
        let (bytes_written, content_hash) = if options.encrypt {
            self.write_encrypted_export(&final_output_path, &manifest, &records_to_export, options)?
        } else {
            self.write_export_file(&final_output_path, &manifest, &records_to_export, options)?
        };

        // Build result
        let result = ExportResult {
            output_path: final_output_path.clone(),
            bytes_written,
            content_hash: content_hash.clone(),
            artifact_count: records_to_export.len() as u64,
            checkpoint_count: checkpoints_to_export.len() as u64,
            event_count: events_to_export.len() as u64,
        };

        // Emit CapsuleExported event (SPEC-KIT-974 requirement)
        let payload = CapsuleExportedPayload {
            destination_type: "file".to_string(),
            destination: Some(final_output_path.display().to_string()),
            format: if options.encrypt {
                "mv2e-v1".to_string()
            } else {
                "mv2-v1".to_string()
            },
            checkpoints_included: checkpoints_to_export
                .iter()
                .map(|cp| cp.checkpoint_id.as_str().to_string())
                .collect(),
            sanitized: options.safe_mode,
            encrypted: options.encrypt,
            exported_at: chrono::Utc::now(),
            content_hash: Some(content_hash),
        };

        // Emit event using spec_id/run_id from options or defaults
        let spec_id = options.spec_id.as_deref().unwrap_or("export");
        let run_id = options.run_id.as_deref().unwrap_or("manual");
        self.emit_capsule_exported(spec_id, run_id, &payload)?;

        tracing::info!(
            output = %final_output_path.display(),
            artifacts = result.artifact_count,
            checkpoints = result.checkpoint_count,
            events = result.event_count,
            bytes = bytes_written,
            encrypted = options.encrypt,
            "Capsule exported successfully"
        );

        Ok(result)
    }

    // =========================================================================
    // Mount Persistence Helpers (SPEC-KIT-974)
    // =========================================================================

    /// Validate mount name for security and filesystem safety.
    ///
    /// ## S974-mount: Security requirements
    /// - No path traversal: `/`, `\`, `..` are forbidden
    /// - Only alphanumeric, underscore, hyphen allowed
    /// - Non-empty, reasonable length (1-64 chars)
    fn validate_mount_name(name: &str) -> Result<()> {
        // Check for empty
        if name.is_empty() {
            return Err(CapsuleError::InvalidMountName {
                name: name.to_string(),
                reason: "Mount name cannot be empty".to_string(),
            });
        }

        // Check length
        if name.len() > 64 {
            return Err(CapsuleError::InvalidMountName {
                name: name.to_string(),
                reason: "Mount name exceeds 64 characters".to_string(),
            });
        }

        // Check for path traversal patterns
        if name.contains('/') || name.contains('\\') || name.contains("..") {
            return Err(CapsuleError::InvalidMountName {
                name: name.to_string(),
                reason: "Path traversal characters not allowed (/, \\, ..)".to_string(),
            });
        }

        // Validate character set: ^[a-zA-Z0-9_-]+$
        let valid = name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !valid {
            return Err(CapsuleError::InvalidMountName {
                name: name.to_string(),
                reason: "Only alphanumeric, underscore, and hyphen allowed".to_string(),
            });
        }

        Ok(())
    }

    /// Get the canonical mounts directory path.
    ///
    /// Returns `./.speckit/memvid/mounts/` relative to the capsule's parent directory.
    fn mounts_dir(&self) -> PathBuf {
        self.config
            .capsule_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("mounts")
    }

    /// Get the registry file path.
    ///
    /// Returns `./.speckit/memvid/mounts.json`
    fn mounts_registry_path(&self) -> PathBuf {
        self.config
            .capsule_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("mounts.json")
    }

    /// Load the mounts registry, creating an empty one if it doesn't exist.
    fn load_mounts_registry(&self) -> Result<MountsRegistry> {
        let registry_path = self.mounts_registry_path();

        if !registry_path.exists() {
            return Ok(MountsRegistry::new());
        }

        let content = std::fs::read_to_string(&registry_path)?;
        serde_json::from_str(&content).map_err(|e| CapsuleError::MountsRegistryError {
            reason: format!("Failed to parse mounts.json: {}", e),
        })
    }

    /// Save the mounts registry atomically (temp file + rename).
    fn save_mounts_registry(&self, registry: &MountsRegistry) -> Result<()> {
        let registry_path = self.mounts_registry_path();
        let parent = registry_path.parent().unwrap_or_else(|| Path::new("."));

        // Ensure parent directory exists
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }

        // Serialize to JSON
        let content = serde_json::to_string_pretty(registry).map_err(|e| {
            CapsuleError::MountsRegistryError {
                reason: format!("Failed to serialize mounts.json: {}", e),
            }
        })?;

        // Atomic write: temp file + rename
        let temp_path = registry_path.with_extension("json.tmp");
        std::fs::write(&temp_path, content)?;
        std::fs::rename(&temp_path, &registry_path)?;

        Ok(())
    }

    /// Copy a file atomically to the mounts directory.
    ///
    /// Uses temp file + rename pattern to ensure no partial writes.
    fn copy_to_mounts_atomic(&self, source: &Path, mount_name: &str) -> Result<PathBuf> {
        let mounts_dir = self.mounts_dir();

        // Ensure mounts directory exists
        if !mounts_dir.exists() {
            std::fs::create_dir_all(&mounts_dir)?;
        }

        // Determine extension from source
        let extension = source
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_else(|| "mv2".to_string());

        let dest_path = mounts_dir.join(format!("{}.{}", mount_name, extension));
        let temp_path = mounts_dir.join(format!("{}.{}.tmp", mount_name, extension));

        // Copy to temp file
        std::fs::copy(source, &temp_path)?;

        // Atomic rename
        std::fs::rename(&temp_path, &dest_path)?;

        Ok(dest_path)
    }

    /// Check if mount already exists and whether it's the same content.
    ///
    /// Returns:
    /// - `Ok(None)` if mount doesn't exist (proceed with copy)
    /// - `Ok(Some(entry))` if mount exists with matching hash (skip copy, reuse)
    /// - `Err(MountHashConflict)` if mount exists with different hash
    fn check_mount_idempotency(
        &self,
        mount_name: &str,
        content_hash: &str,
    ) -> Result<Option<MountEntry>> {
        let registry = self.load_mounts_registry()?;

        match registry.mounts.get(mount_name) {
            Some(existing) => {
                if existing.content_hash == content_hash {
                    // Same content - idempotent success
                    tracing::info!(
                        mount_name = %mount_name,
                        "Mount already exists with matching content hash"
                    );
                    Ok(Some(existing.clone()))
                } else {
                    // Different content - conflict
                    Err(CapsuleError::MountHashConflict {
                        name: mount_name.to_string(),
                    })
                }
            }
            None => Ok(None),
        }
    }

    /// Internal helper to gather stats from source capsule.
    fn gather_source_stats_internal(
        &self,
        options: &ImportOptions,
        is_encrypted: bool,
    ) -> Result<(u64, u64, u64, Vec<String>)> {
        if is_encrypted {
            let passphrase = Self::acquire_passphrase(options.interactive)?;
            let source_handle = Self::open_encrypted(&options.source_path, &passphrase)?;
            let stats = source_handle.stats();
            let checkpoints = source_handle.list_checkpoints();
            let checkpoint_ids: Vec<String> = checkpoints
                .iter()
                .map(|cp| cp.checkpoint_id.to_string())
                .collect();
            Ok((
                stats.uri_count as u64,
                stats.checkpoint_count as u64,
                stats.event_count as u64,
                checkpoint_ids,
            ))
        } else {
            let source_config = CapsuleConfig {
                capsule_path: options.source_path.clone(),
                workspace_id: "imported".to_string(),
                ..Default::default()
            };
            match Self::open_read_only(source_config) {
                Ok(source_handle) => {
                    let stats = source_handle.stats();
                    let checkpoints = source_handle.list_checkpoints();
                    let checkpoint_ids: Vec<String> = checkpoints
                        .iter()
                        .map(|cp| cp.checkpoint_id.to_string())
                        .collect();
                    Ok((
                        stats.uri_count as u64,
                        stats.checkpoint_count as u64,
                        stats.event_count as u64,
                        checkpoint_ids,
                    ))
                }
                Err(_) => Ok((0, 0, 0, Vec::new())),
            }
        }
    }

    /// SPEC-KIT-974 (SK974-1): Import an external capsule as a read-only mount.
    ///
    /// ## Decision IDs
    /// - D103: Imported capsules read-only (attach without mutating workspace memory)
    /// - D104: Auto-register mounted capsules (immediately discoverable)
    /// - D70: Import verification (doctor checks)
    ///
    /// ## Process (S974-mount)
    /// 1. Validate source path exists
    /// 2. Determine + validate mount name (security check)
    /// 3. Run doctor verification
    /// 4. Compute content hash for provenance
    /// 5. Check idempotency (skip if already mounted with same hash)
    /// 6. Open source capsule read-only to gather stats
    /// 7. Atomic copy to mounts directory
    /// 8. Update mounts registry atomically
    /// 9. Emit CapsuleImported event
    /// 10. Rollback on failure
    ///
    /// ## Acceptance Criteria
    /// - Import on a second machine reproduces identical retrieval results
    /// - `speckit capsule import` MUST run doctor checks before mounting
    /// - Warn on unsigned/unverified capsules; hard-fail if `--require-verified`
    /// - Every import writes a `CapsuleImported` event with provenance metadata
    /// - Mount file persisted at `.speckit/memvid/mounts/<NAME>.mv2{e}`
    /// - Registry at `.speckit/memvid/mounts.json` with atomic writes
    pub fn import(&self, options: &ImportOptions) -> Result<ImportResult> {
        if !self.is_open() {
            return Err(CapsuleError::NotOpen);
        }

        // === 1. Validate source exists ===
        if !options.source_path.exists() {
            return Err(CapsuleError::NotFound {
                path: options.source_path.clone(),
            });
        }

        // === 2. Determine and validate mount name ===
        let mount_name = options.mount_as.clone().unwrap_or_else(|| {
            options
                .source_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "imported".to_string())
        });

        // S974-mount: Validate mount name (security)
        Self::validate_mount_name(&mount_name)?;

        // === 3. Run doctor verification first (D70, D104) ===
        let doctor_results = Self::doctor(&options.source_path);
        let has_errors = doctor_results
            .iter()
            .any(|r| matches!(r, DiagnosticResult::Error(_, _)));
        let verification_passed = !has_errors;

        // If require_verified and verification failed, abort before any writes
        if options.require_verified && !verification_passed {
            return Err(CapsuleError::InvalidOperation {
                reason: "Capsule verification failed and --require-verified was set".to_string(),
            });
        }

        // === 4. Compute content hash for provenance tracking ===
        let content_hash = Self::compute_file_hash(&options.source_path)?;

        // === 5. Check idempotency before any writes ===
        if let Some(existing) = self.check_mount_idempotency(&mount_name, &content_hash)? {
            // Already mounted with same content - return success without duplicate writes
            tracing::info!(
                mount_name = %mount_name,
                content_hash = %content_hash,
                "Capsule already mounted (idempotent success)"
            );

            // Still need to gather stats for the result
            let is_encrypted = options
                .source_path
                .extension()
                .map(|e| e == "mv2e")
                .unwrap_or(false);
            let (artifact_count, checkpoint_count, event_count, _) =
                self.gather_source_stats_internal(options, is_encrypted)?;

            return Ok(ImportResult {
                source_path: options.source_path.clone(),
                mount_name,
                content_hash,
                artifact_count,
                checkpoint_count,
                event_count,
                doctor_results,
                verification_passed: existing.verification_passed,
                mounted_path: Some(PathBuf::from(&existing.mounted_path)),
            });
        }

        // === 6. Open source capsule read-only to gather stats (D103) ===
        let is_encrypted = options
            .source_path
            .extension()
            .map(|e| e == "mv2e")
            .unwrap_or(false);

        let (artifact_count, checkpoint_count, event_count, checkpoints_imported) =
            self.gather_source_stats_internal(options, is_encrypted)?;

        // === 7. S974-mount: Atomic copy to mounts directory ===
        let mounted_path = self.copy_to_mounts_atomic(&options.source_path, &mount_name)?;

        // === 8. S974-mount: Update registry atomically ===
        let mut registry = self.load_mounts_registry()?;
        let mount_entry = MountEntry {
            mounted_path: mounted_path.display().to_string(),
            source_path: options.source_path.display().to_string(),
            content_hash: content_hash.clone(),
            format: if is_encrypted {
                "mv2e-v1".to_string()
            } else {
                "mv2-v1".to_string()
            },
            imported_at: chrono::Utc::now(),
            verification_passed,
        };
        registry.mounts.insert(mount_name.clone(), mount_entry);

        // Save registry atomically; rollback mount file on failure
        if let Err(e) = self.save_mounts_registry(&registry) {
            // Rollback: remove the mounted file
            let _ = std::fs::remove_file(&mounted_path);
            return Err(e);
        }

        // === 9. Emit CapsuleImported event (D104: auto-register) ===
        // SPEC-KIT-974 AC#7: Include mount_name and verification_passed in event payload
        let payload = CapsuleImportedPayload {
            source_type: "file".to_string(),
            source: Some(options.source_path.display().to_string()),
            format: if is_encrypted {
                "mv2e-v1".to_string()
            } else {
                "mv2-v1".to_string()
            },
            original_capsule_id: None, // Could be extracted from manifest if present
            checkpoints_imported,
            imported_at: chrono::Utc::now(),
            content_hash: Some(content_hash.clone()),
            mount_name: Some(mount_name.clone()),
            verification_passed: Some(verification_passed),
        };

        // === 10. Rollback on event emission failure ===
        // SPEC-KIT-974 Task 5: Harden rollback - don't clobber registry if load fails
        if let Err(e) = self.emit_capsule_imported("import", &mount_name, &payload) {
            // Best-effort rollback: delete mounted file
            let _ = std::fs::remove_file(&mounted_path);
            // Only modify registry if we can load it; don't overwrite with empty on load failure
            if let Ok(mut rollback_registry) = self.load_mounts_registry() {
                rollback_registry.mounts.remove(&mount_name);
                let _ = self.save_mounts_registry(&rollback_registry);
            }
            return Err(e);
        }

        tracing::info!(
            source = %options.source_path.display(),
            mount_name = %mount_name,
            mounted_path = %mounted_path.display(),
            artifacts = artifact_count,
            checkpoints = checkpoint_count,
            events = event_count,
            verified = verification_passed,
            encrypted = is_encrypted,
            "Capsule imported and mounted successfully"
        );

        Ok(ImportResult {
            source_path: options.source_path.clone(),
            mount_name,
            content_hash,
            artifact_count,
            checkpoint_count,
            event_count,
            doctor_results,
            verification_passed,
            mounted_path: Some(mounted_path),
        })
    }

    /// Compute SHA-256 hash of a file.
    fn compute_file_hash(path: &Path) -> Result<String> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// SPEC-KIT-974 (SK974-3): Garbage collect expired exports and temp files.
    ///
    /// ## Decision IDs
    /// - D20: Capsule growth management (retention/compaction)
    /// - D116: Hybrid retention (TTL + milestone protection)
    ///
    /// ## Process
    /// 1. Scan export directories for .mv2 and .mv2e files
    /// 2. Evaluate each against retention policy
    /// 3. Skip pinned/milestone exports (D116)
    /// 4. Delete expired exports
    /// 5. Clean up orphaned temp files
    /// 6. Record audit trail event for deletions
    ///
    /// ## Acceptance Criteria
    /// - `speckit capsule gc` deletes expired exports older than retention_days unless pinned
    /// - Leaves an audit trail event for deletions
    pub fn gc(&self, config: &GcConfig) -> Result<GcResult> {
        if !self.is_open() {
            return Err(CapsuleError::NotOpen);
        }

        let mut result = GcResult {
            exports_deleted: 0,
            temp_files_deleted: 0,
            bytes_freed: 0,
            exports_preserved: 0,
            exports_skipped: 0,
            dry_run: config.dry_run,
            deleted_paths: Vec::new(),
        };

        // Get workspace root (parent of .speckit)
        let workspace_root = self
            .config
            .capsule_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .unwrap_or(Path::new("."));

        // Standard export location: docs/specs/*/runs/*/*.mv2{e}
        let exports_dir = workspace_root.join("docs").join("specs");
        let temp_dir = workspace_root.join(".speckit").join("tmp");

        let now = std::time::SystemTime::now();

        // Collect export candidates
        let candidates = Self::collect_export_candidates(&exports_dir, now);

        // Evaluate each candidate
        for candidate in candidates {
            if candidate.is_pinned && config.keep_pinned {
                // D116: Milestone protection
                result.exports_preserved += 1;
                continue;
            }

            if candidate.age_days < config.retention_days as u64 {
                // Not yet expired
                result.exports_skipped += 1;
                continue;
            }

            // Candidate is expired and not protected
            if config.dry_run {
                result.exports_deleted += 1;
                result.bytes_freed += candidate.size_bytes;
                result.deleted_paths.push(candidate.path);
            } else {
                // Actually delete
                match std::fs::remove_file(&candidate.path) {
                    Ok(_) => {
                        result.exports_deleted += 1;
                        result.bytes_freed += candidate.size_bytes;
                        result.deleted_paths.push(candidate.path);
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %candidate.path.display(),
                            error = %e,
                            "Failed to delete expired export"
                        );
                    }
                }
            }
        }

        // Clean temp files if configured
        if config.clean_temp_files && temp_dir.exists() {
            result.temp_files_deleted = Self::clean_temp_dir(&temp_dir, config.dry_run)?;
        }

        // Record audit trail event for deletions (if not dry run and there were deletions)
        if !config.dry_run && result.exports_deleted > 0 {
            self.emit_gc_audit_event(&result)?;
        }

        tracing::info!(
            exports_deleted = result.exports_deleted,
            temp_files_deleted = result.temp_files_deleted,
            bytes_freed = result.bytes_freed,
            exports_preserved = result.exports_preserved,
            dry_run = config.dry_run,
            "Garbage collection completed"
        );

        Ok(result)
    }

    /// Collect export file candidates from the given directory.
    fn collect_export_candidates(
        exports_dir: &Path,
        now: std::time::SystemTime,
    ) -> Vec<ExportCandidate> {
        let mut candidates = Vec::new();

        if !exports_dir.exists() {
            return candidates;
        }

        // Walk: docs/specs/<SPEC_ID>/runs/<RUN_ID>/*.mv2{e}
        if let Ok(spec_entries) = std::fs::read_dir(exports_dir) {
            for spec_entry in spec_entries.flatten() {
                let spec_path = spec_entry.path();
                if !spec_path.is_dir() {
                    continue;
                }

                let spec_id = spec_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string());

                let runs_path = spec_path.join("runs");
                if !runs_path.exists() {
                    continue;
                }

                if let Ok(run_entries) = std::fs::read_dir(&runs_path) {
                    for run_entry in run_entries.flatten() {
                        let run_path = run_entry.path();
                        if !run_path.is_dir() {
                            continue;
                        }

                        let run_id = run_path
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string());

                        // Find .mv2 and .mv2e files in run directory
                        if let Ok(file_entries) = std::fs::read_dir(&run_path) {
                            for file_entry in file_entries.flatten() {
                                let file_path = file_entry.path();
                                if let Some(ext) = file_path.extension() {
                                    if ext == "mv2" || ext == "mv2e" {
                                        if let Ok(candidate) = Self::create_export_candidate(
                                            &file_path,
                                            now,
                                            spec_id.clone(),
                                            run_id.clone(),
                                        ) {
                                            candidates.push(candidate);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        candidates
    }

    /// Create an export candidate from a file path.
    fn create_export_candidate(
        path: &Path,
        now: std::time::SystemTime,
        spec_id: Option<String>,
        run_id: Option<String>,
    ) -> Result<ExportCandidate> {
        let metadata = std::fs::metadata(path)?;
        let modified_at = metadata.modified()?;
        let size_bytes = metadata.len();

        let age_secs = now
            .duration_since(modified_at)
            .unwrap_or(std::time::Duration::ZERO)
            .as_secs();
        let age_days = age_secs / 86400;

        // SPEC-KIT-974 Task 4: Check if pinned - support both marker formats for compatibility
        // Preferred: <filename>.pin (e.g., capsule.mv2.pin)
        let preferred_pin = path.with_file_name(format!(
            "{}.pin",
            path.file_name().unwrap_or_default().to_string_lossy()
        ));
        // Legacy: path.with_extension("pin") (e.g., capsule.pin for capsule.mv2)
        let legacy_pin = path.with_extension("pin");
        let is_pinned = preferred_pin.exists() || legacy_pin.exists();

        Ok(ExportCandidate {
            path: path.to_path_buf(),
            size_bytes,
            modified_at,
            age_days,
            is_pinned,
            spec_id,
            run_id,
        })
    }

    /// Clean orphaned temp files.
    fn clean_temp_dir(temp_dir: &Path, dry_run: bool) -> Result<u64> {
        let mut count = 0;

        if let Ok(entries) = std::fs::read_dir(temp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                // Only clean temp files older than 1 day
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        let age = std::time::SystemTime::now()
                            .duration_since(modified)
                            .unwrap_or(std::time::Duration::ZERO);
                        if age.as_secs() > 86400 {
                            if dry_run {
                                count += 1;
                            } else if std::fs::remove_file(&path).is_ok() {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    /// Emit an audit trail event for gc deletions.
    fn emit_gc_audit_event(&self, result: &GcResult) -> Result<()> {
        // Create a simple gate decision event for the gc operation
        let payload = serde_json::json!({
            "gate_name": "CapsuleGC",
            "outcome": "pass",
            "exports_deleted": result.exports_deleted,
            "temp_files_deleted": result.temp_files_deleted,
            "bytes_freed": result.bytes_freed,
            "deleted_paths": result.deleted_paths.iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>(),
        });

        self.emit_event("gc", "cleanup", None, EventType::GateDecision, payload)?;

        Ok(())
    }

    /// S974-003: Write encrypted export file (.mv2e)
    ///
    /// Process:
    /// 1. Write plaintext .mv2 to temp file
    /// 2. Read temp file contents
    /// 3. Encrypt with age passphrase
    /// 4. Write encrypted content to final path
    /// 5. Compute SHA256 of encrypted bytes
    /// 6. Clean up temp file
    fn write_encrypted_export(
        &self,
        final_path: &Path,
        manifest: &ExportManifest,
        records: &[StoredRecord],
        options: &ExportOptions,
    ) -> Result<(u64, String)> {
        use secrecy::ExposeSecret;
        use std::io::BufWriter;

        // Acquire passphrase
        let passphrase = Self::acquire_passphrase(options.interactive)?;

        // Create temp directory for plaintext
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().join("export.mv2");

        // Write plaintext to temp file (reuse existing logic with unencrypted manifest)
        let plaintext_manifest = ExportManifest {
            encrypted: false, // Manifest inside encrypted file shows it's the plaintext version
            ..manifest.clone()
        };
        self.write_export_file(&temp_path, &plaintext_manifest, records, options)?;

        // Read plaintext content
        let plaintext = std::fs::read(&temp_path)?;

        // Encrypt using age passphrase encryption
        let encrypted = {
            let encryptor = age::Encryptor::with_user_passphrase(secrecy::SecretString::from(
                passphrase.expose_secret().to_string(),
            ));
            let mut encrypted = vec![];
            {
                let mut writer = encryptor.wrap_output(&mut encrypted).map_err(|e| {
                    CapsuleError::EncryptionError {
                        reason: format!("Failed to create encryptor: {}", e),
                    }
                })?;
                writer
                    .write_all(&plaintext)
                    .map_err(|e| CapsuleError::EncryptionError {
                        reason: format!("Failed to write encrypted data: {}", e),
                    })?;
                writer.finish().map_err(|e| CapsuleError::EncryptionError {
                    reason: format!("Failed to finalize encryption: {}", e),
                })?;
            }
            encrypted
        };

        // Compute SHA256 of encrypted content
        let mut hasher = Sha256::new();
        hasher.update(&encrypted);
        let content_hash = format!("{:x}", hasher.finalize());

        // Write encrypted content to final path
        let file = File::create(final_path)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&encrypted)?;
        writer.flush()?;

        let bytes_written = encrypted.len() as u64;

        tracing::debug!(
            plaintext_size = plaintext.len(),
            encrypted_size = encrypted.len(),
            "Encrypted capsule export"
        );

        // Temp dir auto-cleans on drop
        Ok((bytes_written, content_hash))
    }

    /// Collect artifact records matching the export filter.
    fn collect_export_records(&self, options: &ExportOptions) -> Result<Vec<StoredRecord>> {
        let all_records = self.stored_records.read().unwrap();

        let filtered: Vec<StoredRecord> = all_records
            .iter()
            .filter(|r| {
                // Only export Artifact records
                if r.kind != RecordKind::Artifact {
                    return false;
                }

                // Parse artifact metadata to check spec/run filter
                if let Ok(art_meta) = serde_json::from_value::<ArtifactRecordMeta>(r.meta.clone()) {
                    // Check if URI matches filter criteria
                    if let Some(ref spec_filter) = options.spec_id {
                        if !art_meta.uri.contains(spec_filter) {
                            return false;
                        }
                    }
                    if let Some(ref run_filter) = options.run_id {
                        if !art_meta.uri.contains(run_filter) {
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        Ok(filtered)
    }

    /// Collect checkpoints matching the export filter.
    fn collect_export_checkpoints(&self, options: &ExportOptions) -> Vec<CheckpointMetadata> {
        let all_checkpoints = self.checkpoints.read().unwrap();

        all_checkpoints
            .iter()
            .filter(|cp| {
                // Filter by spec_id if specified
                if let Some(ref spec_filter) = options.spec_id {
                    if cp.spec_id.as_ref() != Some(spec_filter) {
                        return false;
                    }
                }
                // Filter by run_id if specified
                if let Some(ref run_filter) = options.run_id {
                    if cp.run_id.as_ref() != Some(run_filter) {
                        return false;
                    }
                }
                // Filter by branch if specified
                if let Some(ref branch_filter) = options.branch {
                    if cp.branch_id.as_ref() != Some(&branch_filter.as_str().to_string()) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    /// Collect events matching the export filter.
    ///
    /// ## S974-009: Safe Export Filtering
    /// When `options.safe_mode` is true, ModelCallEnvelope events with
    /// `capture_mode == FullIo` are excluded (may contain sensitive data).
    fn collect_export_events(&self, options: &ExportOptions) -> Vec<RunEventEnvelope> {
        let all_events = self.events.read().unwrap();

        all_events
            .iter()
            .filter(|ev| {
                // Filter by spec_id if specified
                if let Some(ref spec_filter) = options.spec_id {
                    if &ev.spec_id != spec_filter {
                        return false;
                    }
                }
                // Filter by run_id if specified
                if let Some(ref run_filter) = options.run_id {
                    if &ev.run_id != run_filter {
                        return false;
                    }
                }
                // Filter by branch if specified
                if let Some(ref branch_filter) = options.branch {
                    if ev.branch_id.as_ref() != Some(&branch_filter.as_str().to_string()) {
                        return false;
                    }
                }

                // S974-009: Filter unsafe model I/O in safe_mode
                if options.safe_mode && ev.event_type == EventType::ModelCallEnvelope {
                    // Deserialize payload to check capture mode
                    if let Ok(payload) =
                        serde_json::from_value::<ModelCallEnvelopePayload>(ev.payload.clone())
                    {
                        if !payload.capture_mode.is_export_safe() {
                            // FullIo capture mode is not safe for export
                            return false;
                        }
                    }
                    // If deserialization fails, include the event (conservative)
                }

                true
            })
            .cloned()
            .collect()
    }

    /// Write the export file in MV2 format with manifest.
    fn write_export_file(
        &self,
        path: &Path,
        manifest: &ExportManifest,
        records: &[StoredRecord],
        options: &ExportOptions,
    ) -> Result<(u64, String)> {
        use sha2::{Digest, Sha256};
        use std::io::BufWriter;

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        let mut hasher = Sha256::new();

        // Write MV2 header
        writer.write_all(MV2_HEADER)?;
        hasher.update(MV2_HEADER);

        // Write manifest as first record (RecordKind = 4 for Manifest)
        let manifest_json =
            serde_json::to_vec(manifest).map_err(|e| CapsuleError::InvalidOperation {
                reason: format!("Failed to serialize manifest: {}", e),
            })?;
        let manifest_meta = serde_json::json!({
            "type": "export_manifest",
            "version": "1.0.0"
        });
        let manifest_meta_bytes = serde_json::to_vec(&manifest_meta).unwrap();

        // Manifest record format: [len][kind][meta_len][meta][payload]
        let manifest_record_len = 1 + 4 + manifest_meta_bytes.len() + manifest_json.len();
        writer.write_all(&(manifest_record_len as u32).to_le_bytes())?;
        writer.write_all(&[RecordKind::Manifest as u8])?;
        writer.write_all(&(manifest_meta_bytes.len() as u32).to_le_bytes())?;
        writer.write_all(&manifest_meta_bytes)?;
        writer.write_all(&manifest_json)?;

        // Update hash
        hasher.update((manifest_record_len as u32).to_le_bytes());
        hasher.update([RecordKind::Manifest as u8]);
        hasher.update((manifest_meta_bytes.len() as u32).to_le_bytes());
        hasher.update(&manifest_meta_bytes);
        hasher.update(&manifest_json);

        // Copy artifact records from source capsule
        for record in records {
            // Read original payload from source
            let payload = self.read_payload(&PhysicalPointer {
                frame_id: record.seq,
                offset: record.payload_offset,
                length: record.payload_len,
            })?;

            // Serialize metadata
            let meta_bytes = serde_json::to_vec(&record.meta).unwrap();

            // Write record
            let record_len = 1 + 4 + meta_bytes.len() + payload.len();
            writer.write_all(&(record_len as u32).to_le_bytes())?;
            writer.write_all(&[record.kind as u8])?;
            writer.write_all(&(meta_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(&meta_bytes)?;
            writer.write_all(&payload)?;

            // Update hash
            hasher.update((record_len as u32).to_le_bytes());
            hasher.update([record.kind as u8]);
            hasher.update((meta_bytes.len() as u32).to_le_bytes());
            hasher.update(&meta_bytes);
            hasher.update(&payload);
        }

        // Write checkpoints as records (S974-008: use passed options for filtering)
        let checkpoints = self.collect_export_checkpoints(options);
        for cp in &checkpoints {
            let cp_meta = serde_json::to_value(cp).unwrap();
            let cp_meta_bytes = serde_json::to_vec(&cp_meta).unwrap();

            let record_len = 1 + 4 + cp_meta_bytes.len();
            writer.write_all(&(record_len as u32).to_le_bytes())?;
            writer.write_all(&[RecordKind::Checkpoint as u8])?;
            writer.write_all(&(cp_meta_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(&cp_meta_bytes)?;

            hasher.update((record_len as u32).to_le_bytes());
            hasher.update([RecordKind::Checkpoint as u8]);
            hasher.update((cp_meta_bytes.len() as u32).to_le_bytes());
            hasher.update(&cp_meta_bytes);
        }

        // Write events as records (S974-008: use passed options for filtering)
        let events = self.collect_export_events(options);
        for ev in &events {
            let ev_meta = serde_json::to_value(ev).unwrap();
            let ev_meta_bytes = serde_json::to_vec(&ev_meta).unwrap();

            let record_len = 1 + 4 + ev_meta_bytes.len();
            writer.write_all(&(record_len as u32).to_le_bytes())?;
            writer.write_all(&[RecordKind::Event as u8])?;
            writer.write_all(&(ev_meta_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(&ev_meta_bytes)?;

            hasher.update((record_len as u32).to_le_bytes());
            hasher.update([RecordKind::Event as u8]);
            hasher.update((ev_meta_bytes.len() as u32).to_le_bytes());
            hasher.update(&ev_meta_bytes);
        }

        writer.flush()?;

        // Get file size and hash
        let bytes_written = std::fs::metadata(path)?.len();
        let hash = format!("{:x}", hasher.finalize());

        Ok((bytes_written, hash))
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

// =============================================================================
// SPEC-KIT-974: Export Types
// =============================================================================

/// Options for capsule export.
///
/// ## SPEC-KIT-974 MVP
/// - Export produces a single file artifact (.mv2 or .mv2e) with no sidecar files
/// - Default: `--no-encrypt` for MVP backward compatibility
/// - Default: `--safe` mode (exclude raw LLM I/O unless audit.capture_llm_io=full)
///
/// ## S974-003: Encryption
/// - When `encrypt` is true, output is .mv2e (age passphrase encrypted)
/// - Passphrase acquired from SPECKIT_MEMVID_PASSPHRASE env var or interactive prompt
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Output file path (extension will be adjusted: .mv2 or .mv2e based on encrypt flag)
    pub output_path: PathBuf,

    /// Filter by spec ID (None = all specs)
    pub spec_id: Option<String>,

    /// Filter by run ID (None = all runs)
    pub run_id: Option<String>,

    /// Filter by branch (None = all branches)
    pub branch: Option<BranchId>,

    /// Safe export mode: exclude raw LLM I/O unless explicitly captured
    /// Default: true (per SPEC-KIT-974 D77)
    pub safe_mode: bool,

    /// S974-003: Enable encryption (produces .mv2e instead of .mv2)
    /// Default: false for MVP backward compatibility
    pub encrypt: bool,

    /// S974-003: Allow interactive passphrase prompt if env var not set
    /// Default: true (prompts user when SPECKIT_MEMVID_PASSPHRASE not set)
    pub interactive: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("export.mv2"),
            spec_id: None,
            run_id: None,
            branch: None,
            safe_mode: true,
            encrypt: false,
            interactive: true,
        }
    }
}

impl ExportOptions {
    /// Create export options for a specific run.
    pub fn for_run(spec_id: &str, run_id: &str, output_path: impl Into<PathBuf>) -> Self {
        Self {
            output_path: output_path.into(),
            spec_id: Some(spec_id.to_string()),
            run_id: Some(run_id.to_string()),
            branch: Some(BranchId::for_run(run_id)),
            safe_mode: true,
            encrypt: false,
            interactive: true,
        }
    }

    /// Create encrypted export options for a specific run.
    /// S974-003: Produces .mv2e file with age passphrase encryption.
    pub fn for_run_encrypted(spec_id: &str, run_id: &str, output_path: impl Into<PathBuf>) -> Self {
        let mut path: PathBuf = output_path.into();
        // Ensure .mv2e extension for encrypted exports
        path.set_extension("mv2e");
        Self {
            output_path: path,
            spec_id: Some(spec_id.to_string()),
            run_id: Some(run_id.to_string()),
            branch: Some(BranchId::for_run(run_id)),
            safe_mode: true,
            encrypt: true,
            interactive: true,
        }
    }
}

/// Export manifest stored in the exported .mv2 file.
///
/// ## SPEC-KIT-974 Requirement
/// Include manifest/digest in the exported output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifest {
    /// Manifest schema version
    pub version: String,

    /// Export timestamp
    pub exported_at: chrono::DateTime<chrono::Utc>,

    /// Source capsule path (for provenance)
    pub source_capsule_path: String,

    /// Workspace ID
    pub workspace_id: String,

    /// Filter criteria used
    pub filter: ExportFilter,

    /// Number of artifacts exported
    pub artifact_count: u64,

    /// Number of checkpoints exported
    pub checkpoint_count: u64,

    /// Number of events exported
    pub event_count: u64,

    /// Whether safe export mode was used
    pub safe_mode: bool,

    /// Whether the export is encrypted
    pub encrypted: bool,
}

/// Filter criteria recorded in the export manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFilter {
    pub spec_id: Option<String>,
    pub run_id: Option<String>,
    pub branch: Option<String>,
}

/// Result of a capsule export operation.
#[derive(Debug, Clone)]
pub struct ExportResult {
    /// Path to the exported file
    pub output_path: PathBuf,

    /// Total bytes written
    pub bytes_written: u64,

    /// SHA-256 hash of the export content
    pub content_hash: String,

    /// Number of artifacts exported
    pub artifact_count: u64,

    /// Number of checkpoints exported
    pub checkpoint_count: u64,

    /// Number of events exported
    pub event_count: u64,
}

// =============================================================================
// ImportOptions and ImportResult (SPEC-KIT-974/SK974-1)
// =============================================================================

/// Options for capsule import.
///
/// ## SPEC-KIT-974 Requirements (D103, D104)
/// - D103: Imported capsules are read-only (no mutation of imported capsule)
/// - D104: Auto-register mounted capsules (immediately discoverable)
///
/// ## Import Process
/// 1. Open source capsule read-only
/// 2. Run doctor verification
/// 3. Record CapsuleImported event in workspace capsule
/// 4. Return import metadata
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Path to the source capsule file (.mv2 or .mv2e)
    pub source_path: PathBuf,

    /// Mount name for the imported capsule (default: derived from source filename)
    pub mount_as: Option<String>,

    /// Allow interactive passphrase prompt for .mv2e files
    /// Default: true (prompts user when SPECKIT_MEMVID_PASSPHRASE not set)
    pub interactive: bool,

    /// D70: Hard-fail on unverified/unsigned capsules
    /// Default: false (warn only)
    pub require_verified: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            source_path: PathBuf::new(),
            mount_as: None,
            interactive: true,
            require_verified: false,
        }
    }
}

impl ImportOptions {
    /// Create import options for a specific file.
    pub fn for_file(source_path: impl Into<PathBuf>) -> Self {
        Self {
            source_path: source_path.into(),
            ..Default::default()
        }
    }

    /// Set the mount name.
    pub fn with_mount_name(mut self, name: impl Into<String>) -> Self {
        self.mount_as = Some(name.into());
        self
    }

    /// Set require verified flag.
    pub fn require_verified(mut self) -> Self {
        self.require_verified = true;
        self
    }

    /// Set interactive mode for passphrase prompting.
    pub fn with_interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
}

/// Result of a capsule import operation.
#[derive(Debug, Clone)]
pub struct ImportResult {
    /// Path to the source file that was imported
    pub source_path: PathBuf,

    /// Mount name used for the imported capsule
    pub mount_name: String,

    /// SHA-256 hash of the imported content
    pub content_hash: String,

    /// Number of artifacts in the imported capsule
    pub artifact_count: u64,

    /// Number of checkpoints in the imported capsule
    pub checkpoint_count: u64,

    /// Number of events in the imported capsule
    pub event_count: u64,

    /// Doctor verification results
    pub doctor_results: Vec<DiagnosticResult>,

    /// Whether the capsule passed verification
    pub verification_passed: bool,

    /// Path to the mounted capsule file (S974-mount)
    pub mounted_path: Option<PathBuf>,
}

// =============================================================================
// MountsRegistry (SPEC-KIT-974 Mount Persistence)
// =============================================================================

/// Registry of mounted capsules.
///
/// Stored at `./.speckit/memvid/mounts.json` with atomic writes.
/// Schema is versioned for forward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountsRegistry {
    /// Schema version for forward compatibility
    pub schema_version: u32,

    /// Map of mount_name -> MountEntry
    pub mounts: std::collections::HashMap<String, MountEntry>,
}

impl Default for MountsRegistry {
    fn default() -> Self {
        Self {
            schema_version: Self::CURRENT_VERSION,
            mounts: std::collections::HashMap::new(),
        }
    }
}

impl MountsRegistry {
    /// Current schema version
    pub const CURRENT_VERSION: u32 = 1;

    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }
}

/// Individual mount entry in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountEntry {
    /// Path to the mounted capsule (relative to workspace)
    pub mounted_path: String,

    /// Original source path (for provenance)
    pub source_path: String,

    /// SHA-256 content hash
    pub content_hash: String,

    /// Format identifier (mv2-v1 or mv2e-v1)
    pub format: String,

    /// Import timestamp
    pub imported_at: chrono::DateTime<chrono::Utc>,

    /// Whether doctor verification passed
    pub verification_passed: bool,
}

// =============================================================================
// GcConfig and GcResult (SPEC-KIT-974/SK974-3)
// =============================================================================

/// Configuration for capsule garbage collection.
///
/// ## SPEC-KIT-974 Requirements (D20, D116)
/// - D20: Capsule growth management (retention/compaction)
/// - D116: Hybrid retention (TTL + milestone protection)
///
/// ## Retention Rules
/// - Exports older than `retention_days` are deleted
/// - Pinned exports (milestone-marked) are preserved
/// - Audit trail events recorded for all deletions
#[derive(Debug, Clone)]
pub struct GcConfig {
    /// Number of days to retain unpinned exports
    /// Default: 30 (per spec)
    pub retention_days: u32,

    /// Preserve pinned/milestone exports regardless of age
    /// Default: true
    pub keep_pinned: bool,

    /// Also clean up orphaned temp files
    /// Default: true
    pub clean_temp_files: bool,

    /// Dry-run mode: report what would be deleted without deleting
    /// Default: false
    pub dry_run: bool,
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            retention_days: 30, // D20: 30 days default
            keep_pinned: true,  // D116: milestone protection
            clean_temp_files: true,
            dry_run: false,
        }
    }
}

impl GcConfig {
    /// Create a dry-run config for preview.
    pub fn dry_run() -> Self {
        Self {
            dry_run: true,
            ..Default::default()
        }
    }

    /// Set retention days.
    pub fn with_retention_days(mut self, days: u32) -> Self {
        self.retention_days = days;
        self
    }
}

/// Result of a garbage collection operation.
#[derive(Debug, Clone)]
pub struct GcResult {
    /// Number of export files deleted
    pub exports_deleted: u64,

    /// Number of temp files deleted
    pub temp_files_deleted: u64,

    /// Total bytes freed
    pub bytes_freed: u64,

    /// Number of exports preserved (pinned/milestone)
    pub exports_preserved: u64,

    /// Number of exports skipped (not yet expired)
    pub exports_skipped: u64,

    /// Was this a dry run?
    pub dry_run: bool,

    /// Paths of deleted files (for audit trail)
    pub deleted_paths: Vec<PathBuf>,
}

/// An export file candidate for gc evaluation.
#[derive(Debug, Clone)]
pub struct ExportCandidate {
    /// Path to the export file
    pub path: PathBuf,

    /// File size in bytes
    pub size_bytes: u64,

    /// File modification time
    pub modified_at: std::time::SystemTime,

    /// Age in days
    pub age_days: u64,

    /// Whether this export is pinned (milestone-marked)
    pub is_pinned: bool,

    /// Associated spec ID (if determinable)
    pub spec_id: Option<String>,

    /// Associated run ID (if determinable)
    pub run_id: Option<String>,
}
