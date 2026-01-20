//! Evidence integrity verification module (E.4 capability)
//!
//! Implements SHA256 verification for evidence archives per docs/spec-kit/evidence-policy.md §9:
//! - Compute SHA256 checksums for evidence files
//! - Create archives with embedded manifests
//! - Verify archive integrity on restore
//! - Detect corruption or checksum mismatch

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};

// ============================================================================
// Types
// ============================================================================

/// Manifest entry for a single file in an archive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    /// Relative path within the archive.
    pub path: String,
    /// SHA256 checksum of the file contents.
    pub sha256: String,
    /// File size in bytes.
    pub size: u64,
}

/// Archive manifest containing checksums and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveManifest {
    /// SPEC identifier.
    pub spec_id: String,
    /// Archive creation timestamp (ISO 8601).
    pub created_at: String,
    /// SHA256 of the entire archive file.
    pub archive_checksum: String,
    /// Individual file entries with checksums.
    pub files: Vec<FileEntry>,
    /// Total original size before compression.
    pub total_size: u64,
    /// Manifest version for future compatibility.
    pub version: u32,
}

impl ArchiveManifest {
    /// Create a new manifest for a SPEC.
    pub fn new(spec_id: &str) -> Self {
        Self {
            spec_id: spec_id.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            archive_checksum: String::new(),
            files: Vec::new(),
            total_size: 0,
            version: 1,
        }
    }

    /// Add a file entry to the manifest.
    pub fn add_file(&mut self, path: String, sha256: String, size: u64) {
        self.files.push(FileEntry { path, sha256, size });
        self.total_size += size;
    }

    /// Get the expected file count.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

/// Result of an integrity verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityResult {
    /// Archive is valid and matches the expected checksum.
    Valid,
    /// Archive checksum does not match.
    ChecksumMismatch { expected: String, actual: String },
    /// Archive is corrupted (cannot be read or parsed).
    Corrupted { reason: String },
    /// File count mismatch.
    FileCountMismatch { expected: usize, actual: usize },
    /// Individual file checksum mismatch.
    FileChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },
}

impl IntegrityResult {
    /// Returns true if the integrity check passed.
    pub fn is_valid(&self) -> bool {
        matches!(self, IntegrityResult::Valid)
    }
}

/// Result of a restore operation.
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// Whether the restore was successful.
    pub success: bool,
    /// Path where files were restored.
    pub target_path: PathBuf,
    /// Number of files restored.
    pub files_restored: usize,
    /// Total bytes restored.
    pub bytes_restored: u64,
    /// Integrity verification result (if verify was enabled).
    pub integrity: Option<IntegrityResult>,
    /// Warnings or issues encountered.
    pub warnings: Vec<String>,
}

// ============================================================================
// Checksum Functions
// ============================================================================

/// Compute SHA256 checksum of a file.
///
/// Returns the checksum as a lowercase hex string.
pub fn compute_sha256(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// Compute SHA256 checksum of bytes.
pub fn compute_sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    format!("{:x}", hash)
}

/// Compute SHA256 checksum of a string.
pub fn compute_sha256_str(data: &str) -> String {
    compute_sha256_bytes(data.as_bytes())
}

// ============================================================================
// Archive Functions
// ============================================================================

/// Collect all files in a directory (recursively) with their checksums.
fn collect_files_with_checksums(root: &Path, base: &Path) -> io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    if !root.exists() {
        return Ok(entries);
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            entries.extend(collect_files_with_checksums(&path, base)?);
        } else {
            let relative_path = path
                .strip_prefix(base)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
                .to_string_lossy()
                .to_string();

            let sha256 = compute_sha256(&path)?;
            let size = fs::metadata(&path)?.len();

            entries.push(FileEntry {
                path: relative_path,
                sha256,
                size,
            });
        }
    }

    Ok(entries)
}

/// Create an archive manifest for a SPEC's evidence.
///
/// Does NOT create the actual archive file - just computes the manifest.
pub fn create_manifest(spec_id: &str, evidence_root: &Path) -> io::Result<ArchiveManifest> {
    let mut manifest = ArchiveManifest::new(spec_id);

    // Collect files from commands/ and consensus/
    for category in ["commands", "consensus"] {
        let spec_dir = evidence_root.join(category).join(spec_id);
        if spec_dir.exists() {
            let files = collect_files_with_checksums(&spec_dir, evidence_root)?;
            for file in files {
                manifest.add_file(file.path, file.sha256, file.size);
            }
        }
    }

    Ok(manifest)
}

/// Create an archive with manifest for a SPEC's evidence.
///
/// Creates a tar.gz archive containing:
/// - All evidence files from commands/<spec_id>/ and consensus/<spec_id>/
/// - A manifest.json with SHA256 checksums
///
/// Returns the manifest with the archive checksum populated.
pub fn create_archive_with_checksum(
    spec_id: &str,
    evidence_root: &Path,
    output_path: &Path,
) -> io::Result<ArchiveManifest> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tar::Builder;

    let mut manifest = create_manifest(spec_id, evidence_root)?;

    // Create archive file
    let file = File::create(output_path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);

    // Add files to archive
    for category in ["commands", "consensus"] {
        let spec_dir = evidence_root.join(category).join(spec_id);
        if spec_dir.exists() {
            builder.append_dir_all(format!("{}/{}", category, spec_id), &spec_dir)?;
        }
    }

    // Serialize and add manifest
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let manifest_bytes = manifest_json.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_path("manifest.json")?;
    header.set_size(manifest_bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder.append(&header, manifest_bytes)?;

    // Finish archive
    let encoder = builder.into_inner()?;
    encoder.finish()?;

    // Compute archive checksum
    manifest.archive_checksum = compute_sha256(output_path)?;

    Ok(manifest)
}

/// Verify archive integrity by checking its SHA256 checksum.
pub fn verify_archive_integrity(
    archive_path: &Path,
    expected_checksum: &str,
) -> io::Result<IntegrityResult> {
    if !archive_path.exists() {
        return Ok(IntegrityResult::Corrupted {
            reason: "Archive file does not exist".to_string(),
        });
    }

    let actual_checksum = compute_sha256(archive_path)?;

    if actual_checksum == expected_checksum {
        Ok(IntegrityResult::Valid)
    } else {
        Ok(IntegrityResult::ChecksumMismatch {
            expected: expected_checksum.to_string(),
            actual: actual_checksum,
        })
    }
}

/// Extract and verify manifest from an archive.
pub fn extract_manifest(archive_path: &Path) -> io::Result<ArchiveManifest> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let file = File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;

        if path.to_string_lossy() == "manifest.json" {
            let mut contents = String::new();
            entry.read_to_string(&mut contents)?;
            let manifest: ArchiveManifest = serde_json::from_str(&contents)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            return Ok(manifest);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "manifest.json not found in archive",
    ))
}

/// Verify all file checksums within an archive.
pub fn verify_archive_contents(archive_path: &Path) -> io::Result<IntegrityResult> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    // First extract manifest
    let manifest = extract_manifest(archive_path)?;

    // Build checksum map for quick lookup
    let expected_checksums: HashMap<String, &FileEntry> =
        manifest.files.iter().map(|f| (f.path.clone(), f)).collect();

    // Re-open archive and verify each file
    let file = File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    let mut verified_files = 0;

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().to_string();

        // Skip manifest itself
        if path == "manifest.json" {
            continue;
        }

        // Read file contents and compute checksum
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents)?;
        let actual_checksum = compute_sha256_bytes(&contents);

        // Check against manifest
        if let Some(expected) = expected_checksums.get(&path) {
            if actual_checksum != expected.sha256 {
                return Ok(IntegrityResult::FileChecksumMismatch {
                    file: path,
                    expected: expected.sha256.clone(),
                    actual: actual_checksum,
                });
            }
            verified_files += 1;
        }
    }

    // Verify file count
    if verified_files != manifest.files.len() {
        return Ok(IntegrityResult::FileCountMismatch {
            expected: manifest.files.len(),
            actual: verified_files,
        });
    }

    Ok(IntegrityResult::Valid)
}

/// Restore an archive to a target directory with optional verification.
pub fn restore_archive(
    archive_path: &Path,
    target_dir: &Path,
    verify: bool,
) -> io::Result<RestoreResult> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let mut result = RestoreResult {
        success: false,
        target_path: target_dir.to_path_buf(),
        files_restored: 0,
        bytes_restored: 0,
        integrity: None,
        warnings: Vec::new(),
    };

    // Verify integrity first if requested
    if verify {
        let integrity = verify_archive_contents(archive_path)?;
        result.integrity = Some(integrity.clone());

        if !integrity.is_valid() {
            result
                .warnings
                .push(format!("Integrity check failed: {:?}", integrity));
            return Ok(result);
        }
    }

    // Create target directory
    fs::create_dir_all(target_dir)?;

    // Extract archive
    let file = File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().to_string();

        // Skip manifest in extraction (keep it in archive only)
        if path == "manifest.json" {
            continue;
        }

        // Skip directory entries (we create them as needed for files)
        let entry_type = entry.header().entry_type();
        if entry_type.is_dir() {
            continue;
        }

        let target_path = target_dir.join(&path);

        // Create parent directories
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Extract file
        let mut file = File::create(&target_path)?;
        let bytes = io::copy(&mut entry, &mut file)?;

        result.files_restored += 1;
        result.bytes_restored += bytes;
    }

    result.success = true;
    Ok(result)
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_compute_sha256_consistent() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_file(temp_dir.path(), "test.txt", "hello world");

        let checksum1 = compute_sha256(&file_path).unwrap();
        let checksum2 = compute_sha256(&file_path).unwrap();

        assert_eq!(checksum1, checksum2);
        assert_eq!(checksum1.len(), 64); // SHA256 = 64 hex chars
    }

    #[test]
    fn test_compute_sha256_known_value() {
        // "hello world" SHA256 = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
        let checksum = compute_sha256_str("hello world");
        assert_eq!(
            checksum,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_compute_sha256_different_content() {
        let checksum1 = compute_sha256_str("content a");
        let checksum2 = compute_sha256_str("content b");
        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn test_archive_manifest_creation() {
        let mut manifest = ArchiveManifest::new("SPEC-TEST-001");
        manifest.add_file("commands/test.json".to_string(), "abc123".to_string(), 100);
        manifest.add_file(
            "consensus/synth.json".to_string(),
            "def456".to_string(),
            200,
        );

        assert_eq!(manifest.spec_id, "SPEC-TEST-001");
        assert_eq!(manifest.file_count(), 2);
        assert_eq!(manifest.total_size, 300);
        assert_eq!(manifest.version, 1);
    }

    #[test]
    fn test_integrity_result_is_valid() {
        assert!(IntegrityResult::Valid.is_valid());
        assert!(
            !IntegrityResult::ChecksumMismatch {
                expected: "a".to_string(),
                actual: "b".to_string()
            }
            .is_valid()
        );
        assert!(
            !IntegrityResult::Corrupted {
                reason: "test".to_string()
            }
            .is_valid()
        );
    }

    #[test]
    fn test_create_manifest_for_spec() {
        let temp_dir = TempDir::new().unwrap();
        let evidence_root = temp_dir.path();

        // Create test evidence structure
        create_test_file(
            evidence_root,
            "commands/SPEC-TEST-001/telemetry.json",
            r#"{"test": "data"}"#,
        );
        create_test_file(
            evidence_root,
            "consensus/SPEC-TEST-001/synthesis.json",
            r#"{"synthesis": "test"}"#,
        );

        let manifest = create_manifest("SPEC-TEST-001", evidence_root).unwrap();

        assert_eq!(manifest.spec_id, "SPEC-TEST-001");
        assert_eq!(manifest.file_count(), 2);
        assert!(manifest.total_size > 0);
    }

    #[test]
    fn test_verify_nonexistent_archive() {
        let result =
            verify_archive_integrity(Path::new("/nonexistent/archive.tar.gz"), "abc123").unwrap();
        assert!(matches!(result, IntegrityResult::Corrupted { .. }));
    }
}
