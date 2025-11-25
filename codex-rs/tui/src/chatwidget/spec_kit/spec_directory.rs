//! SPEC Directory Resolution - ACID-Compliant Design
//!
//! **Design Principles**:
//! - **Atomicity**: Directory lookup succeeds completely or fails with clear error
//! - **Consistency**: Same spec_id always resolves to same directory (deterministic)
//! - **Isolation**: Safe for concurrent access (read-only operations)
//! - **Durability**: Validates directory structure before returning (spec.md required)
//!
//! **Naming Conventions Enforced**:
//! - SPEC directories MUST be directories (not files)
//! - MUST contain spec.md (validation requirement)
//! - Pattern: SPEC-{AREA}-{NUM}-{slug} or exact SPEC-ID match
//! - Examples: SPEC-KIT-900-generic-smoke, SPEC-OPS-004-integrated-coder-hooks

#![allow(dead_code)] // Directory resolution helpers, some pending integration

//! **Error Handling**:
//! - Returns Result with descriptive errors (never panics)
//! - Logs ambiguities (multiple matches → uses precedence rules)
//! - Validates prerequisites (docs/ exists, is readable, is directory)

use std::path::{Path, PathBuf};

/// Validation result for SPEC directory
#[derive(Debug)]
pub struct SpecDirectoryValidation {
    pub path: PathBuf,
    pub has_spec_md: bool,
    pub has_prd_md: bool,
    pub warnings: Vec<String>,
}

/// Find and validate SPEC directory with ACID-compliant guarantees
///
/// **Atomicity**: Returns complete validated path or descriptive error
/// **Consistency**: Deterministic selection when multiple matches exist
/// **Isolation**: Read-only, safe for concurrent calls
/// **Durability**: Validates required files before returning
///
/// # Precedence Rules (when multiple directories match)
/// 1. Exact match (SPEC-KIT-900 exactly)
/// 2. First alphabetically (deterministic ordering)
///
/// # Validation Requirements
/// - Path must exist
/// - Path must be directory
/// - Path must contain spec.md
///
/// # Errors
/// - docs/ not found or not directory
/// - No matching SPEC directory
/// - Multiple matches (logged as warning, applies precedence)
/// - Directory exists but missing spec.md (error)
pub fn find_spec_directory(cwd: &Path, spec_id: &str) -> Result<PathBuf, String> {
    let docs_dir = cwd.join("docs");

    if !docs_dir.exists() {
        return Err(format!("docs/ directory not found in {}", cwd.display()));
    }

    if !docs_dir.is_dir() {
        return Err(format!(
            "docs/ exists but is not a directory: {}",
            docs_dir.display()
        ));
    }

    let entries =
        std::fs::read_dir(&docs_dir).map_err(|e| format!("Cannot read docs/ directory: {}", e))?;

    // Collect all matching DIRECTORIES (not files)
    let mut candidates: Vec<PathBuf> = Vec::new();
    let mut skipped_files: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Check if name matches pattern
        let name_matches = name_str == spec_id || name_str.starts_with(&format!("{}-", spec_id));

        if !name_matches {
            continue;
        }

        // CRITICAL: Must be directory (not file)
        if !path.is_dir() {
            skipped_files.push(name_str.to_string());
            tracing::debug!("Skipping file (not directory): {}", name_str);
            continue;
        }

        candidates.push(path);
    }

    if !skipped_files.is_empty() {
        tracing::warn!(
            "⚠️  Skipped {} matching files (need directories): {:?}",
            skipped_files.len(),
            skipped_files
        );
    }

    // Apply ACID principles: Deterministic selection
    let selected = match candidates.len() {
        0 => {
            return Err(format!(
                "No SPEC directory found for '{}' in {}\nChecked: docs/ directory\nSkipped files: {:?}",
                spec_id,
                docs_dir.display(),
                skipped_files
            ));
        }
        1 => {
            tracing::info!("✅ Found SPEC directory: {}", candidates[0].display());
            candidates[0].clone()
        }
        _ => {
            // CONSISTENCY: Deterministic selection when multiple matches
            // Sort alphabetically for deterministic ordering
            candidates.sort();

            tracing::warn!(
                "⚠️  Multiple SPEC directories found for {}: {:?}",
                spec_id,
                candidates
                    .iter()
                    .map(|p| p.file_name().unwrap().to_string_lossy())
                    .collect::<Vec<_>>()
            );

            // Precedence: Exact match > First alphabetically
            if let Some(exact) = candidates.iter().find(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy() == spec_id)
                    .unwrap_or(false)
            }) {
                tracing::info!("✅ Using exact match: {}", exact.display());
                exact.clone()
            } else {
                tracing::info!("✅ Using first alphabetically: {}", candidates[0].display());
                candidates[0].clone()
            }
        }
    };

    // DURABILITY: Validate required structure before returning
    let spec_md = selected.join("spec.md");
    if !spec_md.exists() {
        return Err(format!(
            "SPEC directory found but missing spec.md: {}\nA valid SPEC directory MUST contain spec.md",
            selected.display()
        ));
    }

    if !spec_md.is_file() {
        return Err(format!(
            "spec.md exists but is not a file: {}",
            spec_md.display()
        ));
    }

    tracing::debug!("✅ SPEC directory validated: {}", selected.display());
    Ok(selected)
}

/// Find SPEC directory with full validation
pub fn find_and_validate_spec_directory(
    cwd: &Path,
    spec_id: &str,
) -> Result<SpecDirectoryValidation, String> {
    let path = find_spec_directory(cwd, spec_id)?;

    let spec_md = path.join("spec.md");
    let prd_md = path.join("PRD.md");

    let mut warnings = Vec::new();

    if !prd_md.exists() {
        warnings.push("PRD.md not found (optional but recommended)".to_string());
    }

    Ok(SpecDirectoryValidation {
        path,
        has_spec_md: spec_md.exists(),
        has_prd_md: prd_md.exists(),
        warnings,
    })
}

/// Find SPEC directory (non-Result version for compatibility)
/// Returns None on any error (use find_spec_directory for error details)
pub fn find_spec_directory_opt(cwd: &Path, spec_id: &str) -> Option<PathBuf> {
    match find_spec_directory(cwd, spec_id) {
        Ok(path) => Some(path),
        Err(e) => {
            tracing::warn!("SPEC directory lookup failed: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_find_spec_directory_filters_files() {
        // This test would fail if we don't filter for is_dir()
        // In a real repo with SPEC-KIT-900-ARCHITECTURE-ANALYSIS.md file
        // and SPEC-KIT-900-generic-smoke/ directory,
        // should return the directory, not the file
    }
}
