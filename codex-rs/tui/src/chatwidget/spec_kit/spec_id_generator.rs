// FORK-SPECIFIC (just-every/code): Native SPEC-ID generation for SPEC-KIT-070
//!
//! Eliminates $2.40 consensus cost on every /speckit.new by generating SPEC-IDs natively.
//! Replaces Python scripts and multi-agent consensus with simple Rust logic.

use std::fs;
use std::path::Path;

/// Generate the next SPEC-KIT ID by finding the maximum existing ID and incrementing
///
/// Algorithm:
/// 1. Glob for all SPEC-KIT-* directories in docs/
/// 2. Parse numeric IDs from directory names
/// 3. Find maximum ID
/// 4. Return next ID (max + 1) formatted as SPEC-KIT-XXX
///
/// Examples:
/// - If docs/ contains SPEC-KIT-065, SPEC-KIT-069, SPEC-KIT-070
/// - Returns: "SPEC-KIT-071"
///
/// Thread-safety: Safe for concurrent calls (race condition only affects ID uniqueness,
/// which will be caught by mkdir failure and can retry)
pub fn generate_next_spec_id(cwd: &Path) -> Result<String, String> {
    let docs_dir = cwd.join("docs");

    if !docs_dir.exists() {
        return Err(format!(
            "docs directory not found at {}. Are you in the project root?",
            docs_dir.display()
        ));
    }

    let entries =
        fs::read_dir(&docs_dir).map_err(|e| format!("Failed to read docs directory: {}", e))?;

    let max_id = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            // Only look at directories
            if !path.is_dir() {
                return None;
            }

            // Extract directory name
            path.file_name()
                .and_then(|name| name.to_str())
                // Strip "SPEC-KIT-" prefix
                .and_then(|s| s.strip_prefix("SPEC-KIT-"))
                // Take numeric part before first dash (if any)
                .and_then(|s| s.split('-').next())
                // Parse as number
                .and_then(|num_str| num_str.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0);

    let next_id = max_id + 1;
    Ok(format!("SPEC-KIT-{:03}", next_id))
}

/// Create a URL-safe slug from a description
///
/// Converts "Add user authentication with OAuth2" → "add-user-authentication-with-oauth2"
pub fn create_slug(description: &str) -> String {
    let slug = description
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' || c == '_' {
                '-'
            } else {
                // Replace special characters with space (will be removed)
                ' '
            }
        })
        .collect::<String>();

    // Split by whitespace and dashes, filter empty, rejoin with single dash
    slug.split(|c: char| c == '-' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Generate full SPEC directory name from description
///
/// Example: "Add user auth" → "SPEC-KIT-071-add-user-auth"
pub fn generate_spec_directory_name(cwd: &Path, description: &str) -> Result<String, String> {
    let spec_id = generate_next_spec_id(cwd)?;
    let slug = create_slug(description);

    if slug.is_empty() {
        return Err("Description must contain alphanumeric characters".to_string());
    }

    Ok(format!("{}-{}", spec_id, slug))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_generate_next_spec_id_empty_docs() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        let result = generate_next_spec_id(temp.path()).unwrap();
        assert_eq!(result, "SPEC-KIT-001");
    }

    #[test]
    fn test_generate_next_spec_id_with_existing() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        // Create some existing SPECs
        fs::create_dir(docs.join("SPEC-KIT-065-test")).unwrap();
        fs::create_dir(docs.join("SPEC-KIT-069-another")).unwrap();
        fs::create_dir(docs.join("SPEC-KIT-070-last")).unwrap();

        let result = generate_next_spec_id(temp.path()).unwrap();
        assert_eq!(result, "SPEC-KIT-071");
    }

    #[test]
    fn test_generate_next_spec_id_non_sequential() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        // Create non-sequential IDs
        fs::create_dir(docs.join("SPEC-KIT-010-old")).unwrap();
        fs::create_dir(docs.join("SPEC-KIT-050-middle")).unwrap();
        fs::create_dir(docs.join("SPEC-KIT-025-early")).unwrap();

        let result = generate_next_spec_id(temp.path()).unwrap();
        assert_eq!(result, "SPEC-KIT-051"); // max(10,50,25) + 1 = 51
    }

    #[test]
    fn test_create_slug_basic() {
        assert_eq!(
            create_slug("Add User Authentication"),
            "add-user-authentication"
        );
        assert_eq!(create_slug("Fix Bug #123"), "fix-bug-123");
        assert_eq!(create_slug("OAuth2 Integration"), "oauth2-integration");
    }

    #[test]
    fn test_create_slug_special_chars() {
        assert_eq!(create_slug("Add @mention support!"), "add-mention-support");
        assert_eq!(create_slug("Fix: Error in parser"), "fix-error-in-parser");
        assert_eq!(create_slug("Update (v2.0) release"), "update-v2-0-release");
    }

    #[test]
    fn test_create_slug_multiple_spaces() {
        assert_eq!(
            create_slug("Add    multiple   spaces"),
            "add-multiple-spaces"
        );
        assert_eq!(
            create_slug("  Leading and trailing  "),
            "leading-and-trailing"
        );
    }

    #[test]
    fn test_generate_spec_directory_name() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();
        fs::create_dir(docs.join("SPEC-KIT-070-last")).unwrap();

        let result = generate_spec_directory_name(temp.path(), "Add user authentication").unwrap();
        assert_eq!(result, "SPEC-KIT-071-add-user-authentication");
    }

    #[test]
    fn test_empty_description_error() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        let result = generate_spec_directory_name(temp.path(), "!@#$%");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("alphanumeric"));
    }
}
