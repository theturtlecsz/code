//! SPEC-KIT-964 Phase 6: Hermetic isolation validation for multi-agent spawning
//!
//! Validates that required project instruction files exist before spawning agents.
//! This ensures agents operate in a controlled, hermetic environment.

use std::path::Path;

/// Errors that can occur during isolation validation
#[derive(Debug, Clone)]
pub enum IsolationError {
    /// Required instruction file is missing
    MissingInstructionFile(String),
    /// Working directory doesn't exist
    InvalidWorkingDirectory,
}

impl std::fmt::Display for IsolationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsolationError::MissingInstructionFile(file) => {
                write!(
                    f,
                    "Missing required instruction file: {}. Run '/speckit.project' to scaffold.",
                    file
                )
            }
            IsolationError::InvalidWorkingDirectory => {
                write!(f, "Working directory does not exist")
            }
        }
    }
}

impl std::error::Error for IsolationError {}

/// Required instruction files for hermetic agent isolation
const REQUIRED_INSTRUCTION_FILES: &[&str] = &["CLAUDE.md", "AGENTS.md", "GEMINI.md"];

/// SPEC-KIT-964: Validate hermetic isolation before spawning agents
///
/// Checks that:
/// 1. Working directory exists
/// 2. All required instruction files (CLAUDE.md, AGENTS.md, GEMINI.md) exist
///
/// This validation ensures agents have consistent context and prevents
/// reliance on global configurations that could vary between environments.
///
/// # Arguments
/// * `cwd` - Working directory to validate
///
/// # Returns
/// * `Ok(())` - All isolation requirements satisfied
/// * `Err(IsolationError)` - Validation failed with specific reason
pub fn validate_agent_isolation(cwd: &Path) -> Result<(), IsolationError> {
    // 1. Check working directory exists
    if !cwd.exists() {
        tracing::error!(
            "SPEC-KIT-964: Working directory does not exist: {}",
            cwd.display()
        );
        return Err(IsolationError::InvalidWorkingDirectory);
    }

    // 2. Check all required instruction files exist
    for file in REQUIRED_INSTRUCTION_FILES {
        let file_path = cwd.join(file);
        if !file_path.exists() {
            tracing::warn!(
                "SPEC-KIT-964: Missing instruction file: {} (path: {})",
                file,
                file_path.display()
            );
            return Err(IsolationError::MissingInstructionFile(file.to_string()));
        }
    }

    tracing::debug!(
        "SPEC-KIT-964: Isolation validation passed for {}",
        cwd.display()
    );
    Ok(())
}

/// Check if isolation validation is enabled
///
/// Can be disabled via environment variable for development/testing:
/// `SPEC_KIT_SKIP_ISOLATION=1`
pub fn isolation_check_enabled() -> bool {
    std::env::var("SPEC_KIT_SKIP_ISOLATION")
        .map(|v| !matches!(v.as_str(), "1" | "true" | "yes"))
        .unwrap_or(true)
}

/// Validate isolation with optional skip (for environments without instruction files)
///
/// If `SPEC_KIT_SKIP_ISOLATION=1` is set, returns Ok without checking.
/// Otherwise, performs full validation.
pub fn validate_agent_isolation_with_skip(cwd: &Path) -> Result<(), IsolationError> {
    if !isolation_check_enabled() {
        tracing::info!("SPEC-KIT-964: Isolation check skipped (SPEC_KIT_SKIP_ISOLATION=1)");
        return Ok(());
    }
    validate_agent_isolation(cwd)
}

/// Get list of missing instruction files (for error reporting)
/// Useful helper for detailed error messages when isolation check fails
#[allow(dead_code)]
pub fn find_missing_instruction_files(cwd: &Path) -> Vec<String> {
    REQUIRED_INSTRUCTION_FILES
        .iter()
        .filter(|file| !cwd.join(file).exists())
        .map(|s| (*s).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validate_missing_directory() {
        let result = validate_agent_isolation(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(matches!(
            result,
            Err(IsolationError::InvalidWorkingDirectory)
        ));
    }

    #[test]
    fn test_validate_missing_instruction_files() {
        let temp = TempDir::new().unwrap();
        let result = validate_agent_isolation(temp.path());
        assert!(matches!(
            result,
            Err(IsolationError::MissingInstructionFile(_))
        ));
    }

    #[test]
    fn test_validate_partial_instruction_files() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::write(temp.path().join("AGENTS.md"), "# AGENTS.md").unwrap();
        // Missing GEMINI.md

        let result = validate_agent_isolation(temp.path());
        assert!(matches!(
            result,
            Err(IsolationError::MissingInstructionFile(f)) if f == "GEMINI.md"
        ));
    }

    #[test]
    fn test_validate_all_files_present() {
        let temp = TempDir::new().unwrap();
        for file in REQUIRED_INSTRUCTION_FILES {
            fs::write(temp.path().join(file), format!("# {}", file)).unwrap();
        }

        let result = validate_agent_isolation(temp.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_missing_files() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();

        let missing = find_missing_instruction_files(temp.path());
        assert_eq!(missing.len(), 2);
        assert!(missing.contains(&"AGENTS.md".to_string()));
        assert!(missing.contains(&"GEMINI.md".to_string()));
    }
}
