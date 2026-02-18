//! PM-006 T002: Atomic read/write for packet files.
//!
//! Uses the temp-file + fsync + rename pattern for crash-safe writes.
//! Read path provides deterministic error on corruption.

use std::io::Write;
use std::path::{Path, PathBuf};

use super::schema::Packet;

/// Default packet filename within the `.speckit/` directory.
pub const PACKET_FILENAME: &str = "packet.json";

/// Errors from packet I/O operations.
#[derive(Debug, thiserror::Error)]
pub enum PacketIoError {
    /// Packet file not found at expected path.
    #[error("Packet not found at {path}")]
    NotFound { path: PathBuf },

    /// Packet file exists but contains invalid data.
    #[error("Packet corrupted at {path}: {reason}")]
    Corrupted { path: PathBuf, reason: String },

    /// Filesystem I/O error.
    #[error("I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Schema version mismatch.
    #[error("Schema version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },
}

/// Read a packet from disk.
///
/// Returns a deterministic error for missing or corrupted files.
pub fn read_packet(speckit_dir: &Path) -> Result<Packet, PacketIoError> {
    let path = speckit_dir.join(PACKET_FILENAME);

    if !path.exists() {
        return Err(PacketIoError::NotFound { path });
    }

    let content = std::fs::read_to_string(&path).map_err(|e| PacketIoError::Io {
        path: path.clone(),
        source: e,
    })?;

    let packet: Packet = serde_json::from_str(&content).map_err(|e| PacketIoError::Corrupted {
        path: path.clone(),
        reason: e.to_string(),
    })?;

    // Validate schema version
    if !packet.header.schema_version.starts_with("packet@") {
        return Err(PacketIoError::VersionMismatch {
            expected: super::schema::SCHEMA_VERSION.to_string(),
            found: packet.header.schema_version,
        });
    }

    Ok(packet)
}

/// Write a packet to disk atomically.
///
/// Uses the temp-file + fsync + rename pattern:
/// 1. Write to a temporary file in the same directory
/// 2. fsync the temporary file
/// 3. Rename (atomic on POSIX) to the target path
///
/// The packet's epoch is incremented and `last_modified_at` is updated.
pub fn write_packet(speckit_dir: &Path, packet: &mut Packet) -> Result<(), PacketIoError> {
    // Update metadata
    packet.header.epoch += 1;
    packet.header.last_modified_at = chrono::Utc::now().to_rfc3339();

    let target = speckit_dir.join(PACKET_FILENAME);

    // Ensure directory exists
    std::fs::create_dir_all(speckit_dir).map_err(|e| PacketIoError::Io {
        path: speckit_dir.to_path_buf(),
        source: e,
    })?;

    // Write to temp file
    let temp_path = speckit_dir.join(format!(".{PACKET_FILENAME}.tmp"));
    let json = serde_json::to_string_pretty(packet)?;

    let mut file = std::fs::File::create(&temp_path).map_err(|e| PacketIoError::Io {
        path: temp_path.clone(),
        source: e,
    })?;

    file.write_all(json.as_bytes())
        .map_err(|e| PacketIoError::Io {
            path: temp_path.clone(),
            source: e,
        })?;

    // fsync for durability
    file.sync_all().map_err(|e| PacketIoError::Io {
        path: temp_path.clone(),
        source: e,
    })?;

    // Atomic rename
    std::fs::rename(&temp_path, &target).map_err(|e| PacketIoError::Io {
        path: target.clone(),
        source: e,
    })?;

    Ok(())
}
