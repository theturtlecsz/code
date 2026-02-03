// FORK-SPECIFIC (just-every/code): Native SPEC-ID generation (SPECKIT-TASK-0001)
//!
//! Eliminates $2.40 consensus cost on every /speckit.new by generating SPEC-IDs natively.
//! Replaces Python scripts and multi-agent consensus with simple Rust logic.
//!
//! ## ID Formats
//! - **New**: `AREA-FEAT-####` (e.g., `CORE-FEAT-0001`, `TUI-FEAT-0042`)
//! - **Legacy**: `SPEC-KIT-###` (frozen, read-only support)
//!
//! ## Available Areas
//! Default areas: CORE, CLI, TUI, STAGE0, SPECKIT
//! Custom areas: Any string matching `^[A-Z][A-Z0-9]*$`

use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Default areas always available for feature creation
pub const DEFAULT_AREAS: &[&str] = &["CORE", "CLI", "TUI", "STAGE0", "SPECKIT"];

/// Validate that an area matches the required format: ^[A-Z][A-Z0-9]*$
///
/// # Examples
/// ```
/// assert!(validate_area("CORE").is_ok());
/// assert!(validate_area("TUI2").is_ok());
/// assert!(validate_area("core").is_err());  // lowercase
/// assert!(validate_area("1AREA").is_err()); // starts with digit
/// ```
pub fn validate_area(area: &str) -> Result<(), String> {
    if area.is_empty() {
        return Err("AREA cannot be empty".to_string());
    }

    let mut chars = area.chars();

    // First character must be uppercase letter
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => {}
        _ => {
            return Err(format!(
                "Invalid AREA '{}': must start with uppercase letter (A-Z)",
                area
            ));
        }
    }

    // Remaining characters must be uppercase letters or digits
    for c in chars {
        if !c.is_ascii_uppercase() && !c.is_ascii_digit() {
            return Err(format!(
                "Invalid AREA '{}': must match ^[A-Z][A-Z0-9]*$ (uppercase letters and digits only)",
                area
            ));
        }
    }

    Ok(())
}

/// Get list of available areas (default + discovered from existing directories)
///
/// Scans `docs/` for directories matching `<AREA>-FEAT-####-*` pattern and extracts
/// unique area prefixes. Merges with DEFAULT_AREAS and returns sorted list.
pub fn get_available_areas(cwd: &Path) -> Vec<String> {
    let mut areas: HashSet<String> = DEFAULT_AREAS.iter().map(|s| s.to_string()).collect();

    let docs_dir = cwd.join("docs");
    if let Ok(entries) = fs::read_dir(&docs_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Look for pattern: AREA-FEAT-####-*
                // AREA is uppercase letters/digits, FEAT is literal, #### is 4 digits
                if let Some(area) = extract_area_from_feature_dir(name) {
                    areas.insert(area);
                }
            }
        }
    }

    let mut sorted: Vec<String> = areas.into_iter().collect();
    sorted.sort();
    sorted
}

/// Extract AREA from a feature directory name like "CORE-FEAT-0001-some-slug"
fn extract_area_from_feature_dir(name: &str) -> Option<String> {
    // Split by '-' and check pattern: [AREA, "FEAT", digits, ...]
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() < 3 {
        return None;
    }

    let potential_area = parts[0];
    let feat_marker = parts[1];
    let num_part = parts[2];

    // Verify pattern
    if feat_marker != "FEAT" {
        return None;
    }
    if num_part.len() != 4 || !num_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if validate_area(potential_area).is_err() {
        return None;
    }

    Some(potential_area.to_string())
}

/// Generate the next feature ID for a given area
///
/// # Algorithm
/// 1. Validate area format
/// 2. Scan `docs/` for directories matching `{AREA}-FEAT-####-*`
/// 3. Extract numeric IDs, find max
/// 4. Return `{AREA}-FEAT-{max+1:04}`
///
/// # Examples
/// - If docs/ contains CORE-FEAT-0065, CORE-FEAT-0070
/// - `generate_next_feature_id(cwd, "CORE")` returns "CORE-FEAT-0071"
///
/// Thread-safety: Safe for concurrent calls (race condition only affects ID uniqueness,
/// which will be caught by mkdir failure and can retry)
pub fn generate_next_feature_id(cwd: &Path, area: &str) -> Result<String, String> {
    // Validate area format first
    validate_area(area)?;

    let docs_dir = cwd.join("docs");

    if !docs_dir.exists() {
        return Err(format!(
            "docs directory not found at {}. Are you in the project root?",
            docs_dir.display()
        ));
    }

    let entries =
        fs::read_dir(&docs_dir).map_err(|e| format!("Failed to read docs directory: {}", e))?;

    let prefix = format!("{}-FEAT-", area);

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
                // Strip "{AREA}-FEAT-" prefix
                .and_then(|s| s.strip_prefix(&prefix))
                // Take numeric part (first 4 chars or until dash)
                .and_then(|s| {
                    let num_str = s.split('-').next()?;
                    // Must be exactly 4 digits
                    if num_str.len() == 4 && num_str.chars().all(|c| c.is_ascii_digit()) {
                        num_str.parse::<u32>().ok()
                    } else {
                        None
                    }
                })
        })
        .max()
        .unwrap_or(0);

    let next_id = max_id + 1;
    Ok(format!("{}-FEAT-{:04}", area, next_id))
}

/// Generate full feature directory name from area and description
///
/// # Example
/// `generate_feature_directory_name(cwd, "CORE", "Add user auth")` → "CORE-FEAT-0001-add-user-auth"
pub fn generate_feature_directory_name(
    cwd: &Path,
    area: &str,
    description: &str,
) -> Result<String, String> {
    let feature_id = generate_next_feature_id(cwd, area)?;
    let slug = create_slug(description);

    if slug.is_empty() {
        return Err("Description must contain alphanumeric characters".to_string());
    }

    Ok(format!("{}-{}", feature_id, slug))
}

// === Legacy Support (SPEC-KIT-### format) ===

/// Generate the next SPEC-KIT ID (LEGACY - for backward compatibility only)
///
/// **Deprecated**: Use `generate_next_feature_id(cwd, area)` for new specs.
#[deprecated(
    since = "0.1.0",
    note = "Use generate_next_feature_id with area parameter"
)]
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
            if !path.is_dir() {
                return None;
            }

            path.file_name()
                .and_then(|name| name.to_str())
                .and_then(|s| s.strip_prefix("SPEC-KIT-"))
                .and_then(|s| s.split('-').next())
                .and_then(|num_str| num_str.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0);

    let next_id = max_id + 1;
    Ok(format!("SPEC-KIT-{:03}", next_id))
}

/// Generate full SPEC directory name from description (LEGACY)
///
/// **Deprecated**: Use `generate_feature_directory_name(cwd, area, description)` for new specs.
#[deprecated(
    since = "0.1.0",
    note = "Use generate_feature_directory_name with area parameter"
)]
pub fn generate_spec_directory_name(cwd: &Path, description: &str) -> Result<String, String> {
    #[allow(deprecated)]
    let spec_id = generate_next_spec_id(cwd)?;
    let slug = create_slug(description);

    if slug.is_empty() {
        return Err("Description must contain alphanumeric characters".to_string());
    }

    Ok(format!("{}-{}", spec_id, slug))
}

// === Shared Utilities ===

/// Maximum slug length to avoid filesystem limits (255 bytes minus prefix)
const MAX_SLUG_LENGTH: usize = 60;

/// Create a URL-safe slug from a description
///
/// Converts "Add user authentication with OAuth2" → "add-user-authentication-with-oauth2"
/// Truncates to MAX_SLUG_LENGTH characters to avoid filesystem limits.
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
    let full_slug = slug
        .split(|c: char| c == '-' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    // Truncate to max length, but don't cut in the middle of a word
    if full_slug.len() <= MAX_SLUG_LENGTH {
        full_slug
    } else {
        // Find last dash before max length to avoid cutting words
        let truncated = &full_slug[..MAX_SLUG_LENGTH];
        if let Some(last_dash) = truncated.rfind('-') {
            truncated[..last_dash].to_string()
        } else {
            truncated.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // === Area Validation Tests ===

    #[test]
    fn test_validate_area_valid() {
        assert!(validate_area("CORE").is_ok());
        assert!(validate_area("TUI").is_ok());
        assert!(validate_area("TUI2").is_ok());
        assert!(validate_area("SPECKIT").is_ok());
        assert!(validate_area("A").is_ok());
        assert!(validate_area("A1B2C3").is_ok());
    }

    #[test]
    fn test_validate_area_invalid_lowercase() {
        assert!(validate_area("core").is_err());
        assert!(validate_area("Core").is_err());
        assert!(validate_area("COre").is_err());
    }

    #[test]
    fn test_validate_area_invalid_starts_with_digit() {
        assert!(validate_area("1AREA").is_err());
        assert!(validate_area("123").is_err());
    }

    #[test]
    fn test_validate_area_invalid_special_chars() {
        assert!(validate_area("AREA-X").is_err());
        assert!(validate_area("AREA_X").is_err());
        assert!(validate_area("AREA X").is_err());
    }

    #[test]
    fn test_validate_area_empty() {
        assert!(validate_area("").is_err());
    }

    // === Available Areas Tests ===

    #[test]
    fn test_get_available_areas_empty_docs() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        let areas = get_available_areas(temp.path());

        // Should contain all default areas
        for default in DEFAULT_AREAS {
            assert!(areas.contains(&default.to_string()));
        }
    }

    #[test]
    fn test_get_available_areas_with_custom() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        // Create custom area directory
        fs::create_dir(docs.join("CUSTOM-FEAT-0001-test")).unwrap();
        fs::create_dir(docs.join("ANOTHER-FEAT-0042-slug")).unwrap();

        let areas = get_available_areas(temp.path());

        assert!(areas.contains(&"CUSTOM".to_string()));
        assert!(areas.contains(&"ANOTHER".to_string()));
        // Still has defaults
        assert!(areas.contains(&"CORE".to_string()));
    }

    #[test]
    fn test_get_available_areas_ignores_legacy() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        // Create legacy SPEC-KIT directory (should be ignored for area discovery)
        fs::create_dir(docs.join("SPEC-KIT-070-old-spec")).unwrap();

        let areas = get_available_areas(temp.path());

        // SPEC should not be added as an area (SPEC-KIT is legacy format)
        // Only defaults should be present
        assert!(!areas.contains(&"SPEC".to_string()));
    }

    // === Feature ID Generation Tests ===

    #[test]
    fn test_generate_next_feature_id_empty_docs() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        let result = generate_next_feature_id(temp.path(), "CORE").unwrap();
        assert_eq!(result, "CORE-FEAT-0001");
    }

    #[test]
    fn test_generate_next_feature_id_with_existing() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        // Create some existing features
        fs::create_dir(docs.join("CORE-FEAT-0065-test")).unwrap();
        fs::create_dir(docs.join("CORE-FEAT-0069-another")).unwrap();
        fs::create_dir(docs.join("CORE-FEAT-0070-last")).unwrap();

        let result = generate_next_feature_id(temp.path(), "CORE").unwrap();
        assert_eq!(result, "CORE-FEAT-0071");
    }

    #[test]
    fn test_generate_next_feature_id_non_sequential() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        // Create non-sequential IDs
        fs::create_dir(docs.join("CORE-FEAT-0010-old")).unwrap();
        fs::create_dir(docs.join("CORE-FEAT-0050-middle")).unwrap();
        fs::create_dir(docs.join("CORE-FEAT-0025-early")).unwrap();

        let result = generate_next_feature_id(temp.path(), "CORE").unwrap();
        assert_eq!(result, "CORE-FEAT-0051"); // max(10,50,25) + 1 = 51
    }

    #[test]
    fn test_generate_next_feature_id_different_areas() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        // Create features in different areas
        fs::create_dir(docs.join("CORE-FEAT-0010-core-thing")).unwrap();
        fs::create_dir(docs.join("TUI-FEAT-0020-tui-thing")).unwrap();

        // Each area should have independent numbering
        let core_result = generate_next_feature_id(temp.path(), "CORE").unwrap();
        let tui_result = generate_next_feature_id(temp.path(), "TUI").unwrap();

        assert_eq!(core_result, "CORE-FEAT-0011");
        assert_eq!(tui_result, "TUI-FEAT-0021");
    }

    #[test]
    fn test_generate_next_feature_id_invalid_area() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        let result = generate_next_feature_id(temp.path(), "core");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must start with uppercase"));
    }

    // === Feature Directory Name Tests ===

    #[test]
    fn test_generate_feature_directory_name() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();
        fs::create_dir(docs.join("CORE-FEAT-0070-last")).unwrap();

        let result =
            generate_feature_directory_name(temp.path(), "CORE", "Add user authentication")
                .unwrap();
        assert_eq!(result, "CORE-FEAT-0071-add-user-authentication");
    }

    #[test]
    fn test_generate_feature_directory_name_empty_description() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        let result = generate_feature_directory_name(temp.path(), "CORE", "!@#$%");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("alphanumeric"));
    }

    // === Slug Tests ===

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

    // === Legacy Tests (for backward compatibility) ===

    #[test]
    #[allow(deprecated)]
    fn test_generate_next_spec_id_legacy_empty_docs() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        let result = generate_next_spec_id(temp.path()).unwrap();
        assert_eq!(result, "SPEC-KIT-001");
    }

    #[test]
    #[allow(deprecated)]
    fn test_generate_next_spec_id_legacy_with_existing() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        fs::create_dir(docs.join("SPEC-KIT-065-test")).unwrap();
        fs::create_dir(docs.join("SPEC-KIT-069-another")).unwrap();
        fs::create_dir(docs.join("SPEC-KIT-070-last")).unwrap();

        let result = generate_next_spec_id(temp.path()).unwrap();
        assert_eq!(result, "SPEC-KIT-071");
    }

    #[test]
    #[allow(deprecated)]
    fn test_generate_spec_directory_name_legacy() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();
        fs::create_dir(docs.join("SPEC-KIT-070-last")).unwrap();

        let result = generate_spec_directory_name(temp.path(), "Add user authentication").unwrap();
        assert_eq!(result, "SPEC-KIT-071-add-user-authentication");
    }
}
