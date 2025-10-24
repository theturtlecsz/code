//! Integration test for native SPEC-ID generation
//! FORK-SPECIFIC (just-every/code): SPEC-KIT-070 cost optimization

use codex_tui::spec_id_generator::{
    create_slug, generate_next_spec_id, generate_spec_directory_name,
};
use std::path::PathBuf;

#[test]
fn test_generate_next_spec_id_real_repo() {
    // This test runs against the actual repository structure
    // Expected: SPEC-KIT-070 exists, so next should be SPEC-KIT-071
    // CARGO_MANIFEST_DIR is codex-rs/tui, need to go up 2 levels
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // codex-rs/tui -> codex-rs
        .expect("Expected parent")
        .parent() // codex-rs -> repo root
        .expect("Expected repo root")
        .to_path_buf();

    let next_id = generate_next_spec_id(&repo_root).expect("Should generate ID");

    // Verify format
    assert!(next_id.starts_with("SPEC-KIT-"));
    assert_eq!(next_id.len(), 12); // SPEC-KIT-XXX

    // Verify it's >= 071 (since we know 070 exists)
    let num_part = next_id.strip_prefix("SPEC-KIT-").unwrap();
    let num: u32 = num_part.parse().expect("Should be valid number");
    assert!(num >= 71, "Next ID should be at least 071, got {}", num);

    println!("✅ Generated next SPEC-ID: {}", next_id);
}

#[test]
fn test_create_full_spec_name_real_repo() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Expected parent")
        .parent()
        .expect("Expected repo root")
        .to_path_buf();

    let spec_name =
        generate_spec_directory_name(&repo_root, "Test native ID generation").unwrap();

    // Should be SPEC-KIT-071-test-native-id-generation (or higher)
    assert!(spec_name.starts_with("SPEC-KIT-"));
    assert!(spec_name.contains("-test-native-id-generation"));

    println!("✅ Generated full SPEC name: {}", spec_name);
}

#[test]
fn test_slug_creation_examples() {
    // Real-world examples from existing SPECs
    assert_eq!(
        create_slug("Address speckit.validate multiple agent calls"),
        "address-speckit-validate-multiple-agent-calls"
    );
    assert_eq!(
        create_slug("Model cost optimization"),
        "model-cost-optimization"
    );
    assert_eq!(
        create_slug("Add search command"),
        "add-search-command"
    );
}
