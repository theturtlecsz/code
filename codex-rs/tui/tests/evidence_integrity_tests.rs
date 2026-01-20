//! E.4 Evidence Integrity Tests
//!
//! ARB Pass 2 capability test suite for evidence integrity verification (SHA256).
//! Reference: docs/spec-kit/evidence-policy.md §9.1-9.2
//!
//! Tests:
//! - E4.1: SHA256 checksum calculation consistency
//! - E4.2: Archive includes manifest with checksums
//! - E4.3: Valid archive passes verification
//! - E4.4: Corrupted archive detected
//! - E4.5: Restore rejects checksum mismatch
//! - E4.6: Restore validates expected file count

// SPEC-957: Allow test code flexibility
#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui::evidence_integrity::{
    ArchiveManifest, IntegrityResult, compute_sha256, compute_sha256_bytes, compute_sha256_str,
    create_archive_with_checksum, create_manifest, extract_manifest, restore_archive,
    verify_archive_contents, verify_archive_integrity,
};
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create evidence structure for a SPEC.
fn create_test_evidence(evidence_root: &std::path::Path, spec_id: &str) {
    let commands_dir = evidence_root.join("commands").join(spec_id);
    let consensus_dir = evidence_root.join("consensus").join(spec_id);

    fs::create_dir_all(&commands_dir).unwrap();
    fs::create_dir_all(&consensus_dir).unwrap();

    fs::write(
        commands_dir.join("telemetry.json"),
        r#"{"stage": "plan", "timestamp": "2025-01-01T12:00:00Z", "spec_id": "SPEC-TEST"}"#,
    )
    .unwrap();

    fs::write(
        consensus_dir.join("synthesis.json"),
        r#"{"consensus": true, "agents": ["claude-3-opus", "claude-3-sonnet"], "verdict": "OK"}"#,
    )
    .unwrap();
}

/// Create a valid archive for testing.
fn create_valid_archive(
    temp_dir: &TempDir,
    spec_id: &str,
) -> (std::path::PathBuf, ArchiveManifest) {
    let evidence_root = temp_dir.path();
    create_test_evidence(evidence_root, spec_id);

    let archive_path = temp_dir.path().join(format!("{}.tar.gz", spec_id));
    let manifest = create_archive_with_checksum(spec_id, evidence_root, &archive_path).unwrap();

    (archive_path, manifest)
}

// ============================================================================
// E4.1: SHA256 checksum consistency
// ============================================================================

/// E4.1: Same content produces same checksum.
#[test]
fn test_sha256_checksum_calculation() {
    let content = "deterministic test content for hashing";

    let checksum1 = compute_sha256_str(content);
    let checksum2 = compute_sha256_str(content);

    assert_eq!(
        checksum1, checksum2,
        "Same content must produce same checksum"
    );
    assert_eq!(checksum1.len(), 64, "SHA256 produces 64 hex characters");
}

/// E4.1b: Known SHA256 value verification.
#[test]
fn test_sha256_known_value() {
    // "hello world" SHA256 = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
    let checksum = compute_sha256_str("hello world");
    assert_eq!(
        checksum,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

/// E4.1c: Different content produces different checksum.
#[test]
fn test_sha256_different_content_different_hash() {
    let checksum_a = compute_sha256_str("content version A");
    let checksum_b = compute_sha256_str("content version B");

    assert_ne!(
        checksum_a, checksum_b,
        "Different content must produce different checksums"
    );
}

/// E4.1d: File-based checksum matches string-based checksum.
#[test]
fn test_sha256_file_matches_string() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let content = "file content for checksum test";

    fs::write(&file_path, content).unwrap();

    let file_checksum = compute_sha256(&file_path).unwrap();
    let string_checksum = compute_sha256_str(content);

    assert_eq!(file_checksum, string_checksum);
}

/// E4.1e: Empty content has predictable checksum.
#[test]
fn test_sha256_empty_content() {
    // Empty string SHA256 = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    let checksum = compute_sha256_str("");
    assert_eq!(
        checksum,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

// ============================================================================
// E4.2: Archive includes manifest with checksums
// ============================================================================

/// E4.2: Archive contains manifest.json with file checksums.
#[test]
fn test_archive_includes_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let (archive_path, manifest) = create_valid_archive(&temp_dir, "SPEC-MANIFEST-TEST");

    // Verify manifest has correct structure
    assert_eq!(manifest.spec_id, "SPEC-MANIFEST-TEST");
    assert!(!manifest.archive_checksum.is_empty());
    assert!(manifest.file_count() >= 2); // telemetry + synthesis
    assert!(manifest.total_size > 0);
    assert_eq!(manifest.version, 1);

    // Verify each file entry has checksum
    for file in &manifest.files {
        assert!(
            !file.sha256.is_empty(),
            "File {} missing checksum",
            file.path
        );
        assert_eq!(
            file.sha256.len(),
            64,
            "File {} checksum wrong length",
            file.path
        );
        assert!(file.size > 0, "File {} has zero size", file.path);
    }

    // Verify manifest can be extracted from archive
    let extracted = extract_manifest(&archive_path).unwrap();
    assert_eq!(extracted.spec_id, manifest.spec_id);
    assert_eq!(extracted.file_count(), manifest.file_count());
}

/// E4.2b: Manifest file entries have correct paths.
#[test]
fn test_manifest_file_paths() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    create_test_evidence(evidence_root, "SPEC-PATHS-TEST");

    let manifest = create_manifest("SPEC-PATHS-TEST", evidence_root).unwrap();

    // Should have entries for commands/ and consensus/ files
    let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();

    assert!(
        paths.iter().any(|p| p.contains("commands")),
        "Should include commands/ files"
    );
    assert!(
        paths.iter().any(|p| p.contains("consensus")),
        "Should include consensus/ files"
    );
}

// ============================================================================
// E4.3: Valid archive verification
// ============================================================================

/// E4.3: Valid archive passes integrity verification.
#[test]
fn test_verify_valid_archive_succeeds() {
    let temp_dir = TempDir::new().unwrap();
    let (archive_path, manifest) = create_valid_archive(&temp_dir, "SPEC-VALID-TEST");

    // Verify archive checksum
    let result = verify_archive_integrity(&archive_path, &manifest.archive_checksum).unwrap();
    assert!(result.is_valid(), "Valid archive should pass: {:?}", result);

    // Verify all file contents
    let contents_result = verify_archive_contents(&archive_path).unwrap();
    assert!(
        contents_result.is_valid(),
        "Valid archive contents should pass: {:?}",
        contents_result
    );
}

/// E4.3b: Verification returns Valid for matching checksum.
#[test]
fn test_integrity_result_valid() {
    let temp_dir = TempDir::new().unwrap();
    let (archive_path, manifest) = create_valid_archive(&temp_dir, "SPEC-RESULT-TEST");

    let result = verify_archive_integrity(&archive_path, &manifest.archive_checksum).unwrap();

    assert_eq!(result, IntegrityResult::Valid);
    assert!(result.is_valid());
}

// ============================================================================
// E4.4: Corrupted archive detection
// ============================================================================

/// E4.4: Corrupted archive is detected via checksum mismatch.
#[test]
fn test_verify_corrupted_archive_fails() {
    let temp_dir = TempDir::new().unwrap();
    let (archive_path, manifest) = create_valid_archive(&temp_dir, "SPEC-CORRUPT-TEST");

    // Corrupt the archive by appending garbage
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&archive_path)
        .unwrap();
    file.write_all(b"CORRUPTION DATA").unwrap();

    // Archive checksum should now mismatch
    let result = verify_archive_integrity(&archive_path, &manifest.archive_checksum).unwrap();

    assert!(
        matches!(result, IntegrityResult::ChecksumMismatch { .. }),
        "Corrupted archive should fail checksum: {:?}",
        result
    );
}

/// E4.4b: Nonexistent archive returns corrupted status.
#[test]
fn test_verify_nonexistent_archive() {
    let result = verify_archive_integrity(
        std::path::Path::new("/nonexistent/archive.tar.gz"),
        "abc123",
    )
    .unwrap();

    assert!(
        matches!(result, IntegrityResult::Corrupted { .. }),
        "Nonexistent archive should return Corrupted: {:?}",
        result
    );
}

/// E4.4c: Wrong checksum is detected.
#[test]
fn test_verify_wrong_checksum() {
    let temp_dir = TempDir::new().unwrap();
    let (archive_path, _) = create_valid_archive(&temp_dir, "SPEC-WRONG-CHECKSUM");

    // Use incorrect checksum
    let wrong_checksum = "0000000000000000000000000000000000000000000000000000000000000000";
    let result = verify_archive_integrity(&archive_path, wrong_checksum).unwrap();

    match result {
        IntegrityResult::ChecksumMismatch { expected, actual } => {
            assert_eq!(expected, wrong_checksum);
            assert_ne!(actual, wrong_checksum);
        }
        _ => panic!("Expected ChecksumMismatch, got {:?}", result),
    }
}

// ============================================================================
// E4.5: Restore rejects checksum mismatch
// ============================================================================

/// E4.5: Restore with verification rejects corrupted archive.
///
/// Note: Appending garbage to a tar.gz file doesn't corrupt the content -
/// tar/gzip ignore trailing data. Instead, we corrupt the actual content
/// by overwriting bytes in the middle of the file.
#[test]
fn test_restore_rejects_checksum_mismatch() {
    let temp_dir = TempDir::new().unwrap();
    let (archive_path, _) = create_valid_archive(&temp_dir, "SPEC-REJECT-TEST");

    // Corrupt the archive content by overwriting bytes in the middle
    // This will corrupt either the gzip header or the tar content
    let original_content = fs::read(&archive_path).unwrap();
    let mut corrupted = original_content.clone();
    if corrupted.len() > 100 {
        // Overwrite some bytes in the middle (affects gzip/tar parsing)
        corrupted[50] = 0xFF;
        corrupted[51] = 0xFF;
        corrupted[52] = 0xFF;
    }
    fs::write(&archive_path, corrupted).unwrap();

    // Attempt restore with verification enabled
    let restore_dir = temp_dir.path().join("restored");

    // The corrupted archive should either:
    // 1. Fail to parse (IO error) - which we need to handle
    // 2. Parse but fail verification (IntegrityResult invalid)
    match restore_archive(&archive_path, &restore_dir, true) {
        Ok(result) => {
            // If it somehow succeeds, check that integrity is invalid
            assert!(!result.success, "Restore should fail for corrupted archive");
        }
        Err(_) => {
            // IO error is acceptable for corrupted archive
            // This means the archive is unreadable which is also a valid failure mode
        }
    }
}

/// E4.5b: Restore without verification succeeds even for corrupted archive.
#[test]
fn test_restore_without_verification() {
    let temp_dir = TempDir::new().unwrap();
    let (archive_path, _) = create_valid_archive(&temp_dir, "SPEC-NO-VERIFY");

    // Restore without verification
    let restore_dir = temp_dir.path().join("restored");
    let result = restore_archive(&archive_path, &restore_dir, false).unwrap();

    // Should succeed without verification
    assert!(result.success);
    assert!(
        result.integrity.is_none(),
        "No integrity check when verify=false"
    );
}

/// E4.5c: Valid archive restores successfully with verification.
#[test]
fn test_restore_valid_archive_with_verification() {
    let temp_dir = TempDir::new().unwrap();
    let (archive_path, _) = create_valid_archive(&temp_dir, "SPEC-VALID-RESTORE");

    let restore_dir = temp_dir.path().join("restored");
    let result = restore_archive(&archive_path, &restore_dir, true).unwrap();

    assert!(result.success);
    assert!(result.files_restored >= 2);
    assert!(result.bytes_restored > 0);
    assert!(result.integrity.unwrap().is_valid());
}

// ============================================================================
// E4.6: File count validation
// ============================================================================

/// E4.6: Restore validates expected file count.
#[test]
fn test_restore_validates_file_count() {
    let temp_dir = TempDir::new().unwrap();
    let (archive_path, manifest) = create_valid_archive(&temp_dir, "SPEC-COUNT-TEST");

    // Verify the archive contents (includes file count check)
    let result = verify_archive_contents(&archive_path).unwrap();
    assert!(result.is_valid());

    // Restore and verify file count matches
    let restore_dir = temp_dir.path().join("restored");
    let restore_result = restore_archive(&archive_path, &restore_dir, true).unwrap();

    assert!(restore_result.success);
    assert_eq!(
        restore_result.files_restored,
        manifest.file_count(),
        "Restored file count should match manifest"
    );
}

/// E4.6b: File count mismatch detection.
#[test]
fn test_file_count_mismatch_detection() {
    // This tests that IntegrityResult::FileCountMismatch exists and works
    let result = IntegrityResult::FileCountMismatch {
        expected: 5,
        actual: 3,
    };

    assert!(!result.is_valid());
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Archive for SPEC with no files should handle gracefully.
#[test]
fn test_manifest_empty_spec() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    // Create directories but no files
    fs::create_dir_all(evidence_root.join("commands").join("SPEC-EMPTY")).unwrap();
    fs::create_dir_all(evidence_root.join("consensus").join("SPEC-EMPTY")).unwrap();

    let manifest = create_manifest("SPEC-EMPTY", evidence_root).unwrap();

    assert_eq!(manifest.spec_id, "SPEC-EMPTY");
    assert_eq!(manifest.file_count(), 0);
    assert_eq!(manifest.total_size, 0);
}

/// Large file checksum works correctly.
#[test]
fn test_large_content_checksum() {
    // Create 1MB of content
    let content: String = "x".repeat(1_000_000);
    let checksum = compute_sha256_str(&content);

    assert_eq!(checksum.len(), 64);

    // Verify consistency
    let checksum2 = compute_sha256_str(&content);
    assert_eq!(checksum, checksum2);
}

/// Binary content checksum works correctly.
#[test]
fn test_binary_content_checksum() {
    let binary_data: Vec<u8> = (0..=255).collect();
    let checksum = compute_sha256_bytes(&binary_data);

    assert_eq!(checksum.len(), 64);

    // Verify consistency
    let checksum2 = compute_sha256_bytes(&binary_data);
    assert_eq!(checksum, checksum2);
}

/// Archive with nested directories handles paths correctly.
#[test]
fn test_nested_directory_paths() {
    let temp_dir = TempDir::new().unwrap();
    let evidence_root = temp_dir.path();

    // Create nested structure
    let nested_dir = evidence_root
        .join("commands")
        .join("SPEC-NESTED")
        .join("stage-plan")
        .join("retries");
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(nested_dir.join("attempt-1.json"), r#"{"attempt": 1}"#).unwrap();
    fs::write(nested_dir.join("attempt-2.json"), r#"{"attempt": 2}"#).unwrap();

    let manifest = create_manifest("SPEC-NESTED", evidence_root).unwrap();

    assert_eq!(manifest.file_count(), 2);

    // Paths should be relative
    for file in &manifest.files {
        assert!(
            file.path.starts_with("commands/SPEC-NESTED"),
            "Path should be relative: {}",
            file.path
        );
    }
}

/// Special characters in content don't break checksum.
#[test]
fn test_special_characters_checksum() {
    let content = "Special chars: \n\t\r\0 unicode: 你好 emoji: 🎉";
    let checksum = compute_sha256_str(content);

    assert_eq!(checksum.len(), 64);

    // Verify it's deterministic
    let checksum2 = compute_sha256_str(content);
    assert_eq!(checksum, checksum2);
}
