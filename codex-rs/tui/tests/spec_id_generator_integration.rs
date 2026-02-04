//! Integration test for native SPEC-ID generation
//! FORK-SPECIFIC (just-every/code): SPECKIT-TASK-0001 area-scoped feature IDs

use codex_tui::spec_id_generator::{
    DEFAULT_AREAS, create_slug, generate_feature_directory_name, generate_next_feature_id,
    get_available_areas, validate_area,
};
use std::path::PathBuf;

fn get_repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is codex-rs/tui, need to go up 2 levels
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent() // codex-rs/tui -> codex-rs
        .and_then(|p| p.parent()) // codex-rs -> repo root
        .unwrap_or_else(|| panic!("Failed to find repo root from {}", manifest_dir.display()))
        .to_path_buf()
}

#[test]
fn test_generate_next_feature_id_real_repo() {
    let repo_root = get_repo_root();

    // Generate ID for CORE area
    let next_id = generate_next_feature_id(&repo_root, "CORE").expect("Should generate ID");

    // Verify format: CORE-FEAT-####
    assert!(
        next_id.starts_with("CORE-FEAT-"),
        "Expected CORE-FEAT-#### format, got: {next_id}"
    );
    assert_eq!(
        next_id.len(),
        14,
        "Expected 14 chars (CORE-FEAT-####), got {} for '{}'",
        next_id.len(),
        next_id
    );

    // Verify numeric part is valid
    let num_part = next_id.strip_prefix("CORE-FEAT-").unwrap();
    let num: u32 = num_part.parse().expect("Should be valid number");
    assert!(num >= 1, "Next ID should be at least 0001, got {num}");

    println!("Generated next CORE feature ID: {next_id}");
}

#[test]
fn test_generate_feature_directory_name_real_repo() {
    let repo_root = get_repo_root();

    let dir_name = generate_feature_directory_name(&repo_root, "TUI", "Test native ID generation")
        .expect("Should generate directory name");

    // Should be TUI-FEAT-####-test-native-id-generation
    assert!(
        dir_name.starts_with("TUI-FEAT-"),
        "Expected TUI-FEAT-#### prefix, got: {dir_name}"
    );
    assert!(
        dir_name.contains("-test-native-id-generation"),
        "Expected slug in name, got: {dir_name}"
    );

    println!("Generated full feature directory name: {dir_name}");
}

#[test]
fn test_get_available_areas_real_repo() {
    let repo_root = get_repo_root();

    let areas = get_available_areas(&repo_root);

    // Should contain all default areas
    for default in DEFAULT_AREAS {
        assert!(
            areas.contains(&default.to_string()),
            "Missing default area: {default}"
        );
    }

    // Should be sorted
    let mut sorted = areas.clone();
    sorted.sort();
    assert_eq!(areas, sorted, "Areas should be sorted");

    println!("Available areas: {areas:?}");
}

#[test]
fn test_validate_area_examples() {
    // Valid areas
    assert!(validate_area("CORE").is_ok());
    assert!(validate_area("TUI").is_ok());
    assert!(validate_area("CLI").is_ok());
    assert!(validate_area("STAGE0").is_ok());
    assert!(validate_area("SPECKIT").is_ok());
    assert!(validate_area("CUSTOM123").is_ok());

    // Invalid areas
    assert!(validate_area("core").is_err(), "lowercase should fail");
    assert!(validate_area("1AREA").is_err(), "digit start should fail");
    assert!(validate_area("AREA-X").is_err(), "dash should fail");
    assert!(validate_area("").is_err(), "empty should fail");
}

#[test]
fn test_slug_creation_examples() {
    // Real-world examples
    assert_eq!(
        create_slug("Address speckit.validate multiple agent calls"),
        "address-speckit-validate-multiple-agent-calls"
    );
    assert_eq!(
        create_slug("Model cost optimization"),
        "model-cost-optimization"
    );
    assert_eq!(create_slug("Add search command"), "add-search-command");
    assert_eq!(
        create_slug("Area-scoped feature IDs"),
        "area-scoped-feature-ids"
    );
}

#[test]
fn test_missing_area_would_error() {
    let repo_root = get_repo_root();

    // Empty area should fail
    let result = generate_next_feature_id(&repo_root, "");
    assert!(result.is_err(), "Empty area should fail");

    // Invalid area format should fail
    let result = generate_next_feature_id(&repo_root, "lowercase");
    assert!(result.is_err(), "Lowercase area should fail");
}
